//! Thin wrappers around `state.downloads` (a `gl_download::DownloadQueue`)
//! for the Downloads panel. Ported from `DownloadService.ActiveDownloads` /
//! `Home.Downloads.cs`: the queue itself now keeps a registry of every
//! submitted download (see `gl-download`'s `DownloadQueue::active_entries`),
//! so this file just serializes that shared state for the frontend to poll.

use gl_download::{DownloadEntry, DownloadQueue, DownloadStatus};
use serde::Serialize;
use tauri::{Emitter, State};

use crate::state::AppState;

/// Frontend event carrying the full download list on every change. Port of
/// `DownloadService.Changed` (`DownloadService.cs:48`), which the C# Downloads
/// panel and every per-item progress bar re-rendered on. Pushing the snapshot
/// with the event means no panel has to poll on a timer, which matters on the
/// N5100 target.
pub const DOWNLOADS_CHANGED_EVENT: &str = "downloads-changed";

/// Bridges the queue's `Changed` notification onto `DOWNLOADS_CHANGED_EVENT`.
/// Called once from `setup`; the task lives for the process.
pub fn spawn_downloads_bridge(app: tauri::AppHandle, queue: DownloadQueue) {
    let mut rx = queue.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(()) => {}
                // Lagged: notifications carry no payload and we re-read the
                // whole registry below, so a dropped tick loses nothing.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
            let entries: Vec<DownloadEntryDto> =
                queue.active_entries().into_iter().map(Into::into).collect();
            let _ = app.emit(DOWNLOADS_CHANGED_EVENT, &entries);
        }
    });
}

#[derive(Serialize)]
pub struct DownloadEntryDto {
    pub id: String,
    pub label: String,
    pub status: &'static str,
    pub progress: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
}

fn status_str(status: DownloadStatus) -> &'static str {
    match status {
        DownloadStatus::Queued => "Queued",
        DownloadStatus::Downloading => "Downloading",
        DownloadStatus::Completed => "Completed",
        DownloadStatus::Failed => "Failed",
        DownloadStatus::Cancelled => "Cancelled",
        DownloadStatus::Paused => "Paused",
    }
}

impl From<DownloadEntry> for DownloadEntryDto {
    fn from(e: DownloadEntry) -> Self {
        Self {
            id: e.id.to_string(),
            label: e.label,
            status: status_str(e.status),
            progress: e.progress,
            downloaded_bytes: e.downloaded_bytes,
            total_bytes: e.total_bytes,
            error: e.error,
        }
    }
}

/// Lists every download the queue is currently tracking (queued, running, or
/// finished since the last `clear_finished_downloads` call). Mirrors
/// `DownloadsList` in the C# source.
#[tauri::command]
pub fn list_downloads(state: State<'_, AppState>) -> Vec<DownloadEntryDto> {
    state
        .downloads
        .active_entries()
        .into_iter()
        .map(Into::into)
        .collect()
}

/// QOL: pauses a `Downloading` entry, leaving its partial `.tmp` on disk so
/// `resume_download` can continue it via a Range request. A no-op for
/// anything not currently downloading.
#[tauri::command]
pub async fn pause_download(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.downloads.pause_str(&id)
}

/// QOL: resumes a `Paused` download from where it left off.
#[tauri::command]
pub async fn resume_download(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    state.downloads.resume_str(&id)
}

/// Cancels an in-flight download by id. Mirrors `CancelDownload`.
#[tauri::command]
pub async fn cancel_download(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.downloads.cancel_str(&id)
}

/// Drops a single tracked download regardless of status. Mirrors
/// `DownloadService.Remove`.
#[tauri::command]
pub async fn remove_download(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.downloads.remove_str(&id)
}

/// Drops every tracked download that isn't queued/downloading. Mirrors
/// `ClearFinishedDownloads`.
#[tauri::command]
pub fn clear_finished_downloads(state: State<'_, AppState>) {
    state.downloads.clear_finished();
}

/// QOL: resubmits every currently-`Failed` download using its original
/// request, instead of the user having to remove each one and re-trigger
/// whatever originally queued it. Returns how many were retried.
#[tauri::command]
pub fn retry_failed_downloads(state: State<'_, AppState>) -> usize {
    state.downloads.retry_failed()
}

/// QOL: retries one specific download by id, regardless of its status.
#[tauri::command]
pub async fn retry_download(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    state.downloads.retry_str(&id)
}

/// QOL: sweeps finished downloads older than
/// `settings.auto_clear_downloads_after_days`. A no-op when that's `0`
/// (disabled). Meant to be called periodically (and once at startup) by the
/// frontend rather than driven by a backend timer, so it only ever runs
/// while the launcher is actually open.
#[tauri::command]
pub fn sweep_old_downloads(state: State<'_, AppState>) {
    let days = gl_settings::load()
        .map(|s| s.auto_clear_downloads_after_days)
        .unwrap_or(0);
    if days <= 0 {
        return;
    }
    let max_age_ms = (days as i64) * 24 * 60 * 60 * 1000;
    state.downloads.clear_finished_older_than(max_age_ms);
}
