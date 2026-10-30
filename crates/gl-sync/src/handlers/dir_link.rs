//! `SyncTarget::DirLink` handler: keeps a directory identical across
//! instances via a Windows directory junction (no admin elevation needed,
//! unlike symlinks without Developer Mode). Falls back to a real copy when
//! the pack's canonical store and the instance are on different volumes,
//! since junctions require the same volume.
//!
//! Junction creation shells out to `cmd /c mklink /J`, rather than a raw
//! `DeviceIoControl(FSCTL_SET_REPARSE_POINT)` FFI implementation — the raw
//! approach is nontrivial to get exactly right (reparse buffer layout,
//! print-name vs substitute-name quirks) and `mklink /J` is exactly as
//! reliable while being trivially testable by shelling out in tests too.

use crate::error::{io_err, Result, SyncError};
use crate::model::AdoptStrategy;
use crate::report::{ReportAction, SyncReport};
use std::path::{Path, PathBuf};

/// True if `a` and `b` are on the same Windows volume, compared by drive
/// letter of their canonicalized paths. Non-Windows / driveless paths, and
/// paths that don't currently exist (so can't be canonicalized), are
/// treated as "different" so the safer copy fallback is used. Callers that
/// need this check for a path they are about to delete must capture the
/// result *before* deleting it (see `apply()` below) — this function does
/// not fall back to a parent directory.
pub fn same_volume(a: &Path, b: &Path) -> bool {
    fn drive_letter(p: &Path) -> Option<char> {
        let s = p.to_string_lossy();
        // `Path::canonicalize()` on Windows returns a "verbatim" path
        // prefixed with `\\?\` (e.g. `\\?\C:\Users\...`), so the drive
        // letter isn't at bytes[0..2] like it is for a normal path — strip
        // the prefix first or the `X:` check below never matches and every
        // pair of canonicalized paths looks like "different volume".
        let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
        let bytes = s.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' {
            Some((bytes[0] as char).to_ascii_uppercase())
        } else {
            None
        }
    }
    match (
        a.canonicalize().ok().and_then(|p| drive_letter(&p)),
        b.canonicalize().ok().and_then(|p| drive_letter(&p)),
    ) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// Is `path` a junction/reparse point (used to detect "already linked").
pub fn is_junction(path: &Path) -> bool {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        // Windows reparse points (junctions included) set this attribute.
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    } else {
        false
    }
}

fn create_junction(link_path: &Path, target_dir: &Path) -> Result<()> {
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent).map_err(io_err(parent))?;
    }
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link_path)
        .arg(target_dir)
        .status()
        .map_err(io_err(link_path))?;
    if !status.success() {
        return Err(SyncError::Other(format!(
            "mklink /J failed for {} -> {} (exit {:?})",
            link_path.display(),
            target_dir.display(),
            status.code()
        )));
    }
    Ok(())
}

/// A sibling path next to `path`, with `suffix` appended to its file name
/// (not replacing an existing extension the way `Path::with_extension`
/// would — a store/instance directory named e.g. `mods.pack` would
/// otherwise lose the `.pack` part of its name to a staging suffix).
fn staging_sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).map_err(io_err(to))?;
    for entry in std::fs::read_dir(from).map_err(io_err(from))? {
        let entry = entry.map_err(io_err(from))?;
        let dest = to.join(entry.file_name());
        let file_type = entry.file_type().map_err(io_err(from))?;
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest).map_err(io_err(&entry.path()))?;
        }
    }
    Ok(())
}

