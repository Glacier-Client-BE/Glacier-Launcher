//! ATLauncher adapter.
//!
//! Layout: `%APPDATA%\ATLauncher\instances\<Instance Name>\instance.json`,
//! with the game directory being that same folder (ATLauncher, like the
//! CurseForge app, does not nest a `.minecraft` subfolder). Relevant
//! `instance.json` fields (per ATLauncher's own `Instance`/`InstanceV2`
//! model, which is public since ATLauncher is open source): `launcher.name`,
//! `launcher.loaderVersion.type` (`"Forge"`/`"NeoForge"`/`"Fabric"`/
//! `"Quilt"`), `launcher.loaderVersion.version`, and the base Minecraft
//! version under `id`/`minecraftVersion` (older/newer ATLauncher versions
//! disagree on the exact key name, so both are tried).
//!
//! **Unverified against a real install** — implemented from ATLauncher's
//! public source layout, not exercised against a live instance in this
//! sandbox. See `PARITY.md`.

use super::loader_map::from_atlauncher_loader_type;
use super::{common, ImportCandidate, SourceLauncher};
use std::path::Path;

#[derive(serde::Deserialize)]
struct AtInstance {
    #[serde(default)]
    launcher: AtLauncherBlock,
    #[serde(default, rename = "minecraftVersion")]
    minecraft_version: Option<String>,
    #[serde(default)]
    id: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct AtLauncherBlock {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "loaderVersion")]
    loader_version: Option<AtLoaderVersion>,
}

#[derive(serde::Deserialize)]
struct AtLoaderVersion {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    version: String,
}

/// Enumerate every instance under ATLauncher's `instances/` root.
pub fn scan(instances_root: &Path) -> Vec<ImportCandidate> {
    let Ok(entries) = std::fs::read_dir(instances_root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| read_instance(&e.path()))
        .collect()
}

fn read_instance(dir: &Path) -> Option<ImportCandidate> {
    let manifest_path = dir.join("instance.json");
    let text = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: AtInstance = serde_json::from_str(&text).ok()?;

    let name = if manifest.launcher.name.trim().is_empty() {
        dir.file_name().unwrap_or_default().to_string_lossy().into_owned()
    } else {
        manifest.launcher.name
    };

    let mc_version = manifest
        .minecraft_version
        .or(manifest.id)
        .unwrap_or_default();

    let (loader, loader_version) = match manifest.launcher.loader_version {
        Some(lv) => (from_atlauncher_loader_type(&lv.kind), Some(lv.version)),
        None => (None, None),
    };

    Some(ImportCandidate {
        source_launcher: SourceLauncher::ATLauncher,
        name,
        mc_version,
        loader,
        loader_version,
        mod_count: common::count_mods(dir),
        world_count: common::count_worlds(dir),
        size_bytes: common::dir_size_bytes(dir),
        icon_path: None,
        instance_path: dir.to_path_buf(),
    })
}
