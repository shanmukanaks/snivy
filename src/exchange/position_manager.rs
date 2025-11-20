use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub asset: String,
    pub size: f64,
    pub entry_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillEvent {
    pub asset: String,
    pub price: f64,
    pub size: f64,
    pub is_buy: bool,
    pub cloid: Option<String>,
}

#[derive(Clone, Default)]
pub struct PositionManager {
    inner: DashMap<String, Position>,
}

impl PositionManager {
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    pub fn snapshot(&self) -> Vec<Position> {
        self.inner.iter().map(|p| p.value().clone()).collect()
    }

    #[instrument(skip(self))]
    pub fn apply_fill(&self, fill: &FillEvent) {
        self.inner
            .entry(fill.asset.clone())
            .and_modify(|position| {
                if fill.is_buy {
                    position.size += fill.size;
                    position.entry_price = fill.price;
                } else {
                    position.size -= fill.size;
                }
            })
            .or_insert(Position {
                asset: fill.asset.clone(),
                size: if fill.is_buy { fill.size } else { -fill.size },
                entry_price: fill.price,
            });
    }
}
