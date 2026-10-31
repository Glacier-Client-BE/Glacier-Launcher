//! Java Edition screenshot enumeration for the "Java Screenshots" panel.
//! Mirrors the Java-side counterpart of `Services/GameConsoleService.cs`'s
//! screenshot listing (the Bedrock equivalent lives in
//! `gl_bedrock::BedrockScreenshotService`, wired through `bedrock.rs`).
//!
//! Simpler than the Bedrock version: Java only has one source folder,
//! `<instance>/screenshots`, holding `.png` files (no separate OS Captures
//! folder to merge in).

use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::State;

use gl_bedrock::model::ScreenshotFile;

use crate::state::AppState;

/// Formats a modification time as a zero-padded Unix-epoch-seconds string.
/// Not a true ISO timestamp (no chrono dependency available in this crate),
/// but it sorts and displays fine for the screenshot list's purposes.
fn iso_from_systemtime(t: SystemTime) -> String {
    t.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

fn screenshots_dir(state: &State<'_, AppState>, instance_id: &str) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.path_for_instance(instance_id, "screenshots"))
}

fn entry(file: &Path) -> Option<ScreenshotFile> {
    let meta = std::fs::metadata(file).ok()?;
    Some(ScreenshotFile {
        name: file.file_name()?.to_string_lossy().into_owned(),
        path: file.to_string_lossy().into_owned(),
        kind: "screenshot".to_string(),
        size_bytes: meta.len(),
        modified_at: meta
            .modified()
            .ok()
            .map(iso_from_systemtime)
            .unwrap_or_default(),
    })
}

#[tauri::command]
pub async fn list_java_screenshots(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<ScreenshotFile>, String> {
    let dir = screenshots_dir(&state, &instance_id)?;
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for dir_entry in entries.flatten() {
        let file = dir_entry.path();
        if !file.is_file() {
            continue;
        }
        if file
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("png"))
            != Some(true)
        {
            continue;
        }
        if let Some(e) = entry(&file) {
            out.push(e);
        }
    }
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(out)
}

#[tauri::command]
pub async fn delete_java_screenshot(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    // Only screenshots inside some Java instance's `screenshots` folder may be
    // removed; the frontend supplies `path`.
    let target = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        svc.instances().iter().find_map(|inst| {
            let dir = svc.path_for_instance(&inst.id, "screenshots");
            super::ensure_direct_child(&dir, Path::new(&path)).ok()
        })
    }
    .ok_or_else(|| format!("Refusing to delete {path}: not a Java instance screenshot."))?;
    std::fs::remove_file(&target).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_java_screenshots_folder(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    let dir = screenshots_dir(&state, &instance_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
