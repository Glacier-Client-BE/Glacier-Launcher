//! `SyncTarget::FileCopy` handler: a one-shot copy from the pack's canonical
//! file into the instance. Not an ongoing link — after applying, the
//! instance's copy is independent (re-`apply` overwrites it again).

use crate::error::{io_err, Result};
use crate::report::{ReportAction, SyncReport};
use std::path::Path;

pub fn apply(
    report: &mut SyncReport,
    instance_id: &str,
    target_label: &str,
    store_file: &Path,
    instance_file: &Path,
    dry_run: bool,
) -> Result<()> {
    if !store_file.exists() {
        // Nothing canonical yet: seed the store from this instance's file
        // if it has one, otherwise there's nothing to copy.
        if instance_file.exists() {
            if !dry_run {
                if let Some(parent) = store_file.parent() {
                    std::fs::create_dir_all(parent).map_err(io_err(parent))?;
                }
                std::fs::copy(instance_file, store_file).map_err(io_err(store_file))?;
            }
            report.push(
                instance_id,
                target_label,
                ReportAction::NoOp,
                "seeded pack store from this instance's existing file",
            );
        } else {
            report.push(
                instance_id,
                target_label,
                ReportAction::NoOp,
                "no canonical copy yet and instance has none either; nothing to do",
            );
        }
        return Ok(());
    }

    if !dry_run {
        if let Some(parent) = instance_file.parent() {
            std::fs::create_dir_all(parent).map_err(io_err(parent))?;
        }
        std::fs::copy(store_file, instance_file).map_err(io_err(instance_file))?;
    }
    report.push(
        instance_id,
        target_label,
        ReportAction::Copied,
        format!("{} <- {}", instance_file.display(), store_file.display()),
    );
    Ok(())
}

/// Detach: materializing a `FileCopy` is a no-op (it was never linked, the
/// instance already owns an independent copy); discarding removes the file.
pub fn detach(
    report: &mut SyncReport,
    instance_id: &str,
    target_label: &str,
    instance_file: &Path,
    mode: crate::model::DetachMode,
    dry_run: bool,
) -> Result<()> {
    match mode {
        crate::model::DetachMode::Materialize => {
            report.push(
                instance_id,
                target_label,
                ReportAction::DetachedMaterialized,
                "file copy was already independent; nothing to change",
            );
        }
        crate::model::DetachMode::Discard => {
            if instance_file.exists() && !dry_run {
                std::fs::remove_file(instance_file).map_err(io_err(instance_file))?;
            }
            report.push(
                instance_id,
                target_label,
                ReportAction::DetachedDiscarded,
                "file removed",
            );
        }
    }
    Ok(())
}
