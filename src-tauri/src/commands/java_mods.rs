//! Java Edition mod-loader install + installed-mod-jar management. Ports the
//! UI-facing surface of `Services/JavaModLoaderService.cs` and
//! `Services/JavaModAnalyzer.cs`.
//!
//! Also carries several "Mods & content" additions that reuse this same
//! mods-dir plumbing: the Modrinth browse/install commands (feature 12
//! dependency warnings), the update checker (feature 11 + changelog viewer,
//! feature 16), disable-all/bisect mode (feature 14), instance diffing
//! (feature 13), and local-jar drag/drop install (feature 19).

use gl_download::DownloadRequest;
use gl_java::mod_analyzer::{self, ModInfo, ModReport};
use gl_java::mod_loader;
use gl_java::{mod_provenance, ModProvenance, ModSource};
use gl_modpack::models::MrVersion;
use gl_modpack::ModrinthClient;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use crate::state::AppState;

fn instance_mods_dir(state: &State<'_, AppState>, instance_id: &str) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.path_for_instance(instance_id, "mods"))
}

fn instance_versions_dir(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.root_dir().join("versions"))
}

fn java_root_dir(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.root_dir().to_path_buf())
}

fn instance_dir(state: &State<'_, AppState>, instance_id: &str) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.path_for_instance(instance_id, ""))
}

fn runtimes_root(_state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or_else(|| "could not resolve home directory (USERPROFILE)".to_string())?;
    Ok(gl_java::RuntimeDownloadService::default_runtimes_root(
        &home,
    ))
}

