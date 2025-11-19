use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use serde_json::Value;

use super::Strategy;
use super::ma_crossover::MaCrossoverBuilder;
use crate::errors::{AppError, AppResult};

type Factory = Box<dyn Fn(Value) -> AppResult<Box<dyn Strategy>> + Send + Sync>;

static REGISTRY: OnceLock<RwLock<HashMap<&'static str, Factory>>> = OnceLock::new();

pub fn register_builtin_strategies() {
    let registry = REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
    let mut guard = registry.write().unwrap();
    guard.insert("ma_crossover", Box::new(MaCrossoverBuilder::build));
}

pub fn build(id: &str, params: Value) -> AppResult<Box<dyn Strategy>> {
    let registry = REGISTRY
        .get_or_init(|| RwLock::new(HashMap::new()))
        .read()
        .unwrap();
    let factory = registry
        .get(id)
        .ok_or_else(|| AppError::Config(format!("strategy {id} not registered")))?;
    factory(params)
}
