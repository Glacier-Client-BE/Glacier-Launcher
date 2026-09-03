use serde::{Deserialize, Serialize};

/// Info about an available launcher update, mirrors C# `LauncherUpdateInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherUpdateInfo {
    pub tag: String,
    pub download_url: String,
    pub changelog: String,
    pub asset_size: u64,
}

/// A single release entry, mirrors C# `LauncherRelease`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherRelease {
    pub tag: String,
    pub published_at: String,
    pub changelog: String,
}

/// Minimal shape of a GitHub release API object — only the fields we read.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    #[serde(default)]
    pub draft: bool,
    pub tag_name: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}
