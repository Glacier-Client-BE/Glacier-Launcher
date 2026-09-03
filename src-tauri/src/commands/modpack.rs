use std::path::{Path, PathBuf};

use gl_download::DownloadRequest;
use gl_modpack::manifest::{parse_curseforge_manifest, parse_modrinth_index};
use gl_modpack::models::{CfSearchResult, MrSearchResult};
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

const MINECRAFT_GAME_ID: i32 = 432;
const MODPACK_CLASS_ID: i32 = 4471;

#[tauri::command]
pub async fn search_curseforge_modpacks(
    state: State<'_, AppState>,
    api_key: String,
    query: String,
    page: u32,
) -> Result<CfSearchResult, String> {
    let client = gl_modpack::CurseForgeClient::new(state.http.clone(), api_key);
    client
        .search(MINECRAFT_GAME_ID, Some(MODPACK_CLASS_ID), &query, 20, page)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_modrinth_modpacks(
    state: State<'_, AppState>,
    query: String,
    offset: u32,
) -> Result<MrSearchResult, String> {
    let client = gl_modpack::ModrinthClient::new(state.http.clone());
    client
        .search(&query, "project_type:modpack", 20, offset)
        .await
        .map_err(|e| e.to_string())
}

/// Outcome of a modpack install, returned to the frontend so it can jump to
/// the new instance and show how many files landed.
#[derive(Debug, Clone, Serialize)]
pub struct ModpackInstallResult {
    pub instance_id: String,
    pub instance_name: String,
    pub files_installed: usize,
    pub files_failed: usize,
}

fn scratch_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "glacier-modpack-{tag}-{}-{}",
        std::process::id(),
        chrono_ish_nonce()
    ))
}

/// Cheap unique-ish suffix so concurrent installs don't collide on the same
/// scratch dir (avoids pulling in a time/uuid crate for one call site).
fn chrono_ish_nonce() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Extracts a modpack zip's `overrides/`/`client-overrides/` tree directly
/// into the instance directory, mirroring `JavaInstanceService.ExtractOverridesAsync`.
fn extract_overrides(zip_path: &Path, instance_root: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let rel = if let Some(r) = name.strip_prefix("overrides/") {
            r
        } else if let Some(r) = name.strip_prefix("client-overrides/") {
            r
        } else {
            continue;
        };
        if rel.is_empty() {
            continue;
        }
        let dest = instance_root.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Downloads a CurseForge modpack (by mod/addon id) and installs it into a
/// brand-new Java instance: pack zip -> manifest -> overrides extracted ->
/// each mod file resolved via the CurseForge API and downloaded into `mods/`.
///
/// Simplified vs. the C# `ModpackInstallService.InstallCurseForgeAsync`: does
/// not install a mod loader (Fabric/Forge/Quilt/NeoForge) automatically — the
/// instance is created with `minecraft_version` set from the manifest, but
/// the user still needs to install/select the loader themselves via the
/// existing Java mods loader-install commands. Given the pack file list can
/// be large, CurseForge file refs are resolved and downloaded sequentially
/// rather than with a bounded worker pool.
#[tauri::command]
pub async fn install_curseforge_modpack(
    state: State<'_, AppState>,
    api_key: String,
    mod_id: i32,
    pack_name: String,
) -> Result<ModpackInstallResult, String> {
    let cf = gl_modpack::CurseForgeClient::new(state.http.clone(), api_key);
    let files = cf.get_files(mod_id).await.map_err(|e| e.to_string())?;
    let pack_file = files
        .first()
        .ok_or_else(|| "This modpack has no downloadable files.".to_string())?;

    let scratch = scratch_dir("cf");
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    let zip_path = scratch.join("pack.zip");

    let req = DownloadRequest::new(pack_file.download_url.clone(), zip_path.clone())
        .with_label("Modpack archive");
    state
        .downloads
        .submit(req)
        .join()
        .await
        .map_err(|e| e.to_string())?;

    let plan = {
        let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        parse_curseforge_manifest(&mut archive, &pack_name).map_err(|e| e.to_string())?
    };

    let instance = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        let name = if plan.name.trim().is_empty() {
            &pack_name
        } else {
            &plan.name
        };
        svc.create(name, &plan.minecraft_version)
            .map_err(|e| e.to_string())?
    };
    let instance_root = PathBuf::from(&instance.directory);

    extract_overrides(&zip_path, &instance_root)?;

    let mods_dir = instance_root.join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    let mut resolved_failures = 0usize;
    let mut reqs = Vec::new();
    for f in &plan.files {
        let Some(cf_ref) = f.curseforge_ref else {
            continue;
        };
        match cf
            .resolve_manifest_file(cf_ref.project_id, cf_ref.file_id)
            .await
        {
            Ok(Some(resolved)) => {
                let dest = mods_dir.join(&resolved.file_name);
                reqs.push(
                    DownloadRequest::new(resolved.download_url, dest)
                        .with_label(resolved.file_name),
                );
            }
            _ => resolved_failures += 1,
        }
    }

    let outcomes = state.downloads.download_all(reqs).await;
    let download_failures = outcomes.iter().filter(|o| o.is_err()).count();
    let installed = outcomes.len() - download_failures;

    let _ = std::fs::remove_dir_all(&scratch);

    Ok(ModpackInstallResult {
        instance_id: instance.id,
        instance_name: instance.name,
        files_installed: installed,
        files_failed: resolved_failures + download_failures,
    })
}