/// Apply (or, if `dry_run`, only describe) a directory-link target.
///
/// `store_dir` is the pack's canonical copy of this directory (created if
/// absent). `instance_dir` is where it should appear inside the instance.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    report: &mut SyncReport,
    instance_id: &str,
    target_label: &str,
    store_dir: &Path,
    instance_dir: &Path,
    adopt: Option<AdoptStrategy>,
    dry_run: bool,
) -> Result<()> {
    let exists_real = instance_dir.exists() && !is_junction(instance_dir);

    if exists_real && adopt.is_none() {
        report.push(
            instance_id,
            target_label,
            ReportAction::ConflictNeedsResolution,
            format!(
                "{} already exists as a real directory; choose keep-existing, take-pack-version, or merge",
                instance_dir.display()
            ),
        );
        return Ok(());
    }

    if is_junction(instance_dir) {
        report.push(
            instance_id,
            target_label,
            ReportAction::NoOp,
            "already linked".to_string(),
        );
        return Ok(());
    }

    // Resolve existing-content conflict.
    if exists_real {
        match adopt.unwrap() {
            AdoptStrategy::KeepExisting => {
                if !store_dir.exists() {
                    if !dry_run {
                        // Copy to a staging sibling and rename it into place
                        // as `store_dir` atomically, rather than copying
                        // straight into `store_dir`: if the copy fails
                        // partway (e.g. a file locked by a running instance)
                        // this leaves no half-populated `store_dir` behind.
                        // Only once the rename succeeds — meaning
                        // `store_dir` is a complete, verified copy — do we
                        // touch `instance_dir`. If `remove_dir_all` below
                        // then fails, a retry sees `store_dir` already
                        // exists and safely retries just the removal below,
                        // instead of re-copying over (or silently trusting)
                        // a store that was never actually confirmed intact.
                        let staging = staging_sibling(store_dir, "gl-sync-adopt-tmp");
                        if staging.exists() {
                            std::fs::remove_dir_all(&staging).map_err(io_err(&staging))?;
                        }
                        copy_dir_recursive(instance_dir, &staging)?;
                        std::fs::rename(&staging, store_dir).map_err(io_err(store_dir))?;
                        std::fs::remove_dir_all(instance_dir).map_err(io_err(instance_dir))?;
                    }
                } else if !dry_run {
                    // The pack already has a shared copy. "Keep existing" means
                    // this instance's files win, so copy them over the shared
                    // copy, then park the original folder as a backup rather
                    // than deleting it: nothing the player had is ever lost.
                    copy_dir_recursive(instance_dir, store_dir)?;
                    let backup = staging_sibling(instance_dir, "gl-sync-backup");
                    if backup.exists() {
                        std::fs::remove_dir_all(&backup).map_err(io_err(&backup))?;
                    }
                    std::fs::rename(instance_dir, &backup).map_err(io_err(instance_dir))?;
                }
            }
            AdoptStrategy::TakePackVersion => {
                if !dry_run {
                    std::fs::remove_dir_all(instance_dir).map_err(io_err(instance_dir))?;
                }
            }
            AdoptStrategy::Merge => {
                return Err(SyncError::Other(
                    "directory links don't support Merge; use KeepExisting or TakePackVersion"
                        .to_string(),
                ));
            }
        }
    }

    if !store_dir.exists() {
        if !dry_run {
            std::fs::create_dir_all(store_dir).map_err(io_err(store_dir))?;
        }
    }

    // `instance_dir` may no longer exist at this point: the KeepExisting /
    // TakePackVersion branches above just removed it, and on a fresh (no
    // conflict) apply it may never have existed in the first place. Either
    // way `same_volume`'s `canonicalize()` calls would fail on it and the
    // volume check would always report "different volume", silently taking
    // the copy fallback instead of creating the intended junction. Fall
    // back to the nearest existing ancestor (at minimum, the instance's own
    // directory, which always exists) purely for this volume comparison.
    let volume_check_path = if instance_dir.exists() {
        instance_dir.to_path_buf()
    } else {
        instance_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| instance_dir.to_path_buf())
    };

    if !same_volume(store_dir, &volume_check_path) {
        report.push(
            instance_id,
            target_label,
            ReportAction::CopiedInsteadOfLinked,
            format!(
                "{} and {} are on different volumes; junctions require the same volume, copied instead (this instance will drift from the pack until re-applied)",
                store_dir.display(),
                instance_dir.display()
            ),
        );
        if !dry_run {
            copy_dir_recursive(store_dir, instance_dir)?;
        }
        return Ok(());
    }

    if !dry_run {
        create_junction(instance_dir, store_dir)?;
    }
    report.push(
        instance_id,
        target_label,
        ReportAction::LinkCreated,
        format!("junction {} -> {}", instance_dir.display(), store_dir.display()),
    );
    Ok(())
}

