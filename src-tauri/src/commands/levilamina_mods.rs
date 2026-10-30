//! Thin wrappers around `gl_clients::levilamina_mods` for the "LeviLamina
//! Mods" panel (Bedrock-side LeviLamina mod-loader mod browser, distinct
//! from the Java Fabric/Forge mods panel in `java_mods.rs`). Mirrors
//! `Services/LeviLaminaModsService.cs`.
//!
//! Installs follow the original exactly: resolve the release asset, download
//! it to a temp file through the shared queue, wipe the destination folder,
//! then extract it if it's a `.zip` and copy it verbatim otherwise.

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

/// Port of `LeviLaminaModsService.ModsDirectory` (:17-18) —
/// `LeviLaminaService.LeviLaminaDirectory + "/plugins"`. This is the folder
/// the loader itself reads; anywhere else and an installed mod simply never
/// loads.
fn mods_root(state: &AppState) -> PathBuf {
    state.clients_dir.join("LeviLamina").join("plugins")
}

/// Port of the destination in `DownloadAndInstallAsync` (:102):
/// `SanitizeFolderName(mod.RepoName)` — the repo NAME only, not
/// owner-qualified. Two same-named repos from different owners collide, which
/// is what the original does; `levModsInstallingRepo` in the panel keys on
/// `RepoName` alone for the same reason.
fn mod_install_dir(state: &AppState, repo_name: &str) -> PathBuf {
    mods_root(state).join(levilamina_mods::sanitize_folder_name(repo_name))
}

/// Port of `LeviLaminaModsService.IsInstalled` (:41) — a plain
/// `Directory.Exists`. The rewrite additionally required the folder to be
/// non-empty, which would report a mod as missing after a partial extract
/// where the original reports it as present.
///
/// `repo_owner` is accepted but unused: the install path is repo-name-only,
/// matching the original. Keeping the parameter avoids a frontend change and
/// leaves room to disambiguate later without another signature break.
#[tauri::command(async)]
pub fn levilamina_mod_installed(
    state: State<'_, AppState>,
    repo_owner: String,
    repo_name: String,
) -> bool {
    let _ = repo_owner;
    mod_install_dir(&state, &repo_name).is_dir()
}

/// Full port of `LeviLaminaModsService.DownloadAndInstallAsync` (:52-121):
/// resolve the release asset, download it to a temp file, replace the
/// destination folder, then extract a `.zip` or copy any other asset in
/// verbatim.
///
/// `mod_name` is the display name, used only as the download-queue label so it
/// matches the original's `label: mod.Name`.
#[tauri::command]
pub async fn download_levilamina_mod(
    state: State<'_, AppState>,
    repo_owner: String,
    repo_name: String,
    latest_version: String,
    mod_name: Option<String>,
) -> Result<String, String> {
    let asset = levilamina_mods::resolve_latest_asset(
        &state.http,
        &repo_owner,
        &repo_name,
        &latest_version,
    )
    .await
    .map_err(|e| e.to_string())?;

    let dest_dir = mod_install_dir(&state, &repo_name);
    let is_zip = asset.asset_name.to_ascii_lowercase().ends_with(".zip");

    // Download to a temp file first, exactly as the C# does, so a failed or
    // cancelled download can't leave a half-written file sitting in the
    // plugins folder where the loader would try to read it.
    let ext = if is_zip {
        ".zip".to_string()
    } else {
        std::path::Path::new(&asset.asset_name)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default()
    };
    let tmp = std::env::temp_dir().join(format!(
        "glacier_levimod_{}_{}{}",
        std::process::id(),
        levilamina_mods::sanitize_folder_name(&repo_name),
        ext
    ));

    let label = mod_name.unwrap_or_else(|| repo_name.clone());
    let req = gl_download::DownloadRequest::new(&asset.asset_url, &tmp).with_label(label);
    let handle = state.downloads.submit(req);
    let download_result = handle.join().await;

    let install = (|| -> Result<String, String> {
        download_result.map_err(|e| e.to_string())?;

        if dest_dir.exists() {
            std::fs::remove_dir_all(&dest_dir).map_err(|e| e.to_string())?;
        }
        std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

        if is_zip {
            extract_zip(&tmp, &dest_dir).map_err(|e| e.to_string())?;
        } else {
            std::fs::copy(&tmp, dest_dir.join(&asset.asset_name)).map_err(|e| e.to_string())?;
        }
        Ok(dest_dir.to_string_lossy().into_owned())
    })();

    // The C# deletes the temp file in a `finally` and swallows any failure.
    let _ = std::fs::remove_file(&tmp);
    install
}

/// Same helper as `clients.rs` uses for the client archives. `ZipArchive::extract`
/// already rejects entries that escape the destination, so this is not a
/// zip-slip vector.
fn extract_zip(zip_path: &std::path::Path, dest_dir: &std::path::Path) -> std::io::Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    archive
        .extract(dest_dir)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[tauri::command(async)]
pub fn delete_levilamina_mod(
    state: State<'_, AppState>,
    repo_owner: String,
    repo_name: String,
) -> Result<(), String> {
    let _ = repo_owner;
    // The sanitized folder name must be a single plain path component;
    // otherwise `..` (or `...`, which Windows reduces to the parent) would
    // point remove_dir_all at the clients folder itself.
    let folder = levilamina_mods::sanitize_folder_name(&repo_name);
    let folder_trimmed = folder.trim_end_matches('.');
    if folder_trimmed.is_empty() {
        return Err(format!("Refusing unsafe mod folder name: {repo_name:?}"));
    }
    super::safe_file_name(&folder)?;
    let dir = mod_install_dir(&state, &repo_name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn open_levilamina_mods_folder(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.clients_dir.join("LeviLaminaMods");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
