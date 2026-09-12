//! `SyncTarget::Mods` handler.
//!
//! `gl-sync` deliberately does not depend on `gl-modpack` (mod installation,
//! dependency resolution, and provenance belong there — see Phase 3). This
//! handler only diffs "what the pack wants installed" against "what's
//! actually present" (by scanning the instance's `mods/` directory for
//! filenames containing each mod id, matching the same lightweight
//! provenance-free heuristic `gl-java::mod_analyzer` falls back to) and
//! reports which mod ids still need installing. The Tauri command layer,
//! which does have `gl-modpack` in scope, is responsible for actually
//! resolving and installing anything this reports as missing.

use crate::report::{ReportAction, SyncReport};
use std::path::Path;

pub fn preview(
    report: &mut SyncReport,
    instance_id: &str,
    target_label: &str,
    mods_dir: &Path,
    mod_ids: &[String],
) {
    let installed_names: Vec<String> = std::fs::read_dir(mods_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_lowercase())
                .collect()
        })
        .unwrap_or_default();

    for mod_id in mod_ids {
        let needle = mod_id.to_lowercase();
        let present = installed_names.iter().any(|n| n.contains(&needle));
        if present {
            report.push(
                instance_id,
                target_label,
                ReportAction::NoOp,
                format!("{mod_id}: already present"),
            );
        } else {
            report.push(
                instance_id,
                target_label,
                ReportAction::ModInstalled,
                format!("{mod_id}: needs installing (resolve via gl-modpack at the Tauri layer)"),
            );
        }
    }
}
