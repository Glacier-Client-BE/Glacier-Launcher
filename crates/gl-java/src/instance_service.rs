//! Port of the CRUD/layout portions of `Services/JavaInstanceService.cs`.
//!
//! The rest of `JavaInstanceService.cs` is ported elsewhere in this crate:
//! world/backup/datapack listing lives in [`crate::worlds`], mods-folder
//! listing/toggling/deleting lives in [`crate::mod_analyzer`], and modpack
//! overrides extraction / export / import / official-`.minecraft` import
//! live in [`crate::modpack_transfer`] (all previously missing; ported to
//! close that gap). `SetActive`/`SetActiveVersion`'s settings-file
//! bookkeeping is intentionally not duplicated here — it belongs to the
//! Tauri-side app state that owns `SettingsService`'s Rust equivalent, which
//! calls `set_instance_version`/`create`/`get` from this service and persists
//! the active-instance-id itself.

use crate::error::{io_err, JavaError, Result};
use crate::models::{InstanceSettings, JavaInstance, ModLoaderKind};
use crate::util::{slug, write_atomic};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct InstanceService {
    root_dir: PathBuf,
    index_path: PathBuf,
    instances: Mutex<Vec<JavaInstance>>,
}

impl InstanceService {
    /// Load (or initialize) the instance index rooted at `root_dir`
    /// (mirrors `RootDir` = `%USERPROFILE%\Glacier Launcher\java`).
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        let index_path = root_dir.join("instances.json");
        std::fs::create_dir_all(&root_dir).map_err(io_err(&root_dir))?;
        std::fs::create_dir_all(Self::instances_dir_of(&root_dir)).map_err(io_err(&root_dir))?;
        std::fs::create_dir_all(Self::cache_dir_of(&root_dir)).map_err(io_err(&root_dir))?;
        std::fs::create_dir_all(Self::shared_assets_dir_of(&root_dir))
            .map_err(io_err(&root_dir))?;
        std::fs::create_dir_all(Self::shared_libraries_dir_of(&root_dir))
            .map_err(io_err(&root_dir))?;

        let instances = load_index(&index_path)?;

