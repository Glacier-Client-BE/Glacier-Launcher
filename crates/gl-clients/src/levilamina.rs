//! Ported from `LeviLaminaService.cs`: fetches LeviLamina's latest GitHub
//! release and finds the loader DLL inside an already-extracted install.
//!
//! Actual downloading/extraction of the release archive is out of scope
//! here (see the crate-level `// TODO: wire to gl-download` notes) — this
//! module only covers the catalog/metadata side: which release is latest,
//! which asset to fetch, and whether an install is up to date.

use std::path::{Path, PathBuf};

use reqwest::Client;

use crate::error::Result;
use crate::github::{self, GitHubRelease};

pub const RELEASES_API_URL: &str =
    "https://api.github.com/repos/LiteLDev/LeviLamina/releases/latest";

/// The resolved download target for the latest LeviLamina release.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeviLaminaRelease {
    pub tag: String,
    pub zip_url: String,
}

/// Fetches and parses the latest LeviLamina release from GitHub, picking
/// the Windows zip asset. Mirrors `LeviLaminaService.FetchLatestReleaseAsync`.
///
/// Responses are cached briefly by `github::get_text_cached`, replacing the
/// C# `GitHubApiCache` for this lookup.
pub async fn fetch_latest_release(http: &Client) -> Result<LeviLaminaRelease> {
    let body = github::get_text_cached(http, RELEASES_API_URL).await?;
    parse_latest_release(&body)
}

/// Parses a release JSON body into a `LeviLaminaRelease`, without a network
/// round trip. Split out from `fetch_latest_release` so the asset-picking
/// logic is unit-testable against fixture strings.
pub fn parse_latest_release(body: &str) -> Result<LeviLaminaRelease> {
    let release: GitHubRelease = github::parse_release(body)?;
    let zip_url = github::pick_windows_zip_asset(&release)
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_default();
    Ok(LeviLaminaRelease {
        tag: release.tag_name,
        zip_url,
    })
}

/// Mirrors `LeviLaminaService.IsUpToDateAsync`'s comparison: a locally
/// stored `.version` tag against the latest release tag, case-insensitive.
pub fn is_up_to_date(local_tag: &str, remote_tag: &str) -> bool {
    local_tag.eq_ignore_ascii_case(remote_tag)
}

/// Finds the loader DLL inside an extracted LeviLamina install directory.
/// Mirrors `LeviLaminaService.FindLoaderDll`: prefers a filename containing
/// "Lamina", then one containing "Loader", then falls back to the sole DLL
/// if there's exactly one.
pub fn find_loader_dll(levilamina_dir: &Path) -> Option<PathBuf> {
    if !levilamina_dir.is_dir() {
        return None;
    }

    let dlls = collect_dlls(levilamina_dir);
    if dlls.is_empty() {
        return None;
    }

    dlls.iter()
        .find(|p| file_name_contains(p, "lamina"))
        .or_else(|| dlls.iter().find(|p| file_name_contains(p, "loader")))
        .cloned()
        .or_else(|| {
            if dlls.len() == 1 {
                Some(dlls[0].clone())
            } else {
                None
            }
        })
}

fn file_name_contains(path: &Path, needle: &str) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_ascii_lowercase().contains(needle))
        .unwrap_or(false)
}

fn collect_dlls(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(collect_dlls(&path));
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("dll"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE_JSON: &str = r#"
    {
        "tag_name": "v1.0.5",
        "assets": [
            { "name": "LeviLamina-Windows-Release.zip", "browser_download_url": "https://example.com/win.zip" },
            { "name": "LeviLamina-Linux.tar.gz", "browser_download_url": "https://example.com/linux.tar.gz" }
        ]
    }"#;

    #[test]
    fn parses_release_and_picks_windows_zip() {
        let release = parse_latest_release(RELEASE_JSON).unwrap();
        assert_eq!(release.tag, "v1.0.5");
        assert_eq!(release.zip_url, "https://example.com/win.zip");
    }

    #[test]
    fn up_to_date_is_case_insensitive() {
        assert!(is_up_to_date("v1.0.5", "V1.0.5"));
        assert!(!is_up_to_date("v1.0.4", "v1.0.5"));
        assert!(!is_up_to_date("", "v1.0.5"));
    }

    #[test]
    fn finds_loader_dll_by_name_preference() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("other.dll"), b"").unwrap();
        std::fs::write(dir.path().join("LeviLaminaLoader.dll"), b"").unwrap();

        let found = find_loader_dll(dir.path()).unwrap();
        assert_eq!(found.file_name().unwrap(), "LeviLaminaLoader.dll");
    }

    #[test]
    fn falls_back_to_sole_dll() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mystery.dll"), b"").unwrap();

        let found = find_loader_dll(dir.path()).unwrap();
        assert_eq!(found.file_name().unwrap(), "mystery.dll");
    }

    #[test]
    fn returns_none_when_no_dlls() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_loader_dll(dir.path()).is_none());
    }

    #[test]
    fn returns_none_for_missing_directory() {
        assert!(find_loader_dll(Path::new("does/not/exist")).is_none());
    }
}
