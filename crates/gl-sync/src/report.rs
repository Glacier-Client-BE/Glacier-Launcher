//! Structured result type shared by `preview()` and `apply()` so the UI
//! renders the same shape either way — the only difference is whether the
//! actions described actually happened.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReportAction {
    /// A junction was (or would be) created.
    LinkCreated,
    /// Cross-volume fallback: a real copy was (or would be) made instead of
    /// a junction.
    CopiedInsteadOfLinked,
    /// A file was merged in place.
    Merged,
    /// A file was copied once (no ongoing link).
    Copied,
    /// A mod was (or would be) installed to satisfy the pack's mod list.
    ModInstalled,
    /// Nothing needed to change; already in the desired state.
    NoOp,
    /// A conflict was found that needs an explicit adopt-strategy decision
    /// before anything can be touched.
    ConflictNeedsResolution,
    /// The target was detached, materializing real content back into the
    /// instance.
    DetachedMaterialized,
    /// The target was detached and discarded.
    DetachedDiscarded,
    /// The operation failed; see `detail`.
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportEntry {
    pub instance_id: String,
    pub target_label: String,
    pub action: ReportAction,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SyncReport {
    pub pack_id: String,
    /// True for a `preview()` result (nothing was touched), false for an
    /// `apply()`/`detach()` result.
    pub dry_run: bool,
    pub entries: Vec<ReportEntry>,
}

impl SyncReport {
    pub fn new(pack_id: impl Into<String>, dry_run: bool) -> Self {
        Self {
            pack_id: pack_id.into(),
            dry_run,
            entries: Vec::new(),
        }
    }

    pub fn push(
        &mut self,
        instance_id: impl Into<String>,
        target_label: impl Into<String>,
        action: ReportAction,
        detail: impl Into<String>,
    ) {
        self.entries.push(ReportEntry {
            instance_id: instance_id.into(),
            target_label: target_label.into(),
            action,
            detail: detail.into(),
        });
    }

    pub fn has_unresolved_conflicts(&self) -> bool {
        self.entries
            .iter()
            .any(|e| e.action == ReportAction::ConflictNeedsResolution)
    }

    pub fn has_failures(&self) -> bool {
        self.entries
            .iter()
            .any(|e| e.action == ReportAction::Failed)
    }
}
