use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config parse error: {0}")]
    ConfigParse(#[from] config::ConfigError),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("strategy error: {0}")]
    Strategy(String),

    #[error("exchange error: {0}")]
    Exchange(String),

    #[error("other: {0}")]
    Other(String),
}

pub type AppResult<T, E = AppError> = Result<T, E>;
