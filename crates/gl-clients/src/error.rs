//! Error types for the `gl-clients` crate.

use thiserror::Error;

/// Errors that can occur while working with third-party client catalogs,
/// manifests, and metadata.
#[derive(Debug, Error)]
pub enum ClientsError {
    #[error("network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("no downloadable release asset found: {0}")]
    NoAssetFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, ClientsError>;
