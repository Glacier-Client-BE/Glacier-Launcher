//! GitHub releases lookup, mirroring `AutoUpdateService.CheckLauncherUpdateAsync`
//! and `AutoUpdateService.GetRecentReleasesAsync`.
//!
//! Network + caching glue lives on [`UpdateChecker`]; the pure parsing/selection
//! logic (release picking, asset picking) is exposed as free functions so it can
//! be unit-tested against fixture JSON without a live HTTP call.

use crate::cache::GitHubApiCache;
use crate::error::{Result, UpdateError};
use crate::models::{GitHubAsset, GitHubRelease, LauncherRelease, LauncherUpdateInfo};
use crate::version::{is_newer_version, normalize_tag};

/// Checks GitHub releases for the launcher and downloads a resumable update,
/// mirroring the org/repo/HTTP-client bundle the C# `AutoUpdateService` held.
pub struct UpdateChecker {
    client: reqwest::Client,
    cache: GitHubApiCache,
    releases_url: String,
}

impl UpdateChecker {
    pub fn new(org: &str, repo: &str) -> Self {
        Self::with_client(org, repo, reqwest::Client::new())
    }

    pub fn with_client(org: &str, repo: &str, client: reqwest::Client) -> Self {
        Self {
            client,
            cache: GitHubApiCache::new(),
            releases_url: format!("https://api.github.com/repos/{org}/{repo}/releases?per_page=30"),
        }
    }

    /// Fetches the releases list, using the in-memory cache when fresh.
    async fn fetch_releases_json(&self) -> Result<String> {
        if let Some(hit) = self.cache.get_fresh(&self.releases_url) {
            return Ok(hit.body);
        }

        let etag = self.cache.get_etag(&self.releases_url);
        let mut req = self
            .client
            .get(&self.releases_url)
            .header("User-Agent", "Glacier-Launcher");
        if let Some(etag) = &etag {
            req = req.header("If-None-Match", etag);
        }

        match req.send().await {
            Ok(resp) if resp.status() == reqwest::StatusCode::NOT_MODIFIED => {
                self.cache.touch(&self.releases_url);
                self.cache
                    .get_stale(&self.releases_url)
                    .map(|c| c.body)
                    .ok_or(UpdateError::NoReleases)
            }
            Ok(resp) => {
                let new_etag = resp
                    .headers()
                    .get(reqwest::header::ETAG)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let body = resp.error_for_status()?.text().await?;
                self.cache.store(&self.releases_url, body.clone(), new_etag);
                Ok(body)
            }
            Err(err) => {
                // Mirror the C# fallback: serve the last known-good body if a
                // fetch attempt fails outright (network error, rate limit, ...).
                if let Some(stale) = self.cache.get_stale(&self.releases_url) {
                    Ok(stale.body)
                } else {
                    Err(UpdateError::Request(err))
                }
            }
        }
    }

    /// Checks for a launcher update newer than `current_version`.
    ///
    /// Returns `Ok(None)` when there is no newer release (not an error) —
    /// mirrors the C# method returning `null` in that case.
    pub async fn check_launcher_update(
        &self,
        current_version: &str,
    ) -> Result<Option<LauncherUpdateInfo>> {
        let body = self.fetch_releases_json().await?;
        let releases: Vec<GitHubRelease> = serde_json::from_str(&body)?;
        Ok(pick_update(&releases, current_version))
    }

    /// Returns up to `max` recent (non-draft) releases, newest-tagged first
    /// as returned by GitHub, mirroring `GetRecentReleasesAsync`.
    pub async fn recent_releases(&self, max: usize) -> Result<Vec<LauncherRelease>> {
        let body = self.fetch_releases_json().await?;
        let releases: Vec<GitHubRelease> = serde_json::from_str(&body)?;
        Ok(recent_releases(&releases, max))
    }
}

/// Picks the best (newest, non-draft) release and its chosen asset, returning
/// `None` if nothing usable was found or the best release isn't newer than
/// `current_version`. Pure/testable port of the loop body in
/// `CheckLauncherUpdateAsync`.
pub fn pick_update(
    releases: &[GitHubRelease],
    current_version: &str,
) -> Option<LauncherUpdateInfo> {
    let mut best: Option<(&GitHubRelease, String)> = None;

    for release in releases {
        if release.draft {
            continue;
        }
        let tag = match &release.tag_name {
            Some(t) if !t.is_empty() => normalize_tag(t),
            _ => continue,
        };
        if tag.is_empty() {
            continue;
        }

        let is_better = match &best {
            None => true,
            Some((_, best_tag)) => is_newer_version(&tag, best_tag),
        };
        if is_better {
            best = Some((release, tag));
        }
    }

    let (release, tag) = best?;
    if !is_newer_version(&tag, current_version) {
        return None;
    }

    let asset = pick_asset(&release.assets)?;

    Some(LauncherUpdateInfo {
        tag,
        download_url: asset.browser_download_url.clone(),
        changelog: release.body.clone().unwrap_or_default(),
        asset_size: asset.size,
    })
}

