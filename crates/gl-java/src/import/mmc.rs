//! MultiMC / Prism Launcher adapter.
//!
//! Layout (documented MultiMC/Prism format, shared by both forks):
//! `<root>/instances/<Instance Name>/instance.cfg` (INI-ish key=value,
//! notably `name=` and `iconKey=`) and
//! `<root>/instances/<Instance Name>/mmc-pack.json` (`components[]`, each
//! with a `uid` and `version` — `net.minecraft` gives the MC version,
//! `net.fabricmc.fabric-loader`/`org.quiltmc.quilt-loader`/
//! `net.minecraftforge`/`net.neoforged` give the mod loader). The actual
//! `.minecraft`-shaped game directory is `<instance>/.minecraft` (or,
//! rarely, `<instance>/minecraft` for very old MultiMC exports).
//!
//! **Unverified against a real install** — implemented from the publicly
//! documented MultiMC/Prism instance format; see `PARITY.md`.

use super::loader_map::from_mmc_component_uid;
use super::{common, ImportCandidate, SourceLauncher};
use crate::error::Result;
use std::path::Path;

#[derive(serde::Deserialize)]
struct MmcPack {
    #[serde(default)]
    components: Vec<MmcComponent>,
}

#[derive(serde::Deserialize)]
struct MmcComponent {
    uid: String,
    #[serde(default)]
    version: Option<String>,
}

/// Enumerate every instance under a MultiMC/Prism `instances/` root.
/// Skips any subdirectory that doesn't look like an instance rather than
/// failing the whole scan.
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
    let cfg_path = dir.join("instance.cfg");
    let pack_path = dir.join("mmc-pack.json");
    if !cfg_path.is_file() {
        return None;
    }

    let name = parse_instance_cfg_name(&cfg_path).unwrap_or_else(|| {
        dir.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });

    let mc_dir = if dir.join(".minecraft").is_dir() {
        dir.join(".minecraft")
    } else if dir.join("minecraft").is_dir() {
        dir.join("minecraft")
    } else {
        // No game directory yet (never launched) — still worth listing so
        // the user sees it, just with zero mods/worlds/size.
        dir.join(".minecraft")
    };

    let mut mc_version = String::new();
    let mut loader = None;
    let mut loader_version = None;
    if let Ok(text) = std::fs::read_to_string(&pack_path) {
        if let Ok(pack) = serde_json::from_str::<MmcPack>(&text) {
            for c in &pack.components {
                if c.uid == "net.minecraft" {
                    mc_version = c.version.clone().unwrap_or_default();
                } else if let Some(kind) = from_mmc_component_uid(&c.uid) {
                    loader = Some(kind);
                    loader_version = c.version.clone();
                }
            }
        }
    }

    Some(ImportCandidate {
        source_launcher: SourceLauncher::MultiMcOrPrism,
        name,
        mc_version,
        loader,
        loader_version,
        mod_count: common::count_mods(&mc_dir),
        world_count: common::count_worlds(&mc_dir),
        size_bytes: common::dir_size_bytes(dir),
        icon_path: None, // MultiMC icons are theme-key references, not files; not resolved here.
        instance_path: mc_dir,
    })
}

/// `instance.cfg` is a flat `key=value` INI file with no section headers.
/// Extracts just `name=`.
fn parse_instance_cfg_name(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("name=") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Re-exported for `pipeline.rs`'s `Result`-returning call sites, even
/// though `scan` itself never fails outright.
#[allow(dead_code)]
pub fn scan_checked(instances_root: &Path) -> Result<Vec<ImportCandidate>> {
    Ok(scan(instances_root))
}
