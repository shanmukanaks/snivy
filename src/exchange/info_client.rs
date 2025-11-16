use std::sync::Arc;

use alloy::primitives::Address;
use chrono::Utc;
use hyperliquid_rust_sdk::{BaseUrl, InfoClient, Message, Subscription};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tracing::instrument;

use crate::errors::{AppError, AppResult};
use crate::utils::time::interval_to_millis;
use hyperliquid_rust_sdk::CandlesSnapshotResponse;

#[derive(Clone)]
pub struct InfoService {
    inner: Arc<Mutex<InfoClient>>,
}

impl InfoService {
    pub async fn connect(base_url: BaseUrl) -> AppResult<Self> {
        let client = InfoClient::with_reconnect(None, Some(base_url))
            .await
            .map_err(|e| AppError::Exchange(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(client)),
        })
    }

    #[instrument(skip(self))]
    pub async fn latest_price(&self, asset: &str) -> AppResult<f64> {
        let guard = self.inner.lock().await;
        let mids = guard
            .all_mids()
            .await
            .map_err(|e| AppError::Exchange(e.to_string()))?;
        let price = mids
            .get(asset)
            .ok_or_else(|| AppError::Exchange(format!("asset {asset} not found in mids")))?
            .parse::<f64>()
            .map_err(|e| AppError::Exchange(e.to_string()))?;
        Ok(price)
    }

    #[instrument(skip(self))]
    pub async fn candles_snapshot(
        &self,
        asset: &str,
        interval: &str,
        count: usize,
    ) -> AppResult<Vec<f64>> {
        let window_ms = interval_to_millis(interval).ok_or_else(|| {
            AppError::Config(format!(
                "unsupported interval '{interval}' for MA bootstrap"
            ))
        })?;
        let end_time = Utc::now().timestamp_millis() as u64;
        let start_time = end_time.saturating_sub(window_ms * count as u64);
        let guard = self.inner.lock().await;
        let candles = guard
            .candles_snapshot(
                asset.to_string(),
                interval.to_string(),
                start_time,
                end_time,
            )
            .await
            .map_err(|e| AppError::Exchange(e.to_string()))?;
        Ok(Self::extract_closes(candles))
    }

    fn extract_closes(candles: Vec<CandlesSnapshotResponse>) -> Vec<f64> {
        candles
            .into_iter()
            .filter_map(|c| c.close.parse::<f64>().ok())
            .collect()
    }

    pub async fn subscribe(
        &self,
        subscription: Subscription,
    ) -> AppResult<UnboundedReceiver<Message>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut client = self.inner.lock().await;
        client
            .subscribe(subscription, tx)
            .await
            .map_err(|e| AppError::Exchange(e.to_string()))?;
        Ok(rx)
    }

    pub async fn subscribe_user_fills(
        &self,
        address: Address,
    ) -> AppResult<UnboundedReceiver<Message>> {
        self.subscribe(Subscription::UserFills { user: address })
            .await
    }
}
