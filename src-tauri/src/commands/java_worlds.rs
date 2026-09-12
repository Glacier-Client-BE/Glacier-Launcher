//! Java Edition worlds, world backups and datapacks.
//!
//! Thin wrappers over `gl_java::worlds`, which ports the corresponding half of
//! `JavaInstanceService.cs`. These back `Components/BackupsPanel.razor` (the
//! "World Backups" panel) and the Java datapacks tab — neither of which could
//! exist in the rewrite before, because none of this had been ported.
//!
//! Every command resolves the *active* instance's directory the way the C#
//! `PathFor` does (via `ActiveInstance`), so the caller passes only a world or
//! backup path, never a root.

use gl_java::worlds::{self, JavaInstanceFile, JavaWorld};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::state::AppState;

/// Port of `JavaInstanceService.ActiveMinecraftDir` — the directory the
/// worlds/backups/datapacks folders hang off.
fn active_dir(state: &AppState) -> Result<PathBuf, String> {
    let active_id = gl_settings::load()
        .ok()
        .map(|s| s.java_active_instance_id)
        .filter(|s| !s.is_empty());
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(PathBuf::from(
        svc.active_instance(active_id.as_deref()).directory,
    ))
}

/// Port of `ListWorlds`.
#[tauri::command]
pub fn list_java_worlds(state: State<'_, AppState>) -> Result<Vec<JavaWorld>, String> {
    let dir = active_dir(&state)?;
    worlds::list_worlds(&dir).map_err(|e| e.to_string())
}

/// Port of `ListBackups`.
#[tauri::command]
pub fn list_java_backups(state: State<'_, AppState>) -> Result<Vec<JavaInstanceFile>, String> {
    let dir = active_dir(&state)?;
    worlds::list_backups(&dir).map_err(|e| e.to_string())
}

/// Port of `BackupWorldAsync`. The file-name timestamp is stamped here rather
/// than in the crate, so the crate stays free of a clock dependency and its
/// round-trip test can pass a fixed stamp. See `utc_timestamp` for the one
/// deviation from the C# format.
#[tauri::command]
pub async fn backup_java_world(
    state: State<'_, AppState>,
    world_dir: String,
) -> Result<String, String> {
    let dir = active_dir(&state)?;
    let stamp = utc_timestamp();
    // Zipping a large world blocks; keep it off the UI-facing async runtime thread.
    tauri::async_runtime::spawn_blocking(move || {
        worlds::backup_world(&dir, Path::new(&world_dir), &stamp)
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Backs up every world in the active Java instance's `saves/` folder.
/// Called from the frontend before a Java launch when
/// `JavaBackupSavesBeforeLaunch` is enabled — there's no single "current"
/// world to single out ahead of launch (that's chosen from the in-game
/// world list), so every world gets a fresh backup, matching the safety
/// intent of the setting. Best-effort per world: one failing world (e.g.
/// briefly locked) doesn't stop the others from being backed up.
#[tauri::command]
pub async fn backup_active_java_world(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let dir = active_dir(&state)?;
    let worlds = worlds::list_worlds(&dir).map_err(|e| e.to_string())?;
    let stamp = utc_timestamp();

    tauri::async_runtime::spawn_blocking(move || {
        let mut backed_up = Vec::new();
        for world in worlds {
            if let Ok(path) = worlds::backup_world(&dir, Path::new(&world.path), &stamp) {
                backed_up.push(path.to_string_lossy().into_owned());
            }
        }
        backed_up
    })
    .await
    .map_err(|e| e.to_string())
}

/// Port of `RestoreBackupAsync`.
///
/// **Destructive**: this replaces the world folder the archive names, which is
/// what the panel's "Replaces the current world with this backup" tooltip
/// promises. The confirm step is the panel's, matching the original.
#[tauri::command]
pub async fn restore_java_backup(
    state: State<'_, AppState>,
    zip_path: String,
) -> Result<(), String> {
    let dir = active_dir(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        worlds::restore_backup(&dir, Path::new(&zip_path)).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Port of `DeleteBackup` — never fails, as the C# swallows its exception.
#[tauri::command]
pub fn delete_java_backup(zip_path: String) {
    worlds::delete_backup(Path::new(&zip_path));
}

/// Port of `ListDatapacks`.
#[tauri::command]
pub fn list_java_datapacks(
    state: State<'_, AppState>,
    world_name: String,
) -> Result<Vec<JavaInstanceFile>, String> {
    let dir = active_dir(&state)?;
    worlds::list_datapacks(&dir, &world_name).map_err(|e| e.to_string())
}

/// Port of `ImportDatapackFile`.
#[tauri::command]
pub fn import_java_datapack(
    state: State<'_, AppState>,
    world_name: String,
    zip_path: String,
) -> Result<String, String> {
    let dir = active_dir(&state)?;
    worlds::import_datapack(&dir, &world_name, Path::new(&zip_path))
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

/// Port of `DeleteDatapack`.
#[tauri::command]
pub fn delete_java_datapack(path: String) -> Result<(), String> {
    worlds::delete_datapack(Path::new(&path)).map_err(|e| e.to_string())
}

/// Port of `OpenDatapacksFolder`.
#[tauri::command]
pub fn open_java_datapacks_folder(
    state: State<'_, AppState>,
    world_name: String,
) -> Result<(), String> {
    let dir = active_dir(&state)?;
    let target = worlds::datapacks_dir(&dir, &world_name);
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Port of the C# file-name stamp `DateTime.Now:yyyyMMdd-HHmmss`.
///
/// Deviation: this is **UTC**, not local time. The workspace has no date
/// dependency and every other timestamp helper here (`gl-bedrock::now_iso`,
/// `gl-java::now_iso`, `gl-settings::notifications::now_iso`) is UTC too, so
/// matching them keeps backup names sorting consistently with everything else.
/// The only visible effect is the digits in a backup's file name.
fn utc_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };

    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

#[cfg(test)]
mod tests {
    use super::utc_timestamp;

    #[test]
    fn timestamp_has_the_csharp_shape() {
        let stamp = utc_timestamp();
        assert_eq!(stamp.len(), 15, "yyyyMMdd-HHmmss is 15 chars: {stamp}");
        assert_eq!(stamp.as_bytes()[8], b'-');
        assert!(
            stamp
                .chars()
                .enumerate()
                .all(|(i, c)| i == 8 || c.is_ascii_digit()),
            "unexpected: {stamp}"
        );
        assert!(stamp.starts_with("20"), "unexpected year: {stamp}");
    }
}
