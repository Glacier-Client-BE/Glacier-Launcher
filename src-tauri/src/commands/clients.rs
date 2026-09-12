use std::path::PathBuf;

use gl_clients::levilamina::LeviLaminaRelease;
use gl_clients::oderso::DllEntry as OderSoDllEntry;
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    PathBuf::from(path).exists()
}

#[tauri::command]
pub async fn fetch_levilamina_release(
    state: State<'_, AppState>,
) -> Result<LeviLaminaRelease, String> {
    gl_clients::levilamina::fetch_latest_release(&state.http)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_flarial_dll_up_to_date(
    state: State<'_, AppState>,
    dll_path: String,
) -> Result<bool, String> {
    let remote = gl_clients::flarial::fetch_remote_hash(&state.http)
        .await
        .map_err(|e| e.to_string())?;
    let local =
        gl_clients::flarial::local_hash(&PathBuf::from(dll_path)).map_err(|e| e.to_string())?;
    Ok(gl_clients::flarial::is_up_to_date(&local, &remote))
}

fn flarial_path(state: &AppState) -> PathBuf {
    state
        .clients_dir
        .join("Flarial")
        .join(gl_clients::flarial::FILE_NAME)
}

/// Downloads (or re-downloads) the Flarial client DLL, verifying it against
/// the published release hash. Mirrors `FlarialService.DownloadAsync`.
#[tauri::command]
pub async fn download_flarial_client(state: State<'_, AppState>) -> Result<String, String> {
    let remote_hash = gl_clients::flarial::fetch_remote_hash(&state.http)
        .await
        .map_err(|e| e.to_string())?;
    let dest = flarial_path(&state);

    let req = gl_download::DownloadRequest::new(gl_clients::flarial::DOWNLOAD_URL, &dest)
        .with_sha256(remote_hash)
        .with_label("Flarial Client");
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn flarial_client_path(state: State<'_, AppState>) -> Result<String, String> {
    Ok(flarial_path(&state).to_string_lossy().into_owned())
}

/// Whether the Flarial DLL is actually present on disk. `flarial_client_path`
/// returns the intended path whether or not it exists, so it can't answer
/// this. Mirrors `oderso_client_installed`; together they drive the top-bar
/// Clients badge (`clientsHasBadge` in `Home.razor.cs:363`).
#[tauri::command]
pub fn flarial_client_installed(state: State<'_, AppState>) -> bool {
    flarial_path(&state).is_file()
}

#[tauri::command]
pub fn delete_flarial_client(state: State<'_, AppState>) -> Result<(), String> {
    let path = flarial_path(&state);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn oderso_dll_path(state: &AppState, entry: &OderSoDllEntry) -> PathBuf {
    state
        .clients_dir
        .join("OderSo")
        .join(format!("OderSo_{}", entry.name))
}

fn oderso_dir(state: &AppState) -> PathBuf {
    state.clients_dir.join("OderSo")
}

/// `OderSo.ver`, holding `<name>|<sha>` for the entry the user last set
/// active. Port of `OderSoService.VerPath` - the same single-line pipe format
/// so an existing install keeps working.
fn oderso_ver_path(state: &AppState) -> PathBuf {
    oderso_dir(state).join("OderSo.ver")
}

/// Port of `GetStoredEntry`: returns `(name, sha)` for the active entry, or
/// `None` when the file is missing or malformed. Never errors - a corrupt
/// marker means "nothing active", exactly as the C# `catch { return null; }`.
fn oderso_stored_entry(state: &AppState) -> Option<(String, String)> {
    let text = std::fs::read_to_string(oderso_ver_path(state)).ok()?;
    let mut parts = text.trim().split('|');
    let name = parts.next()?.to_string();
    let sha = parts.next()?.to_string();
    Some((name, sha))
}

fn oderso_entry_path(state: &AppState, name: &str) -> PathBuf {
    oderso_dir(state).join(format!("OderSo_{name}"))
}

/// Which OderSo DLL is currently marked active, by repo file name. Port of
/// `GetActiveEntry().Name`; the Versions panel draws the `Active` badge and
/// `oderso-active-row` class from this.
#[tauri::command]
pub fn active_oderso_dll(state: State<'_, AppState>) -> Option<String> {
    oderso_stored_entry(&state).map(|(name, _)| name)
}

/// Whether one specific repo entry has been downloaded. Port of
/// `IsEntryDownloaded`. Takes the repo file name so the panel can ask per row
/// instead of only "is *something* installed".
#[tauri::command]
pub fn oderso_dll_installed(state: State<'_, AppState>, name: String) -> bool {
    oderso_entry_path(&state, &name).is_file()
}

/// Downloads one specific repo entry. Port of `DownloadEntryAsync`.
///
/// Distinct from [`download_oderso_client`], which is
/// `InstallOrUpdateAsync`'s "grab the newest" path: that one ignores which row
/// the user clicked, so the Versions panel needs this to honour the choice.
#[tauri::command]
pub async fn download_oderso_dll(
    state: State<'_, AppState>,
    name: String,
) -> Result<String, String> {
    let entries = gl_clients::oderso::list_dlls(&state.http)
        .await
        .map_err(|e| e.to_string())?;
    let entry = entries
        .into_iter()
        .find(|e| e.name == name)
        .ok_or_else(|| format!("'{name}' is no longer listed in MasonOderSo/oderso-data."))?;

    let dest = oderso_dll_path(&state, &entry);
    let req = gl_download::DownloadRequest::new(gl_clients::oderso::raw_url(&entry), &dest)
        .with_known_total_bytes(entry.size)
        .with_label(format!("OderSo {}", entry.name));
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().into_owned())
}

/// Marks one entry active and returns its DLL path, ready to inject. Port of
/// `SetActiveEntry` plus `GetEntryFilePath`, which `HandleOderSoEntryLaunch`
/// calls back to back.
#[tauri::command]
pub async fn set_active_oderso_dll(
    state: State<'_, AppState>,
    name: String,
) -> Result<String, String> {
    let path = oderso_entry_path(&state, &name);
    if !path.is_file() {
        return Err(format!("'{name}' is not downloaded."));
    }

    // The sha comes from the live listing so the stored marker stays
    // comparable with `IsUpToDateAsync`; a listing failure must not stop the
    // user launching a DLL they already have, so it degrades to an empty sha.
    let sha = gl_clients::oderso::list_dlls(&state.http)
        .await
        .ok()
        .and_then(|entries| entries.into_iter().find(|e| e.name == name))
        .map(|e| e.sha)
        .unwrap_or_default();

    let dir = oderso_dir(&state);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(oderso_ver_path(&state), format!("{name}|{sha}")).map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().into_owned())
}

/// Deletes one entry's DLL, and clears the active marker if it pointed at it.
/// Port of `DeleteEntry`. A missing file is not an error - the row just ends
/// up in the state the caller wanted.
#[tauri::command]
pub fn delete_oderso_dll(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let path = oderso_entry_path(&state, &name);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    if let Some((stored, _)) = oderso_stored_entry(&state) {
        if stored == name {
            let _ = std::fs::remove_file(oderso_ver_path(&state));
        }
    }
    Ok(())
}

/// Whether the active OderSo entry's sha still matches the newest listed
/// entry. Mirrors `OderSoService.IsUpToDateAsync`: nothing active or nothing
/// downloaded reports not-up-to-date, and a listing failure fails open
/// (reports up-to-date) so a network hiccup doesn't nag the user, exactly
/// like the C# `catch { return true; }`.
#[tauri::command]
pub async fn check_oderso_up_to_date(state: State<'_, AppState>) -> Result<bool, String> {
    let Some((name, sha)) = oderso_stored_entry(&state) else {
        return Ok(false);
    };
    if !oderso_entry_path(&state, &name).is_file() {
        return Ok(false);
    }
    let up_to_date = gl_clients::oderso::list_dlls(&state.http)
        .await
        .ok()
        .and_then(|entries| entries.into_iter().next())
        .map(|latest| latest.sha == sha)
        .unwrap_or(true);
    Ok(up_to_date)
}

/// Lists the `.dll` releases published in the OderSo data repo, newest first.
/// Mirrors `OderSoService.ListDllsAsync`.
#[tauri::command]
pub async fn list_oderso_dlls(state: State<'_, AppState>) -> Result<Vec<OderSoDllEntry>, String> {
    gl_clients::oderso::list_dlls(&state.http)
        .await
        .map_err(|e| e.to_string())
}

/// Downloads the latest OderSo client DLL. Mirrors `OderSoService.DownloadAsync`.
#[tauri::command]
pub async fn download_oderso_client(state: State<'_, AppState>) -> Result<String, String> {
    let entries = gl_clients::oderso::list_dlls(&state.http)
        .await
        .map_err(|e| e.to_string())?;
    let entry = entries
        .into_iter()
        .next()
        .ok_or_else(|| "No .dll files found in MasonOderSo/oderso-data.".to_string())?;

    let dest = oderso_dll_path(&state, &entry);
    let req = gl_download::DownloadRequest::new(gl_clients::oderso::raw_url(&entry), &dest)
        .with_known_total_bytes(entry.size)
        .with_label("OderSo Client");
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn oderso_client_installed(state: State<'_, AppState>) -> bool {
    let dir = state.clients_dir.join("OderSo");
    dir.is_dir()
        && std::fs::read_dir(&dir)
            .map(|mut e| e.next().is_some())
            .unwrap_or(false)
}

/// Returns the on-disk path to the installed OderSo client DLL.
#[tauri::command]
pub fn oderso_client_path(state: State<'_, AppState>) -> Result<String, String> {
    let dir = state.clients_dir.join("OderSo");
    let entry = std::fs::read_dir(&dir)
        .map_err(|_| "OderSo client is not installed.".to_string())?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path()
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("dll"))
                .unwrap_or(false)
        })
        .ok_or_else(|| "OderSo client is not installed.".to_string())?;
    Ok(entry.path().to_string_lossy().into_owned())
}

