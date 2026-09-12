//! `SyncTarget::FileMerge` handler for `MergeFormat::OptionsTxt`: merges the
//! pack's canonical `options.txt` into each instance's own, so pack-managed
//! keys converge while instance-local-only keys (key bindings, `lang`,
//! anything the pack doesn't track) are left untouched.

use crate::error::{io_err, Result};
use crate::model::{AdoptStrategy, DetachMode};
use crate::options_format::OptionsDocument;
use crate::report::{ReportAction, SyncReport};
use std::path::Path;

/// Managed keys are derived as "every key present in the pack's canonical
/// document" — the pack owns whatever keys its canonical file contains, and
/// leaves every other key alone.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    report: &mut SyncReport,
    instance_id: &str,
    target_label: &str,
    store_file: &Path,
    instance_file: &Path,
    adopt: Option<AdoptStrategy>,
    dry_run: bool,
) -> Result<()> {
    let mut pack_doc = OptionsDocument::read(store_file)?;
    let instance_exists = instance_file.exists();

    if pack_doc.entries.is_empty() && instance_exists {
        // First run: seed the pack's canonical copy from this instance so
        // there's something to converge on, honoring adopt strategy if one
        // was given (KeepExisting is the natural default here).
        pack_doc = OptionsDocument::read(instance_file)?;
        if !dry_run {
            pack_doc.write(store_file)?;
        }
        report.push(
            instance_id,
            target_label,
            ReportAction::NoOp,
            "seeded pack's canonical options.txt from this instance",
        );
        return Ok(());
    }

    let managed_keys: Vec<String> = pack_doc.entries.iter().map(|(k, _)| k.clone()).collect();

    match adopt {
        Some(AdoptStrategy::TakePackVersion) => {
            if !dry_run {
                pack_doc.write(instance_file)?;
            }
            report.push(
                instance_id,
                target_label,
                ReportAction::Merged,
                "instance options.txt replaced with pack version",
            );
        }
        Some(AdoptStrategy::KeepExisting) => {
            let instance_doc = OptionsDocument::read(instance_file)?;
            if !dry_run {
                instance_doc.write(store_file)?;
            }
            report.push(
                instance_id,
                target_label,
                ReportAction::NoOp,
                "pack's canonical options.txt replaced with this instance's existing values",
            );
        }
        Some(AdoptStrategy::Merge) | None => {
            let mut instance_doc = OptionsDocument::read(instance_file)?;
            instance_doc.merge_from_pack(&pack_doc, &managed_keys);
            if !dry_run {
                instance_doc.write(instance_file)?;
            }
            report.push(
                instance_id,
                target_label,
                ReportAction::Merged,
                format!("{} pack-managed key(s) merged, local-only keys preserved", managed_keys.len()),
            );
        }
    }
    Ok(())
}

pub fn detach(
    report: &mut SyncReport,
    instance_id: &str,
    target_label: &str,
    instance_file: &Path,
    mode: DetachMode,
    dry_run: bool,
) -> Result<()> {
    match mode {
        DetachMode::Materialize => {
            report.push(
                instance_id,
                target_label,
                ReportAction::DetachedMaterialized,
                "merged options.txt left in place with current values",
            );
        }
        DetachMode::Discard => {
            if instance_file.exists() && !dry_run {
                std::fs::remove_file(instance_file).map_err(io_err(instance_file))?;
            }
            report.push(
                instance_id,
                target_label,
                ReportAction::DetachedDiscarded,
                "options.txt removed",
            );
        }
    }
    Ok(())
}
