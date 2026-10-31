//! The community version-ID database — port of
//! `VanillaVersionService.FetchFromCommunityDbAsync`.
//!
//! This is a plain-text list of every `<update-id> <package-name>` pair the
//! maintainer has ever observed from Windows Update, so it fills in builds
//! that WU no longer currently offers (WU's `SyncUpdates` only returns
//! *currently offered* updates — the live list in `wu.rs` shrinks as
//! Microsoft stops offering older builds). Not a Microsoft endpoint.

use crate::error::{BedrockError, Result};
use crate::wu::{detect_channel, parse_version_from_package, WuVersion};

pub const VERSION_DB_URL: &str =
    "https://raw.githubusercontent.com/ddf8196/mc-w10-versiondb-auto-update/master/versions.txt";

/// Fetches and parses the community version database.
pub async fn list_versions(client: &reqwest::Client) -> Result<Vec<WuVersion>> {
    let resp = client
        .get(VERSION_DB_URL)
        .send()
        .await
        .map_err(|e| BedrockError::Wu(format!("community DB request failed: {e}")))?;
    if !resp.status().is_success() {
        return Err(BedrockError::Wu(format!(
            "community DB returned HTTP {}",
            resp.status()
        )));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| BedrockError::Wu(format!("community DB body read failed: {e}")))?;
    Ok(parse_version_db(&text))
}

/// Parses `versions.txt`'s `<update-id> <package-name>` lines, without a
/// network round trip. Mirrors `FetchFromCommunityDbAsync`: skips the
/// section headers (`Releases`/`Betas`/`Previews`), keeps only x64 non-EAppx
/// entries, and dedupes by `version|channel` (first-seen wins, matching the
/// C# `if (!seen.ContainsKey(key))`).
pub fn parse_version_db(text: &str) -> Vec<WuVersion> {
    use std::collections::HashMap;

    let mut seen: HashMap<String, WuVersion> = HashMap::new();

    for raw_line in text.split('\n') {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("releases")
            || lower.starts_with("betas")
            || lower.starts_with("previews")
        {
            continue;
        }

        let Some(space_idx) = line.find(' ') else {
            continue;
        };
        let update_id = &line[..space_idx];
        let package_name = line[space_idx + 1..].trim_start();

        if !package_name.to_ascii_lowercase().contains("x64") {
            continue;
        }
        if package_name.to_ascii_lowercase().contains(".eappx") {
            continue;
        }

        let Some(version) = parse_version_from_package(package_name) else {
            continue;
        };
        let channel = detect_channel(package_name);
        let key = format!("{}|{}", version.to_lowercase(), channel.to_lowercase());

        seen.entry(key).or_insert_with(|| WuVersion {
            version,
            update_id: update_id.to_string(),
            package_name: package_name.to_string(),
            size_bytes: 0,
            channel: channel.to_string(),
            ..Default::default()
        });
    }

    let mut out: Vec<WuVersion> = seen.into_values().collect();
    out.sort_by_key(|v| std::cmp::Reverse(crate::wu::version_key(&v.version)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
Releases
16123456 Microsoft.MinecraftUWP_1.20.0.0_x64__8wekyb3d8bbwe.appx
16123457 Microsoft.MinecraftUWP_1.20.0.0_x64__8wekyb3d8bbwe.EAppx
16123458 Microsoft.MinecraftUWP_1.19.0.0_arm__8wekyb3d8bbwe.appx
Previews
16123459 Microsoft.MinecraftWindowsBeta_1.20.10.0_x64__8wekyb3d8bbwe.appx
";

    #[test]
    fn skips_section_headers_eappx_and_non_x64() {
        let versions = parse_version_db(FIXTURE);
        // Only the two legitimate x64, non-EAppx lines should survive.
        assert_eq!(versions.len(), 2);
        assert!(versions
            .iter()
            .any(|v| v.version == "1.20.0.0" && v.channel == "Release"));
        assert!(versions
            .iter()
            .any(|v| v.version == "1.20.10.0" && v.channel == "Preview"));
    }

    #[test]
    fn first_seen_wins_on_duplicate_key() {
        let text = "\
16000001 Microsoft.MinecraftUWP_1.20.0.0_x64__8wekyb3d8bbwe.appx
16000002 Microsoft.MinecraftUWP_1.20.0.0_x64__8wekyb3d8bbwe.appx
";
        let versions = parse_version_db(text);
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].update_id, "16000001");
    }

    #[test]
    fn sorts_newest_first() {
        let text = "\
1 Microsoft.MinecraftUWP_1.19.0.0_x64__8wekyb3d8bbwe.appx
2 Microsoft.MinecraftUWP_1.21.0.0_x64__8wekyb3d8bbwe.appx
";
        let versions = parse_version_db(text);
        assert_eq!(versions[0].version, "1.21.0.0");
        assert_eq!(versions[1].version, "1.19.0.0");
    }
}
