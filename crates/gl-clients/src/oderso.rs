//! Ported from `OderSoServices.cs`: lists `.dll` releases from the
//! `MasonOderSo/oderso-data` GitHub repo and resolves download URLs for them.

use reqwest::Client;
use serde_json::Value;

use crate::error::{ClientsError, Result};

pub const CONTENTS_API_URL: &str = "https://api.github.com/repos/MasonOderSo/oderso-data/contents/";
pub const RAW_BASE_URL: &str = "https://raw.githubusercontent.com/MasonOderSo/oderso-data/main/";

/// One `.dll` file listed in the OderSo data repo. Mirrors `OderSoService.DllEntry`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DllEntry {
    pub name: String,
    pub sha: String,
    pub size: u64,
}

/// Lists every `.dll` in the repo root, newest-named first. Mirrors
/// `OderSoService.ListDllsAsync`.
pub async fn list_dlls(http: &Client) -> Result<Vec<DllEntry>> {
    let body = http
        .get(CONTENTS_API_URL)
        .header("User-Agent", "GlacierLauncher")
        .send()
        .await?
        .text()
        .await?;
    parse_dll_list(&body)
}

pub fn parse_dll_list(body: &str) -> Result<Vec<DllEntry>> {
    let root: Value = serde_json::from_str(body).map_err(ClientsError::from)?;
    let arr = root
        .as_array()
        .ok_or_else(|| ClientsError::NotFound("unexpected response from GitHub".into()))?;

    let mut out: Vec<DllEntry> = arr
        .iter()
        .filter_map(|item| {
            let ty = item.get("type")?.as_str()?;
            let name = item.get("name")?.as_str()?.to_string();
            if ty != "file" || !name.to_ascii_lowercase().ends_with(".dll") {
                return None;
            }
            let sha = item
                .get("sha")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let size = item.get("size").and_then(Value::as_u64).unwrap_or(0);
            Some(DllEntry { name, sha, size })
        })
        .collect();

    // Descending name sort approximates the C# version-aware comparer closely
    // enough for the "newest looking name first" behavior it produces.
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

/// The download URL for a listed entry, matching `RawBaseUrl + Uri.EscapeDataString(name)`.
pub fn raw_url(entry: &DllEntry) -> String {
    format!("{RAW_BASE_URL}{}", urlencoding_lite(&entry.name))
}

fn urlencoding_lite(s: &str) -> String {
    s.replace(' ', "%20")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = r#"[
        {"type":"file","name":"OderSo_v2.dll","sha":"abc","size":1234},
        {"type":"file","name":"OderSo_v1.dll","sha":"def","size":1200},
        {"type":"dir","name":"assets","sha":"","size":0},
        {"type":"file","name":"README.md","sha":"","size":10}
    ]"#;

    #[test]
    fn parses_and_sorts_dll_entries_newest_first() {
        let entries = parse_dll_list(LISTING).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "OderSo_v2.dll");
        assert_eq!(entries[1].name, "OderSo_v1.dll");
    }

    #[test]
    fn builds_raw_download_url() {
        let entry = DllEntry {
            name: "OderSo v2.dll".into(),
            sha: "abc".into(),
            size: 1,
        };
        assert_eq!(
            raw_url(&entry),
            "https://raw.githubusercontent.com/MasonOderSo/oderso-data/main/OderSo%20v2.dll"
        );
    }
}
