//! Full Java Edition launch orchestration: resolve a runnable `javaw.exe`,
//! resolve auth values, build the classpath/args (via `gl_java::launcher`),
//! and spawn the process with piped stdout/stderr.
//!
//! Deviations from `Services/JavaGameLauncher.cs` (deliberately out of scope
//! for this first pass — surfaced as clear errors instead of silent hangs):
//! - No on-demand Java-runtime download (`JavaRuntimeDownloadService`). If no
//!   matching `javaw.exe` is found, we fail fast with a message telling the
//!   user to install one / point Settings → Java Runtime at it.
//! - No on-demand missing-library auto-install. If any classpath entry is
//!   absent on disk, we fail fast telling the user to reinstall the version.
//! - No live MSA token refresh on launch (`XboxProfileService.RefreshProfileAsync`).
//!   We use the cached access token from settings if present and unexpired,
//!   otherwise fall back to offline mode, exactly like the C# fallback path
//!   when refresh throws.
//! - Windows-registry JRE discovery (`HKLM/HKCU ...JavaSoft/Adoptium`) is not
//!   ported; `JAVA_HOME`, `PATH`, and the bundled `<mcDir>/runtime` tree are.

use crate::error::{LaunchError, Result};
pub use gl_java::launcher::LaunchOptions;
use gl_java::launcher::{self as jl, AuthValues};
use std::path::{Path, PathBuf};
use tokio::process::Child;

/// Everything the caller (a Tauri command) must supply to launch a Java
/// instance. Mirrors the settings/instance fields `JavaGameLauncher` reads.
#[derive(Debug, Clone)]
pub struct JavaLaunchRequest {
    pub version_id: String,
    /// `.minecraft`-shaped directory for this instance (mirrors `MinecraftDir`).
    pub mc_dir: PathBuf,
    /// Explicit runtime path from Settings → Java Runtime, if set.
    pub explicit_java_path: Option<PathBuf>,
    /// Root under which bundled runtimes may already exist (`<mcDir>/runtime`).
    pub bundled_runtime_root: PathBuf,
    /// Root under which Glacier's own downloaded runtimes are cached
    /// (passed to `gl_java::runtime_download::RuntimeDownloadService`).
    pub glacier_runtime_root: PathBuf,
    pub options: LaunchOptions,
    pub auth: ResolvedAuth,
}

/// Pre-resolved auth values — the caller (Tauri command layer, which has
/// access to `LauncherSettings`) decides offline vs. cached-MSA-token, since
/// this crate has no settings/auth dependency of its own.
#[derive(Debug, Clone)]
pub enum ResolvedAuth {
    Offline {
        username: String,
    },
    Msa {
        username: String,
        uuid: String,
        access_token: String,
    },
}

impl ResolvedAuth {
    fn into_auth_values(self) -> AuthValues {
        match self {
            ResolvedAuth::Offline { username } => {
                let name = if username.trim().is_empty() {
                    "Player".to_string()
                } else {
                    username
                };
                let uuid = jl::offline_uuid(&name);
                AuthValues {
                    name,
                    uuid,
                    access_token: "0".to_string(),
                    user_type: "legacy".to_string(),
                }
            }
            ResolvedAuth::Msa {
                username,
                uuid,
                access_token,
            } => AuthValues {
                name: username,
                uuid,
                access_token,
                user_type: "msa".to_string(),
            },
        }
    }
}

pub struct LaunchedJava {
    pub child: Child,
    pub pid: u32,
    /// Sanitised (auth-token-redacted) command line, for display in the live
    /// console window. Mirrors `SanitiseForLog`.
    pub command_line: String,
}

