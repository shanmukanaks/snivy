use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use serde_json::Value;

use super::ma_crossover::MaCrossoverBuilder;
use super::{Strategy, StrategyBuilderContext};
use crate::errors::{AppError, AppResult};

type Factory =
    Box<dyn Fn(Value, StrategyBuilderContext) -> AppResult<Box<dyn Strategy>> + Send + Sync>;

static REGISTRY: OnceLock<RwLock<HashMap<&'static str, Factory>>> = OnceLock::new();

pub fn register_builtin_strategies() {
    let registry = REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
    let mut guard = registry.write().unwrap();
    guard.insert(
        "ma_crossover",
        Box::new(|params, ctx| MaCrossoverBuilder::build(params, ctx)),
    );
}

pub fn build(id: &str, params: Value, ctx: StrategyBuilderContext) -> AppResult<Box<dyn Strategy>> {
    let registry = REGISTRY
        .get_or_init(|| RwLock::new(HashMap::new()))
        .read()
        .unwrap();
    let factory = registry
        .get(id)
        .ok_or_else(|| AppError::Config(format!("strategy {id} not registered")))?;
    factory(params, ctx)
}
