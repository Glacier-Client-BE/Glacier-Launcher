//! Persistence for the sync-pack list, following the exact
//! `gl_persistence::{save, load, write_atomic}` + `Envelope.schema_version`
//! pattern used throughout the project (see `gl-java::instance_service` /
//! `gl-settings`). `gl-sync` deliberately doesn't depend on `gl-settings`,
//! so the launcher root is taken as a constructor parameter, mirroring
//! `gl_java::InstanceService::new(root_dir)`.

use crate::error::{io_err, Result, SyncError};
use crate::model::SyncPack;
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
            Err(_) => Vec::new(), // already quarantined by load()
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
        if let Some(existing) = list.iter_mut().find(|p| p.id == pack.id) {
            *existing = pack;
        } else {
            list.push(pack);
        }
        self.save(&list)
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let mut list = self.packs.lock().expect("sync pack list poisoned");
        let before = list.len();
        list.retain(|p| p.id != id);
        let changed = list.len() != before;
        if changed {
            self.save(&list)?;
            let _ = std::fs::remove_dir_all(Self::store_root_of(&self.root_dir).join(id));
        }
        Ok(changed)
    }

    fn save(&self, list: &[SyncPack]) -> Result<()> {
        gl_persistence::save(&self.index_path, SCHEMA_VERSION, &list.to_vec())
            .map_err(SyncError::from)
    }
}
