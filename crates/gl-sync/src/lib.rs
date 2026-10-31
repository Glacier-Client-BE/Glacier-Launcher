//! Sync packs: NoRiskClient's content-sync feature, ported vendor-neutral.
//!
//! A `SyncPack` defines a set of targets (shared directory links, merged
//! files, one-shot file copies, or mod-id lists) that should stay
//! consistent across a chosen set of Java instances. This crate owns the
//! model, the Windows-junction/merge/copy mechanics, and pack persistence;
//! it deliberately does not depend on `gl-java` or `gl-modpack` — the
//! Tauri command layer (`src-tauri/src/commands/sync_packs.rs`) supplies
//! instance directories and wires `JavaInstance.sync_pack_ids` back up
//! after a successful apply.
//!
//! **Every mutating path is preview-first.** [`engine::SyncEngine::preview`]
//! runs the identical logic as [`engine::SyncEngine::apply`]/`detach` with
//! `dry_run = true`, so the UI can always show what would happen before
//! anything real is touched.

pub mod engine;
pub mod error;
pub mod handlers;
pub mod model;
pub mod options_format;
pub mod registry;
pub mod report;
pub mod resolution;

pub use engine::SyncEngine;
pub use error::{Result, SyncError};
pub use model::{AdoptStrategy, DetachMode, MergeFormat, SyncPack, SyncTarget, VersionOverride};
pub use registry::SyncRegistry;
pub use report::{ReportAction, ReportEntry, SyncReport};

#[cfg(test)]
mod tests;
