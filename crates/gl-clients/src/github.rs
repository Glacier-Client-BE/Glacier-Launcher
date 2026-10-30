//! Shared helpers for talking to the GitHub Releases API, used by
//! LeviLamina's self-update (LeviLaminaService.cs) and its mod registry
//! (LeviLaminaModsService.cs).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::error::{ClientsError, Result};

/// How long a successful GitHub API response is reused before re-fetching.
/// Release metadata changes rarely, and the unauthenticated API rate limit
/// is tight, so repeated UI lookups should not each hit the network.
const RESPONSE_TTL: Duration = Duration::from_secs(5 * 60);

fn response_cache() -> &'static Mutex<HashMap<String, (Instant, String)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (Instant, String)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// GETs a GitHub API URL and returns the response body, reusing a successful
/// response for up to `RESPONSE_TTL`. Non-2xx bodies (for example rate-limit
/// or "Not Found" payloads) are returned to the caller but never cached, so a
/// transient failure does not stick around for the whole TTL.
pub async fn get_text_cached(http: &reqwest::Client, url: &str) -> Result<String> {
    if let Ok(cache) = response_cache().lock() {
        if let Some((at, body)) = cache.get(url) {
            if at.elapsed() < RESPONSE_TTL {
                return Ok(body.clone());
            }
        }
    }

    let resp = http
        .get(url)
        .header("User-Agent", "GlacierLauncher")
        .send()
        .await?;
    let ok = resp.status().is_success();
    let body = resp.text().await?;

    if ok {
        if let Ok(mut cache) = response_cache().lock() {
            cache.retain(|_, (at, _)| at.elapsed() < RESPONSE_TTL);
            cache.insert(url.to_string(), (Instant::now(), body.clone()));
        }
    }
    Ok(body)
}

/// A single release asset, as returned by GitHub's Releases API.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub browser_download_url: String,
}

/// A GitHub release, trimmed to the fields the launcher cares about.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GitHubRelease {
    #[serde(default, rename = "tag_name")]
    pub tag_name: String,
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
    /// Present on GitHub's "Not Found" error payload for a missing tag.
    #[serde(default)]
    pub message: Option<String>,
}

impl GitHubRelease {
    pub fn is_not_found(&self) -> bool {
        matches!(&self.message, Some(m) if m.eq_ignore_ascii_case("not found"))
    }
}

/// Parses a GitHub release JSON body (as returned by `.../releases/latest`
/// or `.../releases/tags/{tag}`).
pub fn parse_release(body: &str) -> Result<GitHubRelease> {
    serde_json::from_str(body).map_err(ClientsError::from)
}

/// Picks the best `.zip` asset from a release, preferring names that look
/// Windows-specific. Mirrors LeviLaminaService.FetchLatestReleaseAsync's
/// asset-selection logic.
pub fn pick_windows_zip_asset(release: &GitHubRelease) -> Option<&GitHubAsset> {
    let candidates: Vec<&GitHubAsset> = release
        .assets
        .iter()
        .filter(|a| a.name.to_ascii_lowercase().ends_with(".zip"))
        .collect();

    candidates
        .iter()
        .find(|a| {
            let lower = a.name.to_ascii_lowercase();
            lower.contains("windows") || lower.contains("win")
        })
        .or_else(|| candidates.first())
        .copied()
}

/// Picks the best downloadable asset for a LeviLamina mod release: prefers
/// a `.zip`, otherwise falls back to the first asset with a download URL.
/// Mirrors LeviLaminaModsService.DownloadAndInstallAsync's asset pick.
pub fn pick_mod_asset(release: &GitHubRelease) -> Option<&GitHubAsset> {
    let candidates: Vec<&GitHubAsset> = release
        .assets
        .iter()
        .filter(|a| !a.browser_download_url.is_empty())
        .collect();

    candidates
        .iter()
        .find(|a| a.name.to_ascii_lowercase().ends_with(".zip"))
        .or_else(|| candidates.first())
        .copied()
}

/// Builds the tag candidates LeviLaminaModsService tries when resolving a
/// mod's `LatestVersion` string to a GitHub release tag: the version as-is,
/// then with (or without) a leading "v".
pub fn tag_candidates(latest_version: &str) -> Vec<String> {
    if latest_version.starts_with('v') || latest_version.starts_with('V') {
        vec![latest_version.to_string(), latest_version[1..].to_string()]
    } else {
        vec![latest_version.to_string(), format!("v{latest_version}")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVILAMINA_RELEASE: &str = r#"
    {
        "tag_name": "v1.2.3",
        "assets": [
            { "name": "LeviLamina-linux.tar.gz", "browser_download_url": "https://example.com/linux.tar.gz" },
            { "name": "LeviLamina-windows-x64.zip", "browser_download_url": "https://example.com/windows.zip" },
            { "name": "LeviLamina-source.zip", "browser_download_url": "https://example.com/source.zip" }
        ]
    }"#;

    #[test]
    fn parses_release_and_picks_windows_zip() {
        let release = parse_release(LEVILAMINA_RELEASE).unwrap();
        assert_eq!(release.tag_name, "v1.2.3");
        let asset = pick_windows_zip_asset(&release).unwrap();
        assert_eq!(asset.name, "LeviLamina-windows-x64.zip");
    }

    #[test]
    fn falls_back_to_first_zip_when_no_windows_asset() {
        let json = r#"{"tag_name":"v1","assets":[
            {"name":"foo-linux.zip","browser_download_url":"https://x/a.zip"},
            {"name":"foo-mac.zip","browser_download_url":"https://x/b.zip"}
        ]}"#;
        let release = parse_release(json).unwrap();
        let asset = pick_windows_zip_asset(&release).unwrap();
        assert_eq!(asset.name, "foo-linux.zip");
    }

    #[test]
    fn detects_not_found_payload() {
        let json = r#"{"message":"Not Found","documentation_url":"https://x"}"#;
        let release = parse_release(json).unwrap();
        assert!(release.is_not_found());
    }

    #[test]
    fn mod_asset_prefers_zip_over_other_files() {
        let json = r#"{"tag_name":"v1","assets":[
            {"name":"readme.txt","browser_download_url":"https://x/readme.txt"},
            {"name":"plugin.zip","browser_download_url":"https://x/plugin.zip"}
        ]}"#;
        let release = parse_release(json).unwrap();
        let asset = pick_mod_asset(&release).unwrap();
        assert_eq!(asset.name, "plugin.zip");
    }

    #[test]
    fn tag_candidates_strip_and_add_v_prefix() {
        assert_eq!(tag_candidates("v1.2.3"), vec!["v1.2.3", "1.2.3"]);
        assert_eq!(tag_candidates("1.2.3"), vec!["1.2.3", "v1.2.3"]);
    }
}
