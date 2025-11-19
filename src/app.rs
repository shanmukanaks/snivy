use std::sync::Arc;

use hyperliquid_rust_sdk::BaseUrl;

use crate::config::Settings;
use crate::engine::runner::Engine;
use crate::errors::{AppError, AppResult};
use crate::exchange::{InfoService, MarketStream, OrderRouter, PositionManager};
use crate::marketdata::feeds::FeedCoordinator;
use crate::storage::journal::Journal;
use crate::strategies::{StrategyContext, build_strategy, register_builtin_strategies};

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

        let _info = InfoService::connect(base_url).await?;

        let strategy_asset = strategy_cfg
            .params
            .get("asset")
            .and_then(|v| v.as_str())
            .unwrap_or("BTC")
            .to_string();

        let market_stream = MarketStream::new(1024);
        market_stream.spawn_mock_feed(strategy_asset)?;

        let feed = FeedCoordinator::new(market_stream.clone());
        let positions = Arc::new(PositionManager::new());
        let order_router = Arc::new(OrderRouter::default());
        let journal = Arc::new(
            Journal::new(&self.settings.persistence.journal_path)
                .map_err(|e| AppError::Other(e.to_string()))?,
        );
        let strategy = build_strategy(&strategy_cfg.id, strategy_cfg.params.clone())?;

        let ctx = StrategyContext::new(order_router.clone(), positions.clone(), journal.clone());
        let engine = Engine::new(feed.clone(), strategy, ctx);
        engine.run().await?;
        Ok(())
    }
}
