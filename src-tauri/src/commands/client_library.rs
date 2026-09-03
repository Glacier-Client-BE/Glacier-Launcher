//! Thin wrappers around `gl_clients::client_library` for the "Client
//! Library" panel: a GitHub-backed catalog of community resource-pack
//! style clients (Indigo-Tools/ClientLibrary), distinct from the
//! `utilities.rs` custom DLL/EXE library. Ported from
//! `Home.ClientLibrary.cs` / `ClientLibraryService.cs`.

use std::path::{Path, PathBuf};

use gl_clients::client_library as lib;
use gl_clients::models::{ClientLibraryDownload, ClientLibraryPack};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn client_library_site_url() -> String {
    lib::site_url()
}

/// Fetches and parses the `paths.json` catalog index. `refresh` exists to
/// mirror the C# signature (`LoadClientLibraryAsync(bool refresh)`); there's
/// no in-process cache layer here yet, so every call re-fetches.
#[tauri::command]
pub async fn get_client_library_packs(
    state: State<'_, AppState>,
    _refresh: bool,
) -> Result<Vec<ClientLibraryPack>, String> {
    let body = state
        .http
        .get(lib::index_url())
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    lib::parse_index(&body).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn client_library_versions(packs: Vec<ClientLibraryPack>) -> Vec<String> {
    lib::versions(&packs)
}

fn resource_packs_dir(instance_dir: &str) -> PathBuf {
    Path::new(instance_dir).join("resource_packs")
}

fn installed_path(instance_dir: &str, download: &ClientLibraryDownload) -> PathBuf {
    // The archive is installed under a folder named after the pack file
    // (minus extension), mirroring how Minecraft expects an extracted
    // resource pack folder rather than a loose .mcpack, while still using
    // the .mcpack/.zip's stem as a stable per-download identity key.
    let stem = Path::new(&download.file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&download.file_name);
    resource_packs_dir(instance_dir).join(stem)
}

#[tauri::command]
pub fn is_client_library_download_installed(
    instance_dir: String,
    download: ClientLibraryDownload,
) -> bool {
    installed_path(&instance_dir, &download).exists()
}

/// Downloads a pack's `.mcpack`/`.mcaddon`/`.zip` (from the Git-LFS media
/// host, since raw.githubusercontent.com only serves LFS pointer files for
/// these) and extracts it into `<instance_dir>/resource_packs/<name>/`.
/// Mirrors `ClientLib2.InstallAsync`.
#[tauri::command]
pub async fn install_client_library_download(
    state: State<'_, AppState>,
    instance_dir: String,
    download: ClientLibraryDownload,
) -> Result<String, String> {
    let dest_dir = installed_path(&instance_dir, &download);
    std::fs::create_dir_all(resource_packs_dir(&instance_dir)).map_err(|e| e.to_string())?;

    let zip_path = resource_packs_dir(&instance_dir).join(format!("_dl_{}", download.file_name));
    let url = lib::media_url(&download.repo_path);
    let req = gl_download::DownloadRequest::new(&url, &zip_path).with_label(download.label.clone());
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    if dest_dir.exists() {
        std::fs::remove_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(&dest_dir).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&zip_path);

    Ok(dest_dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn uninstall_client_library_download(
    instance_dir: String,
    download: ClientLibraryDownload,
) -> Result<(), String> {
    let dest_dir = installed_path(&instance_dir, &download);
    if dest_dir.exists() {
        std::fs::remove_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
