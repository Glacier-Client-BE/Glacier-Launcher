//! Error types for the download engine.

use std::path::PathBuf;

/// Errors that can occur while downloading or queueing a file.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("server returned status {status} for {url}")]
    Status { status: u16, url: String },

    #[error("SHA256 mismatch for {url}: expected {expected}, got {actual}")]
    HashMismatch {
        url: String,
        expected: String,
        actual: String,
    },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("download cancelled")]
    Cancelled,

    #[error("download of {url} failed after {attempts} attempt(s): {source}")]
    RetriesExhausted {
        url: String,
        attempts: u32,
        #[source]
        source: Box<DownloadError>,
    },
}

impl DownloadError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Whether retrying the same request is likely to help.
    /// Mirrors `IsTransient` in the original `DownloadService.cs`.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            DownloadError::Http(_) | DownloadError::Io { .. } | DownloadError::Status { .. }
        )
    }
}

pub type Result<T> = std::result::Result<T, DownloadError>;
