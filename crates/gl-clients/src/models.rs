//! Data models ported from the C# `GlacierLauncher.Models` namespace:
//! ClientInfo.cs, ClientLibraryPack.cs, CustomClientEntry.cs,
//! GlacierClientVersion.cs, and ClientDownloadState.cs.

use serde::{Deserialize, Serialize};

/// Ported from `ClientInfo.cs`'s `ClientType` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientType {
    Latite,
    Flarial,
    OderSo,
}

/// Ported from `ClientInfo.cs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    #[serde(rename = "type")]
    pub client_type: ClientType,
    pub name: String,
    pub version: Option<String>,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub is_up_to_date: bool,
    pub error_message: Option<String>,
    pub progress: f64,
}

/// Ported from `ClientLibraryPack.cs`'s `ClientLibraryDownload`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientLibraryDownload {
    pub label: String,
    pub file_name: String,
    pub repo_path: String,
    pub is_extension: bool,
}

/// Ported from `ClientLibraryPack.cs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientLibraryPack {
    pub version: String,
    pub name: String,
    pub icon_url: String,
    pub banner_url: String,
    pub description: String,
    pub downloads: Vec<ClientLibraryDownload>,
}

impl ClientLibraryPack {
    pub fn key(&self) -> String {
        format!("{}/{}", self.version, self.name)
    }

    pub fn main_downloads(&self) -> Vec<&ClientLibraryDownload> {
        self.downloads.iter().filter(|d| !d.is_extension).collect()
    }

    pub fn extensions(&self) -> Vec<&ClientLibraryDownload> {
        self.downloads.iter().filter(|d| d.is_extension).collect()
    }
}

/// Ported from `CustomClientEntry.cs`. Field names are `PascalCase` on the
/// wire to stay compatible with `library.json` files written by the C#
/// launcher's `CustomClientLibraryService`, which serializes with
/// `System.Text.Json`'s default (unmodified property name) policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomClientEntry {
    #[serde(default, rename = "Id")]
    pub id: String,
    #[serde(default, rename = "Name")]
    pub name: String,
    #[serde(default, rename = "FileName")]
    pub file_name: String,
    #[serde(default = "default_kind", rename = "Kind")]
    pub kind: String,
    #[serde(default, rename = "SourceUrl")]
    pub source_url: String,
    #[serde(default, rename = "Sha256")]
    pub sha256: String,
    #[serde(default, rename = "SizeBytes")]
    pub size_bytes: i64,
    #[serde(default, rename = "AddedIso")]
    pub added_iso: String,
    #[serde(default, rename = "UpdatedIso")]
    pub updated_iso: String,
    #[serde(default = "default_true", rename = "AutoUpdate")]
    pub auto_update: bool,
}

fn default_kind() -> String {
    "dll".to_string()
}

fn default_true() -> bool {
    true
}

impl CustomClientEntry {
    pub fn is_web(&self) -> bool {
        !self.source_url.trim().is_empty()
    }

    pub fn is_exe(&self) -> bool {
        self.kind == "exe"
    }
}

/// Ported from `GlacierClientVersion.cs`. Field names are `camelCase` on the
/// wire, matching `HttpClient.GetFromJsonAsync`'s `JsonSerializerDefaults.Web`
/// used when fetching `versions.json` from the Glacier Client CDN.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlacierClientVersion {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub fabric: bool,
    #[serde(default)]
    pub forge: bool,
    #[serde(default)]
    pub java_version: i32,
    #[serde(default)]
    pub changelog: String,
}

impl GlacierClientVersion {
    pub fn loader_label(&self) -> &'static str {
        if self.fabric {
            "Fabric"
        } else if self.forge {
            "Forge"
        } else {
            "Unknown"
        }
    }

    pub fn local_jar_name(&self) -> String {
        format!("Glacier-{}.jar", self.id)
    }

    /// `~/.glacier/versions/{id}` — mirrors `GlacierClientVersion.InstallDir`.
    pub fn install_dir(&self, user_profile: &std::path::Path) -> std::path::PathBuf {
        user_profile
            .join(".glacier")
            .join("versions")
            .join(&self.id)
    }

    pub fn jar_path(&self, user_profile: &std::path::Path) -> std::path::PathBuf {
        self.install_dir(user_profile).join(self.local_jar_name())
    }

    pub fn is_installed(&self, user_profile: &std::path::Path) -> bool {
        self.jar_path(user_profile).is_file()
    }
}

/// Ported from `GlacierClientVersion.cs`'s `GlacierManifest`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlacierManifest {
    #[serde(default)]
    pub schema_version: i32,
    #[serde(default)]
    pub latest_release: String,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub versions: Vec<GlacierClientVersion>,
    #[serde(default)]
    pub launcher: Option<GlacierLauncherMeta>,
}

/// Ported from `GlacierClientVersion.cs`'s `GlacierLauncherMeta`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlacierLauncherMeta {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha256: String,
}

/// Ported from `ClientDownloadState.cs`. The C# version also owns a
/// `CancellationTokenSource`; that's a call-site concern in Rust (e.g. a
/// `tokio_util::sync::CancellationToken` held by whoever drives the
/// download), so this struct only tracks the plain state fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientDownloadState {
    pub downloaded: bool,
    pub up_to_date: bool,
    pub downloading: bool,
    pub auto_updating: bool,
    pub progress: f64,
    pub error: String,
}

impl ClientDownloadState {
    pub fn has_error(&self) -> bool {
        !self.error.is_empty()
    }

    pub fn begin_download(&mut self) {
        self.downloading = true;
        self.progress = 0.0;
        self.error.clear();
    }

    pub fn end_download(&mut self) {
        self.downloading = false;
        self.progress = 0.0;
    }

    pub fn mark_removed(&mut self) {
        self.downloaded = false;
        self.up_to_date = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_library_pack_splits_main_vs_extension_downloads() {
        let pack = ClientLibraryPack {
            version: "1.21".into(),
            name: "Latite".into(),
            downloads: vec![
                ClientLibraryDownload {
                    label: "Latite".into(),
                    is_extension: false,
                    ..Default::default()
                },
                ClientLibraryDownload {
                    label: "LatiteExt".into(),
                    is_extension: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(pack.main_downloads().len(), 1);
        assert_eq!(pack.extensions().len(), 1);
        assert_eq!(pack.key(), "1.21/Latite");
    }

    #[test]
    fn download_state_lifecycle() {
        let mut state = ClientDownloadState {
            error: "boom".into(),
            ..Default::default()
        };
        assert!(state.has_error());

        state.begin_download();
        assert!(state.downloading);
        assert!(!state.has_error());

        state.end_download();
        assert!(!state.downloading);
        assert_eq!(state.progress, 0.0);
    }

    #[test]
    fn custom_client_entry_defaults_match_csharp() {
        let json = r#"{"Id":"abc","Name":"MyClient","FileName":"my.dll"}"#;
        let entry: CustomClientEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.kind, "dll");
        assert!(entry.auto_update);
        assert!(!entry.is_web());
        assert!(!entry.is_exe());
    }
}
