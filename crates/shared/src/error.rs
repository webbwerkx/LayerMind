//! Unified error types for LayerMind.

/// Top-level error covering all LayerMind failure modes.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("connection error: {0}")]
    Connection(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("telemetry error: {0}")]
    Telemetry(String),

    #[error("printer error: {0}")]
    Printer(String),

    #[error("AI engine error: {0}")]
    Ai(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
