//! Java Edition mod-loader install + installed-mod-jar management. Ports the
//! UI-facing surface of `Services/JavaModLoaderService.cs` and
//! `Services/JavaModAnalyzer.cs`.

use gl_java::mod_analyzer::{self, ModReport};
use gl_java::mod_loader;
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
    mod_analyzer::delete_mod(&mods_dir, &file_name).map_err(|e| e.to_string())
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
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn download_neoforge_installer(
    state: State<'_, AppState>,
    minecraft_version: String,
) -> Result<String, String> {
    let root_dir = java_root_dir(&state)?;
    let client = state.http.clone();
    let path = mod_loader::download_neoforge_installer(&client, &root_dir, &minecraft_version)
        .await
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}
