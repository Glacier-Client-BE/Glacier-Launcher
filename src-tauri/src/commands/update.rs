use gl_update::{LauncherRelease, LauncherUpdateInfo};
use tauri::State;

use crate::state::AppState;

const REPO_ORG: &str = "Glacier-Client-BE";
const REPO_NAME: &str = "Glacier-Launcher";

#[tauri::command]
pub async fn check_launcher_update(
    state: State<'_, AppState>,
) -> Result<Option<LauncherUpdateInfo>, String> {
    let checker = gl_update::UpdateChecker::with_client(REPO_ORG, REPO_NAME, state.http.clone());
    checker
        .check_launcher_update(env!("CARGO_PKG_VERSION"))
        .await
        .map_err(|e| e.to_string())
}

/// Lists recent (non-draft) launcher releases for the News panel's changelog
/// section, mirroring `AutoUpdateService.GetRecentReleasesAsync`.
#[tauri::command]
pub async fn list_launcher_releases(
    state: State<'_, AppState>,
    max: Option<usize>,
) -> Result<Vec<LauncherRelease>, String> {
    let checker = gl_update::UpdateChecker::with_client(REPO_ORG, REPO_NAME, state.http.clone());
    checker
        .recent_releases(max.unwrap_or(12))
        .await
        .map_err(|e| e.to_string())
}

/// The running launcher version, port of `AutoUpdateService.CurrentVersion`.
/// The frontend previously hardcoded this string.
#[tauri::command]
pub fn launcher_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
