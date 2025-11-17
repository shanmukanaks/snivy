use alloy::primitives::Address;
use chrono::{TimeZone, Utc};
use hyperliquid_rust_sdk::{Message, Subscription};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::errors::AppResult;
use crate::exchange::{FillEvent, InfoService};
use crate::marketdata::events::{CandleEvent, MarketEvent};

pub struct MarketStream {
    tx: broadcast::Sender<MarketEvent>,
    task: JoinHandle<()>,
}

impl MarketStream {
    pub async fn connect_candles(
        info: InfoService,
        asset: String,
        interval: String,
        buffer: usize,
    ) -> AppResult<Self> {
        let mut rx = info
            .subscribe(Subscription::Candle {
                coin: asset.clone(),
                interval: interval.clone(),
            })
            .await?;
        let (tx, _) = broadcast::channel(buffer);
        let tx_clone = tx.clone();
        let task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Message::Candle(candle) = message {
                    if let Some(event) = Self::map_candle(&asset, &interval, candle) {
                        let _ = tx_clone.send(MarketEvent::Candle(event));
                    }
                }
            }
        });

        Ok(Self { tx, task })
    }

    fn map_candle(
        asset: &str,
        interval: &str,
        candle: hyperliquid_rust_sdk::Candle,
    ) -> Option<CandleEvent> {
        let data = candle.data;
        let close = data.close.parse::<f64>().ok()?;
        let ts = Utc.timestamp_millis_opt(data.time_close as i64).single()?;
        Some(CandleEvent {
            asset: asset.to_string(),
            close,
            timestamp: ts,
            interval: interval.to_string(),
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MarketEvent> {
        self.tx.subscribe()
    }
}

impl Drop for MarketStream {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn user_fills_stream(
    info: InfoService,
    address: Address,
) -> AppResult<mpsc::UnboundedReceiver<FillEvent>> {
    let mut raw_rx = info.subscribe_user_fills(address).await?;
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(message) = raw_rx.recv().await {
            if let Message::UserFills(fills) = message {
                for trade in fills.data.fills {
                    if let (Ok(price), Ok(size)) =
                        (trade.px.parse::<f64>(), trade.sz.parse::<f64>())
                    {
                        let is_buy = trade.side.eq_ignore_ascii_case("B")
                            || trade.side.eq_ignore_ascii_case("buy");
                        let event = FillEvent {
                            asset: trade.coin.clone(),
                            price,
                            size,
                            is_buy,
                            cloid: trade.cloid.clone(),
                        };
                        let _ = tx.send(event);
                    }
                }
            }
        }
    });
    Ok(rx)
}
