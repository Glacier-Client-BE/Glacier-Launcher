use std::path::PathBuf;

/// Errors produced by the `gl-sync` crate. Every variant carries an
/// idiot-proof, user-facing message — no silent failures, no panics (mirrors
/// this project's robustness convention; see `crates/gl-launch/src/error.rs`
/// and `crates/gl-java/src/error.rs` for the house style this follows).
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("sync pack not found: {0}")]
    PackNotFound(String),

    #[error("instance not found: {0}")]
    InstanceNotFound(String),

    #[error("persistence error: {0}")]
    Persist(#[from] gl_persistence::PersistError),

    #[error("refusing to link over an existing real directory at {0}: choose an adopt strategy first (keep existing, take pack version, or merge)")]
    UnresolvedConflict(PathBuf),

    #[error("a directory link (junction) requires an explicit apply; call preview() first and confirm before apply()")]
    PreviewRequired,

    #[error("options.txt parse error at {0}: {1}")]
    OptionsFormat(PathBuf, String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SyncError>;

/// Helper mirroring `gl_java::error::io_err`: build an `Io` variant closure
/// bound to `path`, so `.map_err(io_err(&path))` reads naturally at call
/// sites.
pub fn io_err(path: &std::path::Path) -> impl Fn(std::io::Error) -> SyncError + '_ {
    move |source| SyncError::Io {
        path: path.to_path_buf(),
        source,
    }
}
