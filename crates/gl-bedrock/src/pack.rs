//! Resource/behavior/skin pack enumeration. Mirrors
//! `Services/BedrockPackService.cs`.

use std::path::{Path, PathBuf};

use crate::error::{IoContext, Result};
use crate::model::{BedrockPack, PackKind};
use crate::util::{directory_size, iso_from_systemtime};

const ICON_NAMES: &[&str] = &["pack_icon.png", "pack_icon.jpg", "pack_icon.jpeg"];

pub struct BedrockPackService;

impl BedrockPackService {
    pub fn new() -> Self {
        Self
    }

    /// Lists the packs found directly under `dir`, tagging each with `kind`.
    /// Sorted by display name (case-insensitive), matching the C# ordering.
    pub fn list_packs(&self, dir: &Path, kind: PackKind) -> Result<Vec<BedrockPack>> {
        let mut result = Vec::new();
        if !dir.is_dir() {
            return Ok(result);
        }

        for entry in std::fs::read_dir(dir).ctx(dir)? {
            let entry = entry.ctx(dir)?;
            let pack_dir = entry.path();
            if !pack_dir.is_dir() {
                continue;
            }

            let name = read_pack_name(&pack_dir).unwrap_or_else(|| {
                pack_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            let modified = std::fs::metadata(&pack_dir)
                .and_then(|m| m.modified())
                .map(iso_from_systemtime)
                .unwrap_or_default();

            result.push(BedrockPack {
                name,
                folder_path: pack_dir.to_string_lossy().into_owned(),
                kind: kind.as_str().to_string(),
                size_bytes: directory_size(&pack_dir),
                modified_at: modified,
                icon_path: find_icon(&pack_dir),
            });
        }

        result.sort_by_key(|p| p.name.to_lowercase());
        Ok(result)
    }

    pub fn delete_pack(&self, path: &Path) -> Result<()> {
        if path.is_dir() {
            std::fs::remove_dir_all(path).ctx(path)?;
        }
        Ok(())
    }
}

impl Default for BedrockPackService {
    fn default() -> Self {
        Self::new()
    }
}

fn find_icon(pack_dir: &Path) -> Option<String> {
    ICON_NAMES
        .iter()
        .map(|name| pack_dir.join(name))
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
}

fn read_pack_name(pack_dir: &Path) -> Option<String> {
    let manifest_path = pack_dir.join("manifest.json");
    let text = std::fs::read_to_string(manifest_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let name = json.get("header")?.get("name")?.as_str()?;
    if name.is_empty() || name.to_lowercase().starts_with("pack.") {
        None
    } else {
        Some(name.to_string())
    }
}

/// Well-known pack directory kinds, matching `BedrockPackService.DirFor`.
/// Callers supply the actual root path for each (com.mojang location
/// resolution is out of this crate's scope — see the module docs on
/// [`crate::instance`]).
pub struct PackDirs {
    pub resource: PathBuf,
    pub behavior: PathBuf,
    pub skin: PathBuf,
    pub resource_dev: PathBuf,
    pub behavior_dev: PathBuf,
    pub skin_dev: PathBuf,
}

impl PackDirs {
    pub fn dir_for(&self, kind: PackKind) -> &Path {
        match kind {
            PackKind::Resource => &self.resource,
            PackKind::Behavior => &self.behavior,
            PackKind::Skin => &self.skin,
            PackKind::ResourceDev => &self.resource_dev,
            PackKind::BehaviorDev => &self.behavior_dev,
            PackKind::SkinDev => &self.skin_dev,
        }
    }
}
