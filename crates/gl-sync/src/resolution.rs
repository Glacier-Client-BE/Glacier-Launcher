//! Figures out, per target/instance pair, whether an adopt-strategy
//! decision is required before anything can be touched, and if so, what the
//! caller (via `VersionOverride`) has already decided.

use crate::model::{AdoptStrategy, SyncPack, SyncTarget};

/// What to do for one target on one instance, as resolved from any
/// `VersionOverride` on the pack. `None` means "no existing content
/// conflicts, nothing to resolve" — the handler can proceed directly.
pub fn resolve(pack: &SyncPack, instance_id: &str, target: &SyncTarget) -> Option<AdoptStrategy> {
    let label = target.label();
    pack.overrides
        .iter()
        .find(|o| o.instance_id == instance_id && o.target_label == label)
        .map(|o| o.adopt_strategy)
}

/// Whether a target's kind can legally use `AdoptStrategy::Merge` — only
/// `FileMerge` targets support genuine merging.
pub fn supports_merge(target: &SyncTarget) -> bool {
    matches!(target, SyncTarget::FileMerge { .. })
}
