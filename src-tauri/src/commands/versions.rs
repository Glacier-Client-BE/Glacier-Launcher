//! Commands backing the "Versions" panel: Latite's versioned GitHub
//! releases, and the vanilla Bedrock version browser (installable builds
//! resolved through the Windows Update SOAP protocol — see
//! `gl_bedrock::wu`, ported from `Services/VanillaVersionService.cs`'s
//! WU-facing methods).

use std::path::PathBuf;

use gl_bedrock::wu::WuVersion;
use gl_clients::latite::LatiteVersion;
use tauri::State;

use crate::state::AppState;

fn vanilla_bedrock_versions_dir() -> PathBuf {
    gl_settings::launcher_root().join("mc-bedrock-versions")
}

pub fn vanilla_bedrock_version_dir(version: &str) -> PathBuf {
    vanilla_bedrock_versions_dir().join(version)
}

fn latite_dir(state: &AppState) -> PathBuf {
    state.clients_dir.join("Latite")
}

fn latite_dll_path(state: &AppState, tag: &str) -> PathBuf {
    latite_dir(state).join(gl_clients::latite::dll_file_name(tag))
}

/// Lists Latite's versioned GitHub releases, newest first as returned by
/// the API. Mirrors `GameLauncher.GetVersionsAsync`.
#[tauri::command]
pub async fn list_latite_versions(
    state: State<'_, AppState>,
) -> Result<Vec<LatiteVersion>, String> {
    gl_clients::latite::list_versions(&state.http)
        .await
        .map_err(|e| e.to_string())
}

/// Whether a specific Latite release tag has already been downloaded.
#[tauri::command]
pub fn latite_version_installed(state: State<'_, AppState>, tag: String) -> bool {
    latite_dll_path(&state, &tag).is_file()
}

/// Downloads a specific Latite release's `.dll` asset. Mirrors
/// `GameLauncher.DownloadVersionAsync`. Returns the local DLL path so the
/// frontend can hand it straight to `inject_bedrock_client`.
#[tauri::command]
pub async fn download_latite_version(
    state: State<'_, AppState>,
    tag: String,
    download_url: String,
) -> Result<String, String> {
    if download_url.is_empty() {
        return Err(
            "This version has no download URL. Refresh the versions list and try again.".into(),
        );
    }

    let dest = latite_dll_path(&state, &tag);
    let req =
        gl_download::DownloadRequest::new(&download_url, &dest).with_label(format!("Latite {tag}"));
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().into_owned())
}

/// Deletes a downloaded Latite release DLL. Mirrors `GameLauncher.DeleteVersion`.
#[tauri::command]
pub fn delete_latite_version(state: State<'_, AppState>, tag: String) -> Result<(), String> {
    let path = latite_dll_path(&state, &tag);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn latite_version_dll_path(state: State<'_, AppState>, tag: String) -> String {
    latite_dll_path(&state, &tag).to_string_lossy().into_owned()
}

/// Lists installable vanilla Bedrock builds currently offered by Windows
/// Update. Mirrors `VanillaVersionService.FetchFromStoreApiAsync`.
#[tauri::command]
pub async fn list_vanilla_bedrock_versions(
    state: State<'_, AppState>,
) -> Result<Vec<WuVersion>, String> {
    gl_bedrock::wu::list_versions(&state.http)
        .await
        .map_err(|e| e.to_string())
}

/// Whether a given vanilla Bedrock build has already been downloaded and
/// extracted locally.
#[tauri::command]
pub fn vanilla_bedrock_version_installed(version: String) -> bool {
    vanilla_bedrock_version_dir(&version)
        .join("AppxManifest.xml")
        .is_file()
}

/// Downloads and extracts a vanilla Bedrock build's `.appx` package.
/// Mirrors `VanillaVersionService.DownloadVersionAsync`. Returns the local
/// extracted directory (containing `AppxManifest.xml`) so the frontend can
/// hand it to `switch_vanilla_bedrock_version`.
#[tauri::command]
pub async fn download_vanilla_bedrock_version(
    state: State<'_, AppState>,
    version: WuVersion,
) -> Result<String, String> {
    let url = gl_bedrock::wu::get_download_url(&state.http, &version.update_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Could not resolve a download URL for this version.".to_string())?;

    let versions_dir = vanilla_bedrock_versions_dir();
    std::fs::create_dir_all(&versions_dir).map_err(|e| e.to_string())?;

    let temp_file = versions_dir.join(format!("{}.appx.tmp", version.version));
    let version_dir = vanilla_bedrock_version_dir(&version.version);

    let mut req = gl_download::DownloadRequest::new(&url, &temp_file)
        .with_label(format!("Bedrock {}", version.version));
    if version.size_bytes > 0 {
        req = req.with_known_total_bytes(version.size_bytes);
    }
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    let staging_dir = versions_dir.join(format!("{}.staging", version.version));
    if staging_dir.exists() {
        let _ = std::fs::remove_dir_all(&staging_dir);
    }

    let extract_result = gl_bedrock::wu::extract_appx(&temp_file, &staging_dir);
    let _ = std::fs::remove_file(&temp_file);
    extract_result.map_err(|e| e.to_string())?;

    if version_dir.exists() {
        std::fs::remove_dir_all(&version_dir).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&staging_dir, &version_dir).map_err(|e| e.to_string())?;

    Ok(version_dir.to_string_lossy().into_owned())
}

/// Deletes a downloaded vanilla Bedrock build's extracted files. Mirrors
/// `VanillaVersionService.DeleteVersion`.
#[tauri::command]
pub fn delete_vanilla_bedrock_version(version: String) -> Result<(), String> {
    let dir = vanilla_bedrock_version_dir(&version);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Unregisters whatever vanilla Bedrock build is currently registered and
/// registers the given downloaded version in its place. Mirrors
/// `VanillaVersionService.SwitchVersionAsync`, minus its Developer Mode
/// precondition check (registering an unsigned/sideloaded package requires
/// Windows Developer Mode; that toggle isn't ported to this build, so
/// `Add-AppxPackage -DevelopmentMode` is left to surface its own error here
/// if it's off). The caller is responsible for persisting
/// `ActiveVanillaVersion` via `set_settings` on success.
#[tauri::command]
pub fn switch_vanilla_bedrock_version(version: String) -> Result<(), String> {
    let manifest = vanilla_bedrock_version_dir(&version).join("AppxManifest.xml");
    if !manifest.is_file() {
        return Err("Version not downloaded. Download it first.".into());
    }

    gl_bedrock::wu::unregister_current().map_err(|e| e.to_string())?;
    gl_bedrock::wu::register_version(&manifest).map_err(|e| e.to_string())
}

/// The installed Minecraft Bedrock build for the footer's version chip.
#[tauri::command]
pub async fn installed_bedrock_version() -> gl_launch::bedrock::InstalledBedrockVersion {
    gl_launch::bedrock::installed_version().await
}
