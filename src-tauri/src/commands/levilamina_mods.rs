//! Thin wrappers around `gl_clients::levilamina_mods` for the "LeviLamina
//! Mods" panel (Bedrock-side LeviLamina mod-loader mod browser, distinct
//! from the Java Fabric/Forge mods panel in `java_mods.rs`). Mirrors
//! `Services/LeviLaminaModsService.cs`.
//!
//! Not ported: the actual download/extract/install-into-mods-folder step
//! (`LeviLaminaModsService.DownloadAndInstallAsync`'s zip-extraction half).
//! `resolve_latest_asset` below resolves the downloadable asset URL; the
//! frontend uses the shared download manager (`gl_download`) to fetch it,
//! same pattern as `clients.rs`'s Flarial/OderSo downloads.

use std::path::PathBuf;
use tauri::State;

use gl_clients::levilamina_mods::{self, LeviLaminaMod};

use crate::state::AppState;

/// Fetches and parses the `lipr` registry. Mirrors
/// `LeviLaminaModsService.LoadAllAsync`.
#[tauri::command]
pub async fn fetch_levilamina_mods(
    state: State<'_, AppState>,
) -> Result<Vec<LeviLaminaMod>, String> {
    let resp = state
        .http
        .get(levilamina_mods::REGISTRY_URL)
        .header("User-Agent", "GlacierLauncher")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let body = resp.text().await.map_err(|e| e.to_string())?;
    levilamina_mods::parse_registry(&body).map_err(|e| e.to_string())
}

fn mod_install_dir(state: &AppState, repo_owner: &str, repo_name: &str) -> PathBuf {
    let folder = levilamina_mods::sanitize_folder_name(&format!("{repo_owner}_{repo_name}"));
    state.clients_dir.join("LeviLaminaMods").join(folder)
}

/// Whether a mod has already been downloaded (its install folder exists and
/// is non-empty).
#[tauri::command]
pub fn levilamina_mod_installed(
    state: State<'_, AppState>,
    repo_owner: String,
    repo_name: String,
) -> bool {
    let dir = mod_install_dir(&state, &repo_owner, &repo_name);
    dir.is_dir()
        && std::fs::read_dir(&dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
}

/// Resolves and downloads the latest release asset for a LeviLamina mod
/// into its install folder. Mirrors the asset-resolution + download half of
/// `LeviLaminaModsService.DownloadAndInstallAsync` (extraction is left as a
/// follow-up — the raw asset is downloaded as-is).
#[tauri::command]
pub async fn download_levilamina_mod(
    state: State<'_, AppState>,
    repo_owner: String,
    repo_name: String,
    latest_version: String,
) -> Result<String, String> {
    let asset = levilamina_mods::resolve_latest_asset(
        &state.http,
        &repo_owner,
        &repo_name,
        &latest_version,
    )
    .await
    .map_err(|e| e.to_string())?;

    let dest = mod_install_dir(&state, &repo_owner, &repo_name).join(&asset.asset_name);
    let req = gl_download::DownloadRequest::new(&asset.asset_url, &dest)
        .with_label(format!("{repo_owner}/{repo_name} {}", asset.tag_used));
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn delete_levilamina_mod(
    state: State<'_, AppState>,
    repo_owner: String,
    repo_name: String,
) -> Result<(), String> {
    let dir = mod_install_dir(&state, &repo_owner, &repo_name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_levilamina_mods_folder(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.clients_dir.join("LeviLaminaMods");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
