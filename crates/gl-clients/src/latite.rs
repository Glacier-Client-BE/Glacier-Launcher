//! Ported from `GameLauncher.GetVersionsAsync`/`FormatLatiteDisplayName`:
//! lists Latite's versioned GitHub releases (`LatiteClient/Latite`) and
//! picks the best `.dll` asset for each release.
//!
//! Actual downloading is `gl-download`'s job; injection is `gl-injection`'s
//! job. This module only covers the catalog/metadata side.

use reqwest::Client;
use serde_json::Value;

use crate::error::{ClientsError, Result};

pub const RELEASES_API_URL: &str =
    "https://api.github.com/repos/LatiteClient/Latite/releases?per_page=60";

/// One Latite release, with the best `.dll` asset already picked.
/// Mirrors the `MinecraftVersion` fields `GameLauncher.GetVersionsAsync` fills in.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LatiteVersion {
    pub tag: String,
    pub display_name: String,
    pub download_url: String,
    pub asset_size: u64,
    pub published_at: Option<String>,
    pub variant: String,
}

/// Fetches and parses the Latite releases list. Mirrors `GameLauncher.GetVersionsAsync`.
pub async fn list_versions(http: &Client) -> Result<Vec<LatiteVersion>> {
    let body = crate::github::get_text_cached(http, RELEASES_API_URL).await?;
    parse_versions(&body)
}

pub fn parse_versions(body: &str) -> Result<Vec<LatiteVersion>> {
    let root: Value = serde_json::from_str(body).map_err(ClientsError::from)?;
    let arr = root
        .as_array()
        .ok_or_else(|| ClientsError::NotFound("unexpected response from GitHub".into()))?;

    let mut out = Vec::new();

    for release in arr {
        let tag = release
            .get("tag_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if tag.is_empty() {
            continue;
        }
        if matches!(tag.as_str(), "nightly" | "debug" | "latest") {
            continue;
        }
        if release
            .get("draft")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }

        let name = release
            .get("name")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or(&tag)
            .to_string();
        let published_at = release
            .get("published_at")
            .and_then(Value::as_str)
            .map(|s| s.to_string());

        let mut best_url: Option<String> = None;
        let mut best_size: u64 = 0;
        let mut best_asset_name: Option<String> = None;
        let mut best_rank = i32::MAX;

        if let Some(assets) = release.get("assets").and_then(Value::as_array) {
            for asset in assets {
                let a_name = asset.get("name").and_then(Value::as_str).unwrap_or("");
                if !a_name.to_ascii_lowercase().ends_with(".dll") {
                    continue;
                }
                if a_name.to_ascii_lowercase().contains("debug") {
                    continue;
                }

                let rank = if a_name.eq_ignore_ascii_case("Latite.dll") {
                    0
                } else if a_name.to_ascii_lowercase().contains("nightly") {
                    1
                } else {
                    2
                };

                if rank < best_rank {
                    best_rank = rank;
                    best_asset_name = Some(a_name.to_string());
                    best_url = asset
                        .get("browser_download_url")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string());
                    best_size = asset.get("size").and_then(Value::as_u64).unwrap_or(0);
                }
            }
        }

        let Some(url) = best_url else { continue };
        let asset_name = best_asset_name.unwrap_or_default();
        let lower_asset = asset_name.to_ascii_lowercase();
        let variant = if lower_asset.contains("nightly") {
            "Nightly"
        } else if lower_asset.contains("debug") {
            "Debug"
        } else {
            "Release"
        }
        .to_string();

        out.push(LatiteVersion {
            tag: tag.clone(),
            display_name: format_display_name(&name, &tag, published_at.as_deref()),
            download_url: url,
            asset_size: best_size,
            published_at,
            variant,
        });
    }

    Ok(out)
}

/// Mirrors `GameLauncher.FormatLatiteDisplayName`: recognizes Latite's
/// `release-YYYYMMDD-HHMMSS` style tags and formats them nicely, falling
/// back to the GitHub release name + published date, or just the name.
fn format_display_name(api_name: &str, tag: &str, published_at: Option<&str>) -> String {
    if let Some((variant, date)) = parse_latite_tag(tag) {
        return format!("Latite {variant} · {date}");
    }
    if let Some(pub_at) = published_at {
        if let Some(formatted) = format_published_at(pub_at) {
            return format!("{api_name} · {formatted}");
        }
    }
    api_name.to_string()
}