        let svc = Self {
            root_dir,
            index_path,
            instances: Mutex::new(instances),
        };
        svc.ensure_default_instance();
        Ok(svc)
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }
    pub fn instances_dir(&self) -> PathBuf {
        Self::instances_dir_of(&self.root_dir)
    }
    pub fn shared_assets_dir(&self) -> PathBuf {
        Self::shared_assets_dir_of(&self.root_dir)
    }
    pub fn shared_libraries_dir(&self) -> PathBuf {
        Self::shared_libraries_dir_of(&self.root_dir)
    }

    fn instances_dir_of(root: &Path) -> PathBuf {
        root.join("instances")
    }
    fn cache_dir_of(root: &Path) -> PathBuf {
        root.join("cache")
    }
    fn shared_assets_dir_of(root: &Path) -> PathBuf {
        Self::cache_dir_of(root).join("assets")
    }
    fn shared_libraries_dir_of(root: &Path) -> PathBuf {
        Self::cache_dir_of(root).join("libraries")
    }

    /// All instances, snapshotted.
    pub fn instances(&self) -> Vec<JavaInstance> {
        self.instances
            .lock()
            .expect("instance list poisoned")
            .clone()
    }

    /// Look up an instance by id (case-insensitive), ensuring its directory
    /// layout exists on disk.
    pub fn get(&self, id: &str) -> Option<JavaInstance> {
        let list = self.instances.lock().expect("instance list poisoned");
        let found = list.iter().find(|x| x.id.eq_ignore_ascii_case(id)).cloned();
        if let Some(inst) = &found {
            let _ = ensure_layout(inst);
        }
        found
    }

    /// The active instance, defaulting to the first instance if
    /// `active_id` doesn't resolve. Mirrors `ActiveInstance`.
    pub fn active_instance(&self, active_id: Option<&str>) -> JavaInstance {
        self.ensure_default_instance();
        let list = self.instances.lock().expect("instance list poisoned");
        let inst = active_id
            .and_then(|id| list.iter().find(|x| x.id.eq_ignore_ascii_case(id)))
            .unwrap_or_else(|| list.first().expect("default instance ensured"))
            .clone();
        let _ = ensure_layout(&inst);
        inst
    }

    pub fn create(&self, name: &str, version_id: &str) -> Result<JavaInstance> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let now = now_iso();
        let base_id = slug(name);
        let mut id = base_id.clone();
        let mut n = 2;
        while list.iter().any(|x| x.id.eq_ignore_ascii_case(&id)) {
            id = format!("{base_id}-{n}");
            n += 1;
        }
        let instance = JavaInstance {
            id: id.clone(),
            name: if name.trim().is_empty() {
                "Default".to_string()
            } else {
                name.trim().to_string()
            },
            version_id: version_id.to_string(),
            directory: self
                .instances_dir()
                .join(&id)
                .to_string_lossy()
                .into_owned(),
            created_at: now.clone(),
            updated_at: now,
            max_ram_mb: 0,
            min_ram_mb: 0,
            ..Default::default()
        };
        ensure_layout(&instance)?;
        list.push(instance.clone());
        save_index(&self.index_path, &list)?;
        Ok(instance)
    }

    /// Like [`Self::create`], but also applies a built-in JVM-args template
    /// (see [`crate::presets`]) by name. An unrecognized/`None` template
    /// leaves `extra_jvm_args` empty, same as a plain `create`.
    pub fn create_with_template(
        &self,
        name: &str,
        version_id: &str,
        template: Option<&str>,
    ) -> Result<JavaInstance> {
        let instance = self.create(name, version_id)?;
        let Some(template) = template else {
            return Ok(instance);
        };
        self.set_instance_jvm_args(
            &instance.id,
            crate::presets::args_for(template),
            Some(template.to_string()),
        )?;
        self.get(&instance.id)
            .ok_or_else(|| JavaError::InstanceNotFound(instance.id.clone()))
    }

    /// Sets the instance's extra JVM args (and, optionally, which built-in
    /// preset they came from — `None` means "custom"). These are read at
    /// launch via `LaunchOptions::extra_jvm_args`.
    pub fn set_instance_jvm_args(
        &self,
        id: &str,
        extra_jvm_args: Vec<String>,
        preset: Option<String>,
    ) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.extra_jvm_args = extra_jvm_args;
        inst.jvm_args_preset = preset;
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Sets (or clears, with `None`) the instance's group/folder label,
    /// following the exact pattern of `rename`/`set_instance_version`.
    pub fn set_instance_group(&self, id: &str, group: Option<String>) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.group = group;
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Sets (or clears) whether an instance is pinned, following the exact
    /// pattern of `set_instance_group`.
    pub fn set_instance_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.pinned = pinned;
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Duplicate an existing instance's metadata and copy its directory tree.
    pub fn duplicate(&self, id: &str) -> Result<JavaInstance> {
        let source = self
            .get(id)
            .ok_or_else(|| JavaError::InstanceNotFound(id.to_string()))?;
        let copy = self.create(&format!("{} Copy", source.name), &source.version_id)?;
        copy_dir_recursive(Path::new(&source.directory), Path::new(&copy.directory))?;

        let mut list = self.instances.lock().expect("instance list poisoned");
        if let Some(stored) = list.iter_mut().find(|x| x.id == copy.id) {
            stored.max_ram_mb = source.max_ram_mb;
            stored.min_ram_mb = source.min_ram_mb;
        }
        save_index(&self.index_path, &list)?;
        Ok(list
            .iter()
            .find(|x| x.id == copy.id)
            .cloned()
            .expect("just inserted"))
    }

    pub fn rename(&self, id: &str, new_name: &str) -> Result<()> {
        if new_name.trim().is_empty() {
            return Ok(());
        }
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.name = new_name.trim().to_string();
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Sets (or clears, when `color` is `None`/empty) the per-instance accent
    /// hex color shown as a dot/border on the instance card.
    pub fn set_accent_color(&self, id: &str, color: Option<String>) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.accent_color = color.filter(|c| !c.trim().is_empty());
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Replaces an instance's sync-pack membership list wholesale (mirrors
    /// `set_instance_group`'s shape). Called by `gl-sync`'s Tauri layer after
    /// a successful apply/detach, so `JavaInstance.sync_pack_ids` always
    /// reflects reality without `gl-java` depending on `gl-sync`.
    pub fn set_instance_sync_packs(&self, id: &str, sync_pack_ids: Vec<String>) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.sync_pack_ids = sync_pack_ids;
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Delete an instance and its directory. Returns `Ok(false)` (mirroring
    /// the C# `bool` return) if the instance doesn't exist or is the last
    /// remaining instance; the latter case is also exposed as
    /// `JavaError::CannotDeleteLastInstance` via `delete_strict`.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(pos) = list.iter().position(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(false);
        };
        if list.len() <= 1 {
            return Ok(false);
        }
        let mut next = list.clone();
        let removed = next.remove(pos);
        save_index(&self.index_path, &next)?;
        *list = next;
        let _ = std::fs::remove_dir_all(&removed.directory);
        Ok(true)
    }

    pub fn set_instance_version(&self, id: &str, version_id: &str) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.version_id = version_id.to_string();
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Replaces an instance's per-instance settings overrides wholesale.
    /// Callers that want to patch a single field should read
    /// `get(id).settings`, mutate the clone, and call this with the result.
    pub fn set_instance_settings(&self, id: &str, settings: InstanceSettings) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.settings = settings;
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Sets (or clears, with `None` loader) the instance's mod loader and
    /// loader version, plus the `versions/<id>` profile directory name the
    /// loader installer produced.
    pub fn set_instance_loader(
        &self,
        id: &str,
        loader: Option<ModLoaderKind>,
        loader_version: Option<String>,
        loader_profile_id: Option<String>,
    ) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.loader = loader;
        inst.loader_version = loader_version;
        inst.loader_profile_id = loader_profile_id;
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    /// Sets (or clears) the instance's preferred account id, an opaque
    /// reference into `gl_settings::LauncherSettings.java_accounts` — this
    /// crate has no dependency on gl-settings, so it stores the id blind.
    pub fn set_instance_account(&self, id: &str, account_id: Option<String>) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.preferred_account_id = account_id;
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    pub fn set_ram(&self, id: &str, min_mb: i32, max_mb: i32) -> Result<()> {
        let mut list = self.instances.lock().expect("instance list poisoned");
        let Some(inst) = list.iter_mut().find(|x| x.id.eq_ignore_ascii_case(id)) else {
            return Ok(());
        };
        inst.max_ram_mb = max_mb.max(0);
        inst.min_ram_mb = min_mb.max(0);
        inst.updated_at = now_iso();
        save_index(&self.index_path, &list)
    }

    pub fn path_for_instance(&self, id: &str, child: &str) -> PathBuf {
        let inst = self.get(id).unwrap_or_else(|| {
            self.instances
                .lock()
                .expect("instance list poisoned")
                .first()
                .cloned()
                .expect("default instance ensured")
        });
        Path::new(&inst.directory).join(child)
    }

    /// Build the per-version instance lock file path, mirroring
    /// `CreateInstanceLockPath`.
    pub fn create_instance_lock_path(
        &self,
        active_minecraft_dir: &Path,
        version_id: &str,
    ) -> PathBuf {
        let safe = if version_id.trim().is_empty() {
            "launch".to_string()
        } else {
            slug(version_id)
        };
        active_minecraft_dir.join(format!("{safe}.lock"))
    }

    fn ensure_default_instance(&self) {
        let is_empty = self
            .instances
            .lock()
            .expect("instance list poisoned")
            .is_empty();
        if is_empty {
            let _ = self.create("Default", "");
        }
    }
}

