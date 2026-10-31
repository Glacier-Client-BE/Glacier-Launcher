//! Modrinth App adapter.
//!
//! Layout: `%APPDATA%\ModrinthApp\profiles\<Profile Name>\profile.json`
//! (the Modrinth App, built on the `theseus` core, calls instances
//! "profiles"), with the game directory being that same folder. Relevant
//! `profile.json` fields: `metadata.name`, `metadata.game_version`,
//! `metadata.loader` (`"vanilla"`/`"fabric"`/`"quilt"`/`"forge"`/
//! `"neoforge"`), `metadata.loader_version.id`. Modrinth App has changed its
//! metadata file shape across releases (it also writes a `profile.json` at
//! the top level in some versions vs nested under `metadata` in others) —
//! both shapes are tried here.
//!
//! **Unverified against a real install** — implemented from Modrinth App's
//! public `theseus` source, not exercised against a live profile in this
//! sandbox. See `PARITY.md`.

use super::loader_map::from_modrinth_loader;
use super::{common, ImportCandidate, SourceLauncher};
use std::path::Path;

#[derive(serde::Deserialize, Default)]
struct MrProfile {
    #[serde(default)]
    metadata: Option<MrMetadata>,
    // Older/flat shape: fields directly on the root object.
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    loader: Option<String>,
    #[serde(default)]
    loader_version: Option<String>,
}

#[derive(serde::Deserialize)]
struct MrMetadata {
    #[serde(default)]
    name: String,
    #[serde(default)]
    game_version: String,
    #[serde(default)]
    loader: String,
    #[serde(default)]
    loader_version: Option<MrLoaderVersion>,
}

#[derive(serde::Deserialize)]
struct MrLoaderVersion {
    #[serde(default)]
    id: String,
}

/// Enumerate every profile under the Modrinth App's `profiles/` root.
pub fn scan(profiles_root: &Path) -> Vec<ImportCandidate> {
    let Ok(entries) = std::fs::read_dir(profiles_root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| read_profile(&e.path()))
        .collect()
}

fn read_profile(dir: &Path) -> Option<ImportCandidate> {
    let manifest_path = dir.join("profile.json");
    let text = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: MrProfile = serde_json::from_str(&text).ok()?;

    let (name, mc_version, loader_str, loader_version) = if let Some(meta) = manifest.metadata {
        (
            meta.name,
            meta.game_version,
            meta.loader,
            meta.loader_version.map(|v| v.id),
        )
    } else {
        (
            manifest.name.unwrap_or_default(),
            manifest.game_version.unwrap_or_default(),
            manifest.loader.unwrap_or_default(),
            manifest.loader_version,
        )
    };

    let name = if name.trim().is_empty() {
        dir.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    } else {
        name
    };

    Some(ImportCandidate {
        source_launcher: SourceLauncher::ModrinthApp,
        name,
        mc_version,
        loader: from_modrinth_loader(&loader_str),
        loader_version,
        mod_count: common::count_mods(dir),
        world_count: common::count_worlds(dir),
        size_bytes: common::dir_size_bytes(dir),
        icon_path: None,
        instance_path: dir.to_path_buf(),
    })
}
