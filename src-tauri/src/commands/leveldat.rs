//! Bedrock `level.dat` summary read + guarded write, ported from
//! `Services/LevelDatEditorService.cs`. This touches real user save data, so
//! the write path is deliberately defensive:
//!
//! 1. Read + parse the existing file first (fail closed if it doesn't parse).
//! 2. Copy the original file to `<file>.bak-<timestamp>` before touching it.
//! 3. Apply the summary, serialize, write to a sibling temp file.
//! 4. Read the temp file back and re-parse it, asserting the re-derived
//!    summary matches what we intended to write (round-trip verification).
//! 5. Only then atomically rename the temp file over the original.
//!
//! If any step fails, the original `level.dat` is left untouched and the
//! `.bak-*` copy (if one was made) is left in place for manual recovery.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use gl_nbt::{
    apply_summary_to_root, read_level_dat, summary_from_root, write_level_dat, LevelDatSummary,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LevelDatSummaryDto {
    pub game_type: i32,
    pub difficulty: i32,
    pub cheats_enabled: bool,
    pub generator: i32,
    pub random_seed: i64,
    pub has_random_seed: bool,
    pub experiments: std::collections::HashMap<String, bool>,
}

impl From<LevelDatSummary> for LevelDatSummaryDto {
    fn from(s: LevelDatSummary) -> Self {
        Self {
            game_type: s.game_type,
            difficulty: s.difficulty,
            cheats_enabled: s.cheats_enabled,
            generator: s.generator,
            random_seed: s.random_seed,
            has_random_seed: s.has_random_seed,
            experiments: s.experiments,
        }
    }
}

impl From<LevelDatSummaryDto> for LevelDatSummary {
    fn from(d: LevelDatSummaryDto) -> Self {
        Self {
            game_type: d.game_type,
            difficulty: d.difficulty,
            cheats_enabled: d.cheats_enabled,
            generator: d.generator,
            random_seed: d.random_seed,
            has_random_seed: d.has_random_seed,
            experiments: d.experiments,
        }
    }
}

/// Resolves `<world_dir>/level.dat`, mirroring the C# service's fixed file
/// name inside a world folder.
fn level_dat_path(world_dir: &str) -> PathBuf {
    Path::new(world_dir).join("level.dat")
}

#[tauri::command]
pub fn read_level_dat_summary(world_dir: String) -> Result<LevelDatSummaryDto, String> {
    let path = level_dat_path(&world_dir);
    let bytes =
        std::fs::read(&path).map_err(|e| format!("couldn't read {}: {e}", path.display()))?;
    let parsed = read_level_dat(&bytes).map_err(|e| e.to_string())?;
    Ok(summary_from_root(&parsed.root).into())
}

/// Applies `summary` onto the world's `level.dat`, with a mandatory backup
/// and round-trip verification before the write is committed. Returns the
/// path of the backup file that was created.
#[tauri::command]
pub fn write_level_dat_summary(
    world_dir: String,
    summary: LevelDatSummaryDto,
) -> Result<String, String> {
    let path = level_dat_path(&world_dir);

    // 1. Read + parse the existing file.
    let original_bytes =
        std::fs::read(&path).map_err(|e| format!("couldn't read {}: {e}", path.display()))?;
    let parsed = read_level_dat(&original_bytes).map_err(|e| e.to_string())?;

    // 2. Back up the original before any mutation.
    let backup_path = backup_path_for(&path);
    std::fs::copy(&path, &backup_path)
        .map_err(|e| format!("couldn't create backup at {}: {e}", backup_path.display()))?;

    // 3. Apply + serialize.
    let mut new_root = parsed.root.clone();
    let summary: LevelDatSummary = summary.into();
    apply_summary_to_root(&mut new_root, &summary);
    let new_bytes =
        write_level_dat(parsed.version, &parsed.root_name, &new_root).map_err(|e| e.to_string())?;

    // Write to a sibling temp file first.
    let tmp_path = append_suffix(&path, ".writing.tmp");
    if let Err(e) = std::fs::write(&tmp_path, &new_bytes) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("couldn't write temp file: {e}"));
    }

    // 4. Round-trip verify: re-read the temp file and confirm the summary we
    // can derive from it matches what we intended to write. If this fails,
    // delete the temp file and leave the original untouched.
    let verify_result = (|| -> Result<(), String> {
        let reread_bytes = std::fs::read(&tmp_path).map_err(|e| e.to_string())?;
        let reparsed = read_level_dat(&reread_bytes).map_err(|e| e.to_string())?;
        let reread_summary = summary_from_root(&reparsed.root);
        if reread_summary != summary {
            return Err(
                "round-trip verification failed: re-read summary didn't match the intended write"
                    .to_string(),
            );
        }
        Ok(())
    })();

    if let Err(e) = verify_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "{e} — original level.dat was NOT modified (backup kept at {})",
            backup_path.display()
        ));
    }

    // 5. Only now, atomically replace the original.
    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "couldn't commit write: {e} — original level.dat was NOT modified (backup kept at {})",
            backup_path.display()
        ));
    }

    Ok(backup_path.to_string_lossy().into_owned())
}

fn backup_path_for(original: &Path) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    append_suffix(original, &format!(".bak-{stamp}"))
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}
