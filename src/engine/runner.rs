use tokio::sync::broadcast;
use tracing::{info, instrument};

use crate::errors::AppResult;
use crate::marketdata::feeds::FeedCoordinator;
use crate::strategies::{Strategy, StrategyContext};

pub struct Engine {
    feed: FeedCoordinator,
    strategy: Box<dyn Strategy>,
    ctx: StrategyContext,
}

impl Engine {
    pub fn new(feed: FeedCoordinator, strategy: Box<dyn Strategy>, ctx: StrategyContext) -> Self {
        Self {
            feed,
            strategy,
            ctx,
        }
    }

    #[instrument(skip_all)]
    pub async fn run(mut self) -> AppResult<()> {
        let mut stream = self.feed.subscribe();
        info!("engine started");
        loop {
            let event = match stream.recv().await {
                Ok(event) => event,
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "market event lagged");
                    continue;
                }
            };
            let resp = self.strategy.on_event(&mut self.ctx, event).await?;
            for intent in resp.intents {
                self.ctx.submit_intent(intent).await;
            }
        }
        Ok(())
    }
}
