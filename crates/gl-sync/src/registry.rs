//! Persistence for the sync-pack list, following the exact
//! `gl_persistence::{save, load, write_atomic}` + `Envelope.schema_version`
//! pattern used throughout the project (see `gl-java::instance_service` /
//! `gl-settings`). `gl-sync` deliberately doesn't depend on `gl-settings`,
//! so the launcher root is taken as a constructor parameter, mirroring
//! `gl_java::InstanceService::new(root_dir)`.

use crate::error::{io_err, Result, SyncError};
use crate::model::{AdoptStrategy, SyncPack, VersionOverride};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SCHEMA_VERSION: u32 = 1;

pub struct SyncRegistry {
    root_dir: PathBuf,
    index_path: PathBuf,
    packs: Mutex<Vec<SyncPack>>,
}

impl SyncRegistry {
    /// `root_dir` is the sync-packs area under the launcher root, e.g.
    /// `%USERPROFILE%\Glacier Launcher\sync`.
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root_dir).map_err(io_err(&root_dir))?;
        std::fs::create_dir_all(Self::store_root_of(&root_dir)).map_err(io_err(&root_dir))?;
        let index_path = root_dir.join("sync-packs.json");
        let packs = match gl_persistence::load::<Vec<SyncPack>>(&index_path) {
            Ok(Some(envelope)) => envelope.data,
            Ok(None) => Vec::new(),
            // A corrupt file was already quarantined by load(); an I/O error
            // must not become an empty list, since the next save would
            // overwrite every pack.
            Err(gl_persistence::PersistError::Json(_)) => Vec::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            root_dir,
            index_path,
            packs: Mutex::new(packs),
        })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn store_root_of(root: &Path) -> PathBuf {
        root.join("store")
    }

    /// Canonical on-disk storage directory for one pack's target content,
    /// e.g. `sync/store/<pack_id>/<sanitized target label>`.
    pub fn target_store_path(&self, pack_id: &str, target_label: &str) -> PathBuf {
        let safe: String = target_label
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        Self::store_root_of(&self.root_dir).join(pack_id).join(safe)
    }

    pub fn list(&self) -> Vec<SyncPack> {
        self.packs.lock().expect("sync pack list poisoned").clone()
    }

    pub fn get(&self, id: &str) -> Option<SyncPack> {
        self.packs
            .lock()
            .expect("sync pack list poisoned")
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    pub fn upsert(&self, pack: SyncPack) -> Result<()> {
        let mut list = self.packs.lock().expect("sync pack list poisoned");
        let mut next = list.clone();
        if let Some(existing) = next.iter_mut().find(|p| p.id == pack.id) {
            *existing = pack;
        } else {
            next.push(pack);
        }
        self.save(&next)?;
        *list = next;
        Ok(())
    }

    /// Records the user's answer to a `ReportAction::ConflictNeedsResolution`
    /// for one (instance, target) pair — replacing any existing override for
    /// that same pair — and persists it. This is the only way `resolve()`
    /// (see `resolution.rs`) ever returns `Some`, so a target stays on the
    /// unresolved "please choose" path (`adopt.is_none()` in the handlers)
    /// until this has been called at least once for it.
    pub fn set_override(
        &self,
        pack_id: &str,
        instance_id: &str,
        target_label: &str,
        adopt_strategy: AdoptStrategy,
    ) -> Result<SyncPack> {
        let mut list = self.packs.lock().expect("sync pack list poisoned");
        let mut next = list.clone();
        let pack = next
            .iter_mut()
            .find(|p| p.id == pack_id)
            .ok_or_else(|| SyncError::PackNotFound(pack_id.to_string()))?;
        match pack
            .overrides
            .iter_mut()
            .find(|o| o.instance_id == instance_id && o.target_label == target_label)
        {
            Some(existing) => existing.adopt_strategy = adopt_strategy,
            None => pack.overrides.push(VersionOverride {
                instance_id: instance_id.to_string(),
                target_label: target_label.to_string(),
                adopt_strategy,
            }),
        }
        let updated = pack.clone();
        self.save(&next)?;
        *list = next;
        Ok(updated)
    }

    /// Removes a previously-set override, if any, so the target goes back to
    /// the pack's default handling (or back to needing a fresh resolution,
    /// for a target kind that still has real conflicting content).
    pub fn clear_override(
        &self,
        pack_id: &str,
        instance_id: &str,
        target_label: &str,
    ) -> Result<SyncPack> {
        let mut list = self.packs.lock().expect("sync pack list poisoned");
        let mut next = list.clone();
        let pack = next
            .iter_mut()
            .find(|p| p.id == pack_id)
            .ok_or_else(|| SyncError::PackNotFound(pack_id.to_string()))?;
        pack.overrides
            .retain(|o| !(o.instance_id == instance_id && o.target_label == target_label));
        let updated = pack.clone();
        self.save(&next)?;
        *list = next;
        Ok(updated)
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let mut list = self.packs.lock().expect("sync pack list poisoned");
        let mut next = list.clone();
        next.retain(|p| p.id != id);
        let changed = next.len() != list.len();
        if changed {
            self.save(&next)?;
            *list = next;
            let _ = std::fs::remove_dir_all(Self::store_root_of(&self.root_dir).join(id));
        }
        Ok(changed)
    }

    fn save(&self, list: &[SyncPack]) -> Result<()> {
        gl_persistence::save(&self.index_path, SCHEMA_VERSION, &list.to_vec())
            .map_err(SyncError::from)
    }
}
