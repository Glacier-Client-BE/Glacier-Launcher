//! Ported from `ClientLibraryService.cs`: parses the `paths.json` file-list
//! index published by the `Indigo-Tools/ClientLibrary` GitHub repo into a
//! catalog of resource-pack style "client" downloads (Latite, etc.).
//!
//! Actual download + `.mcpack` extraction (via `System.IO.Compression` in
//! the C# original) is out of scope here — see the crate-level
//! `// TODO: wire to gl-download` notes. This module covers parsing the
//! index and building the raw/media download URLs.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::error::{ClientsError, Result};
use crate::models::{ClientLibraryDownload, ClientLibraryPack};

pub const OWNER: &str = "Indigo-Tools";
pub const REPO: &str = "ClientLibrary";
pub const BRANCH: &str = "main";

pub fn index_url() -> String {
    format!("https://raw.githubusercontent.com/{OWNER}/{REPO}/{BRANCH}/paths.json")
}

pub fn site_url() -> String {
    format!("https://github.com/{OWNER}/{REPO}")
}

fn encode_path(path: &str) -> String {
    path.split('/')
        .map(urlencoding_escape)
        .collect::<Vec<_>>()
        .join("/")
}

/// Minimal path-segment percent-encoding, matching the set of characters
/// `Uri.EscapeDataString` escapes for typical repo path segments (letters,
/// digits, and `-_.~` pass through unescaped).
fn urlencoding_escape(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        let c = byte as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub fn raw_url(repo_path: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/{OWNER}/{REPO}/{BRANCH}/{}",
        encode_path(repo_path)
    )
}

/// The `.mcpack` binaries are Git LFS pointers on raw.githubusercontent.com;
/// the actual bytes only come from the media host. Mirrors
/// `ClientLibraryService.MediaUrl`.
pub fn media_url(repo_path: &str) -> String {
    format!(
        "https://media.githubusercontent.com/media/{OWNER}/{REPO}/{BRANCH}/{}",
        encode_path(repo_path)
    )
}

#[derive(Debug, Deserialize)]
struct PathEntry {
    #[serde(default)]
    path: String,
    #[serde(default)]
    content: Value,
}

/// Parses `paths.json` into a list of `ClientLibraryPack`s, grouped by
/// `{version}/{name}`. Mirrors `ClientLibraryService.GetPacksAsync`'s
/// parsing/grouping logic (the caching and `LastError` state are left to
/// the caller).
pub fn parse_index(body: &str) -> Result<Vec<ClientLibraryPack>> {
    let entries: Vec<PathEntry> = serde_json::from_str(body).map_err(ClientsError::from)?;

    // Preserve first-seen order per key, like the C# `Dictionary` does.
    let mut order: Vec<String> = Vec::new();
    let mut packs: BTreeMap<String, ClientLibraryPack> = BTreeMap::new();

    for entry in entries {
        let parts: Vec<&str> = entry.path.split('/').collect();
        if parts.len() < 3 {
            continue;
        }
        if !parts[0].eq_ignore_ascii_case("ClientLibrary") {
            continue;
        }

        let version = parts[1].replace('_', ".");
        let name = parts[2].to_string();
        let key = format!("{version}/{name}");

        if !packs.contains_key(&key) {
            order.push(key.clone());
            packs.insert(
                key.clone(),
                ClientLibraryPack {
                    version: version.clone(),
                    name: name.clone(),
                    ..Default::default()
                },
            );
        }
        let pack = packs.get_mut(&key).expect("just inserted");

        let file = parts[parts.len() - 1];
        let lower = file.to_ascii_lowercase();

        if lower == "pack_icon.png" {
            pack.icon_url = raw_url(&entry.path);
        } else if lower == "pack_banner.png" {
            pack.banner_url = raw_url(&entry.path);
        } else if lower == "description.txt" {
            if let Some(s) = entry.content.as_str() {
                pack.description = s.to_string();
            }
        } else if lower.ends_with(".mcpack")
            || lower.ends_with(".mcaddon")
            || lower.ends_with(".zip")
        {
            let label = file
                .rsplit_once('.')
                .map(|(stem, _)| stem.to_string())
                .unwrap_or_else(|| file.to_string());
            pack.downloads.push(ClientLibraryDownload {
                label,
                file_name: file.to_string(),
                repo_path: entry.path.clone(),
                is_extension: entry.path.to_ascii_lowercase().contains("/extensions/"),
            });
        }
    }

    let mut result: Vec<ClientLibraryPack> = order
        .into_iter()
        .filter_map(|key| packs.remove(&key))
        .filter(|p| !p.downloads.is_empty())
        .collect();

    // Newest version first, then name ascending — folder names like "1_21"
    // vs "1_9" sort wrong as strings, so compare numerically per segment.
    result.sort_by(|a, b| {
        version_key(&b.version)
            .cmp(&version_key(&a.version))
            .then_with(|| {
                a.name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase())
            })
    });

    Ok(result)
}

