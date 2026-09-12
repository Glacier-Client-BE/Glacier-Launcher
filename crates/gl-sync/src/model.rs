//! Data model for sync packs: a named set of targets (directory links, file
//! merges, plain file copies, or mod-id lists) that can be applied
//! consistently across multiple Java instances so shared content (e.g.
//! resourcepacks, `options.txt`) doesn't need manual copying.

use serde::{Deserialize, Serialize};

/// A sync pack: a reusable definition of what should stay consistent across
/// the instances it's applied to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncPack {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub targets: Vec<SyncTarget>,
    /// Per-instance exceptions for individual targets (e.g. "this instance
    /// keeps its own options.txt even though the pack manages it").
    #[serde(default)]
    pub overrides: Vec<VersionOverride>,
    pub created_at: String,
    pub updated_at: String,
}

/// One thing a sync pack keeps consistent. Each variant is handled by its
/// own module under `handlers/`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum SyncTarget {
    /// A directory (relative to the instance's `.minecraft`-shaped root,
    /// e.g. `"resourcepacks"`) that should be a junction to a single shared
    /// copy owned by the pack.
    DirLink { relative_path: String },
    /// A single file that should be structurally merged (not overwritten)
    /// across instances, e.g. `options.txt`.
    FileMerge {
        relative_path: String,
        format: MergeFormat,
    },
    /// A single file that should be a plain one-shot copy from the pack's
    /// canonical version (no ongoing link, no merge — just "start from
    /// this").
    FileCopy { relative_path: String },
    /// A list of mod ids (Modrinth/CurseForge project ids, or provenance
    /// ids from `gl-modpack`) that every applied instance should have
    /// installed.
    Mods { mod_ids: Vec<String> },
}

impl SyncTarget {
    /// The relative path this target concerns, for targets that have one.
    /// `Mods` has no single path so returns `None`.
    pub fn relative_path(&self) -> Option<&str> {
        match self {
            SyncTarget::DirLink { relative_path } => Some(relative_path),
            SyncTarget::FileMerge { relative_path, .. } => Some(relative_path),
            SyncTarget::FileCopy { relative_path } => Some(relative_path),
            SyncTarget::Mods { .. } => None,
        }
    }

    /// A short, stable label used in reports and as a resolution/override key.
    pub fn label(&self) -> String {
        match self {
            SyncTarget::DirLink { relative_path } => format!("dir-link:{relative_path}"),
            SyncTarget::FileMerge { relative_path, .. } => format!("file-merge:{relative_path}"),
            SyncTarget::FileCopy { relative_path } => format!("file-copy:{relative_path}"),
            SyncTarget::Mods { .. } => "mods".to_string(),
        }
    }
}

/// Structured merge formats a `FileMerge` target understands. Starts with
/// just Minecraft's `options.txt` (`key:value` per line); more formats can
/// be added without touching the engine.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeFormat {
    OptionsTxt,
}

/// What happens the first time a target is linked/merged into an instance
/// that already has its own copy of that content.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdoptStrategy {
    /// Keep the instance's existing content; the pack's canonical copy is
    /// replaced by it (for a `DirLink`, the existing directory becomes the
    /// pack's shared copy; for a `FileMerge`, the instance's values win).
    KeepExisting,
    /// Discard the instance's existing content; the pack's canonical copy
    /// wins outright.
    TakePackVersion,
    /// Only valid for mergeable targets (`FileMerge`): combine both,
    /// pack-managed keys win, instance-local keys pass through.
    Merge,
}

/// What happens when a target is detached from an instance (the instance
/// stops tracking the pack for that target).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DetachMode {
    /// Copy the real current content back into the instance directory so
    /// nothing is lost (turns a junction into a real directory with the
    /// same contents, or leaves a merged file's current text in place).
    Materialize,
    /// Just remove the link/tracking; the instance ends up with nothing
    /// there (only ever appropriate when the caller has already confirmed
    /// data loss is acceptable — the engine never chooses this silently).
    Discard,
}

/// A per-instance exception: this instance does not follow the pack's
/// default handling for one target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionOverride {
    pub instance_id: String,
    pub target_label: String,
    pub adopt_strategy: AdoptStrategy,
}
