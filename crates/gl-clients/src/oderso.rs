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
        .error_for_status()?
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

    // A plain descending byte-wise sort put "1.26.X" ahead of "1.26.4X"
    // (`'X'` > `'4'` as a byte), scattering the list into groups like
    // "1.26.X, 1.26.4X, 1.26.3X, ..." instead of a clean newest-first run.
    // `version_cmp` treats runs of digits as numbers so "4X" properly
    // outranks "3X", which outranks a bare "X".
    out.sort_by(|a, b| version_cmp(&b.name, &a.name));
    Ok(out)
}

/// Natural/version-aware string comparison: runs of ASCII digits compare
/// numerically (so "10" > "9"), everything else compares byte-wise. Where
/// one side has a digit run and the other doesn't at the same position, the
/// digit run wins — a named build like "4X" is treated as newer than a bare
/// "X" tag. Ad hoc (this repo's names aren't semver), but much closer to
/// what "newest first" means here than either a plain string sort or no
/// sort at all.
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let (xd, yd) = (x.is_ascii_digit(), y.is_ascii_digit());
                if xd && yd {
                    let mut an: u64 = 0;
                    while let Some(&c) = ai.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        an = an
                            .saturating_mul(10)
                            .saturating_add(c.to_digit(10).unwrap() as u64);
                        ai.next();
                    }
                    let mut bn: u64 = 0;
                    while let Some(&c) = bi.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        bn = bn
                            .saturating_mul(10)
                            .saturating_add(c.to_digit(10).unwrap() as u64);
                        bi.next();
                    }
                    match an.cmp(&bn) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else if xd != yd {
                    return if xd {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    };
                } else {
                    ai.next();
                    bi.next();
                    match x.cmp(&y) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

/// The download URL for a listed entry, matching `RawBaseUrl + Uri.EscapeDataString(name)`.
pub fn raw_url(entry: &DllEntry) -> String {
    format!("{RAW_BASE_URL}{}", urlencoding_lite(&entry.name))
}

fn urlencoding_lite(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
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
    fn sorts_dotted_version_names_numerically_not_byte_wise() {
        const REAL_WORLD: &str = r#"[
            {"type":"file","name":"1.21.X.dll","sha":"","size":1},
            {"type":"file","name":"1.21.8X.dll","sha":"","size":1},
            {"type":"file","name":"1.21.9X.dll","sha":"","size":1},
            {"type":"file","name":"1.26.X.dll","sha":"","size":1},
            {"type":"file","name":"1.26.1X.dll","sha":"","size":1},
            {"type":"file","name":"1.26.2X.dll","sha":"","size":1},
            {"type":"file","name":"1.26.3X.dll","sha":"","size":1},
            {"type":"file","name":"1.26.4X.dll","sha":"","size":1}
        ]"#;
        let entries = parse_dll_list(REAL_WORLD).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        // Newest first: highest numbered build within each dotted prefix
        // before its bare "X" tag, byte-wise sort would put "1.26.X.dll"
        // ahead of every numbered "1.26.<n>X.dll" build.
        assert_eq!(
            names,
            vec![
                "1.26.4X.dll",
                "1.26.3X.dll",
                "1.26.2X.dll",
                "1.26.1X.dll",
                "1.26.X.dll",
                "1.21.9X.dll",
                "1.21.8X.dll",
                "1.21.X.dll",
            ]
        );
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
