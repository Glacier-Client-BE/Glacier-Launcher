use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("network request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("failed to parse GitHub API response: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("no usable release asset found")]
    NoAsset,

    #[error("no releases found")]
    NoReleases,
}

pub type Result<T> = std::result::Result<T, UpdateError>;
