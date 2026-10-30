//! Ported from `LeviLaminaModsService.cs`: parses the `lipr` community mod
//! registry (a GitHub-hosted `index.json`) and resolves each mod's latest
//! release asset from GitHub.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ClientsError, Result};
use crate::github::{self, GitHubRelease};

pub const REGISTRY_URL: &str = "https://raw.githubusercontent.com/LiteLDev/lipr/main/index.json";

/// Ported from `LeviLaminaModsService.LeviLaminaMod`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeviLaminaMod {
    pub repo_owner: String,
    pub repo_name: String,
    pub name: String,
    pub description: String,
    pub avatar_url: String,
    pub latest_version: String,
    pub stars: i64,
}

/// Parses the `lipr` `index.json` registry body into the list of
/// LeviLamina mods, mirroring `LeviLaminaModsService.LoadAllAsync`:
/// - only `packages` entries keyed `github.com/{owner}/{repo}`
/// - only entries tagged with both "levilamina" and "mod" (substring,
///   case-insensitive)
/// - the "latest version" is the last entry of the first non-empty
///   `variants[*].versions` array encountered
/// - sorted by star count, descending
pub fn parse_registry(body: &str) -> Result<Vec<LeviLaminaMod>> {
    let root: Value = serde_json::from_str(body).map_err(ClientsError::from)?;

    let mut mods = Vec::new();
    let Some(packages) = root.get("packages").and_then(Value::as_object) else {
        return Ok(mods);
    };

    for (key, pkg) in packages {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() < 3 || !parts[0].eq_ignore_ascii_case("github.com") {
            continue;
        }
        let owner = parts[1].to_string();
        let repo = parts[2].to_string();

        let Some(info) = pkg.get("info") else {
            continue;
        };

        let tags: Vec<String> = info
            .get("tags")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|t| t.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let is_levilamina_mod = tags
            .iter()
            .any(|t| t.to_ascii_lowercase().contains("levilamina"))
            && tags.iter().any(|t| t.to_ascii_lowercase().contains("mod"));
        if !is_levilamina_mod {
            continue;
        }

        let name = info
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(&repo)
            .to_string();
        let description = info
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let avatar_url = info
            .get("avatar_url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let stars = pkg
            .get("stargazer_count")
            .and_then(Value::as_i64)
            .unwrap_or(0);

        let mut latest_version = String::new();
        if let Some(variants) = pkg.get("variants").and_then(Value::as_object) {
            for variant in variants.values() {
                if let Some(versions) = variant.get("versions").and_then(Value::as_array) {
                    if let Some(last) = versions.last().and_then(Value::as_str) {
                        if !last.is_empty() {
                            latest_version = last.to_string();
                            break;
                        }
                    }
                }
            }
        }
        if latest_version.is_empty() {
            continue;
        }

        mods.push(LeviLaminaMod {
            repo_owner: owner,
            repo_name: repo,
            name,
            description,
            avatar_url,
            latest_version,
            stars,
        });
    }

    mods.sort_by_key(|m| std::cmp::Reverse(m.stars));
    Ok(mods)
}

/// Mirrors `LeviLaminaModsService.SearchAsync`: a case-insensitive substring
/// match against name or description.
pub fn search<'a>(all: &'a [LeviLaminaMod], query: &str) -> Vec<&'a LeviLaminaMod> {
    if query.trim().is_empty() {
        return all.iter().collect();
    }
    let q = query.to_ascii_lowercase();
    all.iter()
        .filter(|m| {
            m.name.to_ascii_lowercase().contains(&q)
                || m.description.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

/// Sanitizes a repo name into a filesystem-safe folder name, replacing any
/// character invalid in a Windows filename. Mirrors
/// `LeviLaminaModsService.SanitizeFolderName`.
pub fn sanitize_folder_name(name: &str) -> String {
    const INVALID: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut sanitized: String = name
        .chars()
        .map(|c| {
            if INVALID.contains(&c) || (c as u32) < 32 {
                '_'
            } else {
                c
            }
        })
        .collect();
    sanitized = sanitized.trim().to_string();
    sanitized
}

/// The resolved download target for a mod's latest release, mirroring
/// `LeviLaminaModsService.DownloadAndInstallAsync`'s asset resolution
/// (excluding the actual download/extract step, which is out of scope).
#[derive(Debug, Clone)]
pub struct ResolvedModAsset {
    pub asset_url: String,
    pub asset_name: String,
    pub tag_used: String,
}

/// Fetches release-by-tag JSON for each candidate tag (as computed by
/// `github::tag_candidates`) until one resolves to a downloadable asset.
pub async fn resolve_latest_asset(
    http: &Client,
    repo_owner: &str,
    repo_name: &str,
    latest_version: &str,
) -> Result<ResolvedModAsset> {
    let mut last_err: Option<ClientsError> = None;

    for tag in github::tag_candidates(latest_version) {
        let url =
            format!("https://api.github.com/repos/{repo_owner}/{repo_name}/releases/tags/{tag}");
        let body = match github::get_text_cached(http, &url).await {
            Ok(b) => b,
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        };

        let release: GitHubRelease = match github::parse_release(&body) {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        };
        if release.is_not_found() {
            continue;
        }

        if let Some(asset) = github::pick_mod_asset(&release) {
            return Ok(ResolvedModAsset {
                asset_url: asset.browser_download_url.clone(),
                asset_name: asset.name.clone(),
                tag_used: tag,
            });
        }
    }

    Err(last_err.unwrap_or_else(|| {
        ClientsError::NoAssetFound(format!(
            "No downloadable release asset found for {repo_owner}/{repo_name} {latest_version}."
        ))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY_JSON: &str = r#"
    {
        "packages": {
            "github.com/Owner1/CoolMod": {
                "stargazer_count": 42,
                "info": {
                    "name": "Cool Mod",
                    "description": "A cool LeviLamina mod",
                    "avatar_url": "https://example.com/avatar.png",
                    "tags": ["levilamina", "mod", "utility"]
                },
                "variants": {
                    "default": { "versions": ["1.0.0", "1.1.0", "1.2.0"] }
                }
            },
            "github.com/Owner2/NotAMod": {
                "stargazer_count": 100,
                "info": {
                    "name": "Not A Mod",
                    "description": "Some other tool",
                    "tags": ["levilamina", "tool"]
                },
                "variants": {
                    "default": { "versions": ["1.0.0"] }
                }
            },
            "github.com/Owner3/NoVersions": {
                "stargazer_count": 5,
                "info": {
                    "name": "No Versions",
                    "tags": ["levilamina", "mod"]
                },
                "variants": {}
            },
            "npm/some-package": {
                "info": { "tags": ["levilamina", "mod"] }
            }
        }
    }"#;

    #[test]
    fn parses_registry_filters_and_sorts() {
        let mods = parse_registry(REGISTRY_JSON).unwrap();
        assert_eq!(mods.len(), 1);
        let m = &mods[0];
        assert_eq!(m.repo_owner, "Owner1");
        assert_eq!(m.repo_name, "CoolMod");
        assert_eq!(m.name, "Cool Mod");
        assert_eq!(m.latest_version, "1.2.0");
        assert_eq!(m.stars, 42);
    }

    #[test]
    fn search_matches_name_or_description_case_insensitively() {
        let mods = vec![LeviLaminaMod {
            repo_owner: "o".into(),
            repo_name: "r".into(),
            name: "Fancy Overlay".into(),
            description: "Adds a cool HUD".into(),
            avatar_url: "".into(),
            latest_version: "1.0".into(),
            stars: 0,
        }];
        assert_eq!(search(&mods, "fancy").len(), 1);
        assert_eq!(search(&mods, "HUD").len(), 1);
        assert_eq!(search(&mods, "nope").len(), 0);
        assert_eq!(search(&mods, "").len(), 1);
    }

    #[test]
    fn sanitize_folder_name_replaces_invalid_chars() {
        assert_eq!(sanitize_folder_name("weird:name/here"), "weird_name_here");
        assert_eq!(sanitize_folder_name("  spaced  "), "spaced");
    }

    #[test]
    fn empty_packages_object_returns_empty_list() {
        let mods = parse_registry(r#"{"packages":{}}"#).unwrap();
        assert!(mods.is_empty());
    }
}
