//! Backs the "Launchers" panel. Despite the panel title, the driving
//! code-behind method is `RefreshJavaClients` (see `Pages/Home.Panels.cs`),
//! which detects locally-installed Lunar Client / Badlion Client
//! (`LunarBadlionService.Detect`) and lets the user pick an offline Lunar
//! multiversion install to direct-launch. This is a separate concept from
//! both the Java Fabric/Forge mod loaders (`java_mods.rs`) and the Bedrock
//! third-party clients list.
//!
//! Not ported: `LunarBadlionService.ReadUninstallExe`'s registry-uninstall-key
//! fallback (see the doc comment on `gl_clients::lunar_badlion` — no
//! registry crate is in scope), and the "Glacier custom client" manifest
//! (`EnsureGlacierManifestAsync`) referenced alongside this panel in the
//! original, which is a distinct feature covered by the existing custom
//! client tooling (`custom_client_library.rs` / `glacier_client.rs`) and not
//! duplicated here.

use gl_clients::lunar_badlion::{self, LunarBadlionInfo};

#[derive(Debug, Clone, serde::Serialize)]
pub struct LunarBadlionStatus {
    pub lunar_installed: bool,
    pub badlion_installed: bool,
    pub can_direct_launch_lunar: bool,
    pub lunar_installed_versions: Vec<String>,
    pub lunar_exe_path: Option<String>,
    pub badlion_exe_path: Option<String>,
}

impl From<LunarBadlionInfo> for LunarBadlionStatus {
    fn from(info: LunarBadlionInfo) -> Self {
        Self {
            lunar_installed: info.lunar_installed(),
            badlion_installed: info.badlion_installed(),
            can_direct_launch_lunar: info.can_direct_launch_lunar(),
            lunar_installed_versions: info.lunar_installed_versions.clone(),
            lunar_exe_path: info
                .lunar_exe_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            badlion_exe_path: info
                .badlion_exe_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
        }
    }
}

/// Mirrors `RefreshJavaClients` / `OpenJavaClientsAsync`'s `LunarBadlion.Detect()` call.
#[tauri::command]
pub fn detect_lunar_badlion() -> Result<LunarBadlionStatus, String> {
    let local_app_data = dirs_local_app_data();
    let user_profile = dirs_user_profile();
    Ok(lunar_badlion::detect(&local_app_data, &user_profile).into())
}

/// Launches the standalone Lunar Client or Badlion Client executable.
/// Mirrors the exe-spawn half of `LaunchLunar`/`LaunchBadlion` (the original
/// just starts the installed launcher exe for the user to click Play in).
#[tauri::command]
pub fn launch_client_exe(exe_path: String) -> Result<(), String> {
    std::process::Command::new(&exe_path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Resolves the jars/natives/JRE needed to direct-launch a specific Lunar
/// offline multiversion install. Mirrors the file-discovery half of
/// `LunarBadlionService.TryDirectLaunch`; actually spawning the JVM is left
/// to the frontend via the existing Java launch pipeline once this returns.
#[tauri::command]
pub fn resolve_lunar_direct_launch(
    version_id: String,
) -> Result<Option<DirectLaunchFilesDto>, String> {
    let local_app_data = dirs_local_app_data();
    let user_profile = dirs_user_profile();
    let info = lunar_badlion::detect(&local_app_data, &user_profile);
    Ok(
        lunar_badlion::resolve_direct_launch(&info, &version_id).map(|f| DirectLaunchFilesDto {
            jre_path: f.jre_path.to_string_lossy().into_owned(),
            version_dir: f.version_dir.to_string_lossy().into_owned(),
            natives_dir: f.natives_dir.to_string_lossy().into_owned(),
            jars: f
                .jars
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        }),
    )
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectLaunchFilesDto {
    pub jre_path: String,
    pub version_dir: String,
    pub natives_dir: String,
    pub jars: Vec<String>,
}

fn dirs_local_app_data() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
}

fn dirs_user_profile() -> std::path::PathBuf {
    std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
}
