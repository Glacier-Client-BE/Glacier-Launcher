//! Port of `Services/JavaModLoaderService.cs`: installs Fabric/Quilt version
//! profiles and downloads Forge/NeoForge installer jars.
//!
//! Deviation from the C# original: Forge/NeoForge installers are only
//! downloaded here (mirroring `DownloadForgeInstallerAsync` /
//! `DownloadNeoForgeInstallerAsync`) — actually *running* the installer jar
//! (which requires a JVM + the Forge installer's own install-client flow) is
//! left to a later pass; the download is real and the jar path is returned so
//! callers can invoke it with the instance's Java runtime.

use crate::download::{download_to_file, get_string};
use crate::error::{io_err, JavaError, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Install a Fabric loader profile for `minecraft_version` into `versions_dir`
/// (mirrors `InstallFabricAsync`). Returns the resolved profile id.
pub async fn install_fabric(
    client: &reqwest::Client,
    versions_dir: &Path,
    minecraft_version: &str,
    loader_version: Option<&str>,
) -> Result<String> {
    let loader = match loader_version {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => latest_fabric_loader(client).await?,
    };
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        urlencode(minecraft_version),
        urlencode(&loader)
    );
    let id = format!("fabric-loader-{loader}-{minecraft_version}");
    install_profile(client, versions_dir, &id, &url, "Fabric", minecraft_version).await
}

/// Install a Quilt loader profile (mirrors `InstallQuiltAsync`).
pub async fn install_quilt(
    client: &reqwest::Client,
    versions_dir: &Path,
    minecraft_version: &str,
    loader_version: Option<&str>,
) -> Result<String> {
    let loader = match loader_version {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => latest_quilt_loader(client).await?,
    };
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
        urlencode(minecraft_version),
        urlencode(&loader)
    );
    let id = format!("quilt-loader-{loader}-{minecraft_version}");
    install_profile(client, versions_dir, &id, &url, "Quilt", minecraft_version).await
}

/// Download the Forge installer jar for `minecraft_version` into
/// `<root_dir>/loaders/forge/`, returning the local path (mirrors
/// `DownloadForgeInstallerAsync`).
pub async fn download_forge_installer(
    client: &reqwest::Client,
    root_dir: &Path,
    minecraft_version: &str,
    forge_version: Option<&str>,
) -> Result<PathBuf> {
    let build = match forge_version {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => latest_forge_version(client, minecraft_version).await?,
    };
    let artifact = resolve_forge_artifact(client, minecraft_version, &build).await;
    let file = format!("forge-{artifact}-installer.jar");
    let url =
        format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{artifact}/{file}");
    let dest = root_dir.join("loaders").join("forge").join(&file);
    download_to_file(client, &url, &dest).await?;
    Ok(dest)
}

/// Download the NeoForge installer jar (mirrors `DownloadNeoForgeInstallerAsync`).
pub async fn download_neoforge_installer(
    client: &reqwest::Client,
    root_dir: &Path,
    minecraft_version: &str,
) -> Result<PathBuf> {
    let ver = latest_neoforge_version(client, minecraft_version).await?;
    let file = format!("neoforge-{ver}-installer.jar");
    let url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{ver}/{file}");
    let dest = root_dir.join("loaders").join("neoforge").join(&file);
    download_to_file(client, &url, &dest).await?;
    Ok(dest)
}

