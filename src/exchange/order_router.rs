use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::errors::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntent {
    pub asset: String,
    pub side: OrderSide,
    pub size: f64,
    pub description: String,
}

#[derive(Clone, Default)]
pub struct OrderRouter;

impl OrderRouter {
    #[instrument(skip(self))]
    pub async fn submit(&self, intent: OrderIntent) -> AppResult<String> {
        info!(?intent, "order intent received");
        // TODO: need to integrate hyperliquid ExchangeClient here
        Ok(format!(
            "mock-cloid-{}-{}",
            intent.asset,
            match intent.side {
                OrderSide::Buy => "buy",
                OrderSide::Sell => "sell",
            }
        ))
    }
}
