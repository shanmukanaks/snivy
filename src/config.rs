use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    #[serde(default = "TelemetryConfig::default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub json: bool,
}

impl TelemetryConfig {
    fn default_log_level() -> String {
        "info".to_string()
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: Self::default_log_level(),
            json: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    #[serde(default = "ExchangeConfig::default_network")]
    pub network: String,
    #[serde(default)]
    pub rate_limit_per_minute: u32,
    pub api_key: Option<String>,
    pub secret_key: Option<String>,
    #[serde(default)]
    pub signer_private_key: Option<String>,
    #[serde(default)]
    pub signer_private_key_env: Option<String>,
}

impl ExchangeConfig {
    fn default_network() -> String {
        "mainnet".to_string()
    }
}

impl Default for ExchangeConfig {
    fn default() -> Self {
        Self {
            network: Self::default_network(),
            rate_limit_per_minute: 600,
            api_key: None,
            secret_key: None,
            signer_private_key: None,
            signer_private_key_env: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    #[serde(default = "PersistenceConfig::default_path")]
    pub snapshot_path: String,
    #[serde(default = "PersistenceConfig::default_journal_path")]
    pub journal_path: String,
    #[serde(default)]
    pub snapshot_interval_secs: u64,
}

impl PersistenceConfig {
    fn default_path() -> String {
        "data/snapshots".into()
    }

    fn default_journal_path() -> String {
        "data/journal.log".into()
    }
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            snapshot_path: Self::default_path(),
            journal_path: Self::default_journal_path(),
            snapshot_interval_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInstanceConfig {
    pub id: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub exchange: ExchangeConfig,
    #[serde(default)]
    pub persistence: PersistenceConfig,
    #[serde(default)]
    pub strategies: Vec<StrategyInstanceConfig>,
}

impl Settings {
    pub fn load_from(path: impl AsRef<Path>) -> AppResult<Self> {
        let builder = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .add_source(config::Environment::with_prefix("SNIVY").separator("__"));
        let cfg = builder.build()?;
        Ok(cfg.try_deserialize()?)
    }

    pub fn ensure_strategy(&self) -> AppResult<&StrategyInstanceConfig> {
        self.strategies
            .iter()
            .find(|s| s.enabled)
            .ok_or_else(|| AppError::Config("no enabled strategy found".into()))
    }
}