/// Detach a directory-link target from one instance.
pub fn detach(
    report: &mut SyncReport,
    instance_id: &str,
    target_label: &str,
    instance_dir: &Path,
    mode: crate::model::DetachMode,
    dry_run: bool,
) -> Result<()> {
    if !instance_dir.exists() {
        report.push(instance_id, target_label, ReportAction::NoOp, "nothing to detach");
        return Ok(());
    }
    let was_junction = is_junction(instance_dir);
    match mode {
        crate::model::DetachMode::Materialize => {
            if was_junction {
                // Read the real target contents (via the junction) and
                // copy them back as a plain directory before removing the
                // reparse point, so no data is lost.
                let real: PathBuf = instance_dir
                    .canonicalize()
                    .unwrap_or_else(|_| instance_dir.to_path_buf());
                if !dry_run {
                    let tmp = staging_sibling(instance_dir, "gl-sync-materialize-tmp");
                    let junction_stub = staging_sibling(instance_dir, "gl-sync-materialize-junction");
                    if tmp.exists() {
                        std::fs::remove_dir_all(&tmp).map_err(io_err(&tmp))?;
                    }
                    copy_dir_recursive(&real, &tmp)?;
                    // Move the junction out of the way, then swap the real
                    // copy into `instance_dir`'s place, *before* removing
                    // the junction reparse point — so a crash at any point
                    // in this sequence still leaves either the original
                    // junction (still fully functional, just renamed) or
                    // the materialized copy sitting at `instance_dir`.
                    // Never a window where nothing is there at all, unlike
                    // removing the junction first and renaming second.
                    std::fs::rename(instance_dir, &junction_stub).map_err(io_err(instance_dir))?;
                    std::fs::rename(&tmp, instance_dir).map_err(io_err(instance_dir))?;
                    remove_junction(&junction_stub)?;
                }
            }
            report.push(
                instance_id,
                target_label,
                ReportAction::DetachedMaterialized,
                "real content preserved in instance directory",
            );
        }
        crate::model::DetachMode::Discard => {
            if !dry_run {
                if was_junction {
                    remove_junction(instance_dir)?;
                } else {
                    std::fs::remove_dir_all(instance_dir).map_err(io_err(instance_dir))?;
                }
            }
            report.push(
                instance_id,
                target_label,
                ReportAction::DetachedDiscarded,
                "content discarded",
            );
        }
    }
    Ok(())
}

fn remove_junction(path: &Path) -> Result<()> {
    // `remove_dir` (not `remove_dir_all`) on a junction removes only the
    // reparse point, leaving the real target directory's contents intact —
    // exactly what an already-materialized detach needs.
    std::fs::remove_dir(path).map_err(io_err(path))
}

#[cfg(test)]
mod keep_existing_tests {
    use super::*;

    /// "Keep existing" with a shared copy already present used to delete the
    /// instance's own folder outright. Its files must survive.
    #[test]
    fn keep_existing_never_loses_instance_files() {
        let tmp = tempfile::tempdir().unwrap();
        let store = tmp.path().join("store").join("mods");
        let inst = tmp.path().join("inst").join("mods");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::create_dir_all(&inst).unwrap();
        std::fs::write(store.join("shared.jar"), "pack").unwrap();
        std::fs::write(store.join("both.jar"), "pack").unwrap();
        std::fs::write(inst.join("mine.jar"), "player").unwrap();
        std::fs::write(inst.join("both.jar"), "player").unwrap();

        let mut report = SyncReport::new("p", false);
        apply(&mut report, "i", "mods", &store, &inst, Some(AdoptStrategy::KeepExisting), false).unwrap();

        assert_eq!(std::fs::read_to_string(store.join("mine.jar")).unwrap(), "player");
        assert_eq!(std::fs::read_to_string(store.join("both.jar")).unwrap(), "player");
        assert!(store.join("shared.jar").exists());
        let backup = staging_sibling(&inst, "gl-sync-backup");
        assert_eq!(std::fs::read_to_string(backup.join("mine.jar")).unwrap(), "player");
    }
}
