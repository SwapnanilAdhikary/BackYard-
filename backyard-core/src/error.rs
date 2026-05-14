use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackyardError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("job not found: {0}")]
    NotFound(String),
    #[error("handler not registered for job type: {0}")]
    UnknownJobType(String),
    #[error("max retries exceeded")]
    MaxRetriesExceeded,
    #[error("shutdown signal received")]
    Shutdown,
    #[error("job execution error: {0}")]
    Execution(String),
}

pub type Result<T, E = BackyardError> = std::result::Result<T, E>;
