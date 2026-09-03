//! Resumable-update marker file, mirroring the `PendingMarkerPath` /
//! `TryResumePendingUpdate` logic in `AutoUpdateService.cs`.
//!
//! The C# version writes a plain-text marker file (containing the path to
//! the downloaded-but-not-yet-swapped-in update exe) into the temp directory
//! when a self-replace copy loop in the generated `.bat` script fails
//! repeatedly (e.g. the old exe's file handle is still held by an antivirus
//! scan). On next launch, `TryResumePendingUpdate` reads that marker, deletes
//! it, and — if the referenced file still exists — re-attempts the swap.
//!
//! This crate doesn't own process/OS swap orchestration (that's platform
//! glue that belongs in the Tauri host, which knows its own current exe
//! path), so we port only the state-tracking half: reading, writing, and
//! validating the marker. The caller is expected to perform the actual
//! file-swap + relaunch using the [`PathBuf`] this returns.
//!
//! # TODO
//! Bulk update-file download is intentionally not implemented here — wire it
//! to `gl-download` once that crate exists. For now, callers can issue a
//! plain `reqwest` GET for the release asset (see [`crate::releases`]).

use std::path::{Path, PathBuf};

use crate::error::Result;

const MARKER_FILE_NAME: &str = "GlacierLauncher_pending_update.txt";

/// Path to the pending-update marker file, mirroring
/// `Path.Combine(Path.GetTempPath(), "GlacierLauncher_pending_update.txt")`.
pub fn marker_path() -> PathBuf {
    std::env::temp_dir().join(MARKER_FILE_NAME)
}

/// Writes the marker file recording `tmp_path` as the pending update's
/// downloaded exe, so a later launch can resume the swap. Mirrors the
/// `echo {tmpPath}> "{PendingMarkerPath}"` line in the generated batch
/// script's failure branch.
pub fn write_pending_marker(tmp_path: &Path) -> Result<()> {
    std::fs::write(marker_path(), tmp_path.to_string_lossy().as_ref())?;
    Ok(())
}

/// Removes the marker file if present, ignoring a missing file. Mirrors
/// `if exist "{PendingMarkerPath}" del "{PendingMarkerPath}"` on the
/// successful-copy path.
pub fn clear_pending_marker() -> Result<()> {
    let path = marker_path();
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// Checks for and consumes a pending-update marker left over from a prior
/// launch, mirroring `TryResumePendingUpdate`.
///
/// Reads the marker (if any), deletes it immediately (matching the C#
/// read-then-delete order so a corrupt/stale marker isn't retried forever),
/// and returns the recorded update-exe path only if that file still exists
/// on disk. Returns `Ok(None)` for "nothing to resume" — mirrors the C#
/// method returning `false` for a missing marker, empty path, or missing
/// target file (the C# also swallows all exceptions into `false`, so this
/// port never surfaces marker-file I/O errors as failures either).
pub fn take_pending_update() -> Result<Option<PathBuf>> {
    let marker = marker_path();
    if !marker.exists() {
        return Ok(None);
    }

    let contents = match std::fs::read_to_string(&marker) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    // Best-effort delete, mirroring the C# unconditional `File.Delete`
    // immediately after reading, regardless of what happens next.
    let _ = std::fs::remove_file(&marker);

    let tmp_path = contents.trim();
    if tmp_path.is_empty() {
        return Ok(None);
    }

    let tmp_path = PathBuf::from(tmp_path);
    if !tmp_path.exists() {
        return Ok(None);
    }

    Ok(Some(tmp_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // The marker path is a fixed location in the OS temp dir, so serialize
    // these tests to avoid cross-test interference.
    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn no_marker_returns_none() {
        let _guard = LOCK.lock().unwrap();
        let _ = clear_pending_marker();
        assert!(take_pending_update().unwrap().is_none());
    }

    #[test]
    fn marker_pointing_at_missing_file_is_ignored() {
        let _guard = LOCK.lock().unwrap();
        let ghost = std::env::temp_dir().join("gl_update_test_ghost_file_does_not_exist.exe");
        let _ = std::fs::remove_file(&ghost);
        write_pending_marker(&ghost).unwrap();

        assert!(take_pending_update().unwrap().is_none());
        // Marker should have been consumed even though the target was missing.
        assert!(!marker_path().exists());
    }

    #[test]
    fn marker_pointing_at_real_file_is_returned_and_consumed() {
        let _guard = LOCK.lock().unwrap();
        let real = std::env::temp_dir().join("gl_update_test_real_file.exe");
        std::fs::write(&real, b"fake exe bytes").unwrap();
        write_pending_marker(&real).unwrap();

        let resumed = take_pending_update().unwrap();
        assert_eq!(resumed.as_deref(), Some(real.as_path()));
        assert!(!marker_path().exists());

        let _ = std::fs::remove_file(&real);
    }

    #[test]
    fn clear_pending_marker_is_idempotent() {
        let _guard = LOCK.lock().unwrap();
        clear_pending_marker().unwrap();
        clear_pending_marker().unwrap();
    }
}
