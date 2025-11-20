mod context;
pub mod ma_crossover;
pub mod registry;

pub use context::StrategyContext;
pub use registry::{build as build_strategy, register_builtin_strategies};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::errors::AppResult;
use crate::exchange::{FillEvent, InfoService, OrderIntent};
use crate::marketdata::events::MarketEvent;
use crate::storage::persistence::SnapshotStore;
use hyperliquid_rust_sdk::BaseUrl;

#[derive(Debug, Clone)]
pub enum StrategyAction {
    None,
    Alert(String),
}

#[derive(Debug, Clone, Default)]
pub struct StrategyResponse {
    pub intents: Vec<OrderIntent>,
    pub actions: Vec<StrategyAction>,
}

impl StrategyResponse {
    pub fn idle() -> Self {
        Self::default()
    }

    pub fn with_intent(intent: OrderIntent) -> Self {
        Self {
            intents: vec![intent],
            actions: vec![],
        }
    }
}

#[derive(Clone)]
pub struct StrategyBuilderContext {
    pub base_url: BaseUrl,
    pub info: InfoService,
    pub snapshot_store: SnapshotStore,
}

#[async_trait]
pub trait Strategy: Send + Sync {
    fn id(&self) -> &'static str;

    async fn on_event(
        &mut self,
        ctx: &mut StrategyContext,
        event: MarketEvent,
    ) -> AppResult<StrategyResponse>;

    async fn on_interval(
        &mut self,
        _ctx: &mut StrategyContext,
        _timestamp: DateTime<Utc>,
    ) -> AppResult<StrategyResponse> {
        Ok(StrategyResponse::idle())
    }

    async fn on_fill(
        &mut self,
        _ctx: &mut StrategyContext,
        _fill: FillEvent,
    ) -> AppResult<StrategyResponse> {
        Ok(StrategyResponse::idle())
    }

    async fn shutdown(&mut self, _ctx: &mut StrategyContext) -> AppResult<()> {
        Ok(())
    }

    fn snapshot_state(&self) -> Value;
    fn restore_state(&mut self, state: Value);
}