/// Picks the update asset from a release's asset list: prefer the first
/// `.exe`, falling back to the first asset otherwise. Mirrors the loop in
/// `CheckLauncherUpdateAsync`.
pub fn pick_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset> {
    if let Some(exe) = assets
        .iter()
        .find(|a| a.name.to_ascii_lowercase().ends_with(".exe"))
    {
        return Some(exe);
    }
    assets.first()
}

/// Pure port of the loop in `GetRecentReleasesAsync`: non-draft releases,
/// in the order GitHub returned them, capped at `max`.
pub fn recent_releases(releases: &[GitHubRelease], max: usize) -> Vec<LauncherRelease> {
    releases
        .iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let tag = r.tag_name.clone().unwrap_or_default();
            if tag.is_empty() {
                return None;
            }
            Some(LauncherRelease {
                tag,
                published_at: r.published_at.clone().unwrap_or_default(),
                changelog: r.body.clone().unwrap_or_default(),
            })
        })
        .take(max)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    type AssetFixture<'a> = (&'a str, &'a str, u64);
    type ReleaseFixture<'a> = (&'a str, bool, &'a [AssetFixture<'a>]);

    fn fixture(tags_and_assets: &[ReleaseFixture]) -> Vec<GitHubRelease> {
        tags_and_assets
            .iter()
            .map(|(tag, draft, assets)| GitHubRelease {
                draft: *draft,
                tag_name: Some(tag.to_string()),
                body: Some(format!("changelog for {tag}")),
                published_at: Some("2026-01-01T00:00:00Z".to_string()),
                assets: assets
                    .iter()
                    .map(|(name, url, size)| GitHubAsset {
                        name: name.to_string(),
                        browser_download_url: url.to_string(),
                        size: *size,
                    })
                    .collect(),
            })
            .collect()
    }

    #[test]
    fn picks_newest_non_draft_with_newer_version() {
        let releases = fixture(&[
            ("v1.5.0", true, &[("Setup.exe", "http://x/1.5.0.exe", 100)]),
            ("v1.3.0", false, &[("Setup.exe", "http://x/1.3.0.exe", 90)]),
            ("v1.4.0", false, &[("Setup.exe", "http://x/1.4.0.exe", 95)]),
        ]);
        let update = pick_update(&releases, "1.2.0").expect("should find update");
        assert_eq!(update.tag, "1.4.0");
        assert_eq!(update.download_url, "http://x/1.4.0.exe");
        assert_eq!(update.asset_size, 95);
    }

    #[test]
    fn no_update_when_current_is_latest() {
        let releases = fixture(&[("v1.4.0", false, &[("Setup.exe", "http://x/1.4.0.exe", 95)])]);
        assert!(pick_update(&releases, "1.4.0").is_none());
        assert!(pick_update(&releases, "1.5.0").is_none());
    }

    #[test]
    fn skips_releases_with_no_usable_asset() {
        let releases = fixture(&[("v1.4.0", false, &[])]);
        assert!(pick_update(&releases, "1.0.0").is_none());
    }

    #[test]
    fn asset_prefers_exe_over_other_files() {
        let assets = vec![
            GitHubAsset {
                name: "checksums.txt".into(),
                browser_download_url: "http://x/sums".into(),
                size: 10,
            },
            GitHubAsset {
                name: "Setup.exe".into(),
                browser_download_url: "http://x/setup".into(),
                size: 200,
            },
        ];
        let picked = pick_asset(&assets).expect("asset");
        assert_eq!(picked.name, "Setup.exe");
    }

    #[test]
    fn asset_falls_back_to_first_when_no_exe() {
        let assets = vec![
            GitHubAsset {
                name: "checksums.txt".into(),
                browser_download_url: "http://x/sums".into(),
                size: 10,
            },
            GitHubAsset {
                name: "archive.zip".into(),
                browser_download_url: "http://x/zip".into(),
                size: 20,
            },
        ];
        let picked = pick_asset(&assets).expect("asset");
        assert_eq!(picked.name, "checksums.txt");
    }

    #[test]
    fn recent_releases_skips_drafts_and_caps_at_max() {
        let releases = fixture(&[
            ("v1.0.0", false, &[]),
            ("v1.1.0", true, &[]),
            ("v1.2.0", false, &[]),
            ("v1.3.0", false, &[]),
        ]);
        let recent = recent_releases(&releases, 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].tag, "v1.0.0");
        assert_eq!(recent[1].tag, "v1.2.0");
    }
}
