use serde::{Deserialize, Serialize};

/// A single Java Edition version as reported by the Mojang piston-meta manifest,
/// annotated with local install-state flags. Mirrors the C# `JavaVersion` model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct JavaVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub release_time: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub compliance_level: i32,

    // Not present in Mojang's manifest JSON (hence `default` for
    // deserializing it), but very much needed by frontend consumers of this
    // struct over Tauri IPC, so these are NOT skip_serializing.
    #[serde(default)]
    pub is_installed: bool,
    #[serde(default)]
    pub has_jar: bool,
    #[serde(default)]
    pub is_active: bool,
}

impl JavaVersion {
    /// Human label mirroring the C# `TypeLabel` computed property.
    pub fn type_label(&self) -> &str {
        match self.kind.as_str() {
            "release" => "Release",
            "snapshot" => "Snapshot",
            "old_beta" => "Beta",
            "old_alpha" => "Alpha",
            other => other,
        }
    }
}

/// A managed Java Edition install instance (a `.minecraft`-shaped directory tree).
/// Mirrors the C# `JavaInstance` model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct JavaInstance {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub directory: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub max_ram_mb: i32,
    #[serde(default)]
    pub min_ram_mb: i32,
}
