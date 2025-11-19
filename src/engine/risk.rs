use crate::exchange::order_router::OrderIntent;

#[derive(Debug, Clone)]
pub struct RiskLimits {
    pub max_position: f64,
}

impl RiskLimits {
    pub fn allow(&self, _intent: &OrderIntent) -> bool {
        true
    }
}
