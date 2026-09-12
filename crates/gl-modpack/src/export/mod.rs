//! Export an installed instance's mods back out to a shareable modpack
//! archive (`.mrpack` or CurseForge zip), the inverse of `manifest.rs`.

pub mod curseforge;
pub mod mrpack;

use serde::{Deserialize, Serialize};

/// One mod's provenance as read from `<instance>/mods/mods.json`.
///
/// This mirrors the shape documented for Phase 3's
/// `crates/gl-java/src/mod_provenance.rs` (`{filename, source, project_id,
/// version_id, sha1}` per mod). That file did not exist yet when Phase 6 was
/// written, so this module reads the raw JSON directly with the same shape
/// rather than depending on Phase 3's crate — see the note in PARITY.md
/// about reconciling the two once Phase 3 lands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModProvenanceEntry {
    pub filename: String,
    /// "modrinth" | "curseforge" | "unknown" (local/manual jar).
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    /// CurseForge uses numeric ids; kept as separate optional fields so this
    /// struct can represent either source without guessing types.
    #[serde(default)]
    pub cf_project_id: Option<i64>,
    #[serde(default)]
    pub cf_file_id: Option<i64>,
    #[serde(default)]
    pub sha1: Option<String>,
    /// Direct download URL, when known (e.g. already resolved by an earlier
    /// install), used as a fallback if project/version ids are absent.
    #[serde(default)]
    pub download_url: Option<String>,
}

/// Reads `<instance>/mods/mods.json`, if present. Returns an empty list
/// (not an error) when the file is missing, since Phase 3's provenance
/// tracking may not have run for every mod (e.g. manually dropped-in jars).
pub fn read_mod_provenance(instance_dir: &std::path::Path) -> Vec<ModProvenanceEntry> {
    let path = instance_dir.join("mods").join("mods.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    // Accept either a bare array or `{ "mods": [...] }` for extra defensiveness
    // against Phase 3's actual output shape.
    if let Ok(list) = serde_json::from_slice::<Vec<ModProvenanceEntry>>(&bytes) {
        return list;
    }
    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(default)]
        mods: Vec<ModProvenanceEntry>,
    }
    serde_json::from_slice::<Wrapper>(&bytes)
        .map(|w| w.mods)
        .unwrap_or_default()
}

/// Options shared by both export writers.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub pack_name: String,
    pub pack_version: String,
    pub minecraft_version: String,
    /// "fabric" | "quilt" | "forge" | "neoforge", or empty for vanilla.
    pub loader: String,
    pub loader_version: String,
}

/// Folders copied into `overrides/` verbatim when they exist and aren't
/// resolvable to a download (config, resourcepacks not from a known source,
/// shaderpacks, etc).
pub const OVERRIDE_DIRS: &[&str] = &["config", "resourcepacks", "shaderpacks"];
