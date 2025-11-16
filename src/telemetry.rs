use tracing_subscriber::{EnvFilter, fmt};

use crate::config::TelemetryConfig;
use crate::errors::AppResult;

pub fn init(cfg: &TelemetryConfig) -> AppResult<()> {
    let env_filter = EnvFilter::try_new(&cfg.log_level)
        .unwrap_or_else(|_| EnvFilter::new(TelemetryConfig::default().log_level));

    if cfg.json {
        fmt::fmt()
            .with_env_filter(env_filter)
            .json()
            .with_ansi(false)
            .with_target(false)
            .event_format(fmt::format().json())
            .try_init()
            .map_err(|e| crate::errors::AppError::Other(e.to_string()))
    } else {
        fmt::fmt()
            .with_env_filter(env_filter)
            .compact()
            .with_target(false)
            .try_init()
            .map_err(|e| crate::errors::AppError::Other(e.to_string()))
    }
}
