//! Data models shared by the Bedrock services (mirrors the C# `Models/Bedrock*.cs`
//! and `Models/BedrockBackup.cs`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BedrockInstance {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version_tag: String,
    pub directory: String,
    pub created_at: String,
    pub updated_at: String,
    /// Optional per-instance accent hex color (e.g. "#7289da") shown as a
    /// small dot/border on the instance card. `None` falls back to the
    /// global theme accent.
    #[serde(default)]
    pub accent_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BedrockWorld {
    pub name: String,
    pub folder_path: String,
    pub size_bytes: u64,
    pub modified_at: String,
    pub icon_path: Option<String>,
}

/// Which pack folder a [`BedrockPack`] was enumerated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackKind {
    Resource,
    Behavior,
    Skin,
    ResourceDev,
    BehaviorDev,
    SkinDev,
}

impl PackKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PackKind::Resource => "resource",
            PackKind::Behavior => "behavior",
            PackKind::Skin => "skin",
            PackKind::ResourceDev => "resource-dev",
            PackKind::BehaviorDev => "behavior-dev",
            PackKind::SkinDev => "skin-dev",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BedrockPack {
    pub name: String,
    pub folder_path: String,
    pub kind: String,
    pub size_bytes: u64,
    pub modified_at: String,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BedrockBackup {
    pub name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenshotFile {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size_bytes: u64,
    pub modified_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionKind {
    Toggle,
    Integer,
    Decimal,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionEntry {
    pub key: String,
    pub value: String,
    pub kind: OptionKind,
    pub label: String,
    pub group: String,
    pub hint: String,
    pub known: bool,
}

impl OptionEntry {
    pub fn is_on(&self) -> bool {
        matches!(self.value.as_str(), "1" | "true" | "True")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedShader {
    pub file_name: String,
    pub source_name: String,
    pub applied_iso: String,
    pub size_bytes: u64,
    pub had_stock: bool,
}
