use std::sync::Arc;

use tracing::Span;

use crate::errors::AppResult;
use crate::exchange::{OrderIntent, OrderRouter, PositionManager};
use crate::storage::journal::Journal;

#[derive(Clone)]
pub struct StrategyContext {
    order_router: Arc<OrderRouter>,
    positions: Arc<PositionManager>,
    journal: Arc<Journal>,
    span: Span,
}

impl StrategyContext {
    pub fn new(
        order_router: Arc<OrderRouter>,
        positions: Arc<PositionManager>,
        journal: Arc<Journal>,
    ) -> Self {
        Self {
            order_router,
            positions,
            journal,
            span: tracing::info_span!("strategy"),
        }
    }

    pub fn positions(&self) -> Vec<crate::exchange::position_manager::Position> {
        self.positions.snapshot()
    }

    pub fn journal(&self) -> Arc<Journal> {
        self.journal.clone()
    }

    pub fn span(&self) -> Span {
        self.span.clone()
    }

    pub fn positions_handle(&self) -> Arc<PositionManager> {
        self.positions.clone()
    }

    pub fn order_router(&self) -> Arc<OrderRouter> {
        self.order_router.clone()
    }

    pub async fn submit_intent(&self, intent: OrderIntent) -> AppResult<()> {
        self.order_router.submit(intent.clone()).await.map(|_| ())
    }
}
