use std::sync::Arc;

use hyperliquid_rust_sdk::InfoClient;
use tokio::sync::Mutex;
use tracing::instrument;

use crate::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct InfoService {
    inner: Arc<Mutex<InfoClient>>,
}

impl InfoService {
    pub async fn connect(base_url: hyperliquid_rust_sdk::BaseUrl) -> AppResult<Self> {
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
}
