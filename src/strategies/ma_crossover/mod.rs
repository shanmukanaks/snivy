use std::collections::VecDeque;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{instrument, warn};

use crate::errors::{AppError, AppResult};
use crate::exchange::order_router::{OrderIntent, OrderSide, OrderTif};
use crate::marketdata::events::MarketEvent;
use crate::marketdata::indicators::MovingAverage;
use crate::strategies::{Strategy, StrategyBuilderContext, StrategyContext, StrategyResponse};
use crate::utils::math::format_decimal;

const SNAPSHOT_PREFIX: &str = "ma_crossover";

#[derive(Debug, Clone, Deserialize)]
pub struct MaCrossoverParams {
    pub asset: String,
    pub short_window: usize,
    pub long_window: usize,
    #[serde(default = "default_trade_size")]
    pub trade_size: String,
    #[serde(default = "default_interval")]
    pub candle_interval: String,
    #[serde(default = "default_slippage_bps")]
    pub slippage_bps: u32,
    #[serde(default = "default_max_position")]
    pub max_position: f64,
    #[serde(default = "default_order_rate_limit")]
    pub max_order_rate_per_min: u32,
    #[serde(default = "default_bootstrap_candles")]
    pub bootstrap_candles: usize,
}

fn default_trade_size() -> String {
    "0.01".to_string()
}

fn default_interval() -> String {
    "1m".to_string()
}

fn default_slippage_bps() -> u32 {
    5
}

fn default_max_position() -> f64 {
    0.05
}

fn default_order_rate_limit() -> u32 {
    30
}