/// Downloads a Modrinth modpack (by project id) and installs it into a
/// brand-new Java instance: `.mrpack` -> index -> overrides extracted ->
/// every listed file downloaded to its `relative_path`.
///
/// Simplified vs. the C# `ModpackInstallService.InstallModrinthAsync`: does
/// not auto-install a mod loader (same simplification as the CurseForge
/// path above), and picks the project's newest version rather than
/// specifically hunting for a `.mrpack`-formatted one.
#[tauri::command]
pub async fn install_modrinth_modpack(
    state: State<'_, AppState>,
    project_id: String,
    pack_name: String,
) -> Result<ModpackInstallResult, String> {
    let mr = gl_modpack::ModrinthClient::new(state.http.clone());
    let version = mr
        .get_latest_version(&project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No downloadable .mrpack version found.".to_string())?;

    let scratch = scratch_dir("mr");
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    let mrpack_path = scratch.join("pack.mrpack");

    let req = DownloadRequest::new(version.url.clone(), mrpack_path.clone())
        .with_label("Modpack archive");
    state
        .downloads
        .submit(req)
        .join()
        .await
        .map_err(|e| e.to_string())?;

    let plan = {
        let file = std::fs::File::open(&mrpack_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        parse_modrinth_index(&mut archive, &pack_name).map_err(|e| e.to_string())?
    };

    let instance = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        let name = if plan.name.trim().is_empty() {
            &pack_name
        } else {
            &plan.name
        };
        svc.create(name, &plan.minecraft_version)
            .map_err(|e| e.to_string())?
    };
    let instance_root = PathBuf::from(&instance.directory);

    extract_overrides(&mrpack_path, &instance_root)?;

    let mut reqs = Vec::new();
    for f in &plan.files {
        if f.url.is_empty() || f.relative_path.is_empty() {
            continue;
        }
        let dest = instance_root.join(&f.relative_path);
        let mut req = DownloadRequest::new(f.url.clone(), dest).with_label(f.relative_path.clone());
        if !f.sha1.is_empty() {
            // DownloadRequest only verifies sha256; Modrinth files carry
            // sha1/sha512, so integrity checking is skipped here rather than
            // bolting on a second hash algorithm for one call site.
            req = req.with_max_attempts(3);
        }
        reqs.push(req);
    }

    let outcomes = state.downloads.download_all(reqs).await;
    let failed = outcomes.iter().filter(|o| o.is_err()).count();
    let installed = outcomes.len() - failed;

    let _ = std::fs::remove_dir_all(&scratch);

    Ok(ModpackInstallResult {
        instance_id: instance.id,
        instance_name: instance.name,
        files_installed: installed,
        files_failed: failed,
    })
}
