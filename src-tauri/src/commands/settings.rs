//! Thin #[tauri::command] wrappers. Business logic lives in the gl-* crates;
//! commands here only deserialize args, call into a crate, and map errors.

use gl_settings::LauncherSettings;

/// CurseForge key baked in by the release workflow (repo secret), used when
/// the user hasn't entered their own — as the C# launcher shipped it.
const BUILTIN_CURSEFORGE_KEY: Option<&str> = option_env!("GLACIER_CURSEFORGE_API_KEY");

#[tauri::command(async)]
pub fn get_settings() -> Result<LauncherSettings, String> {
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    if settings.curse_forge_api_key.trim().is_empty() {
        if let Some(key) = BUILTIN_CURSEFORGE_KEY {
            settings.curse_forge_api_key = key.to_string();
        }
    }
    Ok(settings)
}

#[tauri::command(async)]
pub fn set_settings(mut settings: LauncherSettings) -> Result<(), String> {
    // Never write the built-in key to the user's settings file.
    if BUILTIN_CURSEFORGE_KEY.is_some_and(|k| settings.curse_forge_api_key == k) {
        settings.curse_forge_api_key.clear();
    }
    if let Ok(disk) = gl_settings::load() {
        keep_newer_java_token(&mut settings, &disk);
    }
    gl_settings::save(&settings).map_err(|e| e.to_string())
}

/// The frontend saves whole settings snapshots, so one taken before a launch-time
/// token refresh would roll the rotated refresh token back and sign the user out.
fn keep_newer_java_token(incoming: &mut LauncherSettings, disk: &LauncherSettings) {
    if incoming.java_uuid.is_empty() || incoming.java_uuid != disk.java_uuid {
        return;
    }
    let expiry = |s: &str| super::launch::time_from_rfc3339(s).unwrap_or(0);
    if expiry(&disk.java_access_token_expiry) > expiry(&incoming.java_access_token_expiry) {
        incoming.java_access_token = disk.java_access_token.clone();
        incoming.java_access_token_expiry = disk.java_access_token_expiry.clone();
        incoming.xbox_live_refresh_token = disk.xbox_live_refresh_token.clone();
    }
}
