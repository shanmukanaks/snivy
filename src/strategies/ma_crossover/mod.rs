use std::collections::VecDeque;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;

use crate::errors::{AppError, AppResult};
use crate::exchange::order_router::{OrderIntent, OrderSide};
use crate::marketdata::events::MarketEvent;
use crate::marketdata::indicators::MovingAverage;
use crate::strategies::{Strategy, StrategyContext, StrategyResponse};

#[derive(Debug, Clone, Deserialize)]
pub struct MaCrossoverParams {
    pub asset: String,
    pub short_window: usize,
    pub long_window: usize,
    #[serde(default = "default_trade_size")]
    pub trade_size: f64,
}

fn default_trade_size() -> f64 {
    0.001
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaCrossoverState {
    pub short_window: VecDeque<f64>,
    pub long_window: VecDeque<f64>,
    pub last_signal: Option<SignalSide>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalSide {
    Long,
    Short,
    Flat,
}

pub struct MaCrossoverStrategy {
    params: MaCrossoverParams,
    short_ma: MovingAverage,
    long_ma: MovingAverage,
    last_signal: SignalSide,
}

pub struct MaCrossoverBuilder;

impl MaCrossoverBuilder {
    pub fn build(params: Value) -> AppResult<Box<dyn Strategy>> {
        let params: MaCrossoverParams = serde_json::from_value(params)
            .map_err(|e| AppError::Config(format!("invalid MA params: {e}")))?;
        if params.short_window >= params.long_window {
            return Err(AppError::Config(
                "short_window must be < long_window".into(),
            ));
        }
        Ok(Box::new(MaCrossoverStrategy::new(params)))
    }
}

impl MaCrossoverStrategy {
    fn new(params: MaCrossoverParams) -> Self {
        Self {
            short_ma: MovingAverage::new(params.short_window),
            long_ma: MovingAverage::new(params.long_window),
            params,
            last_signal: SignalSide::Flat,
        }
    }

    fn evaluate(&mut self, price: f64) -> Option<OrderIntent> {
        let short = self.short_ma.update(price)?;
        let long = self.long_ma.update(price)?;

        if short > long && self.last_signal != SignalSide::Long {
            self.last_signal = SignalSide::Long;
            Some(OrderIntent {
                asset: self.params.asset.clone(),
                side: OrderSide::Buy,
                size: self.params.trade_size,
                description: "ma_crossover_long".into(),
            })
        } else if short < long && self.last_signal != SignalSide::Short {
            self.last_signal = SignalSide::Short;
            Some(OrderIntent {
                asset: self.params.asset.clone(),
                side: OrderSide::Sell,
                size: self.params.trade_size,
                description: "ma_crossover_short".into(),
            })
        } else {
            None
        }
    }
}

#[async_trait]
impl Strategy for MaCrossoverStrategy {
    fn id(&self) -> &'static str {
        "ma_crossover"
    }

    #[instrument(skip(self, ctx))]
    async fn on_event(
        &mut self,
        ctx: &mut StrategyContext,
        event: MarketEvent,
    ) -> AppResult<StrategyResponse> {
        let _ = ctx;
        match event {
            MarketEvent::Candle(candle) if candle.asset == self.params.asset => {
                if let Some(intent) = self.evaluate(candle.close) {
                    Ok(StrategyResponse::with_intent(intent))
                } else {
                    Ok(StrategyResponse::idle())
                }
            }
            MarketEvent::Trade(trade) if trade.asset == self.params.asset => {
                if let Some(intent) = self.evaluate(trade.price) {
                    Ok(StrategyResponse::with_intent(intent))
                } else {
                    Ok(StrategyResponse::idle())
                }
            }
            _ => Ok(StrategyResponse::idle()),
        }
    }

    fn snapshot_state(&self) -> Value {
        serde_json::to_value(MaCrossoverState {
            short_window: VecDeque::new(),
            long_window: VecDeque::new(),
            last_signal: Some(self.last_signal.clone()),
        })
        .unwrap_or_default()
    }

    fn restore_state(&mut self, state: Value) {
        if let Ok(state) = serde_json::from_value::<MaCrossoverState>(state) {
            if let Some(signal) = state.last_signal {
                self.last_signal = signal;
            }
        }
    }
}
