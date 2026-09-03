//! Multiple Bedrock install instance management. Mirrors
//! `Services/BedrockInstanceService.cs`.
//!
//! Deviations from the C# original:
//! - The C# service reads/writes the active instance id through the shared
//!   `SettingsService`. To keep this crate self-contained (and independently
//!   testable against temp dirs) the active id is persisted alongside the
//!   instance list in this crate's own `instances.json` index instead.
//! - `SwitchToAsync`'s call into `VanillaVersionService` to also switch the
//!   installed game version is out of scope here (that lives in a different
//!   crate/service); callers should perform that step themselves after
//!   `switch_to` returns. See the `TODO` below.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backup::{BedrockBackupService, MANAGED_FOLDERS};
use crate::error::{BedrockError, IoContext, Result};
use crate::model::BedrockInstance;
use crate::util::{copy_directory, now_iso, slug, write_atomic};

#[derive(Debug, Default, Serialize, Deserialize)]
struct IndexFile {
    #[serde(default)]
    instances: Vec<BedrockInstance>,
    #[serde(default)]
    active_id: Option<String>,
}

/// Manages a set of independent Bedrock "instances" — each a self-contained
/// snapshot of the managed com.mojang folders that can be swapped in/out of
/// the live com.mojang directory.
pub struct BedrockInstanceService {
    root_dir: PathBuf,
    instances_dir: PathBuf,
    index_path: PathBuf,
    index: IndexFile,
}

impl BedrockInstanceService {
    /// `root_dir` is the service's private storage root (e.g.
    /// `~/Glacier Launcher/bedrock-instances`). `live_root` is the live
    /// com.mojang directory to seed the default instance from, if any.
    pub fn new(root_dir: impl Into<PathBuf>, live_root: Option<&Path>) -> Result<Self> {
        let root_dir = root_dir.into();
        let instances_dir = root_dir.join("instances");
        let index_path = root_dir.join("instances.json");

        std::fs::create_dir_all(&root_dir).ctx(&root_dir)?;
        std::fs::create_dir_all(&instances_dir).ctx(&instances_dir)?;

        let index = Self::load_index(&index_path)?;

        let mut svc = Self {
            root_dir,
            instances_dir,
            index_path,
            index,
        };
        svc.ensure_default_instance(live_root)?;
        Ok(svc)
    }

