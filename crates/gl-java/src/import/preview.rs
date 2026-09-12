//! Build a richer [`ImportCandidate`] preview (mod list) without copying
//! anything. The `scan_*` adapters already populate `mod_count`/
//! `world_count`/`size_bytes` cheaply for list views; this is for the
//! confirm-before-import step where the user wants to see actual mod
//! filenames.

use super::ImportCandidate;
use std::path::Path;

/// Re-derive a candidate for a single known instance path plus the mod
/// filenames found in it. Used by `preview_import` so the UI can show a
/// mod list before committing to the copy.
pub fn mod_filenames(candidate: &ImportCandidate) -> Vec<String> {
    list_jar_names(&candidate.instance_path.join("mods"))
}

fn list_jar_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("jar"))
                .unwrap_or(false)
        })
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    names.sort();
    names
}
