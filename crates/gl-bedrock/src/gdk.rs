//! The community GDK unpack archive — port of `Services/GdkArchiveService.cs`.
//!
//! Microsoft's own appx / Windows-Update channel stops at **1.21.120**: after
//! the GDK move, newer Bedrock builds were never published there. Without this
//! source the version browser simply cannot offer 1.26 / v26 and up, which is
//! why the MC Versions panel says so in its info popover.
//!
//! Entries here are third-party mirrors, not a Microsoft endpoint. Each asset
//! carries a published SHA-256 which the download path verifies, and rows
//! sourced this way are marked `GDK` in the UI so the distinction is visible
//! to the user rather than hidden.

use serde_json::Value;

use crate::error::{BedrockError, Result};
use crate::wu::WuVersion;

const REPO: &str = "bubbles-wow/mcbe-gdk-unpack-archive";
const API_URL: &str =
    "https://api.github.com/repos/bubbles-wow/mcbe-gdk-unpack-archive/releases?per_page=100";

/// Marker written into [`WuVersion::source`] for entries from this archive.
pub const SOURCE: &str = "gdk";

/// Matches the release assets this archive publishes, e.g.
/// `Microsoft.MinecraftUWP_1.21.0.3_x64__8wekyb3d8bbwe.zip`. Port of
/// `GdkArchiveService.AssetRegex`; written by hand rather than with a regex
/// crate, since the pattern is fixed and this avoids a dependency.
///
/// Returns `(family, version)` where family is `MinecraftUWP` or
/// `MinecraftWindowsBeta`.
fn parse_asset_name(name: &str) -> Option<(&'static str, String)> {
    let rest = name.strip_prefix("Microsoft.")?;
    let (family, rest) = if let Some(r) = strip_prefix_ci(rest, "MinecraftUWP_") {
        ("MinecraftUWP", r)
    } else if let Some(r) = strip_prefix_ci(rest, "MinecraftWindowsBeta_") {
        ("MinecraftWindowsBeta", r)
    } else {
        return None;
    };

    let rest = strip_suffix_ci(rest, "_x64__8wekyb3d8bbwe.zip")?;

    // Exactly four dot-separated numeric parts.
    let parts: Vec<&str> = rest.split('.').collect();
    if parts.len() != 4
        || !parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    Some((family, rest.to_string()))
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

fn strip_suffix_ci<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    if s.len() >= suffix.len() && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix) {
        Some(&s[..s.len() - suffix.len()])
    } else {
        None
    }
}

/// Fetches the archive's releases and turns their matching assets into
/// version entries. Port of `GdkArchiveService.GetVersionsAsync`.
pub async fn list_versions(client: &reqwest::Client) -> Result<Vec<WuVersion>> {
    let body = client
        .get(API_URL)
        .header("User-Agent", "GlacierLauncher")
        .send()
        .await
        .map_err(|e| BedrockError::Wu(e.to_string()))?
        .text()
        .await
        .map_err(|e| BedrockError::Wu(e.to_string()))?;
    parse_releases(&body)
}

