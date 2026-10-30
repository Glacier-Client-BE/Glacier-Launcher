//! Phase 5: import instances from an external launcher (MultiMC/Prism,
//! CurseForge app, ATLauncher, Modrinth App) into Glacier. Thin wrappers
//! over `gl_java::import::*` — all detection/parsing/copy logic lives there.

use gl_java::import::{atlauncher, curseforge_app, detect, mmc, modrinth_app, pipeline, preview};
use gl_java::{DetectedLauncher, ImportCandidate, JavaInstance, SourceLauncher};
use tauri::State;

use crate::state::AppState;

/// Scan `%APPDATA%` for install roots of every supported external launcher.
/// Never errors — an unresolvable `%APPDATA%` or nothing found just yields
/// an empty list, since "no other launchers installed" is a normal outcome.
#[tauri::command(async)]
pub fn detect_external_launchers() -> Vec<DetectedLauncher> {
    detect::detect_all()
}

/// List every instance found under one detected launcher's root.
#[tauri::command(async)]
pub fn scan_launcher_instances(
    launcher: SourceLauncher,
    root_path: String,
) -> Result<Vec<ImportCandidate>, String> {
    let root = std::path::Path::new(&root_path);
    let instances_dir = detect::instances_dir(launcher, root);
    let candidates = match launcher {
        SourceLauncher::MultiMcOrPrism => mmc::scan(&instances_dir),
        SourceLauncher::CurseForgeApp => curseforge_app::scan(&instances_dir),
        SourceLauncher::ATLauncher => atlauncher::scan(&instances_dir),
        SourceLauncher::ModrinthApp => modrinth_app::scan(&instances_dir),
    };
    Ok(candidates)
}

/// Richer preview for one already-scanned candidate: the actual list of mod
/// jar filenames found in its `mods/` folder (the list view's `mod_count`
/// badge is cheap; this is for the confirm-before-import step).
#[tauri::command(async)]
pub fn preview_import(candidate: ImportCandidate) -> Result<Vec<String>, String> {
    Ok(preview::mod_filenames(&candidate))
}

/// Outcome of importing a single candidate, reported by name so the caller
/// doesn't need to re-derive it from an error string. Kept exactly as
/// serializable JSON: `{ name, instance: JavaInstance | null, error: string | null }`.
#[derive(serde::Serialize)]
pub struct ImportOutcome {
    name: String,
    instance: Option<JavaInstance>,
    error: Option<String>,
}

/// Import a batch of candidates. Per-candidate outcome, not all-or-nothing —
/// one failing import (name collision, disk full, unreadable source dir)
/// must not block the rest of the batch.
#[tauri::command(async)]
pub fn import_instances(
    state: State<'_, AppState>,
    candidates: Vec<ImportCandidate>,
) -> Result<Vec<ImportOutcome>, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;

    let results = pipeline::import_many(&svc, &candidates);
    Ok(results
        .into_iter()
        .map(|(name, result)| match result {
            Ok(instance) => ImportOutcome {
                name,
                instance: Some(instance),
                error: None,
            },
            Err(e) => ImportOutcome {
                name,
                instance: None,
                error: Some(e.to_string()),
            },
        })
        .collect())
}
