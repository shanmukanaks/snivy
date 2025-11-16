use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleEvent {
    pub asset: String,
    pub close: f64,
    pub timestamp: DateTime<Utc>,
    pub interval: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEvent {
    pub asset: String,
    pub price: f64,
    pub size: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketEvent {
    Candle(CandleEvent),
    Trade(TradeEvent),
}
