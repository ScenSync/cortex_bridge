//! Common error types for EasyTier integration

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EasyTierError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("FFI error: {0}")]
    FfiError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for EasyTierError {
    fn from(err: anyhow::Error) -> Self {
        EasyTierError::Internal(err.to_string())
    }
}

impl From<std::io::Error> for EasyTierError {
    fn from(err: std::io::Error) -> Self {
        EasyTierError::NetworkError(err.to_string())
    }
}