fn default_bootstrap_candles() -> usize {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MaCrossoverSnapshot {
    short_values: Vec<f64>,
    long_values: Vec<f64>,
    last_signal: SignalSide,
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
    info: crate::exchange::InfoService,
    snapshot_store: crate::storage::persistence::SnapshotStore,
    snapshot_key: String,
    bootstrapped: bool,
    rate_limiter: OrderRateLimiter,
}

pub struct MaCrossoverBuilder;

impl MaCrossoverBuilder {
    pub fn build(params: Value, ctx: StrategyBuilderContext) -> AppResult<Box<dyn Strategy>> {
        let mut params: MaCrossoverParams = serde_json::from_value(params)
            .map_err(|e| AppError::Config(format!("invalid MA params: {e}")))?;
        if params.short_window >= params.long_window {
            return Err(AppError::Config(
                "short_window must be < long_window".into(),
            ));
        }
        if params.bootstrap_candles < params.long_window {
            params.bootstrap_candles = params.long_window * 2;
        }
        let snapshot_key = format!("{SNAPSHOT_PREFIX}_{}", params.asset.to_lowercase());

        let rate_limit = params.max_order_rate_per_min.max(1);
        let mut strategy = MaCrossoverStrategy {
            short_ma: MovingAverage::new(params.short_window),
            long_ma: MovingAverage::new(params.long_window),
            params,
            last_signal: SignalSide::Flat,
            info: ctx.info.clone(),
            snapshot_store: ctx.snapshot_store.clone(),
            snapshot_key,
            bootstrapped: false,
            rate_limiter: OrderRateLimiter::new(60, rate_limit),
        };

        // attempt to load cached state immediately
        strategy.load_from_snapshot()?;

        Ok(Box::new(strategy))
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
        if !matches_asset(&event, &self.params.asset) {
            return Ok(StrategyResponse::idle());
        }

        self.ensure_bootstrap().await?;
        let price = match &event {
            MarketEvent::Candle(candle) => candle.close,
            MarketEvent::Trade(trade) => trade.price,
        };

        if let Some(intent) = self.evaluate(price, ctx).await? {
            self.persist_state()?;
            return Ok(StrategyResponse::with_intent(intent));
        }

        Ok(StrategyResponse::idle())
    }

    #[instrument(skip(self, ctx))]
    async fn on_fill(
        &mut self,
        ctx: &mut StrategyContext,
        _fill: crate::exchange::FillEvent,
    ) -> AppResult<StrategyResponse> {
        self.sync_signal_from_positions(ctx);
        self.persist_state()?;
        Ok(StrategyResponse::idle())
    }

    fn snapshot_state(&self) -> Value {
        serde_json::to_value(self.build_snapshot()).unwrap_or_default()
    }

    fn restore_state(&mut self, state: Value) {
        if let Ok(snapshot) = serde_json::from_value::<MaCrossoverSnapshot>(state) {
            self.restore_from_snapshot(snapshot);
        }
    }
}

impl MaCrossoverStrategy {
    async fn ensure_bootstrap(&mut self) -> AppResult<()> {
        if self.bootstrapped {
            return Ok(());
        }

        if self.short_ma.is_ready() && self.long_ma.is_ready() {
            self.bootstrapped = true;
            return Ok(());
        }

        let closes = self
            .info
            .candles_snapshot(
                &self.params.asset,
                &self.params.candle_interval,
                self.params.bootstrap_candles,
            )
            .await?;

        for px in closes {
            self.short_ma.update(px);
            self.long_ma.update(px);
        }

        self.bootstrapped = true;
        self.persist_state()?;
        Ok(())
    }

    async fn evaluate(
        &mut self,
        price: f64,
        ctx: &StrategyContext,
    ) -> AppResult<Option<OrderIntent>> {
        let short = match self.short_ma.update(price) {
            Some(value) => value,
            None => return Ok(None),
        };
        let long = match self.long_ma.update(price) {
            Some(value) => value,
            None => return Ok(None),
        };

        let target_signal = if short > long {
            SignalSide::Long
        } else if short < long {
            SignalSide::Short
        } else {
            SignalSide::Flat
        };

        if target_signal == self.last_signal || target_signal == SignalSide::Flat {
            return Ok(None);
        }

        if !self.rate_limiter.allow() {
            warn!("rate limiter blocked order submission");
            return Ok(None);
        }

        let net_position = self.net_position(ctx);
        let side = if matches!(target_signal, SignalSide::Long) {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let reduce_only = match side {
            OrderSide::Buy => net_position < 0.0,
            OrderSide::Sell => net_position > 0.0,
        };

        if !self.within_limits(net_position, &side, reduce_only) {
            return Ok(None);
        }

        let limit_px = self.limit_price(price, &side);
        let intent = OrderIntent {
            asset: self.params.asset.clone(),
            side,
            size: self.params.trade_size.clone(),
            limit_px,
            tif: OrderTif::Ioc,
            reduce_only,
            client_tag: format!("ma_cross_{target_signal:?}"),
            cloid: None,
        };

        self.last_signal = target_signal;
        Ok(Some(intent))
    }

    fn within_limits(&self, net_position: f64, side: &OrderSide, reduce_only: bool) -> bool {
        if reduce_only {
            return true;
        }

        match side {
            OrderSide::Buy => net_position < self.params.max_position,
            OrderSide::Sell => net_position > -self.params.max_position,
        }
    }

    fn limit_price(&self, price: f64, side: &OrderSide) -> String {
        let bps = self.params.slippage_bps as f64 / 10_000.0;
        let adjusted = match side {
            OrderSide::Buy => price * (1.0 + bps),
            OrderSide::Sell => price * (1.0 - bps),
        };
        format_decimal(adjusted)
    }

    fn net_position(&self, ctx: &StrategyContext) -> f64 {
        ctx.positions()
            .into_iter()
            .find(|pos| pos.asset == self.params.asset)
            .map(|pos| pos.size)
            .unwrap_or(0.0)
    }

    fn sync_signal_from_positions(&mut self, ctx: &StrategyContext) {
        let net = self.net_position(ctx);
        self.last_signal = if net > 0.0 {
            SignalSide::Long
        } else if net < 0.0 {
            SignalSide::Short
        } else {
            SignalSide::Flat
        };
    }

    fn persist_state(&self) -> AppResult<()> {
        let snapshot = self.build_snapshot();
        self.snapshot_store
            .save(&self.snapshot_key, &snapshot)
            .map_err(|e| AppError::Other(e.to_string()))
    }

    fn load_from_snapshot(&mut self) -> AppResult<()> {
        if let Some(snapshot) = self
            .snapshot_store
            .load::<MaCrossoverSnapshot>(&self.snapshot_key)
            .map_err(|e| AppError::Other(e.to_string()))?
        {
            self.restore_from_snapshot(snapshot);
            self.bootstrapped = true;
        }
        Ok(())
    }

    fn restore_from_snapshot(&mut self, snapshot: MaCrossoverSnapshot) {
        self.short_ma.seed(&snapshot.short_values);
        self.long_ma.seed(&snapshot.long_values);
        self.last_signal = snapshot.last_signal;
    }

    fn build_snapshot(&self) -> MaCrossoverSnapshot {
        MaCrossoverSnapshot {
            short_values: self.short_ma.values(),
            long_values: self.long_ma.values(),
            last_signal: self.last_signal.clone(),
        }
    }
}

fn matches_asset(event: &MarketEvent, asset: &str) -> bool {
    match event {
        MarketEvent::Candle(candle) => candle.asset == asset,
        MarketEvent::Trade(trade) => trade.asset == asset,
    }
}

struct OrderRateLimiter {
    max_per_minute: u32,
    timestamps: VecDeque<Instant>,
    window: Duration,
}

impl OrderRateLimiter {
    fn new(window_seconds: u32, max_per_minute: u32) -> Self {
        Self {
            max_per_minute,
            timestamps: VecDeque::new(),
            window: Duration::from_secs(window_seconds as u64),
        }
    }

    fn allow(&mut self) -> bool {
        let now = Instant::now();
        while let Some(ts) = self.timestamps.front() {
            if now.duration_since(*ts) > self.window {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        if self.timestamps.len() as u32 >= self.max_per_minute {
            false
        } else {
            self.timestamps.push_back(now);
            true
        }
    }
}