/// Split out from the network call so the asset filtering and field mapping
/// are testable against a captured payload.
pub fn parse_releases(body: &str) -> Result<Vec<WuVersion>> {
    let root: Value = serde_json::from_str(body)?;
    let Some(releases) = root.as_array() else {
        // A non-array body is GitHub reporting an error (rate limit, etc.).
        // The C# returns an empty list here rather than throwing.
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for release in releases {
        if release.get("draft").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let tag = release
            .get("tag_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if tag.is_empty() {
            continue;
        }
        let Some(assets) = release.get("assets").and_then(Value::as_array) else {
            continue;
        };

        for asset in assets {
            let name = asset
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some((family, version)) = parse_asset_name(name) else {
                continue;
            };
            let url = asset
                .get("browser_download_url")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if url.is_empty() {
                continue;
            }

            // GitHub reports the digest as `sha256:<hex>`; anything else (or a
            // missing digest) means we have nothing to verify against.
            let digest = asset
                .get("digest")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let sha256 = strip_prefix_ci(digest, "sha256:")
                .unwrap_or_default()
                .to_string();

            out.push(WuVersion {
                version,
                update_id: String::new(),
                package_name: name.to_string(),
                size_bytes: asset.get("size").and_then(Value::as_u64).unwrap_or(0),
                channel: if family.eq_ignore_ascii_case("MinecraftWindowsBeta") {
                    "Preview".to_string()
                } else {
                    "Release".to_string()
                },
                source: SOURCE.to_string(),
                friendly_version: tag.to_string(),
                download_url: url.to_string(),
                sha256,
            });
        }
    }
    Ok(out)
}

/// The repo these builds are mirrored from, for anything that wants to name
/// the source to the user.
pub fn repo() -> &'static str {
    REPO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_two_published_asset_families() {
        assert_eq!(
            parse_asset_name("Microsoft.MinecraftUWP_1.21.0.3_x64__8wekyb3d8bbwe.zip"),
            Some(("MinecraftUWP", "1.21.0.3".to_string()))
        );
        assert_eq!(
            parse_asset_name("Microsoft.MinecraftWindowsBeta_1.26.40.1_x64__8wekyb3d8bbwe.zip"),
            Some(("MinecraftWindowsBeta", "1.26.40.1".to_string()))
        );
    }

    #[test]
    fn asset_matching_is_case_insensitive_like_the_regex() {
        assert!(
            parse_asset_name("Microsoft.MINECRAFTUWP_1.21.0.3_X64__8WEKYB3D8BBWE.ZIP").is_some()
        );
    }

    #[test]
    fn rejects_anything_that_is_not_a_package_asset() {
        for bad in [
            "README.md",
            "Microsoft.MinecraftUWP_1.21.0.3_x86__8wekyb3d8bbwe.zip",
            "Microsoft.MinecraftUWP_1.21.0.3_x64__8wekyb3d8bbwe.appx",
            "Microsoft.SomethingElse_1.21.0.3_x64__8wekyb3d8bbwe.zip",
            // three parts, not four
            "Microsoft.MinecraftUWP_1.21.0_x64__8wekyb3d8bbwe.zip",
            // non-numeric part
            "Microsoft.MinecraftUWP_1.21.0.beta_x64__8wekyb3d8bbwe.zip",
        ] {
            assert!(parse_asset_name(bad).is_none(), "should reject {bad}");
        }
    }

    #[test]
    fn maps_release_fields_and_skips_drafts() {
        let body = r#"[
          {
            "draft": true,
            "tag_name": "skipme",
            "assets": [{"name":"Microsoft.MinecraftUWP_1.21.0.3_x64__8wekyb3d8bbwe.zip",
                        "browser_download_url":"https://example.invalid/a.zip","size":10,
                        "digest":"sha256:aaaa"}]
          },
          {
            "tag_name": "v1.26.40",
            "assets": [
              {"name":"Microsoft.MinecraftWindowsBeta_1.26.40.1_x64__8wekyb3d8bbwe.zip",
               "browser_download_url":"https://example.invalid/b.zip","size":1234,
               "digest":"sha256:deadbeef"},
              {"name":"notes.txt","browser_download_url":"https://example.invalid/n.txt","size":1}
            ]
          }
        ]"#;
        let out = parse_releases(body).unwrap();
        assert_eq!(out.len(), 1, "drafts and non-package assets are skipped");
        let v = &out[0];
        assert_eq!(v.version, "1.26.40.1");
        assert_eq!(v.friendly_version, "v1.26.40");
        assert_eq!(v.channel, "Preview");
        assert_eq!(v.source, SOURCE);
        assert_eq!(v.download_url, "https://example.invalid/b.zip");
        assert_eq!(v.sha256, "deadbeef");
        assert_eq!(v.size_bytes, 1234);
        assert!(
            v.update_id.is_empty(),
            "GDK entries have no Windows Update id"
        );
    }

    #[test]
    fn a_missing_or_odd_digest_yields_no_hash_rather_than_a_bad_one() {
        let body = r#"[{"tag_name":"t","assets":[
            {"name":"Microsoft.MinecraftUWP_1.21.0.3_x64__8wekyb3d8bbwe.zip",
             "browser_download_url":"https://example.invalid/a.zip","size":1,"digest":"md5:xyz"}
        ]}]"#;
        let out = parse_releases(body).unwrap();
        assert_eq!(out.len(), 1);
        assert!(
            out[0].sha256.is_empty(),
            "a non-sha256 digest must not be treated as one"
        );
    }

    #[test]
    fn an_error_body_is_an_empty_list_not_a_failure() {
        // GitHub answers rate limiting with an object, not an array.
        let out = parse_releases(r#"{"message":"API rate limit exceeded"}"#).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn an_asset_without_a_download_url_is_skipped() {
        let body = r#"[{"tag_name":"t","assets":[
            {"name":"Microsoft.MinecraftUWP_1.21.0.3_x64__8wekyb3d8bbwe.zip","size":1}
        ]}]"#;
        assert!(parse_releases(body).unwrap().is_empty());
    }
}
