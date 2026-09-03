//! Error types shared across the `gl-bedrock` services.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BedrockError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("instance not found: {0}")]
    InstanceNotFound(String),

    #[error("cannot delete the last remaining instance")]
    LastInstance,

    #[error("cannot delete the active instance")]
    ActiveInstance,

    #[error("backup file not found: {0}")]
    BackupNotFound(PathBuf),

    #[error("nothing to back up yet — no worlds or packs found")]
    NothingToBackUp,

    #[error("path not found: {0}")]
    NotFound(PathBuf),

    #[error("Minecraft Bedrock isn't installed for this user. Install it from the Microsoft Store, then try again.")]
    PackageNotFound,

    #[error("{0}")]
    LaunchFailed(String),

    #[error("Windows Update: {0}")]
    Wu(String),
}

pub type Result<T> = std::result::Result<T, BedrockError>;

/// Small helper for attaching a path to an [`std::io::Error`].
pub(crate) trait IoContext<T> {
    fn ctx(self, path: impl Into<PathBuf>) -> Result<T>;
}

impl<T> IoContext<T> for std::result::Result<T, std::io::Error> {
    fn ctx(self, path: impl Into<PathBuf>) -> Result<T> {
        self.map_err(|source| BedrockError::Io {
            path: path.into(),
            source,
        })
    }
}
