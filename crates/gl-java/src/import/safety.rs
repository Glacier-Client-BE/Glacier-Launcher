//! Pre-flight checks run before an import copy starts: refuse to import
//! into an instance id/path that already exists, and refuse when there
//! isn't enough free disk space for the copy.

use super::common::dir_size_bytes;
use crate::error::{JavaError, Result};
use crate::instance_service::InstanceService;
use crate::util::slug;
use std::path::Path;

/// Errs if an instance with the slug of `name` already exists in `service`,
/// mirroring the id-collision rule `InstanceService::create` already
/// resolves by appending `-2`, `-3`, ... — import intentionally does NOT
/// auto-suffix, because silently landing in an existing instance's directory
/// (or a same-named-but-different one) is exactly the kind of surprise this
/// check exists to prevent. Callers that want auto-rename should pick a new
/// `name` and retry.
pub fn ensure_no_id_collision(service: &InstanceService, name: &str) -> Result<()> {
    let candidate_id = slug(name);
    if service
        .instances()
        .iter()
        .any(|i| i.id.eq_ignore_ascii_case(&candidate_id))
    {
        return Err(JavaError::Other(format!(
            "An instance named '{name}' (id '{candidate_id}') already exists. Rename it or choose a different name before importing."
        )));
    }
    Ok(())
}

/// Errs if the destination drive doesn't have enough free space for a copy
/// of `source_dir`'s current on-disk size, with a 5% safety margin. Uses the
/// same `GetDiskFreeSpaceExW` idea as `src-tauri/src/commands/launch.rs`'s
/// `free_disk_bytes`, reimplemented here so this crate stays free of a
/// dependency on the `src-tauri` binary crate (which can't be depended on
/// from a library crate anyway).
pub fn ensure_enough_disk_space(source_dir: &Path, dest_drive_probe: &Path) -> Result<()> {
    let needed = dir_size_bytes(source_dir);
    let margin = needed / 20; // 5%
    let Some(free) = free_disk_bytes(dest_drive_probe) else {
        // Can't determine free space (non-Windows, or the call failed) —
        // don't block the import on an unknown; the copy itself will fail
        // loudly and safely if disk is actually full.
        return Ok(());
    };
    if free < needed.saturating_add(margin) {
        return Err(JavaError::Other(format!(
            "Not enough free disk space: need ~{} MB, only {} MB free.",
            (needed.saturating_add(margin)) / 1_048_576,
            free / 1_048_576
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn free_disk_bytes(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            dir_name: *const u16,
            free_bytes_avail: *mut u64,
            total_bytes: *mut u64,
            total_free_bytes: *mut u64,
        ) -> i32;
    }
    // The destination directory may not exist yet — probe its closest
    // existing ancestor so the Win32 call has a real path to resolve.
    let mut probe = path.to_path_buf();
    while !probe.exists() {
        let Some(parent) = probe.parent() else {
            return None;
        };
        probe = parent.to_path_buf();
    }
    let absolute = std::fs::canonicalize(&probe).unwrap_or(probe);
    let wide: Vec<u16> = absolute
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut free = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free) };
    if ok != 0 {
        Some(free)
    } else {
        None
    }
}

#[cfg(not(windows))]
fn free_disk_bytes(_path: &Path) -> Option<u64> {
    None
}