async fn install_profile(
    client: &reqwest::Client,
    versions_dir: &Path,
    id: &str,
    url: &str,
    loader_name: &str,
    minecraft_version: &str,
) -> Result<String> {
    let resp = client.get(url).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND
        || resp.status() == reqwest::StatusCode::BAD_REQUEST
    {
        return Err(JavaError::Other(format!(
            "{loader_name} does not support Minecraft {minecraft_version}."
        )));
    }
    let resp = resp.error_for_status().map_err(JavaError::Http)?;
    let json = resp.text().await?;
    let parsed: Value = serde_json::from_str(&json)?;
    let profile_id = parsed
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(id)
        .to_string();

    let dir = versions_dir.join(&profile_id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(io_err(&dir))?;
    let dest = dir.join(format!("{profile_id}.json"));
    crate::util::write_atomic(&dest, &json)?;
    Ok(profile_id)
}

async fn latest_fabric_loader(client: &reqwest::Client) -> Result<String> {
    let json = get_string(client, "https://meta.fabricmc.net/v2/versions/loader").await?;
    let parsed: Value = serde_json::from_str(&json)?;
    let arr = parsed.as_array().cloned().unwrap_or_default();
    for item in &arr {
        if item.get("stable").and_then(Value::as_bool) == Some(true) {
            if let Some(v) = item.get("version").and_then(Value::as_str) {
                return Ok(v.to_string());
            }
        }
    }
    arr.first()
        .and_then(|v| v.get("version"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| JavaError::Other("no Fabric loader versions found".to_string()))
}

async fn latest_quilt_loader(client: &reqwest::Client) -> Result<String> {
    let json = get_string(client, "https://meta.quiltmc.org/v3/versions/loader").await?;
    let parsed: Value = serde_json::from_str(&json)?;
    let arr = parsed.as_array().cloned().unwrap_or_default();
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
    newest.ok_or_else(|| JavaError::Other("no Quilt loader versions found".to_string()))
}

fn is_pre_release(version: &str) -> bool {
    let lower = version.to_lowercase();
    lower.contains("-beta") || lower.contains("-pre") || lower.contains("-rc")
}

async fn resolve_forge_artifact(
    client: &reqwest::Client,
    minecraft_version: &str,
    build: &str,
) -> String {
    let coordinate = format!("{minecraft_version}-{build}");
    let Ok(xml) = get_string(
        client,
        "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml",
    )
    .await
    else {
        return coordinate;
    };
    let matches: Vec<&str> = xml
        .split("<version>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</version>").next())
        .filter(|v| *v == coordinate || v.starts_with(&format!("{coordinate}-")))
        .collect();
    matches
        .iter()
        .find(|v| **v == coordinate)
        .or_else(|| matches.first())
        .map(|v| v.to_string())
        .unwrap_or(coordinate)
}

async fn latest_forge_version(client: &reqwest::Client, minecraft_version: &str) -> Result<String> {
    let json = get_string(
        client,
        "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
    )
    .await?;
    let parsed: Value = serde_json::from_str(&json)?;
    if let Some(promos) = parsed.get("promos") {
        if let Some(rec) = promos
            .get(format!("{minecraft_version}-recommended"))
            .and_then(Value::as_str)
        {
            return Ok(rec.to_string());
        }
        if let Some(lat) = promos
            .get(format!("{minecraft_version}-latest"))
            .and_then(Value::as_str)
        {
            return Ok(lat.to_string());
        }
    }
    Err(JavaError::Other(format!(
        "No Forge version found for Minecraft {minecraft_version}. You can enter a version manually from files.minecraftforge.net."
    )))
}

async fn latest_neoforge_version(
    client: &reqwest::Client,
    minecraft_version: &str,
) -> Result<String> {
    let json = get_string(
        client,
        "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge",
    )
    .await?;
    let parsed: Value = serde_json::from_str(&json)?;
    let versions = parsed
        .get("versions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let parts: Vec<&str> = minecraft_version.split('.').collect();
    let prefix = if parts.len() >= 3 {
        format!("{}.{}.", parts[1], parts[2])
    } else if parts.len() >= 2 {
        format!("{}.0.", parts[1])
    } else {
        return Err(JavaError::Other(format!(
            "No NeoForge version found for Minecraft {minecraft_version}. NeoForge supports 1.20.2+."
        )));
    };

    let mut best: Option<String> = None;
    for v in &versions {
        if let Some(ver) = v.as_str() {
            if ver.starts_with(&prefix) {
                best = Some(ver.to_string());
            }
        }
    }
    best.ok_or_else(|| {
        JavaError::Other(format!(
            "No NeoForge version found for Minecraft {minecraft_version}. NeoForge supports 1.20.2+."
        ))
    })
}

fn urlencode(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for b in v.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