#[tauri::command]
pub fn delete_oderso_client(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.clients_dir.join("OderSo");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn levilamina_dir(state: &AppState) -> PathBuf {
    state.clients_dir.join("LeviLamina")
}

/// Downloads the latest LeviLamina release archive and extracts it in place.
/// Mirrors the download half of `LeviLaminaService.InstallOrUpdateAsync`;
/// injection itself is `gl-injection`'s job, invoked separately.
#[tauri::command]
pub async fn download_levilamina_client(state: State<'_, AppState>) -> Result<String, String> {
    let release = gl_clients::levilamina::fetch_latest_release(&state.http)
        .await
        .map_err(|e| e.to_string())?;
    if release.zip_url.is_empty() {
        return Err("No Windows release asset found for LeviLamina.".into());
    }

    let dir = levilamina_dir(&state);
    let zip_path = dir.join(format!("{}.zip", release.tag));

    let req = gl_download::DownloadRequest::new(&release.zip_url, &zip_path)
        .with_label("LeviLamina");
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    extract_zip(&zip_path, &dir).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&zip_path);
    std::fs::write(dir.join(".version"), &release.tag).map_err(|e| e.to_string())?;

    gl_clients::levilamina::find_loader_dll(&dir)
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or_else(|| "LeviLamina archive did not contain a loader DLL.".to_string())
}

