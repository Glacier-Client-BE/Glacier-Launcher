//! Error types for the gl-modpack crate.

use thiserror::Error;

/// Errors that can occur while interacting with modpack/mod APIs, parsing
/// manifests, or inspecting installed jars.
#[derive(Debug, Error)]
pub enum ModpackError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("CurseForge API key is not set")]
    MissingApiKey,

    #[error("HTTP {status}: {message}")]
    Status { status: u16, message: String },

    #[error("{0}")]
    NotFound(String),

    #[error("unsupported format: {0}")]
    Unsupported(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ModpackError>;
