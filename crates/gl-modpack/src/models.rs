//! Shared data models: CurseForge/Modrinth search results, modpack install
//! plans, and jar metadata used by mod analysis.

use serde::{Deserialize, Serialize};

/// A CurseForge addon (mod/modpack/resource pack/etc.) search result entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CfAddon {
    pub id: i32,
    pub name: String,
    pub summary: String,
    pub thumbnail_url: String,
    pub author_name: String,
    pub download_count: i64,
    pub class_id: i32,
    pub latest_file_id: i32,
    pub category_name: String,
}

/// A downloadable CurseForge file belonging to an addon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CfFile {
    pub file_id: i32,
    pub file_name: String,
    pub download_url: String,
    pub file_length: i64,
    pub display_name: String,
}

/// Result of a CurseForge search request.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CfSearchResult {
    pub addons: Vec<CfAddon>,
    pub total_count: i32,
}

/// A Modrinth project (mod/modpack/resource pack/shader) search result entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MrProject {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: String,
    pub author: String,
    pub downloads: i64,
    pub project_type: String,
    pub category_name: Option<String>,
}

/// A downloadable Modrinth project version file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MrVersion {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub url: String,
    pub size: i64,
}

/// Result of a Modrinth search request.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MrSearchResult {
    pub projects: Vec<MrProject>,
    pub total_count: i32,
}

/// A mod loader kind, kept distinct because each has its own manifest shape
/// and install flow (Fabric/Quilt use a JSON version-profile API; Forge and
/// NeoForge ship a self-extracting installer jar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl ModLoader {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModLoader::Fabric => "fabric",
            ModLoader::Quilt => "quilt",
            ModLoader::Forge => "forge",
            ModLoader::NeoForge => "neoforge",
        }
    }

    /// Parses a loader name from CurseForge's `minecraft.modLoaders[].id`
    /// prefix or Modrinth's `dependencies` key, e.g. "fabric-loader" -> Fabric.
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        match name.as_str() {
            "fabric" | "fabric-loader" => Some(ModLoader::Fabric),
            "quilt" | "quilt-loader" => Some(ModLoader::Quilt),
            "forge" => Some(ModLoader::Forge),
            "neoforge" => Some(ModLoader::NeoForge),
            _ => None,
        }
    }
}

/// One file that needs to be fetched (or resolved, for CurseForge project
/// references) to complete a modpack install.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModpackFile {
    pub url: String,
    pub relative_path: String,
    pub sha1: String,
    pub sha512: String,
    /// -1 when unknown.
    pub size: i64,
    /// Set only for CurseForge manifest entries, which reference files by
    /// project+file id instead of carrying a direct URL.
    pub curseforge_ref: Option<CurseForgeFileRef>,
}

/// A CurseForge project+file id pair from `manifest.json`'s `files` array;
/// must be resolved via the CurseForge API to obtain a download URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurseForgeFileRef {
    pub project_id: i32,
    pub file_id: i32,
}

/// A parsed, source-agnostic modpack install plan (from either a CurseForge
/// `manifest.json` or a Modrinth `modrinth.index.json`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModpackPlan {
    pub name: String,
    pub minecraft_version: String,
    pub loader: Option<ModLoader>,
    pub loader_version: String,
    pub files: Vec<ModpackFile>,
}

/// Metadata read from a single installed mod jar.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModInfo {
    pub file_name: String,
    pub mod_id: String,
    pub name: String,
    pub provides: Vec<String>,
    pub required_deps: Vec<String>,
    pub loader: String,
    pub disabled: bool,
}

/// The result of analyzing a set of mod jars: parsed metadata plus warnings
/// about duplicate mod ids or missing required dependencies.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModReport {
    pub mods: Vec<ModInfo>,
    pub warnings: Vec<String>,
}
