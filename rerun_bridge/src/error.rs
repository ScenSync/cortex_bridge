//! Error types for rerun_bridge

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RerunBridgeError {
    #[error("Failed to create recording: {0}")]
    RecordingCreation(String),

    #[error("Failed to log data: {0}")]
    LoggingFailed(String),

    #[error("Failed to convert to RRD: {0}")]
    ConversionFailed(String),

    #[error("Invalid data format: {0}")]
    InvalidData(String),

    #[error("MCAP parsing error: {0}")]
    MCAPError(String),
}

pub type Result<T> = std::result::Result<T, RerunBridgeError>;