    pub fn instances_dir(&self) -> &Path {
        &self.instances_dir
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn instances(&self) -> Vec<BedrockInstance> {
        self.index.instances.clone()
    }

    pub fn active_instance_id(&self) -> Option<&str> {
        self.index.active_id.as_deref()
    }

    pub fn active_instance(&self) -> Result<BedrockInstance> {
        let id = self.index.active_id.as_deref();
        let found = id.and_then(|id| self.find(id));
        found
            .or_else(|| self.index.instances.first().cloned())
            .ok_or_else(|| BedrockError::InstanceNotFound("<none>".to_string()))
    }

    pub fn create(&mut self, name: &str, version_tag: &str) -> Result<BedrockInstance> {
        let now = now_iso();
        let base_id = slug(name);
        let mut id = base_id.clone();
        let mut n = 2;
        while self.find(&id).is_some() {
            id = format!("{base_id}-{n}");
            n += 1;
        }

        let name = if name.trim().is_empty() {
            "New Instance".to_string()
        } else {
            name.trim().to_string()
        };

        let directory = self.instances_dir.join(&id);
        std::fs::create_dir_all(&directory).ctx(&directory)?;

        let instance = BedrockInstance {
            id,
            name,
            version_tag: version_tag.to_string(),
            directory: directory.to_string_lossy().into_owned(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.index.instances.push(instance.clone());
        self.save_index()?;
        Ok(instance)
    }

    pub fn set_version(&mut self, id: &str, version_tag: &str) -> Result<()> {
        let now = now_iso();
        let instance = self
            .find_mut(id)
            .ok_or_else(|| BedrockError::InstanceNotFound(id.to_string()))?;
        instance.version_tag = version_tag.to_string();
        instance.updated_at = now;
        self.save_index()
    }

    pub fn rename(&mut self, id: &str, new_name: &str) -> Result<()> {
        if new_name.trim().is_empty() {
            return Ok(());
        }
        let now = now_iso();
        let instance = self
            .find_mut(id)
            .ok_or_else(|| BedrockError::InstanceNotFound(id.to_string()))?;
        instance.name = new_name.trim().to_string();
        instance.updated_at = now;
        self.save_index()
    }

    /// Deletes an instance and its on-disk directory. Refuses to delete the
    /// last remaining instance or the currently-active one, mirroring the
    /// C# guard rails (returns `Ok(false)` rather than erroring, matching
    /// the original's `bool` return).
    pub fn delete(&mut self, id: &str) -> Result<bool> {
        if self.index.instances.len() <= 1 {
            return Ok(false);
        }
        if self
            .index
            .active_id
            .as_deref()
            .map(|a| a.eq_ignore_ascii_case(id))
            == Some(true)
        {
            return Ok(false);
        }

        let Some(pos) = self
            .index
            .instances
            .iter()
            .position(|i| i.id.eq_ignore_ascii_case(id))
        else {
            return Ok(false);
        };

        let instance = self.index.instances.remove(pos);
        let dir = Path::new(&instance.directory);
        if dir.is_dir() {
            let _ = std::fs::remove_dir_all(dir);
        }
        self.save_index()?;
        Ok(true)
    }

    /// Swaps the live com.mojang managed folders with the target instance's
    /// snapshot: the current live state is archived into the currently
    /// active instance's directory, then the target instance's directory is
    /// copied into the live root.
    ///
    /// TODO: wire the post-switch game-version change (VanillaVersionService
    /// equivalent) once that service exists in another crate — the C#
    /// version optionally switches the installed Minecraft build to match
    /// `instance.version_tag` after the data swap.
    pub fn switch_to(&mut self, id: &str, live_root: &Path) -> Result<bool> {
        let target = self
            .find(id)
            .ok_or_else(|| BedrockError::InstanceNotFound(id.to_string()))?;

        if self
            .index
            .active_id
            .as_deref()
            .map(|a| a.eq_ignore_ascii_case(id))
            == Some(true)
        {
            return Ok(false);
        }

        let current = self.index.active_id.clone().and_then(|a| self.find(&a));

        if let Some(current) = &current {
            sync_folder_set(live_root, Path::new(&current.directory))?;
        }
        sync_folder_set(Path::new(&target.directory), live_root)?;

        self.index.active_id = Some(target.id.clone());
        if let Some(stored) = self.find_mut(&target.id) {
            stored.updated_at = now_iso();
        }
        self.save_index()?;
        Ok(true)
    }

    /// Archives the live com.mojang state into the active instance's
    /// directory without switching instances (mirrors `SaveActiveAsync`).
    pub fn save_active(&self, live_root: &Path) -> Result<()> {
        let active = self.active_instance()?;
        sync_folder_set(live_root, Path::new(&active.directory))
    }

    pub fn backup_instance(
        &self,
        instance: &BedrockInstance,
        backups: &BedrockBackupService,
    ) -> Result<PathBuf> {
        backups.create_backup_from_folder(Path::new(&instance.directory), Some(&instance.name))
    }

    pub fn restore_into_instance(
        &self,
        backup_path: &Path,
        instance: &BedrockInstance,
        backups: &BedrockBackupService,
    ) -> Result<()> {
        backups.restore_backup_into_folder(backup_path, Path::new(&instance.directory))
    }

    fn find(&self, id: &str) -> Option<BedrockInstance> {
        self.index
            .instances
            .iter()
            .find(|i| i.id.eq_ignore_ascii_case(id))
            .cloned()
    }

    fn find_mut(&mut self, id: &str) -> Option<&mut BedrockInstance> {
        self.index
            .instances
            .iter_mut()
            .find(|i| i.id.eq_ignore_ascii_case(id))
    }

    fn ensure_default_instance(&mut self, live_root: Option<&Path>) -> Result<()> {
        if self.index.instances.is_empty() {
            let instance = self.create("Default", "")?;
            if let Some(live_root) = live_root {
                let _ = sync_folder_set(live_root, Path::new(&instance.directory));
            }
            self.index.active_id = Some(instance.id);
            self.save_index()?;
        } else if self.index.active_id.is_none() {
            self.index.active_id = self.index.instances.first().map(|i| i.id.clone());
            self.save_index()?;
        }
        Ok(())
    }

    fn load_index(index_path: &Path) -> Result<IndexFile> {
        if !index_path.exists() {
            return Ok(IndexFile::default());
        }
        let json = match std::fs::read_to_string(index_path) {
            Ok(json) => json,
            Err(_) => return Ok(IndexFile::default()),
        };
        match serde_json::from_str(&json) {
            Ok(index) => Ok(index),
            Err(_) => {
                // Quarantine the corrupt file so it doesn't keep failing to
                // parse on every subsequent launch, mirroring
                // JsonStore.QuarantineCorrupt.
                let quarantined = index_path.with_extension("json.corrupt");
                let _ = std::fs::rename(index_path, quarantined);
                Ok(IndexFile::default())
            }
        }
    }

    fn save_index(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.index)?;
        write_atomic(&self.index_path, json.as_bytes())
    }
}

/// Replaces each managed folder under `dest_root` with the corresponding
/// folder from `source_root` (deleting the destination copy first), mirroring
/// `BedrockInstanceService.SyncFolderSet`.
fn sync_folder_set(source_root: &Path, dest_root: &Path) -> Result<()> {
    std::fs::create_dir_all(dest_root).ctx(dest_root)?;
    for folder in MANAGED_FOLDERS {
        let src = source_root.join(folder);
        let dst = dest_root.join(folder);

        if dst.is_dir() {
            let _ = std::fs::remove_dir_all(&dst);
        }
        if !src.is_dir() {
            continue;
        }
        copy_directory(&src, &dst)?;
    }
    Ok(())
}
