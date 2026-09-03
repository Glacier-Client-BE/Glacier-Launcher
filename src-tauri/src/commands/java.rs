use gl_java::{JavaInstance, JavaVersion};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn list_java_versions(state: State<'_, AppState>) -> Result<Vec<JavaVersion>, String> {
    let versions_dir = {
        let instances = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        instances.root_dir().join("versions")
    };
    let mut guard = state.java_versions.lock().await;
    guard
        .get_versions(&versions_dir, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_java_instances(state: State<'_, AppState>) -> Result<Vec<JavaInstance>, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.instances())
}

#[tauri::command]
pub fn create_java_instance(
    state: State<'_, AppState>,
    name: String,
    version_id: String,
) -> Result<JavaInstance, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.create(&name, &version_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_java_instance(
    state: State<'_, AppState>,
    id: String,
    new_name: String,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.rename(&id, &new_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_java_instance(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.delete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_java_instance_ram(
    state: State<'_, AppState>,
    id: String,
    min_mb: i32,
    max_mb: i32,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_ram(&id, min_mb, max_mb).map_err(|e| e.to_string())
}
