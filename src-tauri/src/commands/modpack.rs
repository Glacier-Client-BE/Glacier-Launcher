use std::path::{Path, PathBuf};

use gl_download::DownloadRequest;
use gl_modpack::manifest::{parse_curseforge_manifest, parse_modrinth_index};
use gl_modpack::models::{CfSearchResult, MrSearchResult};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

const MINECRAFT_GAME_ID: i32 = 432;
const MODPACK_CLASS_ID: i32 = 4471;

/// Which pack a Java instance was installed from, and which file/version —
/// written next to the instance on install so a later launch can ask "is
/// there a newer file for this same project?" without re-parsing anything.
/// Its absence just means the instance wasn't installed from a modpack (a
/// blank instance, or one built by hand), not an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackSource {
    source: String, // "curseforge" | "modrinth"
    project_id: String,
    file_id: String,
    pack_name: String,
}

const PACK_SOURCE_FILE: &str = ".glacier-pack.json";

fn pack_source_path(instance_root: &Path) -> PathBuf {
    instance_root.join(PACK_SOURCE_FILE)
}

fn write_pack_source(instance_root: &Path, src: &PackSource) {
    // Best-effort: a failure here just means update-checking won't work for
    // this instance later, which must not fail the install that already
    // succeeded.
    if let Ok(json) = serde_json::to_string_pretty(src) {
        let _ = std::fs::write(pack_source_path(instance_root), json);
    }
}

fn read_pack_source(instance_root: &Path) -> Option<PackSource> {
    let body = std::fs::read_to_string(pack_source_path(instance_root)).ok()?;
    serde_json::from_str(&body).ok()
}

#[derive(Debug, Clone, Serialize)]
pub struct PackManifestIds {
    pub mod_ids: Vec<String>,
}

/// Snapshot of an instance's `mods/` file names, used only to diff
/// before/after an `update_modpack` call so the UI can show what changed.
#[tauri::command]
pub async fn read_pack_manifest_ids(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<PackManifestIds, String> {
    let instance_root = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        let instance = svc
            .instances()
            .into_iter()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| "instance not found".to_string())?;
        PathBuf::from(instance.directory)
    };
    let mods_dir = instance_root.join("mods");
    let mut mod_ids: Vec<String> = std::fs::read_dir(&mods_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    mod_ids.sort();
    Ok(PackManifestIds { mod_ids })
}

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
            Ok(Some(resolved)) if super::safe_file_name(&resolved.file_name).is_ok() => {
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

    write_pack_source(
        &instance_root,
        &PackSource {
            source: "curseforge".into(),
            project_id: mod_id.to_string(),
            file_id: pack_file.file_id.to_string(),
            pack_name: pack_name.clone(),
        },
    );

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

    write_pack_source(
        &instance_root,
        &PackSource {
            source: "modrinth".into(),
            project_id: project_id.clone(),
            file_id: version.id.clone(),
            pack_name: pack_name.clone(),
        },
    );

    Ok(ModpackInstallResult {
        instance_id: instance.id,
        instance_name: instance.name,
        files_installed: installed,
        files_failed: failed,
    })
}

/// One instance's modpack-update status, for the startup auto-update sweep
/// and (eventually) a manual "check for pack updates" button.
#[derive(Debug, Clone, Serialize)]
pub struct ModpackUpdateStatus {
    pub instance_id: String,
    pub instance_name: String,
    pub source: String,
    pub pack_name: String,
    pub has_update: bool,
}

/// Checks every Java instance that was installed from a CurseForge/Modrinth
/// modpack (tracked via the `.glacier-pack.json` sidecar `install_*_modpack`
/// writes) for a newer file/version of the same project. Read-only — doesn't
/// touch any instance files. CurseForge instances are skipped (not reported
/// as having no update — just omitted) when `curseforge_api_key` is blank,
/// since checking would just fail.
#[tauri::command]
pub async fn check_modpack_updates(
    state: State<'_, AppState>,
    curseforge_api_key: String,
) -> Result<Vec<ModpackUpdateStatus>, String> {
    let instances = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        svc.instances()
    };

    let mr = gl_modpack::ModrinthClient::new(state.http.clone());
    let cf = (!curseforge_api_key.trim().is_empty())
        .then(|| gl_modpack::CurseForgeClient::new(state.http.clone(), curseforge_api_key));

    let mut out = Vec::new();
    for instance in instances {
        let root = PathBuf::from(&instance.directory);
        let Some(src) = read_pack_source(&root) else {
            continue;
        };

        let latest_file_id = match src.source.as_str() {
            "modrinth" => mr
                .get_latest_version(&src.project_id)
                .await
                .ok()
                .flatten()
                .map(|v| v.id),
            "curseforge" => {
                let Some(cf) = &cf else { continue };
                let Ok(project_id) = src.project_id.parse::<i32>() else {
                    continue;
                };
                cf.get_files(project_id)
                    .await
                    .ok()
                    .and_then(|files| files.first().map(|f| f.file_id.to_string()))
            }
            _ => None,
        };

        let Some(latest_file_id) = latest_file_id else {
            continue;
        };

        out.push(ModpackUpdateStatus {
            instance_id: instance.id,
            instance_name: instance.name,
            source: src.source,
            pack_name: src.pack_name,
            has_update: latest_file_id != src.file_id,
        });
    }

    Ok(out)
}