fn ensure_layout(instance: &JavaInstance) -> Result<()> {
    let dir = Path::new(&instance.directory);
    for sub in [
        "versions",
        "saves",
        "mods",
        "config",
        "resourcepacks",
        "shaderpacks",
        "screenshots",
        "crash-reports",
    ] {
        std::fs::create_dir_all(dir.join(sub)).map_err(io_err(dir.join(sub)))?;
    }
    Ok(())
}

/// A missing index is a fresh install. An unreadable index is an error rather
/// than an empty list, since the next save would overwrite every instance's
/// metadata. A corrupt index is quarantined so it isn't silently clobbered.
fn load_index(index_path: &Path) -> Result<Vec<JavaInstance>> {
    let text = match std::fs::read_to_string(index_path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err(index_path)(e)),
    };
    match serde_json::from_str(&text) {
        Ok(list) => Ok(list),
        Err(_) => {
            let _ = std::fs::rename(index_path, index_path.with_extension("json.corrupt"));
            Ok(Vec::new())
        }
    }
}

fn save_index(index_path: &Path, list: &[JavaInstance]) -> Result<()> {
    let json = serde_json::to_string_pretty(list)?;
    write_atomic(index_path, &json)
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target).map_err(io_err(target))?;
    if !source.exists() {
        return Ok(());
    }
    for entry in walk_dir(source) {
        let rel = entry.strip_prefix(source).expect("walked under source");
        let dest = target.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(io_err(&dest))?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(io_err(parent))?;
            }
            std::fs::copy(&entry, &dest).map_err(io_err(&dest))?;
        }
    }
    Ok(())
}

fn walk_dir(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            out.push(path);
        }
    }
    out
}

fn now_iso() -> String {
    // ISO-8601 UTC timestamp with second precision — sufficient for the
    // created/updated bookkeeping fields (which are opaque strings to callers).
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix_secs(secs)
}

fn format_unix_secs(secs: u64) -> String {
    // Minimal civil-from-days conversion (Howard Hinnant's algorithm) to avoid
    // a chrono dependency for a purely informational timestamp field.
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m_ = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m_ <= 2 { y + 1 } else { y };

    format!("{y:04}-{m_:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}
