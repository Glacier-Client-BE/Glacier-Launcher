//! CurseForge app (Overwolf) adapter.
//!
//! Layout: each instance is a folder directly under
//! `%APPDATA%\CurseForge\Instances\<Instance Name>\`, itself containing a
//! `minecraftinstance.json` manifest and the game files directly (the
//! instance folder *is* the `.minecraft` directory — no nested subfolder,
//! unlike MultiMC). Relevant `minecraftinstance.json` fields: `name`,
//! `gameVersion` (Minecraft version), `baseModLoader.name` (e.g.
//! `"forge-47.2.0"`).
//!
//! **Unverified against a real install** — the CurseForge app is
//! closed-source; this is implemented from widely-reported community
//! documentation of its manifest shape, not an official spec. See
//! `PARITY.md`.

use super::loader_map::from_curseforge_loader_name;
use super::{common, ImportCandidate, SourceLauncher};
use std::path::Path;

#[derive(serde::Deserialize)]
struct CfManifest {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "gameVersion")]
    game_version: String,
    #[serde(default, rename = "baseModLoader")]
    base_mod_loader: Option<CfLoader>,
}

#[derive(serde::Deserialize)]
struct CfLoader {
    #[serde(default)]
    name: String,
}

/// Enumerate every instance under the CurseForge app's `Instances/` root.
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
    let manifest_path = dir.join("minecraftinstance.json");
    let text = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: CfManifest = serde_json::from_str(&text).ok()?;

    let name = if manifest.name.trim().is_empty() {
        dir.file_name().unwrap_or_default().to_string_lossy().into_owned()
    } else {
        manifest.name
    };

    let (loader, loader_version) = match &manifest.base_mod_loader {
        Some(l) if !l.name.is_empty() => (
            from_curseforge_loader_name(&l.name),
            l.name.split_once('-').map(|(_, v)| v.to_string()),
        ),
        _ => (None, None),
    };

    Some(ImportCandidate {
        source_launcher: SourceLauncher::CurseForgeApp,
        name,
        mc_version: manifest.game_version,
        loader,
        loader_version,
        mod_count: common::count_mods(dir),
        world_count: common::count_worlds(dir),
        size_bytes: common::dir_size_bytes(dir),
        icon_path: None,
        instance_path: dir.to_path_buf(),
    })
}