/// Parses a dotted version string into comparable numeric components,
/// mirroring `ClientLibraryService.ParseVersion`'s numeric (not
/// lexicographic) ordering. Unparseable segments sort as 0.
fn version_key(v: &str) -> Vec<u32> {
    v.split('.')
        .map(|p| p.parse::<u32>().unwrap_or(0))
        .collect()
}

/// Distinct versions present in a pack list, newest first. Mirrors
/// `ClientLibraryService.Versions`.
pub fn versions(packs: &[ClientLibraryPack]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for p in packs {
        if !seen.contains(&p.version) {
            seen.push(p.version.clone());
        }
    }
    seen.sort_by_key(|v| std::cmp::Reverse(version_key(v)));
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATHS_JSON: &str = r#"
    [
        { "path": "ClientLibrary/1_21/Latite/pack_icon.png", "content": null },
        { "path": "ClientLibrary/1_21/Latite/description.txt", "content": "The Latite client pack" },
        { "path": "ClientLibrary/1_21/Latite/Latite.mcpack", "content": null },
        { "path": "ClientLibrary/1_21/Latite/Extensions/LatiteExt.mcpack", "content": null },
        { "path": "ClientLibrary/1_9/OldPack/OldPack.mcpack", "content": null },
        { "path": "ClientLibrary/1_21/EmptyPack/readme.md", "content": null },
        { "path": "not-a-client-library/foo/bar.mcpack", "content": null }
    ]"#;

    #[test]
    fn parses_and_groups_by_version_and_name() {
        let packs = parse_index(PATHS_JSON).unwrap();
        // EmptyPack has no downloads and should be dropped; unrelated root skipped.
        assert_eq!(packs.len(), 2);

        // 1.21 before 1.9 (numeric compare, not lexicographic).
        assert_eq!(packs[0].version, "1.21");
        assert_eq!(packs[1].version, "1.9");

        let latite = &packs[0];
        assert_eq!(latite.name, "Latite");
        assert_eq!(latite.description, "The Latite client pack");
        assert!(latite.icon_url.contains("pack_icon.png"));
        assert_eq!(latite.downloads.len(), 2);
        assert_eq!(latite.main_downloads().len(), 1);
        assert_eq!(latite.extensions().len(), 1);
    }

    #[test]
    fn versions_are_sorted_numerically_descending() {
        let packs = parse_index(PATHS_JSON).unwrap();
        assert_eq!(
            versions(&packs),
            vec!["1.21".to_string(), "1.9".to_string()]
        );
    }

    #[test]
    fn media_and_raw_urls_encode_path_segments() {
        let path = "ClientLibrary/1_21/Latite/Latite.mcpack";
        assert_eq!(
            raw_url(path),
            "https://raw.githubusercontent.com/Indigo-Tools/ClientLibrary/main/ClientLibrary/1_21/Latite/Latite.mcpack"
        );
        assert_eq!(
            media_url(path),
            "https://media.githubusercontent.com/media/Indigo-Tools/ClientLibrary/main/ClientLibrary/1_21/Latite/Latite.mcpack"
        );
    }

    #[test]
    fn empty_array_yields_no_packs() {
        assert!(parse_index("[]").unwrap().is_empty());
    }
}
