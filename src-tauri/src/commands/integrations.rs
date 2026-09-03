use gl_integrations::models::{Announcement, NewsPost};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn get_announcement(state: State<'_, AppState>) -> Result<Option<Announcement>, String> {
    Ok(gl_integrations::announcements::get_announcement(&state.http).await)
}

#[tauri::command]
pub async fn get_news(state: State<'_, AppState>) -> Result<Option<Vec<NewsPost>>, String> {
    Ok(gl_integrations::news::get_news(&state.http)
        .await
        .map(gl_integrations::news::decode))
}

#[tauri::command]
pub fn start_discord_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.start();
    Ok(())
}

#[tauri::command]
pub fn stop_discord_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.stop();
    Ok(())
}

#[tauri::command]
pub fn set_discord_in_game_presence(
    state: State<'_, AppState>,
    version_tag: String,
    client_name: Option<String>,
) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.set_in_game_presence(&version_tag, client_name.as_deref());
    Ok(())
}

#[tauri::command]
pub fn set_discord_idle_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.set_idle_presence();
    Ok(())
}
