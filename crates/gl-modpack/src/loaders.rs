//! Mod loader metadata/version resolution for Fabric, Quilt, Forge and
//! NeoForge, mirroring `JavaModLoaderService`. Each loader has a distinct API
//! shape:
//! - Fabric/Quilt: a JSON "version profile" (a launcher `version.json`) is
//!   fetched directly and can be written straight into the versions folder.
//! - Forge/NeoForge: only a version string is resolved here; the actual
//!   install is a self-extracting installer jar that must be downloaded and
//!   run with a JVM, which is out of scope for this crate.

use serde_json::Value;

use crate::error::{ModpackError, Result};

const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
const QUILT_META: &str = "https://meta.quiltmc.org/v3";
const FORGE_MAVEN_METADATA: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_VERSIONS: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

pub struct LoaderClient {
    http: reqwest::Client,
}

/// A resolved Fabric/Quilt launcher version profile: the JSON document that
/// should be written to `<versions>/<id>/<id>.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionProfile {
    pub id: String,
    pub json: Value,
}

impl LoaderClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    async fn get_json(&self, url: &str) -> Result<(reqwest::StatusCode, Value)> {
        let resp = self.http.get(url).send().await?;
        let status = resp.status();
        let json = resp.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, json))
    }

    /// Fetches the Fabric loader version profile for a given Minecraft
    /// version; resolves the latest stable loader release if `loader_version`
    /// is empty.
    pub async fn resolve_fabric_profile(
        &self,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<VersionProfile> {
        let loader = if loader_version.trim().is_empty() {
            self.latest_fabric_loader().await?
        } else {
            loader_version.to_string()
        };
        let url = format!(
            "{FABRIC_META}/versions/loader/{}/{}/profile/json",
            urlencoding_encode(minecraft_version),
            urlencoding_encode(&loader)
        );
        self.fetch_profile(
            &url,
            &format!("fabric-loader-{loader}-{minecraft_version}"),
            "Fabric",
            minecraft_version,
        )
        .await
    }

    /// Fetches the Quilt loader version profile, mirroring
    /// `resolve_fabric_profile`.
    pub async fn resolve_quilt_profile(
        &self,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<VersionProfile> {
        let loader = if loader_version.trim().is_empty() {
            self.latest_quilt_loader().await?
        } else {
            loader_version.to_string()
        };
        let url = format!(
            "{QUILT_META}/versions/loader/{}/{}/profile/json",
            urlencoding_encode(minecraft_version),
            urlencoding_encode(&loader)
        );
        self.fetch_profile(
            &url,
            &format!("quilt-loader-{loader}-{minecraft_version}"),
            "Quilt",
            minecraft_version,
        )
        .await
    }

    async fn fetch_profile(
        &self,
        url: &str,
        fallback_id: &str,
        loader_name: &str,
        minecraft_version: &str,
    ) -> Result<VersionProfile> {
        let (status, json) = self.get_json(url).await?;
        if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::BAD_REQUEST {
            return Err(ModpackError::NotFound(format!(
                "{loader_name} does not support Minecraft {minecraft_version}."
            )));
        }
        if !status.is_success() {
            return Err(ModpackError::Status {
                status: status.as_u16(),
                message: format!("failed to fetch {loader_name} profile"),
            });
        }
        let id = json
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or(fallback_id)
            .to_string();
        Ok(VersionProfile { id, json })
    }

    async fn latest_fabric_loader(&self) -> Result<String> {
        let (status, json) = self
            .get_json(&format!("{FABRIC_META}/versions/loader"))
            .await?;
        if !status.is_success() {
            return Err(ModpackError::Status {
                status: status.as_u16(),
                message: "failed to fetch Fabric loader versions".into(),
            });
        }
        let arr = json.as_array().cloned().unwrap_or_default();
        for item in &arr {
            if item.get("stable").and_then(Value::as_bool).unwrap_or(false) {
                if let Some(v) = item.get("version").and_then(Value::as_str) {
                    return Ok(v.to_string());
                }
            }
        }
        arr.first()
            .and_then(|i| i.get("version"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| ModpackError::NotFound("No Fabric loader versions available.".into()))
    }

    async fn latest_quilt_loader(&self) -> Result<String> {
        let (status, json) = self
            .get_json(&format!("{QUILT_META}/versions/loader"))
            .await?;
        if !status.is_success() {
            return Err(ModpackError::Status {
                status: status.as_u16(),
                message: "failed to fetch Quilt loader versions".into(),
            });
        }
        let arr = json.as_array().cloned().unwrap_or_default();
        let mut newest: Option<String> = None;
        for item in &arr {
            let version = item
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if newest.is_none() {
                newest = Some(version.clone());
            }
            if !is_pre_release(&version) {
                return Ok(version);
            }
        }
        newest.ok_or_else(|| ModpackError::NotFound("No Quilt loader versions available.".into()))
    }

    /// Resolves the recommended (or latest) Forge build number for a
    /// Minecraft version from the promotions feed.
    pub async fn latest_forge_version(&self, minecraft_version: &str) -> Result<String> {
        let (status, json) = self.get_json(FORGE_PROMOTIONS).await?;
        if !status.is_success() {
            return Err(ModpackError::Status {
                status: status.as_u16(),
                message: "failed to fetch Forge promotions".into(),
            });
        }
        let promos = json.get("promos");
        if let Some(v) = promos
            .and_then(|p| p.get(format!("{minecraft_version}-recommended")))
            .and_then(Value::as_str)
        {
            return Ok(v.to_string());
        }
        if let Some(v) = promos
            .and_then(|p| p.get(format!("{minecraft_version}-latest")))
            .and_then(Value::as_str)
        {
            return Ok(v.to_string());
        }
        Err(ModpackError::NotFound(format!(
            "No Forge version found for Minecraft {minecraft_version}. You can enter a version manually from files.minecraftforge.net."
        )))
    }

    /// Resolves the full Forge maven artifact coordinate (e.g.
    /// `1.19.2-43.2.0`) for a Minecraft version + build, disambiguating
    /// builds that have extra maven-metadata suffixes.
    pub async fn resolve_forge_artifact(
        &self,
        minecraft_version: &str,
        build: &str,
    ) -> Result<String> {
        let coordinate = format!("{minecraft_version}-{build}");
        let resp = match self.http.get(FORGE_MAVEN_METADATA).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => return Ok(coordinate),
        };
        let Ok(xml) = resp.text().await else {
            return Ok(coordinate);
        };
        let versions = extract_xml_tag_values(&xml, "version");
        if versions.iter().any(|v| v == &coordinate) {
            return Ok(coordinate);
        }
        if let Some(v) = versions
            .iter()
            .find(|v| v.starts_with(&format!("{coordinate}-")))
        {
            return Ok(v.clone());
        }
        Ok(coordinate)
    }

    /// Resolves the newest NeoForge version whose branch prefix matches the
    /// given Minecraft version (NeoForge versions look like `20.1.100` for MC
    /// `1.20.1`).
    pub async fn latest_neoforge_version(&self, minecraft_version: &str) -> Result<String> {
        let (status, json) = self.get_json(NEOFORGE_VERSIONS).await?;
        if !status.is_success() {
            return Err(ModpackError::Status {
                status: status.as_u16(),
                message: "failed to fetch NeoForge versions".into(),
            });
        }
        let versions = json
            .get("versions")
            .and_then(Value::as_array)
            .ok_or_else(|| ModpackError::NotFound("Could not fetch NeoForge versions.".into()))?;

        let parts: Vec<&str> = minecraft_version.split('.').collect();
        let prefix = if parts.len() >= 3 {
            format!("{}.{}.", parts[1], parts[2])
        } else if parts.len() == 2 {
            format!("{}.0.", parts[1])
        } else {
            return Err(ModpackError::NotFound(format!(
                "No NeoForge version found for Minecraft {minecraft_version}. NeoForge supports 1.20.2+."
            )));
        };

        let mut best: Option<String> = None;
        for v in versions {
            if let Some(ver) = v.as_str() {
                if ver.starts_with(&prefix) {
                    best = Some(ver.to_string());
                }
            }
        }
        best.ok_or_else(|| ModpackError::NotFound(format!(
            "No NeoForge version found for Minecraft {minecraft_version}. NeoForge supports 1.20.2+."
        )))
    }
}

fn is_pre_release(version: &str) -> bool {
    let lower = version.to_ascii_lowercase();
    lower.contains("-beta") || lower.contains("-pre") || lower.contains("-rc")
}

/// Extracts the text content of every `<tag>...</tag>` occurrence — enough
/// for the flat `<versions><version>x</version>...</versions>` shape of
/// Forge's maven-metadata.xml without pulling in an XML crate.
fn extract_xml_tag_values(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let after_open = &rest[start + open.len()..];
        if let Some(end) = after_open.find(&close) {
            out.push(after_open[..end].to_string());
            rest = &after_open[end + close.len()..];
        } else {
            break;
        }
    }
    out
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pre_release_quilt_versions() {
        assert!(is_pre_release("0.20.0-beta.3"));
        assert!(is_pre_release("0.20.0-rc.1"));
        assert!(!is_pre_release("0.20.0"));
    }

    #[test]
    fn extracts_xml_versions() {
        let xml = "<metadata><versioning><versions><version>1.19.2-43.2.0</version><version>1.19.2-43.2.1</version></versions></versioning></metadata>";
        let versions = extract_xml_tag_values(xml, "version");
        assert_eq!(versions, vec!["1.19.2-43.2.0", "1.19.2-43.2.1"]);
    }

    #[test]
    fn neoforge_prefix_uses_minor_patch_of_minecraft_version() {
        // 1.20.1 -> prefix "20.1."
        let parts: Vec<&str> = "1.20.1".split('.').collect();
        let prefix = format!("{}.{}.", parts[1], parts[2]);
        assert_eq!(prefix, "20.1.");
    }
}
