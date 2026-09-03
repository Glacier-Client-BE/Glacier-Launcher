//! Shared error type for the gl-integrations crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("discord rpc error: {0}")]
    Discord(String),

    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, IntegrationError>;