#[tauri::command]
pub fn list_java_mods(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<ModReport, String> {
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    Ok(mod_analyzer::analyze_dir(&mods_dir))
}

#[tauri::command]
pub fn set_java_mod_enabled(
    state: State<'_, AppState>,
    instance_id: String,
    file_name: String,
    enabled: bool,
) -> Result<String, String> {
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    mod_analyzer::set_mod_enabled(&mods_dir, &file_name, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_java_mod(
    state: State<'_, AppState>,
    instance_id: String,
    file_name: String,
) -> Result<(), String> {
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    mod_analyzer::delete_mod(&mods_dir, &file_name).map_err(|e| e.to_string())?;
    let _ = mod_provenance::remove(&mods_dir, &file_name);
    Ok(())
}

#[tauri::command]
pub fn open_java_mods_folder(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&mods_dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_fabric_loader(
    state: State<'_, AppState>,
    minecraft_version: String,
    loader_version: Option<String>,
) -> Result<String, String> {
    let versions_dir = instance_versions_dir(&state)?;
    let client = state.http.clone();
    mod_loader::install_fabric(
        &client,
        &versions_dir,
        &minecraft_version,
        loader_version.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_quilt_loader(
    state: State<'_, AppState>,
    minecraft_version: String,
    loader_version: Option<String>,
) -> Result<String, String> {
    let versions_dir = instance_versions_dir(&state)?;
    let client = state.http.clone();
    mod_loader::install_quilt(
        &client,
        &versions_dir,
        &minecraft_version,
        loader_version.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_forge_installer(
    state: State<'_, AppState>,
    minecraft_version: String,
    forge_version: Option<String>,
    instance_id: Option<String>,
) -> Result<String, String> {
    let root_dir = java_root_dir(&state)?;
    let client = state.http.clone();
    let path = mod_loader::download_forge_installer(
        &client,
        &root_dir,
        &minecraft_version,
        forge_version.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    if let Some(instance_id) = instance_id {
        let target_dir = instance_dir(&state, &instance_id)?;
        let runtimes_root = runtimes_root(&state)?;
        mod_loader::run_loader_installer(&path, &target_dir, &runtimes_root)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn download_neoforge_installer(
    state: State<'_, AppState>,
    minecraft_version: String,
    instance_id: Option<String>,
) -> Result<String, String> {
    let root_dir = java_root_dir(&state)?;
    let client = state.http.clone();
    let path = mod_loader::download_neoforge_installer(&client, &root_dir, &minecraft_version)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(instance_id) = instance_id {
        let target_dir = instance_dir(&state, &instance_id)?;
        let runtimes_root = runtimes_root(&state)?;
        mod_loader::run_loader_installer(&path, &target_dir, &runtimes_root)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(path.to_string_lossy().into_owned())
}

// ── Modrinth browse/install (feature 12: dependency warnings) ─────────────

/// Searches Modrinth projects of any type. `project_type` defaults to "mod"
/// (the Java "Mods & content" browse tab), but a caller can pass e.g.
/// "resourcepack" or "shader" to reuse the same search for other content.
#[tauri::command]
pub async fn search_modrinth_projects(
    state: State<'_, AppState>,
    query: String,
    project_type: Option<String>,
    offset: u32,
) -> Result<gl_modpack::models::MrSearchResult, String> {
    let client = ModrinthClient::new(state.http.clone());
    let facet = project_type.unwrap_or_else(|| "mod".to_string());
    client
        .search(&query, &facet, 20, offset)
        .await
        .map_err(|e| e.to_string())
}

/// One dependency the mod being installed declares as required, with a
/// best-effort guess at whether it's already present in the instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDependencyWarning {
    pub project_id: String,
    pub title: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallModrinthModResult {
    pub file_name: String,
    /// Required dependencies the API reports that don't look installed yet.
    /// The frontend shows these as a warning; nothing is auto-installed.
    pub dependency_warnings: Vec<ModDependencyWarning>,
}

/// Downloads a Modrinth project's newest version into an instance's `mods/`
/// folder, then reports any required dependencies that don't appear to
/// already be installed (feature 12). Does not install the dependencies.
#[tauri::command]
pub async fn install_modrinth_mod(
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
) -> Result<InstallModrinthModResult, String> {
    let client = ModrinthClient::new(state.http.clone());
    let version = client
        .get_latest_version(&project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "This project has no downloadable version.".to_string())?;

    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    let dest = mods_dir.join(&version.file_name);
    let req = DownloadRequest::new(version.url.clone(), dest).with_label(version.file_name.clone());
    state
        .downloads
        .submit(req)
        .join()
        .await
        .map_err(|e| e.to_string())?;

    let _ = mod_provenance::record(
        &mods_dir,
        ModProvenance {
            filename: version.file_name.clone(),
            source: ModSource::Modrinth,
            project_id: project_id.clone(),
            version_id: version.id.clone(),
            sha1: version.sha1.clone(),
        },
    );

    let installed = mod_analyzer::analyze_dir(&mods_dir);
    let mut warnings = Vec::new();
    for dep in &version.dependencies {
        if dep.dependency_type != "required" {
            continue;
        }
        let Some(dep_project_id) = &dep.project_id else {
            continue;
        };
        let title = client
            .get_project_title(dep_project_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| dep_project_id.clone());
        let title_lower = title.to_lowercase();
        let already_installed = installed.mods.iter().any(|m| {
            !m.disabled
                && (m.name.to_lowercase().contains(&title_lower)
                    || title_lower.contains(&m.mod_id.to_lowercase()) && !m.mod_id.is_empty())
        });
        warnings.push(ModDependencyWarning {
            project_id: dep_project_id.clone(),
            title,
            installed: already_installed,
        });
    }

    Ok(InstallModrinthModResult {
        file_name: version.file_name,
        dependency_warnings: warnings,
    })
}

// ── Update checker + changelog viewer (features 11 & 16) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModUpdateInfo {
    pub file_name: String,
    pub mod_id: String,
    pub name: String,
    pub available: MrVersion,
}

/// Batch-checks every enabled mod jar in an instance against Modrinth by
/// content hash, returning the ones with a newer matching version. Mods not
/// published on Modrinth (or CurseForge-only mods, since CurseForge's hash
/// lookup requires an API key/isn't part of the public endpoint set) are
/// silently skipped rather than reported as errors.
#[tauri::command]
pub async fn check_java_mod_updates(
    state: State<'_, AppState>,
    instance_id: String,
    loaders: Vec<String>,
    game_versions: Vec<String>,
) -> Result<Vec<ModUpdateInfo>, String> {
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    let report = mod_analyzer::analyze_dir(&mods_dir);
    let client = ModrinthClient::new(state.http.clone());
    let provenance = mod_provenance::load(&mods_dir);

    let mut updates = Vec::new();
    for m in report.mods.iter().filter(|m| !m.disabled) {
        // Prefer provenance (project_id -> latest version) over the
        // filename-guessing hash lookup: it survives a mod being renamed on
        // disk and doesn't depend on Modrinth's hash-lookup endpoint having
        // indexed this exact file. Falls back to the hash-based approach for
        // mods with no provenance record (locally dropped jars), preserving
        // the pre-existing behaviour for that path.
        let prov = provenance
            .mods
            .get(&m.file_name)
            .filter(|p| matches!(p.source, ModSource::Modrinth));

        let latest = if let Some(prov) = prov {
            match client.get_latest_version(&prov.project_id).await {
                Ok(Some(v)) if v.id != prov.version_id => Some(v),
                _ => None,
            }
        } else {
            let path = mods_dir.join(&m.file_name);
            let Ok(hash) = gl_modpack::sha1_hex_file(&path) else {
                continue;
            };
            match client
                .get_update_for_hash(&hash, &loaders, &game_versions)
                .await
            {
                Ok(Some(v)) if v.file_name != m.file_name => Some(v),
                _ => None,
            }
        };

        if let Some(latest) = latest {
            updates.push(ModUpdateInfo {
                file_name: m.file_name.clone(),
                mod_id: m.mod_id.clone(),
                name: m.name.clone(),
                available: latest,
            });
        }
    }
    Ok(updates)
}

/// Downloads a mod's newer version and replaces the old jar. The changelog
/// shown before calling this comes straight from `ModUpdateInfo.available`
/// (feature 16) — no separate fetch is needed.
#[tauri::command]
pub async fn update_java_mod(
    state: State<'_, AppState>,
    instance_id: String,
    old_file_name: String,
    version: MrVersion,
) -> Result<String, String> {
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    let dest = mods_dir.join(&version.file_name);
    let req = DownloadRequest::new(version.url.clone(), dest).with_label(version.file_name.clone());
    state
        .downloads
        .submit(req)
        .join()
        .await
        .map_err(|e| e.to_string())?;

    if old_file_name != version.file_name {
        let old_path = mods_dir.join(&old_file_name);
        if old_path.exists() {
            std::fs::remove_file(old_path).map_err(|e| e.to_string())?;
        }
    }

    // A prior provenance record (from install_modrinth_mod or a previous
    // update) may exist under the old filename; find its project id (if
    // any) so the new record stays a Modrinth-sourced entry rather than
    // being silently dropped.
    let existing_project_id = mod_provenance::load(&mods_dir)
        .mods
        .get(&old_file_name)
        .map(|p| p.project_id.clone())
        .unwrap_or_default();
    let _ = mod_provenance::remove(&mods_dir, &old_file_name);
    let _ = mod_provenance::record(
        &mods_dir,
        ModProvenance {
            filename: version.file_name.clone(),
            source: ModSource::Modrinth,
            project_id: existing_project_id,
            version_id: version.id.clone(),
            sha1: version.sha1.clone(),
        },
    );

    Ok(version.file_name)
}

/// One (old jar, new version) pair for "update all".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingModUpdate {
    pub old_file_name: String,
    pub version: MrVersion,
}

#[tauri::command]
pub async fn update_all_java_mods(
    state: State<'_, AppState>,
    instance_id: String,
    updates: Vec<PendingModUpdate>,
) -> Result<Vec<String>, String> {
    let mut installed = Vec::new();
    for u in updates {
        let name = update_java_mod(state.clone(), instance_id.clone(), u.old_file_name, u.version)
            .await?;
        installed.push(name);
    }
    Ok(installed)
}

// ── Disable-all / bisect mode (feature 14) ─────────────────────────────────

/// Toggles every mod jar in an instance between enabled/disabled at once,
/// for crash troubleshooting (bisecting which mod causes a crash). Reuses
/// `mod_analyzer::set_mod_enabled`'s idiot-proof rename logic per file, so a
/// partially-renamed folder from a previous crash is left alone rather than
/// overwritten.
#[tauri::command]
pub fn set_all_java_mods_enabled(
    state: State<'_, AppState>,
    instance_id: String,
    enabled: bool,
) -> Result<usize, String> {
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    let report = mod_analyzer::analyze_dir(&mods_dir);
    let mut toggled = 0usize;
    for m in &report.mods {
        if m.disabled == !enabled {
            // Already in the requested state.
            continue;
        }
        match mod_analyzer::set_mod_enabled(&mods_dir, &m.file_name, enabled) {
            Ok(_) => toggled += 1,
            Err(e) => return Err(format!("failed on {}: {e}", m.file_name)),
        }
    }
    Ok(toggled)
}

// ── Modpack diffing (feature 13) ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDiffPair {
    pub a: ModInfo,
    pub b: ModInfo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceModDiff {
    pub only_a: Vec<ModInfo>,
    pub only_b: Vec<ModInfo>,
    /// Present (by mod id) in both instances but the jar file name differs -
    /// a proxy for "different version" since jar file names conventionally
    /// embed the mod version.
    pub different_version: Vec<ModDiffPair>,
    pub same: Vec<ModInfo>,
}

/// Compares the installed mods of two instances, keyed by loader mod id
/// (falling back to the jar file stem for mods without readable metadata).
#[tauri::command]
pub fn diff_java_instances(
    state: State<'_, AppState>,
    instance_a: String,
    instance_b: String,
) -> Result<InstanceModDiff, String> {
    let dir_a = instance_mods_dir(&state, &instance_a)?;
    let dir_b = instance_mods_dir(&state, &instance_b)?;
    let report_a = mod_analyzer::analyze_dir(&dir_a);
    let report_b = mod_analyzer::analyze_dir(&dir_b);

    fn key(m: &ModInfo) -> String {
        if !m.mod_id.is_empty() {
            m.mod_id.to_lowercase()
        } else {
            m.file_name.to_lowercase()
        }
    }

    let mut diff = InstanceModDiff::default();
    let map_b: std::collections::HashMap<String, &ModInfo> =
        report_b.mods.iter().map(|m| (key(m), m)).collect();
    let mut seen_b_keys = std::collections::HashSet::new();

    for m in &report_a.mods {
        let k = key(m);
        match map_b.get(&k) {
            Some(other) => {
                seen_b_keys.insert(k);
                if other.file_name == m.file_name {
                    diff.same.push(m.clone());
                } else {
                    diff.different_version.push(ModDiffPair {
                        a: m.clone(),
                        b: (*other).clone(),
                    });
                }
            }
            None => diff.only_a.push(m.clone()),
        }
    }
    for m in &report_b.mods {
        if !seen_b_keys.contains(&key(m)) {
            diff.only_b.push(m.clone());
        }
    }
    Ok(diff)
}

// ── Local jar drag-and-drop install (feature 19) ────────────────────────────

/// Copies a jar file already on disk (dropped onto the launcher window via
/// Tauri's native `onDragDropEvent`, which — unlike the raw browser
/// `DragEvent` — carries a real filesystem path) into an instance's `mods/`
/// folder.
#[tauri::command]
pub fn install_local_mod_jar(
    state: State<'_, AppState>,
    instance_id: String,
    source_path: String,
) -> Result<String, String> {
    let source = PathBuf::from(&source_path);
    if source.extension().and_then(|e| e.to_str()) != Some("jar") {
        return Err("Only .jar files can be installed as mods.".to_string());
    }
    let file_name = source
        .file_name()
        .ok_or_else(|| "Invalid file path.".to_string())?
        .to_string_lossy()
        .into_owned();

    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    let dest = mods_dir.join(&file_name);
    if dest.exists() {
        return Err(format!("{file_name} is already installed in this instance."));
    }
    std::fs::copy(&source, &dest).map_err(|e| e.to_string())?;
    Ok(file_name)
}