/// Runs the full pipeline and spawns the process. Does not track/lock the
/// instance or persist "last used version" — the Tauri command layer (which
/// owns `AppState`) does that, mirroring how `AcquireInstanceLock` /
/// `_instances.SetActiveVersion` live in the caller in the C# original too.
pub async fn launch(req: JavaLaunchRequest) -> Result<LaunchedJava> {
    let version_dir = req.mc_dir.join("versions").join(&req.version_id);
    let json_path = version_dir.join(format!("{}.json", req.version_id));
    if !json_path.is_file() {
        return Err(LaunchError::VersionNotInstalled(req.version_id.clone()));
    }

    let profile = jl::read_version_profile(&req.version_id, &req.mc_dir)?;

    let required_major: u32 = profile.min_java_major.parse().unwrap_or(8);
    let javaw = resolve_java_runtime(&req, &profile.min_java_major)
        .ok_or(LaunchError::JavaRuntimeNotFound(required_major))?;
    let actual_major = jl::detect_java_major(&javaw);

    let classpath = jl::build_classpath(&profile);
    if classpath.is_empty() {
        return Err(LaunchError::NoClasspath);
    }
    let missing: Vec<&PathBuf> = classpath.iter().filter(|p| !p.is_file()).collect();
    if !missing.is_empty() {
        return Err(LaunchError::MissingLibraries {
            count: missing.len(),
            first: missing[0].display().to_string(),
        });
    }

    let jvm_args = jl::build_jvm_args(
        &profile,
        &req.mc_dir,
        &req.version_id,
        actual_major,
        &req.options,
    )?;
    let auth = req.auth.clone_for_log();
    let game_args =
        jl::build_game_args(&profile, &req.mc_dir, &req.version_id, &auth, &req.options);

    let command_line = sanitise(
        &jl::render_command_line(
            &javaw,
            &jvm_args,
            &classpath,
            &profile.main_class,
            &game_args,
        ),
        &auth.access_token,
    );

    let child = jl::spawn(
        &javaw,
        &jvm_args,
        &classpath,
        &profile.main_class,
        &game_args,
        &req.mc_dir,
    )?;

    let pid = child
        .id()
        .ok_or_else(|| LaunchError::Other("java process exited immediately after spawn".into()))?;

    Ok(LaunchedJava {
        child,
        pid,
        command_line,
    })
}

impl ResolvedAuth {
    fn clone_for_log(&self) -> AuthValues {
        self.clone().into_auth_values()
    }
}

fn sanitise(args: &str, access_token: &str) -> String {
    if access_token.is_empty() || access_token == "0" {
        args.to_string()
    } else {
        args.replace(access_token, "<access-token>")
    }
}

/// Mirrors `JavaGameLauncher.ResolveJavaRuntime`, minus the Windows-registry
/// lookup (see module docs).
fn resolve_java_runtime(req: &JavaLaunchRequest, min_java_major: &str) -> Option<PathBuf> {
    if let Some(p) = &req.explicit_java_path {
        if p.is_file() {
            return Some(p.clone());
        }
    }

    if let Ok(req_major) = min_java_major.parse::<u32>() {
        if req_major > 0 {
            let rt = gl_java::runtime_download::RuntimeDownloadService::new(
                req.glacier_runtime_root.clone(),
            );
            if let Some(p) = rt.get_cached_javaw(req_major) {
                return Some(p);
            }
        }
    }

    if req.bundled_runtime_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&req.bundled_runtime_root) {
            let components: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            for component in &components {
                if !min_java_major.is_empty() && !component_matches_major(component, min_java_major)
                {
                    continue;
                }
                if let Some(javaw) = find_javaw_under(component) {
                    return Some(javaw);
                }
            }
            for component in &components {
                if let Some(javaw) = find_javaw_under(component) {
                    return Some(javaw);
                }
            }
        }
    }

    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let p = Path::new(&java_home).join("bin").join("javaw.exe");
        if p.is_file() {
            return Some(p);
        }
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for seg in path_env.split(';').filter(|s| !s.is_empty()) {
            let p = Path::new(seg.trim()).join("javaw.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }

    None
}

fn component_matches_major(component_dir: &Path, major: &str) -> bool {
    let name = component_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match major {
        "8" => name.contains("legacy"),
        "16" => name.contains("alpha"),
        "17" => name.contains("beta") || name.contains("gamma"),
        "21" => name.contains("delta"),
        "25" => name.contains("epsilon"),
        _ => false,
    }
}

fn find_javaw_under(root: &Path) -> Option<PathBuf> {
    if !root.is_dir() {
        return None;
    }
    fn walk(dir: &Path) -> Option<PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = walk(&path) {
                    return Some(found);
                }
            } else if path
                .file_name()
                .map(|n| n.eq_ignore_ascii_case("javaw.exe"))
                .unwrap_or(false)
            {
                return Some(path);
            }
        }
        None
    }
    walk(root)
}
