use std::path::PathBuf;

/// Errors produced by the `gl-java` crate.
#[derive(Debug, thiserror::Error)]
pub enum JavaError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to extract archive: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("instance not found: {0}")]
    InstanceNotFound(String),

    #[error("cannot delete the last remaining instance")]
    CannotDeleteLastInstance,

    #[error("version {0} is not installed (missing version JSON)")]
    VersionNotInstalled(String),

    #[error("version JSON for {0} is missing mainClass")]
    MissingMainClass(String),

    #[error("no libraries could be resolved on disk for this version")]
    NoLibrariesResolved,

    #[error("could not find or download a matching javaw executable")]
    NoJavaRuntime,

    #[error("Adoptium API returned status {status} for Java {major}")]
    AdoptiumStatus { status: u16, major: u32 },

    #[error("no downloadable Adoptium package found for Java {0}")]
    AdoptiumNoPackage(u32),

    #[error("extracted Java {major} but could not find javaw.exe under {dir}")]
    JavawNotFoundAfterExtract { major: u32, dir: PathBuf },

    #[error("this instance is already launching or running")]
    AlreadyRunning,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, JavaError>;

pub(crate) fn io_err(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> JavaError {
    let path = path.into();
    move |source| JavaError::Io { path, source }
}
