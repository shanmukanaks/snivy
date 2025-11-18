use std::sync::Arc;

use tokio::sync::{broadcast, mpsc::UnboundedReceiver};
use tracing::{info, instrument};

use crate::errors::AppResult;
use crate::exchange::{FillEvent, PositionManager};
use crate::marketdata::feeds::FeedCoordinator;
use crate::strategies::{Strategy, StrategyContext};

pub struct Engine {
    feed: FeedCoordinator,
    fills: Option<UnboundedReceiver<FillEvent>>,
    strategy: Box<dyn Strategy>,
    ctx: StrategyContext,
    positions: Arc<PositionManager>,
}

impl Engine {
    pub fn new(
        feed: FeedCoordinator,
        fills: Option<UnboundedReceiver<FillEvent>>,
        strategy: Box<dyn Strategy>,
        ctx: StrategyContext,
        positions: Arc<PositionManager>,
    ) -> Self {
        Self {
            feed,
            fills,
            strategy,
            ctx,
            positions,
        }
    }

    #[instrument(skip_all)]
    pub async fn run(mut self) -> AppResult<()> {
        let mut market_stream = self.feed.subscribe();
        info!("engine started");

        loop {
            if let Some(fill_rx) = self.fills.as_mut() {
                tokio::select! {
                    evt = market_stream.recv() => {
                        if !self.handle_market_event(evt).await? {
                            break;
                        }
                    }
                    fill = fill_rx.recv() => {
                        if !self.handle_fill(fill).await? {
                            self.fills = None;
                        }
                    }
                }
            } else {
                let evt = market_stream.recv().await;
                if !self.handle_market_event(evt).await? {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_market_event(
        &mut self,
        event: Result<crate::marketdata::events::MarketEvent, broadcast::error::RecvError>,
    ) -> AppResult<bool> {
        match event {
            Ok(event) => {
                let resp = self.strategy.on_event(&mut self.ctx, event).await?;
                for intent in resp.intents {
                    self.ctx.submit_intent(intent).await?;
                }
                Ok(true)
            }
            Err(broadcast::error::RecvError::Closed) => Ok(false),
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "market event lagged");
                Ok(true)
            }
        }
    }

    async fn handle_fill(&mut self, fill: Option<FillEvent>) -> AppResult<bool> {
        match fill {
            Some(fill) => {
                self.positions.apply_fill(&fill);
                let resp = self.strategy.on_fill(&mut self.ctx, fill.clone()).await?;
                for intent in resp.intents {
                    self.ctx.submit_intent(intent).await?;
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
