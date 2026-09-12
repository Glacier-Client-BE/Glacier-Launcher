//! Stage -> copy -> finalize -> (rollback on failure) for a single
//! [`ImportCandidate`].
//!
//! Reuses [`InstanceService::create`] for id allocation/layout and
//! [`InstanceService::set_instance_loader`] for finalizing the detected
//! loader, exactly like the rest of this crate does. The directory-copy
//! step is a local `copy_dir_recursive` rather than a shared helper:
//! `InstanceService::duplicate` and `modpack_transfer::import_official_minecraft`
//! each already carry their own private copy of the same recursive-copy
//! logic (see `PARITY.md` for why a three-way shared helper wasn't
//! extracted — those two functions are out of this phase's edit scope and
//! reworking them to share code was judged too invasive to risk here).

use super::safety;
use super::ImportCandidate;
use crate::error::{io_err, JavaError, Result};
use crate::instance_service::InstanceService;
use crate::models::JavaInstance;
use std::path::{Path, PathBuf};

/// Outcome of importing one candidate — deliberately not a bare `Result` at
/// the call-site boundary in `import_instances`, so one candidate's failure
/// never aborts the others; this type is what gets wrapped per-item there.
pub fn import_one(service: &InstanceService, candidate: &ImportCandidate) -> Result<JavaInstance> {
    safety::ensure_no_id_collision(service, &candidate.name)?;
    safety::ensure_enough_disk_space(&candidate.instance_path, service.instances_dir().as_path())?;

    // Stage: allocate the new instance's id/metadata/empty layout first, so
    // we know exactly which directory to clean up if anything below fails.
    let created = service.create(&candidate.name, &candidate.mc_version)?;
    let dest_dir = PathBuf::from(&created.directory);

    let copy_result = copy_dir_recursive(&candidate.instance_path, &dest_dir);
    if let Err(e) = copy_result {
        rollback(service, &created.id, &dest_dir);
        return Err(e);
    }

    if let Err(e) = service.set_instance_loader(
        &created.id,
        candidate.loader,
        candidate.loader_version.clone(),
        None,
    ) {
        rollback(service, &created.id, &dest_dir);
        return Err(e);
    }

    service
        .get(&created.id)
        .ok_or_else(|| JavaError::InstanceNotFound(created.id.clone()))
}

/// Import every candidate, reporting a per-item `Result` rather than
/// stopping at the first failure — matches the `import_instances` Tauri
/// command's "not all-or-nothing" contract.
pub fn import_many(
    service: &InstanceService,
    candidates: &[ImportCandidate],
) -> Vec<(String, Result<JavaInstance>)> {
    candidates
        .iter()
        .map(|c| (c.name.clone(), import_one(service, c)))
        .collect()
}

/// Delete the partially-created instance directory and its index entry on
/// any failure after `create` succeeded. Best-effort: a failure to clean up
/// is logged (well, silently swallowed — this crate has no tracing
/// dependency call sites for this deep a helper) rather than compounding the
/// original error.
fn rollback(service: &InstanceService, instance_id: &str, dest_dir: &Path) {
    let _ = std::fs::remove_dir_all(dest_dir);
    let _ = service.delete(instance_id);
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target).map_err(io_err(target))?;
    if !source.exists() {
        return Err(JavaError::Other(format!(
            "source instance directory not found: {}",
            source.display()
        )));
    }
    for entry in walk_dir(source) {
        let rel = entry.strip_prefix(source).expect("walked under source");
        let dest = target.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(io_err(&dest))?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(io_err(parent))?;
            }
            std::fs::copy(&entry, &dest).map_err(io_err(&dest))?;
        }
    }
    Ok(())
}

fn walk_dir(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            out.push(path);
        }
    }
    out
}
