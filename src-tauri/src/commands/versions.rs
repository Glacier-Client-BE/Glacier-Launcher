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
#[tauri::command(async)]
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
#[tauri::command(async)]
pub fn delete_latite_version(state: State<'_, AppState>, tag: String) -> Result<(), String> {
    super::safe_file_name(&gl_clients::latite::dll_file_name(&tag))?;
    let path = latite_dll_path(&state, &tag);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn latite_version_dll_path(state: State<'_, AppState>, tag: String) -> String {
    latite_dll_path(&state, &tag).to_string_lossy().into_owned()
}

/// Mirrors `VanillaVersionService.EnsureDiskSpace`: requires 4x the download
/// size free (room for the temp `.appx` plus its extracted contents plus
/// slack), erroring out before the download starts rather than partway
/// through. `download_bytes <= 0` (unknown size) is a no-op, same as C#.
fn ensure_disk_space(path: &std::path::Path, download_bytes: i64) -> Result<(), String> {
    if download_bytes <= 0 {
        return Ok(());
    }
    let Some(free) = available_free_space(path) else {
        // Matches the C#'s `catch { }` around the whole check: if we can't
        // determine free space, don't block the download over it.
        return Ok(());
    };
    let required = (download_bytes as u64).saturating_mul(4);
    if free < required {
        let gb = |b: u64| b as f64 / 1_073_741_824.0;
        return Err(format!(
            "Not enough free space — about {:.1} GB is needed and {:.1} GB is available.",
            gb(required),
            gb(free)
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn available_free_space(path: &std::path::Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            dir_name: *const u16,
            free_bytes_avail: *mut u64,
            total_bytes: *mut u64,
            total_free_bytes: *mut u64,
        ) -> i32;
    }

    let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let wide: Vec<u16> = absolute
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free) };
    if ok != 0 {
        Some(free)
    } else {
        None
    }
}

#[cfg(not(windows))]
fn available_free_space(_path: &std::path::Path) -> Option<u64> {
    None
}

fn version_cache_path() -> PathBuf {
    gl_settings::launcher_root()
        .join("cache")
        .join("mc-versions.json")
}

/// Persists the merged version list so a total fetch failure (offline, all
/// three sources unreachable) can still show the last-known list instead of
/// an empty one. Mirrors `SaveVersionCache` — best-effort, errors ignored.
fn save_version_cache(versions: &[WuVersion]) {
    let path = version_cache_path();
    let Some(dir) = path.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    if let Ok(json) = serde_json::to_string_pretty(versions) {
        let _ = std::fs::write(&path, json);
    }
}

/// Mirrors `LoadVersionCache` — a missing/corrupt cache file is not an
/// error, it just means "nothing to fall back to".
fn load_version_cache() -> Vec<WuVersion> {
    let Ok(text) = std::fs::read_to_string(version_cache_path()) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Lists installable vanilla Bedrock builds from all three sources, merged.
/// Mirrors `VanillaVersionService.GetVersionsAsync` (:49-99).
///
/// Windows Update alone stops at 1.21.120 — after the GDK move newer builds
/// were never published there — and WU's live sync only returns *currently
/// offered* updates, dropping builds it has stopped offering. The community
/// version-ID database (`community_db`) and the community GDK archive
/// (`gdk`) each fill in a different gap. Merge key is `version|channel`,
/// matching the C# `MergeKey`; sources are merged in the same order the C#
/// applies them (community DB, then Store/WU, then GDK archive), so a later
/// source wins a tie exactly as `merged[MergeKey(v)] = v` does.
///
/// Any subset of sources failing is survivable: whatever the others returned
/// is still shown. If *all three* fail (or all return empty), the last
/// successfully merged list is loaded from disk instead of showing nothing.
#[tauri::command]
pub async fn list_vanilla_bedrock_versions(
    state: State<'_, AppState>,
) -> Result<Vec<WuVersion>, String> {
    use std::collections::HashMap;

    let community = gl_bedrock::community_db::list_versions(&state.http).await;
    let wu = gl_bedrock::wu::list_versions(&state.http).await;
    let gdk = gl_bedrock::gdk::list_versions(&state.http).await;

    let mut errors = Vec::new();
    if let Err(e) = &community {
        errors.push(format!("Community DB failed: {e}"));
    }
    if let Err(e) = &wu {
        errors.push(e.to_string());
    }
    if let Err(e) = &gdk {
        errors.push(format!("GDK archive failed: {e}"));
    }

    let mut merged: HashMap<String, WuVersion> = HashMap::new();
    for v in community.unwrap_or_default() {
        merged.insert(merge_key(&v), v);
    }
    for v in wu.unwrap_or_default() {
        merged.insert(merge_key(&v), v);
    }
    for v in gdk.unwrap_or_default() {
        merged.insert(merge_key(&v), v);
    }

    let mut versions: Vec<WuVersion> = merged.into_values().collect();
    versions.sort_by_key(|v| std::cmp::Reverse(gl_bedrock::wu::version_key(&v.version)));

    if !versions.is_empty() {
        save_version_cache(&versions);
        Ok(versions)
    } else {
        let cached = load_version_cache();
        if !cached.is_empty() {
            Ok(cached)
        } else if !errors.is_empty() {
            Err(errors.join(" | "))
        } else {
            Ok(versions)
        }
    }
}

/// Port of `VanillaVersionService.MergeKey` (:119) — `Version|Channel`,
/// compared case-insensitively as the C# dictionary is.
fn merge_key(v: &WuVersion) -> String {
    format!("{}|{}", v.version.to_lowercase(), v.channel.to_lowercase())
}

/// Whether a given vanilla Bedrock build has already been downloaded and
/// extracted locally.
#[tauri::command(async)]
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
    // Port of the `IsGdk` branch in `DownloadVersionAsync` (:204-218): archive
    // entries carry a direct URL and a published hash; Windows Update entries
    // resolve theirs from the update id and have nothing to verify against.
    let is_gdk = version.source.eq_ignore_ascii_case(gl_bedrock::gdk::SOURCE);
    let (url, expected_sha) = if is_gdk {
        if version.download_url.is_empty() {
            return Err(
                "This archive entry has no download URL. Refresh the list and try again."
                    .to_string(),
            );
        }
        let sha = if version.sha256.is_empty() {
            None
        } else {
            Some(version.sha256.clone())
        };
        (version.download_url.clone(), sha)
    } else {
        let resolved = gl_bedrock::wu::get_download_url(&state.http, &version.update_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Could not resolve a download URL for this version.".to_string())?;
        (resolved, None)
    };

    let versions_dir = vanilla_bedrock_versions_dir();
    std::fs::create_dir_all(&versions_dir).map_err(|e| e.to_string())?;

    ensure_disk_space(&versions_dir, version.size_bytes as i64)?;

    let temp_file = versions_dir.join(format!("{}.appx.tmp", version.version));
    let version_dir = vanilla_bedrock_version_dir(&version.version);

    let mut req = gl_download::DownloadRequest::new(&url, &temp_file)
        .with_label(format!("Bedrock {}", version.version));
    if version.size_bytes > 0 {
        req = req.with_known_total_bytes(version.size_bytes);
    }
    if let Some(sha) = expected_sha {
        // The archive is a third-party mirror, so its published hash is the
        // only integrity check available; the C# passes it the same way.
        req = req.with_sha256(sha);
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
#[tauri::command(async)]
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
#[tauri::command(async)]
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
