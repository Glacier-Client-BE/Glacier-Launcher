//! Thin #[tauri::command] wrappers. Business logic lives in the gl-* crates;
//! commands here only deserialize args, call into a crate, and map errors.

use gl_settings::LauncherSettings;

#[tauri::command]
pub fn get_settings() -> Result<LauncherSettings, String> {
    gl_settings::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_settings(settings: LauncherSettings) -> Result<(), String> {
    gl_settings::save(&settings).map_err(|e| e.to_string())
}
