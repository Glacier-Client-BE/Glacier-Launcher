//! Tauri commands for `gl-sync` (Phase 7): sync packs — shared
//! directory-links/file-merges/mod-lists applied consistently across Java
//! instances.
//!
//! `preview_sync` is always safe to call and never mutates anything; the
//! frontend is expected to show its `SyncReport` and only call
//! `apply_sync_pack` after the user confirms. On a successful apply, the
//! affected instances' `JavaInstance.sync_pack_ids` are updated via
//! `InstanceService::set_instance_sync_packs` so the pack shows up on each
//! instance's own record without `gl-java` depending on `gl-sync`.

use std::collections::HashMap;
use std::path::PathBuf;

use gl_sync::{AdoptStrategy, DetachMode, SyncPack, SyncReport, SyncTarget};
use tauri::State;

use crate::state::AppState;

fn now_iso() -> String {
    // Matches the lightweight timestamp approach used elsewhere in this
    // codebase (see `gl_java::instance_service::now_iso`) — no extra chrono
    // dependency for a display-only field.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn instance_dir(state: &AppState, id: &str) -> Result<PathBuf, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.get(id)
        .map(|inst| PathBuf::from(inst.directory))
        .ok_or_else(|| format!("instance not found: {id}"))
}

fn instance_dirs(state: &AppState, ids: &[String]) -> Result<HashMap<String, PathBuf>, String> {
    let mut map = HashMap::new();
    for id in ids {
        map.insert(id.clone(), instance_dir(state, id)?);
    }
    Ok(map)
}

#[tauri::command(async)]
pub fn list_sync_packs(state: State<'_, AppState>) -> Result<Vec<SyncPack>, String> {
    Ok(state.sync_engine.registry().list())
}

#[tauri::command(async)]
pub fn create_sync_pack(
    state: State<'_, AppState>,
    id: String,
    name: String,
    targets: Vec<SyncTarget>,
) -> Result<SyncPack, String> {
    let now = now_iso();
    let pack = SyncPack {
        id,
        name,
        targets,
        overrides: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    };
    state
        .sync_engine
        .registry()
        .upsert(pack.clone())
        .map_err(|e| e.to_string())?;
    Ok(pack)
}

#[tauri::command(async)]
pub fn update_sync_pack(state: State<'_, AppState>, pack: SyncPack) -> Result<(), String> {
    let mut pack = pack;
    pack.updated_at = now_iso();
    state
        .sync_engine
        .registry()
        .upsert(pack)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn delete_sync_pack(state: State<'_, AppState>, pack_id: String) -> Result<bool, String> {
    state
        .sync_engine
        .registry()
        .delete(&pack_id)
        .map_err(|e| e.to_string())
}

/// Always callable, never mutates. The frontend must show this report to
/// the user before offering `apply_sync_pack`.
#[tauri::command(async)]
pub fn preview_sync(
    state: State<'_, AppState>,
    pack_id: String,
    instance_ids: Vec<String>,
) -> Result<SyncReport, String> {
    let dirs = instance_dirs(&state, &instance_ids)?;
    state
        .sync_engine
        .preview(&pack_id, &dirs)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn apply_sync_pack(
    state: State<'_, AppState>,
    pack_id: String,
    instance_ids: Vec<String>,
) -> Result<SyncReport, String> {
    let dirs = instance_dirs(&state, &instance_ids)?;
    let report = state
        .sync_engine
        .apply(&pack_id, &dirs)
        .map_err(|e| e.to_string())?;

    if !report.has_failures() {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        for id in &instance_ids {
            if let Some(inst) = svc.get(id) {
                let mut ids = inst.sync_pack_ids;
                if !ids.iter().any(|x| x == &pack_id) {
                    ids.push(pack_id.clone());
                    let _ = svc.set_instance_sync_packs(id, ids);
                }
            }
        }
    }

    Ok(report)
}

/// Resolves a `ReportAction::ConflictNeedsResolution` entry the frontend got
/// back from `preview_sync`/`apply_sync_pack`: persists the user's choice
/// (`adopt_strategy`) as a `VersionOverride` for this exact
/// (instance, target) pair, then immediately re-applies the pack to just
/// this instance so the resolution takes effect right away rather than
/// waiting for the next full apply. On success, also marks the instance as
/// tracking this pack (mirroring `apply_sync_pack`'s own bookkeeping) — this
/// is the natural "first real attach" moment for a target that had
/// conflicting content.
#[tauri::command(async)]
pub fn resolve_sync_conflict(
    state: State<'_, AppState>,
    pack_id: String,
    instance_id: String,
    target_label: String,
    adopt_strategy: AdoptStrategy,
) -> Result<SyncReport, String> {
    state
        .sync_engine
        .registry()
        .set_override(&pack_id, &instance_id, &target_label, adopt_strategy)
        .map_err(|e| e.to_string())?;

    let dirs = instance_dirs(&state, std::slice::from_ref(&instance_id))?;
    let report = state
        .sync_engine
        .apply(&pack_id, &dirs)
        .map_err(|e| e.to_string())?;

    if !report.has_failures() {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        if let Some(inst) = svc.get(&instance_id) {
            let mut ids = inst.sync_pack_ids;
            if !ids.iter().any(|x| x == &pack_id) {
                ids.push(pack_id.clone());
                let _ = svc.set_instance_sync_packs(&instance_id, ids);
            }
        }
    }

    Ok(report)
}

/// Clears a previously-set per-instance override for one target, reverting
/// it to the pack's default handling. Does not re-apply anything — the
/// frontend should re-run `preview_sync`/`apply_sync_pack` afterward if it
/// wants the effect to take place immediately.
#[tauri::command(async)]
pub fn clear_sync_conflict_override(
    state: State<'_, AppState>,
    pack_id: String,
    instance_id: String,
    target_label: String,
) -> Result<SyncPack, String> {
    state
        .sync_engine
        .registry()
        .clear_override(&pack_id, &instance_id, &target_label)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn detach_sync_pack(
    state: State<'_, AppState>,
    pack_id: String,
    instance_id: String,
    mode: DetachMode,
) -> Result<SyncReport, String> {
    let dir = instance_dir(&state, &instance_id)?;
    let report = state
        .sync_engine
        .detach(&pack_id, &instance_id, &dir, mode)
        .map_err(|e| e.to_string())?;

    if !report.has_failures() {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        if let Some(inst) = svc.get(&instance_id) {
            let ids: Vec<String> = inst
                .sync_pack_ids
                .into_iter()
                .filter(|x| x != &pack_id)
                .collect();
            let _ = svc.set_instance_sync_packs(&instance_id, ids);
        }
    }

    Ok(report)
}

/// Re-runs `preview` for the pack against every instance currently
/// tracking it (via `JavaInstance.sync_pack_ids`), so the UI can show a
/// pack's current status without the caller re-collecting instance ids.
#[tauri::command(async)]
pub fn get_sync_report(
    state: State<'_, AppState>,
    pack_id: String,
) -> Result<SyncReport, String> {
    let ids: Vec<String> = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        svc.instances()
            .into_iter()
            .filter(|i| i.sync_pack_ids.iter().any(|x| x == &pack_id))
            .map(|i| i.id)
            .collect()
    };
    let dirs = instance_dirs(&state, &ids)?;
    state
        .sync_engine
        .preview(&pack_id, &dirs)
        .map_err(|e| e.to_string())
}
