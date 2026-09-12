//! Orchestrates applying a sync pack to a set of instances. **Always
//! dry-run first**: `preview()` runs the exact same target-resolution logic
//! as `apply()` but every handler is called with `dry_run = true`, so real
//! files are never touched until the caller has seen the report and
//! explicitly calls `apply()`.

use crate::error::{Result, SyncError};
use crate::handlers::{dir_link, file_copy, mods, options_merge};
use crate::model::{DetachMode, MergeFormat, SyncPack, SyncTarget};
use crate::registry::SyncRegistry;
use crate::report::SyncReport;
use crate::resolution::resolve;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct SyncEngine {
    registry: SyncRegistry,
}

impl SyncEngine {
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            registry: SyncRegistry::new(root_dir)?,
        })
    }

    pub fn registry(&self) -> &SyncRegistry {
        &self.registry
    }

    /// Dry-run: describes exactly what `apply` would do, touching nothing.
    pub fn preview(
        &self,
        pack_id: &str,
        instance_dirs: &HashMap<String, PathBuf>,
    ) -> Result<SyncReport> {
        self.run(pack_id, instance_dirs, true)
    }

    /// Actually applies the pack. Callers should always show the caller a
    /// `preview()` result first and only call this after user confirmation
    /// (enforced by the Tauri command layer, not by this signature, since a
    /// library-level "already previewed" flag would just be trusted state).
    pub fn apply(
        &self,
        pack_id: &str,
        instance_dirs: &HashMap<String, PathBuf>,
    ) -> Result<SyncReport> {
        self.run(pack_id, instance_dirs, false)
    }

    fn run(
        &self,
        pack_id: &str,
        instance_dirs: &HashMap<String, PathBuf>,
        dry_run: bool,
    ) -> Result<SyncReport> {
        let pack = self
            .registry
            .get(pack_id)
            .ok_or_else(|| SyncError::PackNotFound(pack_id.to_string()))?;
        let mut report = SyncReport::new(pack_id, dry_run);

        for (instance_id, instance_dir) in instance_dirs {
            for target in &pack.targets {
                self.apply_target(&mut report, &pack, instance_id, instance_dir, target, dry_run)?;
            }
        }
        Ok(report)
    }

    fn apply_target(
        &self,
        report: &mut SyncReport,
        pack: &SyncPack,
        instance_id: &str,
        instance_dir: &Path,
        target: &SyncTarget,
        dry_run: bool,
    ) -> Result<()> {
        let label = target.label();
        let adopt = resolve(pack, instance_id, target);

        match target {
            SyncTarget::DirLink { relative_path } => {
                let store_dir = self.registry.target_store_path(&pack.id, &label);
                let instance_path = instance_dir.join(relative_path);
                dir_link::apply(report, instance_id, &label, &store_dir, &instance_path, adopt, dry_run)
            }
            SyncTarget::FileCopy { relative_path } => {
                let store_file = self.registry.target_store_path(&pack.id, &label);
                let instance_path = instance_dir.join(relative_path);
                file_copy::apply(report, instance_id, &label, &store_file, &instance_path, dry_run)
            }
            SyncTarget::FileMerge { relative_path, format } => match format {
                MergeFormat::OptionsTxt => {
                    let store_file = self.registry.target_store_path(&pack.id, &label);
                    let instance_path = instance_dir.join(relative_path);
                    options_merge::apply(
                        report,
                        instance_id,
                        &label,
                        &store_file,
                        &instance_path,
                        adopt,
                        dry_run,
                    )
                }
            },
            SyncTarget::Mods { mod_ids } => {
                let mods_dir = instance_dir.join("mods");
                mods::preview(report, instance_id, &label, &mods_dir, mod_ids);
                Ok(())
            }
        }
    }

    /// Detach one instance from one target of a pack.
    pub fn detach(
        &self,
        pack_id: &str,
        instance_id: &str,
        instance_dir: &Path,
        mode: DetachMode,
    ) -> Result<SyncReport> {
        let pack = self
            .registry
            .get(pack_id)
            .ok_or_else(|| SyncError::PackNotFound(pack_id.to_string()))?;
        let mut report = SyncReport::new(pack_id, false);

        for target in &pack.targets {
            let label = target.label();
            match target {
                SyncTarget::DirLink { relative_path } => {
                    let instance_path = instance_dir.join(relative_path);
                    dir_link::detach(&mut report, instance_id, &label, &instance_path, mode, false)?;
                }
                SyncTarget::FileCopy { relative_path } => {
                    let instance_path = instance_dir.join(relative_path);
                    file_copy::detach(&mut report, instance_id, &label, &instance_path, mode, false)?;
                }
                SyncTarget::FileMerge { relative_path, format } => match format {
                    MergeFormat::OptionsTxt => {
                        let instance_path = instance_dir.join(relative_path);
                        options_merge::detach(&mut report, instance_id, &label, &instance_path, mode, false)?;
                    }
                },
                SyncTarget::Mods { .. } => {
                    // No file-level state to detach; mods stay installed.
                }
            }
        }
        Ok(report)
    }
}