#[tauri::command]
pub fn levilamina_client_installed(state: State<'_, AppState>) -> bool {
    levilamina_dir(&state).join(".version").is_file()
}

/// Whether the installed LeviLamina's `.version` tag matches the latest
/// GitHub release tag. Mirrors `LeviLaminaService.IsUpToDateAsync`: not
/// installed reports not-up-to-date, and a failed release fetch fails
/// closed (reports not-up-to-date) exactly like the C# `catch { return
/// false; }` - unlike OderSo's fail-open, this one nags on network errors.
#[tauri::command]
pub async fn check_levilamina_up_to_date(state: State<'_, AppState>) -> Result<bool, String> {
    let dir = levilamina_dir(&state);
    let version_path = dir.join(".version");
    if !version_path.is_file() {
        return Ok(false);
    }
    let local_tag = std::fs::read_to_string(&version_path).unwrap_or_default();
    let local_tag = local_tag.trim();

    Ok(gl_clients::levilamina::fetch_latest_release(&state.http)
        .await
        .map(|release| gl_clients::levilamina::is_up_to_date(local_tag, &release.tag))
        .unwrap_or(false))
}

/// Returns the on-disk path to the installed LeviLamina loader DLL.
#[tauri::command]
pub fn levilamina_client_path(state: State<'_, AppState>) -> Result<String, String> {
    let dir = levilamina_dir(&state);
    gl_clients::levilamina::find_loader_dll(&dir)
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or_else(|| "LeviLamina is not installed.".to_string())
}

#[tauri::command]
pub fn delete_levilamina_client(state: State<'_, AppState>) -> Result<(), String> {
    let dir = levilamina_dir(&state);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn extract_zip(zip_path: &std::path::Path, dest_dir: &std::path::Path) -> std::io::Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    archive
        .extract(dest_dir)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
