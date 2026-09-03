//! Notification feed commands — thin wrappers over
//! `gl_settings::NotificationService` (port of `Services/NotificationService.cs`).
//!
//! The service holds its own interior mutability, so `AppState` stores it
//! directly rather than behind another `Mutex`.

use gl_settings::AppNotification;
use tauri::State;

use crate::state::AppState;

/// All notifications, newest first.
#[tauri::command]
pub fn list_notifications(state: State<'_, AppState>) -> Vec<AppNotification> {
    state.notifications.items()
}

/// Count of unread notifications — the top bar's bell badge adds this to the
/// active download count, matching the original's `bellBadgeCount`.
#[tauri::command]
pub fn notification_unread_count(state: State<'_, AppState>) -> usize {
    state.notifications.unread_count()
}

#[tauri::command]
pub fn add_notification(
    state: State<'_, AppState>,
    title: String,
    message: String,
    kind: Option<String>,
    url: Option<String>,
) -> AppNotification {
    state.notifications.add(
        &title,
        &message,
        kind.as_deref().unwrap_or("info"),
        url.as_deref().unwrap_or_default(),
    )
}

#[tauri::command]
pub fn mark_notification_read(state: State<'_, AppState>, id: String) {
    state.notifications.mark_read(&id);
}

#[tauri::command]
pub fn mark_all_notifications_read(state: State<'_, AppState>) {
    state.notifications.mark_all_read();
}

#[tauri::command]
pub fn remove_notification(state: State<'_, AppState>, id: String) {
    state.notifications.remove(&id);
}

#[tauri::command]
pub fn clear_notifications(state: State<'_, AppState>) {
    state.notifications.clear();
}
