//! DTOs for Forge/NeoForge's `install_profile.json` (the "modern", 1.13+
//! processors-based installer format both loaders share, since NeoForge's
//! installer is a fork of Forge's). Shape reverse-engineered from real
//! installer jars: top-level `data` map of placeholder name -> {client,
//! server} maven-coordinate-or-literal, a `processors` array of jars to run
//! in order, and a `libraries` array to resolve onto disk before running
//! them.
//!
//! Older (pre-1.13) Forge installers used a different, simpler
//! `install_profile.json` (a single `versionInfo` + `install` block with no
//! `processors`) and are NOT supported here — see `patcher.rs` for the
//! detection guard.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// One side of a `data` map entry: either a literal value (wrapped in `[...]`
/// in the raw JSON, already stripped by [`DataEntry::literal`]) or a maven
/// coordinate to resolve against the libraries dir.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DataSides {
    pub client: String,
    pub server: String,
}

impl DataSides {
    /// Picks the client or server value.
    pub fn for_side(&self, side: &str) -> &str {
        if side.eq_ignore_ascii_case("server") {
            &self.server
        } else {
            &self.client
        }
    }
}

/// A single entry in `install_profile.json`'s `data` map, e.g.
/// `"MAPPINGS": {"client": "[net.minecraftforge:mcp_config...]", "server": "..."}`.
/// A value like `[literal text]` is a literal (used verbatim, no maven
/// resolution); anything else is a maven coordinate resolved onto disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataValue {
    Literal(String),
    Artifact(String),
}

impl DataValue {
    pub fn parse(raw: &str) -> Self {
        if let Some(inner) = raw.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            DataValue::Literal(inner.to_string())
        } else {
            DataValue::Artifact(raw.to_string())
        }
    }
}

/// One entry of the `libraries` array: a maven coordinate plus (usually) a
/// direct download URL/sha1/size, matching the vanilla launcher library
/// schema Forge/NeoForge reuse.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallLibrary {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryArtifact {
    pub path: Option<String>,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

/// One entry of the `processors` array: a jar to run (via its manifest
/// `Main-Class`) with a resolved classpath and argument list, restricted to
/// `sides` (empty/absent means "both").
#[derive(Debug, Clone, Deserialize)]
pub struct Processor {
    pub jar: String,
    #[serde(default)]
    pub classpath: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub sides: Vec<String>,
    /// Optional `{output_path: expected_sha1}` map some processors declare so
    /// the installer can skip re-running a processor whose output already
    /// matches (an optimization, not a correctness requirement for us).
    #[serde(default)]
    pub outputs: HashMap<String, String>,
}

impl Processor {
    pub fn applies_to_side(&self, side: &str) -> bool {
        self.sides.is_empty() || self.sides.iter().any(|s| s.eq_ignore_ascii_case(side))
    }
}

/// The parsed `install_profile.json` document.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallProfile {
    #[serde(default)]
    pub spec: u32,
    #[serde(default)]
    pub profile: String,
    /// The loader version id this profile installs, e.g. `1.20.1-47.2.0`.
    pub version: String,
    /// Base Minecraft version, e.g. `1.20.1`.
    pub minecraft: String,
    /// Path (inside the installer jar) of the vanilla-launcher-format
    /// `version.json` to write to `versions/<version>/<version>.json`.
    #[serde(default)]
    pub json: Option<String>,
    #[serde(default)]
    pub data: HashMap<String, DataSides>,
    #[serde(default)]
    pub processors: Vec<Processor>,
    #[serde(default)]
    pub libraries: Vec<InstallLibrary>,
}

impl InstallProfile {
    pub fn parse(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    /// True if this is the modern processors-based format we can install.
    /// Legacy (pre-1.13) `install_profile.json` has no `processors`/`data`
    /// and instead nests everything under an `install`/`versionInfo` pair.
    pub fn is_modern(raw: &Value) -> bool {
        raw.get("processors").is_some() && raw.get("data").is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "spec": 0,
        "profile": "forge",
        "version": "1.20.1-47.2.0",
        "minecraft": "1.20.1",
        "json": "/version.json",
        "data": {
            "MAPPINGS": {
                "client": "[net.minecraftforge:mcp_config:1.20.1@zip]",
                "server": "[net.minecraftforge:mcp_config:1.20.1@zip]"
            },
            "BINPATCH": {
                "client": "/data/client.lzma",
                "server": "/data/server.lzma"
            }
        },
        "processors": [
            {
                "jar": "net.minecraftforge:installertools:1.5.0",
                "classpath": ["net.minecraftforge:forgeflower:1.5.0"],
                "args": ["--task", "MCP_DATA", "--input", "{MAPPINGS}"],
                "sides": ["client"]
            }
        ],
        "libraries": [
            {
                "name": "net.minecraftforge:installertools:1.5.0",
                "downloads": {
                    "artifact": {
                        "path": "net/minecraftforge/installertools/1.5.0/installertools-1.5.0.jar",
                        "url": "https://maven.minecraftforge.net/net/minecraftforge/installertools/1.5.0/installertools-1.5.0.jar",
                        "sha1": "deadbeef",
                        "size": 1234
                    }
                }
            }
        ]
    }"#;

    #[test]
    fn parses_real_shape() {
        let profile = InstallProfile::parse(SAMPLE).unwrap();
        assert_eq!(profile.version, "1.20.1-47.2.0");
        assert_eq!(profile.processors.len(), 1);
        assert_eq!(profile.libraries.len(), 1);
        assert!(profile.processors[0].applies_to_side("client"));
        assert!(!profile.processors[0].applies_to_side("server"));
    }

    #[test]
    fn data_value_distinguishes_literal_from_artifact() {
        assert_eq!(
            DataValue::parse("[net.minecraftforge:mcp_config:1.20.1@zip]"),
            DataValue::Literal("net.minecraftforge:mcp_config:1.20.1@zip".to_string())
        );
        assert_eq!(
            DataValue::parse("/data/client.lzma"),
            DataValue::Artifact("/data/client.lzma".to_string())
        );
    }

    #[test]
    fn is_modern_detects_processors_and_data() {
        let v: Value = serde_json::from_str(SAMPLE).unwrap();
        assert!(InstallProfile::is_modern(&v));
        let legacy: Value = serde_json::from_str(r#"{"install": {}, "versionInfo": {}}"#).unwrap();
        assert!(!InstallProfile::is_modern(&legacy));
    }
}
