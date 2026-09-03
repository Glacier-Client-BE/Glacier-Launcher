//! Skin library and account skin/cape management.
//!
//! Thin wrappers over `gl_integrations::skin_library` (port of
//! `SkinLibraryService.cs`) and `gl_integrations::skins` (port of
//! `SkinService.cs`). Both crates were already fully ported but **not a single
//! command was registered**, so `Components/SkinLibraryPanel.razor` could not
//! exist and the Profile panel's skin/cape controls had nothing to call.
//!
//! The skins directory is `<launcher root>/skins`, matching the C#
//! `SkinLibraryService.SkinsDir` (which resolves it from
//! `LauncherUtilityService.LauncherRoot`); the crate takes it as a parameter
//! rather than owning that policy.

use gl_integrations::models::SavedSkin;
use gl_integrations::{skin_library, skins};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::state::AppState;

fn skins_dir() -> PathBuf {
    gl_settings::launcher_root().join("skins")
}

/// Port of `SkinLibraryService.List()`.
#[tauri::command]
pub fn list_saved_skins() -> Result<Vec<SavedSkin>, String> {
    skin_library::list_skins(&skins_dir()).map_err(|e| e.to_string())
}

/// Port of `AddFromFileAsync`. The frontend supplies the path from
/// tauri-plugin-dialog's native picker, the same route the settings
/// export/import commands use — WebView2 gives no real path for a dropped or
/// `<input type=file>` file, so the pick must originate in the host.
#[tauri::command]
pub fn add_skin_from_file(source_path: String, slim: bool) -> Result<String, String> {
    skin_library::add_from_file(&skins_dir(), Path::new(&source_path), slim)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

/// Port of `AddFromUsernameAsync` — resolves a player's current skin from
/// Mojang by name and saves it into the library.
#[tauri::command]
pub async fn add_skin_from_username(
    state: State<'_, AppState>,
    username: String,
) -> Result<String, String> {
    skin_library::add_from_username(&state.http, &skins_dir(), &username)
        .await
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

/// Port of `SkinLibraryService.Delete` — never fails, as the crate's
/// `delete_skin` swallows the error exactly as the C# does.
#[tauri::command]
pub fn delete_saved_skin(path: String) {
    skin_library::delete_skin(Path::new(&path));
}

/// Port of `SkinService.UploadSkinAsync`. Reads the PNG here so the
/// integration crate stays free of filesystem policy, as its doc comment asks.
///
/// The token comes from settings rather than the caller: the original reads
/// `SettingsService.Settings.JavaAccessToken` at the call site, and passing an
/// access token through the frontend would put it in reach of any page script.
#[tauri::command]
pub async fn apply_skin(
    state: State<'_, AppState>,
    path: String,
    slim: bool,
) -> Result<(), String> {
    let token = gl_settings::load()
        .map_err(|e| e.to_string())?
        .java_access_token;
    if token.trim().is_empty() {
        return Err("Sign in with Microsoft (Java) first to apply skins.".to_string());
    }

    let file_path = PathBuf::from(&path);
    let file_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "skin.png".to_string());
    let bytes = std::fs::read(&file_path).map_err(|e| e.to_string())?;

    skins::upload_skin(&state.http, &token, &file_name, bytes, slim).await
}

/// Port of `SkinService.ResetSkinAsync`.
#[tauri::command]
pub async fn reset_skin(state: State<'_, AppState>) -> Result<(), String> {
    let token = gl_settings::load()
        .map_err(|e| e.to_string())?
        .java_access_token;
    skins::reset_skin(&state.http, &token).await
}

/// Port of `SkinService.GetCapesAsync`. Returns an empty list rather than an
/// error when signed out, matching the crate and the C#.
#[tauri::command]
pub async fn get_capes(
    state: State<'_, AppState>,
) -> Result<Vec<gl_integrations::models::CapeInfo>, String> {
    let token = gl_settings::load()
        .map_err(|e| e.to_string())?
        .java_access_token;
    Ok(skins::get_capes(&state.http, &token).await)
}

/// Port of `SkinService.SetCapeAsync`.
#[tauri::command]
pub async fn set_cape(state: State<'_, AppState>, cape_id: String) -> Result<(), String> {
    let token = gl_settings::load()
        .map_err(|e| e.to_string())?
        .java_access_token;
    skins::set_cape(&state.http, &token, &cape_id).await
}

/// Port of `SkinService.HideCapeAsync`.
#[tauri::command]
pub async fn hide_cape(state: State<'_, AppState>) -> Result<(), String> {
    let token = gl_settings::load()
        .map_err(|e| e.to_string())?
        .java_access_token;
    skins::hide_cape(&state.http, &token).await
}