/// Updates an already-installed modpack instance in place: re-resolves the
/// pack's newest file/version, re-downloads its mod files into the existing
/// `mods/` folder (overwriting same-named files, adding new ones — nothing
/// outside the pack's own file list is touched, so mods the user added by
/// hand are left alone), re-extracts `overrides/` over the instance root
/// (matching how other launchers apply pack updates — pack-shipped configs
/// are expected to change with the pack), and rewrites the sidecar with the
/// new file id. Unlike `install_*_modpack`, this never creates a new
/// instance or deletes anything.
#[tauri::command]
pub async fn update_modpack(
    state: State<'_, AppState>,
    instance_id: String,
    curseforge_api_key: String,
) -> Result<ModpackInstallResult, String> {
    let instance = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        svc.instances()
            .into_iter()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| "instance not found".to_string())?
    };
    let instance_root = PathBuf::from(&instance.directory);
    let src = read_pack_source(&instance_root)
        .ok_or_else(|| "this instance wasn't installed from a tracked modpack".to_string())?;

    match src.source.as_str() {
        "modrinth" => {
            let mr = gl_modpack::ModrinthClient::new(state.http.clone());
            let version = mr
                .get_latest_version(&src.project_id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "No downloadable .mrpack version found.".to_string())?;

            let scratch = scratch_dir("mr-update");
            std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
            let mrpack_path = scratch.join("pack.mrpack");
            let req = DownloadRequest::new(version.url.clone(), mrpack_path.clone())
                .with_label("Modpack update");
            state
                .downloads
                .submit(req)
                .join()
                .await
                .map_err(|e| e.to_string())?;

            let plan = {
                let file = std::fs::File::open(&mrpack_path).map_err(|e| e.to_string())?;
                let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
                parse_modrinth_index(&mut archive, &src.pack_name).map_err(|e| e.to_string())?
            };
            extract_overrides(&mrpack_path, &instance_root)?;

            let mut reqs = Vec::new();
            for f in &plan.files {
                if f.url.is_empty() || f.relative_path.is_empty() {
                    continue;
                }
                let dest = instance_root.join(&f.relative_path);
                reqs.push(DownloadRequest::new(f.url.clone(), dest).with_label(f.relative_path.clone()));
            }
            let outcomes = state.downloads.download_all(reqs).await;
            let failed = outcomes.iter().filter(|o| o.is_err()).count();
            let installed = outcomes.len() - failed;
            let _ = std::fs::remove_dir_all(&scratch);

            write_pack_source(
                &instance_root,
                &PackSource { file_id: version.id, ..src },
            );

            Ok(ModpackInstallResult {
                instance_id: instance.id,
                instance_name: instance.name,
                files_installed: installed,
                files_failed: failed,
            })
        }
        "curseforge" => {
            if curseforge_api_key.trim().is_empty() {
                return Err("No CurseForge API key set in Settings.".to_string());
            }
            let cf = gl_modpack::CurseForgeClient::new(state.http.clone(), curseforge_api_key);
            let project_id: i32 = src
                .project_id
                .parse()
                .map_err(|_| "corrupt pack source record".to_string())?;
            let files = cf.get_files(project_id).await.map_err(|e| e.to_string())?;
            let pack_file = files
                .first()
                .ok_or_else(|| "This modpack has no downloadable files.".to_string())?;

            let scratch = scratch_dir("cf-update");
            std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
            let zip_path = scratch.join("pack.zip");
            let req = DownloadRequest::new(pack_file.download_url.clone(), zip_path.clone())
                .with_label("Modpack update");
            state
                .downloads
                .submit(req)
                .join()
                .await
                .map_err(|e| e.to_string())?;

            let plan = {
                let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
                let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
                parse_curseforge_manifest(&mut archive, &src.pack_name).map_err(|e| e.to_string())?
            };
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
                    Ok(Some(resolved)) if super::safe_file_name(&resolved.file_name).is_ok() => {
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

            let new_file_id = pack_file.file_id.to_string();
            write_pack_source(
                &instance_root,
                &PackSource { file_id: new_file_id, ..src },
            );

            Ok(ModpackInstallResult {
                instance_id: instance.id,
                instance_name: instance.name,
                files_installed: installed,
                files_failed: resolved_failures + download_failures,
            })
        }
        other => Err(format!("unknown pack source: {other}")),
    }
}
