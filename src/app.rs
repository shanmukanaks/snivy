use std::sync::Arc;

use hyperliquid_rust_sdk::BaseUrl;

use crate::config::{Settings, StrategyInstanceConfig};
use crate::engine::runner::Engine;
use crate::errors::{AppError, AppResult};
use crate::exchange::{self, InfoService, MarketStream, OrderRouter, PositionManager};
use crate::marketdata::feeds::FeedCoordinator;
use crate::storage::journal::Journal;
use crate::storage::persistence::SnapshotStore;
use crate::strategies::{
    StrategyBuilderContext, StrategyContext, build_strategy, register_builtin_strategies,
};
use crate::utils::secrets::read_env;

pub struct App {
    settings: Settings,
}

impl App {
    pub fn new(settings: Settings) -> Self {
        register_builtin_strategies();
        Self { settings }
    }

    pub async fn run(self) -> AppResult<()> {
        let strategy_cfg = self.settings.ensure_strategy()?.clone();
        let base_url = match self.settings.exchange.network.to_lowercase().as_str() {
            "testnet" => BaseUrl::Testnet,
            "local" => BaseUrl::Localhost,
            _ => BaseUrl::Mainnet,
        };

        let info = InfoService::connect(base_url).await?;
        let snapshot_store = SnapshotStore::new(&self.settings.persistence.snapshot_path);
        let signer_key = self.resolve_signer_key()?;
        let order_router = Arc::new(OrderRouter::new(base_url, &signer_key).await?);
        let wallet_address = order_router.wallet_address();
        let asset = extract_param_string(&strategy_cfg, "asset", "BTC");
        let candle_interval = extract_param_string(&strategy_cfg, "candle_interval", "1m");

        let market_stream =
            MarketStream::connect_candles(info.clone(), asset.clone(), candle_interval, 1024)
                .await?;

        let feed = FeedCoordinator::new(market_stream);
        let positions = Arc::new(PositionManager::new());
        let journal = Arc::new(
            Journal::new(&self.settings.persistence.journal_path)
                .map_err(|e| AppError::Other(e.to_string()))?,
        );

        let builder_ctx = StrategyBuilderContext {
            base_url,
            info: info.clone(),
            snapshot_store: snapshot_store.clone(),
        };

        let strategy = build_strategy(&strategy_cfg.id, strategy_cfg.params.clone(), builder_ctx)?;

        let ctx = StrategyContext::new(order_router.clone(), positions.clone(), journal.clone());
        let fill_rx = match exchange::user_fills_stream(info.clone(), wallet_address).await {
            Ok(rx) => Some(rx),
            Err(e) => {
                tracing::warn!(error = %e, "unable to subscribe to user fills");
                None
            }
        };

        let engine = Engine::new(feed, fill_rx, strategy, ctx, positions.clone());
        engine.run().await?;
        Ok(())
    }

    fn resolve_signer_key(&self) -> AppResult<String> {
        if let Some(env_key) = &self.settings.exchange.signer_private_key_env {
            if let Some(value) = read_env(env_key) {
                return Ok(value);
            } else {
                return Err(AppError::Config(format!(
                    "environment variable {env_key} not set"
                )));
            }
        }

        self.settings
            .exchange
            .signer_private_key
            .clone()
            .ok_or_else(|| {
                AppError::Config("exchange.signer_private_key[_env] must be provided".into())
            })
    }
}

fn extract_param_string(cfg: &StrategyInstanceConfig, key: &str, default: &str) -> String {
    cfg.params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default.to_string())
}
