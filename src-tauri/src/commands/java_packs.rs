//! New in Phase 3: Java resourcepacks/shaderpacks management (`gl_java::packs`),
//! plus the unified mod search/dependency-resolve/install commands
//! (`gl_modpack::{unified,dependencies}`) and CurseForge mod install. Kept out
//! of `java_mods.rs` to reduce merge conflicts with concurrent phase work on
//! that file.

use std::path::PathBuf;

use gl_download::DownloadRequest;
use gl_java::packs::{self, PackKind};
use gl_java::worlds::JavaInstanceFile;
use gl_java::{mod_provenance, ModProvenance, ModSource};
use gl_modpack::{
    dependencies, unified, CurseForgeClient, InstallPlan, ModrinthClient, PlannedFile, Provider,
    UnifiedMod,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

fn instance_mc_dir(state: &State<'_, AppState>, instance_id: &str) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.path_for_instance(instance_id, ""))
}

fn instance_mods_dir(state: &State<'_, AppState>, instance_id: &str) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.path_for_instance(instance_id, "mods"))
}

// ── Resource packs / shader packs ───────────────────────────────────────────

#[tauri::command]
pub fn list_java_resourcepacks(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<JavaInstanceFile>, String> {
    let dir = instance_mc_dir(&state, &instance_id)?;
    packs::list_packs(&dir, PackKind::ResourcePack).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_java_resourcepack(
    state: State<'_, AppState>,
    instance_id: String,
    source_path: String,
) -> Result<String, String> {
    let dir = instance_mc_dir(&state, &instance_id)?;
    let dest = packs::import_pack(&dir, PackKind::ResourcePack, &PathBuf::from(source_path))
        .map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn delete_java_resourcepack(path: String) -> Result<(), String> {
    packs::delete_pack(&PathBuf::from(path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_java_shaderpacks(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<JavaInstanceFile>, String> {
    let dir = instance_mc_dir(&state, &instance_id)?;
    packs::list_packs(&dir, PackKind::ShaderPack).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_java_shaderpack(
    state: State<'_, AppState>,
    instance_id: String,
    source_path: String,
) -> Result<String, String> {
    let dir = instance_mc_dir(&state, &instance_id)?;
    let dest = packs::import_pack(&dir, PackKind::ShaderPack, &PathBuf::from(source_path))
        .map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn delete_java_shaderpack(path: String) -> Result<(), String> {
    packs::delete_pack(&PathBuf::from(path)).map_err(|e| e.to_string())
}

// ── Unified mod search ──────────────────────────────────────────────────────

/// Searches either provider for mods. `curseforge_api_key` may be empty when
/// `provider` is "modrinth"; passing it empty for "curseforge" fails with a
/// clear "API key not set" error rather than a confusing HTTP failure.
#[tauri::command]
pub async fn search_mods(
    state: State<'_, AppState>,
    provider: String,
    query: String,
    curseforge_api_key: Option<String>,
    offset: u32,
) -> Result<Vec<UnifiedMod>, String> {
    let provider = match provider.as_str() {
        "modrinth" => Provider::Modrinth,
        "curseforge" => Provider::CurseForge,
        other => return Err(format!("unknown provider: {other}")),
    };
    let modrinth = ModrinthClient::new(state.http.clone());
    let cf = curseforge_api_key
        .filter(|k| !k.trim().is_empty())
        .map(|k| CurseForgeClient::new(state.http.clone(), k));

    unified::search_mods(&modrinth, cf.as_ref(), provider, &query, 20, offset)
        .await
        .map_err(|e| e.to_string())
}

// ── Dependency resolution + install ─────────────────────────────────────────

/// Resolves a Modrinth project's transitive required-dependency plan. Does
/// NOT install anything — the frontend shows this plan in a confirm modal
/// first, then calls `install_resolved_mods`.
#[tauri::command]
pub async fn resolve_mod_dependencies(
    state: State<'_, AppState>,
    provider: String,
    project_id: String,
    version_id: Option<String>,
) -> Result<InstallPlan, String> {
    match provider.as_str() {
        "modrinth" => {
            let client = ModrinthClient::new(state.http.clone());
            dependencies::resolve_modrinth_plan(&client, &project_id, version_id.as_deref())
                .await
                .map_err(|e| e.to_string())
        }
        "curseforge" => Err(
            "Dependency resolution for CurseForge mods is not available (no public dependency \
             metadata endpoint); install directly with install_curseforge_mod instead."
                .to_string(),
        ),
        other => Err(format!("unknown provider: {other}")),
    }
}

/// Downloads every file in a previously-resolved plan into an instance's
/// `mods/` folder via the shared `DownloadQueue` (same progress-event path as
/// every other download in the app), recording provenance for each.
#[tauri::command]
pub async fn install_resolved_mods(
    state: State<'_, AppState>,
    instance_id: String,
    plan: InstallPlan,
) -> Result<Vec<String>, String> {
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    let mut installed = Vec::new();
    for file in &plan.files {
        install_planned_file(&state, &mods_dir, file).await?;
        installed.push(file.filename.clone());
    }
    Ok(installed)
}

async fn install_planned_file(
    state: &State<'_, AppState>,
    mods_dir: &std::path::Path,
    file: &PlannedFile,
) -> Result<(), String> {
    let dest = mods_dir.join(&file.filename);
    let req = DownloadRequest::new(file.file_url.clone(), dest).with_label(file.filename.clone());
    state
        .downloads
        .submit(req)
        .join()
        .await
        .map_err(|e| e.to_string())?;

    let source = match file.provider {
        Provider::Modrinth => ModSource::Modrinth,
        Provider::CurseForge => ModSource::CurseForge,
    };
    mod_provenance::record(
        mods_dir,
        ModProvenance {
            filename: file.filename.clone(),
            source,
            project_id: file.project_id.clone(),
            version_id: file.version_id.clone(),
            sha1: file.sha1.clone(),
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Installs a single CurseForge mod file directly (no dependency walk — see
/// `resolve_mod_dependencies`'s documented gap for CurseForge).
#[tauri::command]
pub async fn install_curseforge_mod(
    state: State<'_, AppState>,
    instance_id: String,
    curseforge_api_key: String,
    mod_id: i32,
    file_id: i32,
) -> Result<String, String> {
    let client = CurseForgeClient::new(state.http.clone(), curseforge_api_key);
    let plan = dependencies::resolve_curseforge_plan(&client, mod_id, file_id)
        .await
        .map_err(|e| e.to_string())?;
    let Some(file) = plan.files.first() else {
        return Err(plan
            .conflicts
            .into_iter()
            .next()
            .unwrap_or_else(|| "CurseForge file could not be resolved.".to_string()));
    };
    let mods_dir = instance_mods_dir(&state, &instance_id)?;
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    install_planned_file(&state, &mods_dir, file).await?;
    Ok(file.filename.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModVersionsResult {
    pub versions: Vec<gl_modpack::UnifiedVersion>,
}

/// Fetches downloadable versions/files for a single unified mod (used by the
/// UI to populate a version picker before resolving dependencies).
#[tauri::command]
pub async fn get_mod_versions(
    state: State<'_, AppState>,
    provider: String,
    project_id: String,
    curseforge_api_key: Option<String>,
) -> Result<ModVersionsResult, String> {
    let provider = match provider.as_str() {
        "modrinth" => Provider::Modrinth,
        "curseforge" => Provider::CurseForge,
        other => return Err(format!("unknown provider: {other}")),
    };
    let modrinth = ModrinthClient::new(state.http.clone());
    let cf = curseforge_api_key
        .filter(|k| !k.trim().is_empty())
        .map(|k| CurseForgeClient::new(state.http.clone(), k));

    let versions = unified::fetch_versions(&modrinth, cf.as_ref(), provider, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ModVersionsResult { versions })
}
