use std::sync::Arc;

use tokio::sync::{Mutex, broadcast};
use tracing::{info, instrument};

use crate::errors::AppResult;
use crate::marketdata::events::{MarketEvent, TradeEvent};
use crate::utils::time;

pub type MarketStreamHandle = broadcast::Receiver<MarketEvent>;

#[derive(Clone)]
pub struct MarketStream {
    tx: broadcast::Sender<MarketEvent>,
    _task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl MarketStream {
    pub fn new(buffer: usize) -> Self {
        let (tx, _) = broadcast::channel(buffer);
        Self {
            tx,
            _task: Arc::new(Mutex::new(None)),
        }
    }

    pub fn subscribe(&self) -> MarketStreamHandle {
        self.tx.subscribe()
    }

    #[instrument(skip(self))]
    pub fn spawn_mock_feed(&self, asset: String) -> AppResult<()> {
        let tx = self.tx.clone();
        let handle = tokio::spawn(async move {
            loop {
                let event = MarketEvent::Trade(TradeEvent {
                    asset: asset.clone(),
                    price: 50_000.0,
                    size: 0.001,
                    timestamp: time::now(),
                });
                if tx.send(event).is_err() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        info!("mock feed spawned");
        tokio::spawn(async move {
            if let Err(e) = handle.await {
                tracing::error!(error = %e, "mock feed exited");
            }
        });

        Ok(())
    }

    pub async fn shutdown(&self) -> AppResult<()> {
        if let Some(handle) = self._task.lock().await.take() {
            handle.abort();
        }
        Ok(())
    }
}
