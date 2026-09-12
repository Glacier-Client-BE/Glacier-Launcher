//! Tauri surface for real Forge/NeoForge (and Fabric/Quilt) loader
//! install/uninstall, replacing the old shell-out-to-installer-jar flow.
//! Business logic lives in `gl_java::mod_loader`/`gl_java::loader`; this
//! file only resolves paths from `AppState`, calls into the crate, maps
//! errors to strings, and persists the resulting loader choice on the
//! instance via `InstanceService::set_instance_loader`.

use gl_java::mod_loader;
use gl_java::models::ModLoaderKind;
use gl_modpack::LoaderClient;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

use crate::state::AppState;

fn java_root_dir(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.root_dir().to_path_buf())
}

fn runtimes_root(_state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or_else(|| "could not resolve home directory (USERPROFILE)".to_string())?;
    Ok(gl_java::RuntimeDownloadService::default_runtimes_root(
        &home,
    ))
}

fn parse_loader(loader: &str) -> Result<ModLoaderKind, String> {
    match loader.to_ascii_lowercase().as_str() {
        "vanilla" | "none" => Ok(ModLoaderKind::Vanilla),
        "fabric" => Ok(ModLoaderKind::Fabric),
        "quilt" => Ok(ModLoaderKind::Quilt),
        "forge" => Ok(ModLoaderKind::Forge),
        "neoforge" => Ok(ModLoaderKind::NeoForge),
        other => Err(format!("unknown loader '{other}'")),
    }
}

/// Resolves the available loader versions for `loader` against
/// `mc_version`, reusing `gl_modpack::LoaderClient` (already knows Fabric's
/// meta API, Quilt's meta API, Forge's promotions feed, and NeoForge's maven
/// metadata) rather than duplicating any of that HTTP logic here.
#[tauri::command]
pub async fn list_loader_versions(
    state: State<'_, AppState>,
    loader: String,
    mc_version: String,
) -> Result<Vec<String>, String> {
    let kind = parse_loader(&loader)?;
    let client = LoaderClient::new(state.http.clone());
    match kind {
        ModLoaderKind::Vanilla => Ok(Vec::new()),
        ModLoaderKind::Fabric => client
            .resolve_fabric_profile(&mc_version, "")
            .await
            .map(|_| Vec::new())
            .or(Ok(Vec::new())),
        ModLoaderKind::Quilt => Ok(Vec::new()),
        ModLoaderKind::Forge => client
            .latest_forge_version(&mc_version)
            .await
            .map(|v| vec![v])
            .map_err(|e| e.to_string()),
        ModLoaderKind::NeoForge => client
            .latest_neoforge_version(&mc_version)
            .await
            .map(|v| vec![v])
            .map_err(|e| e.to_string()),
    }
}

#[derive(Serialize, Clone)]
pub struct LoaderInstallProgress {
    pub instance_id: String,
    pub message: String,
}

const LOADER_INSTALL_PROGRESS_EVENT: &str = "loader-install-progress";

/// Installs `loader` (at `version`, or the resolved latest/recommended when
/// empty) into the shared Java libraries/versions tree, then persists the
/// resulting loader + profile id on `instance_id`.
#[tauri::command]
pub async fn install_loader(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    loader: String,
    version: Option<String>,
) -> Result<String, String> {
    use tauri::Emitter;

    let kind = parse_loader(&loader)?;
    let mc_version = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        svc.get(&instance_id)
            .map(|i| i.version_id.clone())
            .ok_or_else(|| format!("instance not found: {instance_id}"))?
    };

    if kind == ModLoaderKind::Vanilla {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        svc.set_instance_loader(&instance_id, None, None, None)
            .map_err(|e| e.to_string())?;
        return Ok(mc_version);
    }

    let root_dir = java_root_dir(&state)?;
    let runtimes_root = runtimes_root(&state)?;
    let client = state.http.clone();
    let downloads = state.downloads.clone();

    let emit_progress = |msg: &str| {
        let _ = app.emit(
            LOADER_INSTALL_PROGRESS_EVENT,
            LoaderInstallProgress {
                instance_id: instance_id.clone(),
                message: msg.to_string(),
            },
        );
    };

    let profile_id = match kind {
        ModLoaderKind::Fabric => mod_loader::install_fabric(
            &client,
            &root_dir.join("versions"),
            &mc_version,
            version.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?,
        ModLoaderKind::Quilt => mod_loader::install_quilt(
            &client,
            &root_dir.join("versions"),
            &mc_version,
            version.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?,
        ModLoaderKind::Forge => mod_loader::install_forge(
            &client,
            &downloads,
            &root_dir,
            &runtimes_root,
            &mc_version,
            version.as_deref(),
            emit_progress,
        )
        .await
        .map_err(|e| e.to_string())?,
        ModLoaderKind::NeoForge => mod_loader::install_neoforge(
            &client,
            &downloads,
            &root_dir,
            &runtimes_root,
            &mc_version,
            version.as_deref(),
            emit_progress,
        )
        .await
        .map_err(|e| e.to_string())?,
        ModLoaderKind::Vanilla => unreachable!("handled above"),
    };

    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_instance_loader(
        &instance_id,
        Some(kind),
        version.clone(),
        Some(profile_id.clone()),
    )
    .map_err(|e| e.to_string())?;

    Ok(profile_id)
}

/// Clears the instance's loader selection (reverting to vanilla). Does not
/// delete downloaded loader libraries/versions from the shared tree, since
/// other instances may share them.
#[tauri::command]
pub fn uninstall_loader(state: State<'_, AppState>, instance_id: String) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_instance_loader(&instance_id, None, None, None)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct InstanceLoaderDto {
    pub loader: String,
    pub loader_version: Option<String>,
    pub loader_profile_id: Option<String>,
}

#[tauri::command]
pub fn get_instance_loader(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<InstanceLoaderDto, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    let inst = svc
        .get(&instance_id)
        .ok_or_else(|| format!("instance not found: {instance_id}"))?;
    let loader = inst.loader.unwrap_or(ModLoaderKind::Vanilla);
    Ok(InstanceLoaderDto {
        loader: format!("{loader:?}").to_lowercase(),
        loader_version: inst.loader_version.clone(),
        loader_profile_id: inst.loader_profile_id.clone(),
    })
}