/// Parses tags like `release-20240102-030405` into ("Release", "2024-01-02 03:04").
fn parse_latite_tag(tag: &str) -> Option<(String, String)> {
    let lower = tag.to_ascii_lowercase();
    let (kind, rest) = if let Some(r) = lower.strip_prefix("nightly-") {
        ("Nightly", r)
    } else if let Some(r) = lower.strip_prefix("debug-") {
        ("Debug", r)
    } else if let Some(r) = lower.strip_prefix("release-") {
        ("Release", r)
    } else {
        return None;
    };

    // rest should be YYYYMMDD-HHMMSS
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() != 2 || parts[0].len() != 8 || parts[1].len() != 6 {
        return None;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit())
        || !parts[1].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }

    let (y, m, d) = (&parts[0][0..4], &parts[0][4..6], &parts[0][6..8]);
    let (hh, mm) = (&parts[1][0..2], &parts[1][2..4]);
    Some((kind.to_string(), format!("{y}-{m}-{d} {hh}:{mm}")))
}

/// Best-effort ISO-8601 -> "YYYY-MM-DD HH:MM" formatting without pulling in
/// a chrono dependency: GitHub's `published_at` is always
/// `YYYY-MM-DDTHH:MM:SSZ`, so a direct slice suffices.
fn format_published_at(published_at: &str) -> Option<String> {
    if published_at.len() < 16 {
        return None;
    }
    let date = &published_at[0..10];
    let time = &published_at[11..16];
    Some(format!("{date} {time}"))
}

/// The path a downloaded Latite DLL should live at, mirroring
/// `GameLauncher.GetDllPath`: `<clients_dir>/Latite/Latite_<tag>.dll`.
pub fn dll_file_name(tag: &str) -> String {
    format!("Latite_{tag}.dll")
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASES_JSON: &str = r#"[
        {
            "tag_name": "release-20240102-030405",
            "name": "Release build",
            "draft": false,
            "published_at": "2024-01-02T03:04:05Z",
            "assets": [
                { "name": "Latite.dll", "browser_download_url": "https://example.com/latite.dll", "size": 1000 },
                { "name": "LatiteDebug.dll", "browser_download_url": "https://example.com/debug.dll", "size": 2000 }
            ]
        },
        {
            "tag_name": "nightly",
            "name": "rolling nightly",
            "draft": false,
            "assets": []
        },
        {
            "tag_name": "v1.0-draft",
            "name": "draft",
            "draft": true,
            "assets": [
                { "name": "Latite.dll", "browser_download_url": "https://example.com/draft.dll", "size": 1 }
            ]
        },
        {
            "tag_name": "no-assets",
            "name": "no assets",
            "draft": false,
            "assets": [
                { "name": "readme.txt", "browser_download_url": "https://example.com/readme.txt", "size": 1 }
            ]
        }
    ]"#;

    #[test]
    fn parses_and_filters_releases() {
        let versions = parse_versions(RELEASES_JSON).unwrap();
        // "nightly" tag literal is filtered out, draft is filtered out, no-dll-asset is filtered out.
        assert_eq!(versions.len(), 1);
        let v = &versions[0];
        assert_eq!(v.tag, "release-20240102-030405");
        assert_eq!(v.download_url, "https://example.com/latite.dll");
        assert_eq!(v.asset_size, 1000);
        assert_eq!(v.variant, "Release");
        assert_eq!(v.display_name, "Latite Release · 2024-01-02 03:04");
    }

    #[test]
    fn falls_back_to_name_and_date_for_non_standard_tags() {
        let json = r#"[{
            "tag_name": "v2.5.0",
            "name": "Version 2.5.0",
            "draft": false,
            "published_at": "2023-06-15T12:30:00Z",
            "assets": [{ "name": "Latite.dll", "browser_download_url": "https://x/a.dll", "size": 5 }]
        }]"#;
        let versions = parse_versions(json).unwrap();
        assert_eq!(versions[0].display_name, "Version 2.5.0 · 2023-06-15 12:30");
    }

    #[test]
    fn builds_dll_file_name() {
        assert_eq!(
            dll_file_name("release-20240102-030405"),
            "Latite_release-20240102-030405.dll"
        );
    }
}
