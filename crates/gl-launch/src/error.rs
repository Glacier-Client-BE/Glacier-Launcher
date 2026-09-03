use thiserror::Error;

/// Launch-pipeline errors. Every variant carries an idiot-proof, user-facing
/// message — no silent failures, no panics (mirrors this project's
/// robustness convention: see `Services/LaunchDiagnosticsService.cs`, which
/// this error set's messages are ported from).
#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("Version {0} isn't installed. Click Install on the row in Versions to download it.")]
    VersionNotInstalled(String),

    #[error(
        "Could not find javaw.exe. Set Settings → Java Runtime, or install a Java {0} runtime."
    )]
    JavaRuntimeNotFound(u32),

    #[error("Couldn't resolve any libraries on disk. Run this version once in the official launcher so libraries download, then retry.")]
    NoClasspath,

    #[error("{count} required librar{plural} missing on disk: {first}{more}. Try installing this version from the Versions tab first.", plural = if *count == 1 { "y is" } else { "ies are" }, more = if *count > 1 { format!(" (+{} more)", count - 1) } else { String::new() })]
    MissingLibraries { count: usize, first: String },

    #[error("This Java instance is already launching or running.")]
    AlreadyRunning,

    #[error("Java instance not found: {0}")]
    InstanceNotFound(String),

    #[error("Minecraft Bedrock isn't installed for this user. Install it from the Microsoft Store, then try again.")]
    BedrockNotInstalled,

    #[error("'{0}' is missing from the installed package ({1}). Repair or reinstall Minecraft from the Microsoft Store.")]
    BedrockExeMissing(String, String),

    #[error(
        "Minecraft did not start. Please launch Minecraft manually and then click Launch again."
    )]
    BedrockLaunchTimedOut,

    #[error("Every launch method failed. Details: {0}")]
    BedrockActivationFailed(String),

    #[error("Injection failed: {0}")]
    InjectionFailed(String),

    #[error("{0}")]
    Io(String),

    #[error("{0}")]
    Java(#[from] gl_java::error::JavaError),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, LaunchError>;
