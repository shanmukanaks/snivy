use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::instrument;

use crate::exchange::MarketStream;
use crate::marketdata::events::MarketEvent;

#[derive(Clone)]
pub struct FeedCoordinator {
    inner: Arc<MarketStream>,
}

impl FeedCoordinator {
    pub fn new(stream: MarketStream) -> Self {
        Self {
            inner: Arc::new(stream),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MarketEvent> {
        self.inner.subscribe()
    }

    #[instrument(skip_all)]
    pub async fn forward_to_strategy<F>(&self, mut handler: F)
    where
        F: FnMut(MarketEvent) + Send + 'static,
    {
        let mut stream = BroadcastStream::new(self.inner.subscribe());
        while let Some(Ok(event)) = stream.next().await {
            handler(event);
        }
    }

    pub async fn wait_ready(&self) {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
