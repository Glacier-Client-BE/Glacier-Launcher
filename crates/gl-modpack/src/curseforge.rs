//! CurseForge API client (`api.curseforge.com`), mirroring the C#
//! `CurseForgeService`.

use serde_json::Value;

use crate::error::{ModpackError, Result};
use crate::models::{CfAddon, CfFile, CfSearchResult};

const BASE_URL: &str = "https://api.curseforge.com";

pub const GAME_ID_BEDROCK: i32 = 78022;
pub const GAME_ID_JAVA: i32 = 432;

pub const BEDROCK_CLASS_ADDONS: i32 = 4984;
pub const BEDROCK_CLASS_MAPS: i32 = 6913;
pub const BEDROCK_CLASS_SKINS: i32 = 6925;
pub const BEDROCK_CLASS_TEXTURE_PACKS: i32 = 6929;
pub const BEDROCK_CLASS_SCRIPTS: i32 = 6940;

pub const JAVA_CLASS_MODS: i32 = 6;
pub const JAVA_CLASS_MODPACKS: i32 = 4471;
pub const JAVA_CLASS_RESOURCE_PACKS: i32 = 12;
pub const JAVA_CLASS_WORLDS: i32 = 17;
pub const JAVA_CLASS_SHADER_PACKS: i32 = 6552;

/// Thin client over the CurseForge REST API. The API key is passed via the
/// `x-api-key` header, as required by CurseForge's Eternal / Core API.
pub struct CurseForgeClient {
    http: reqwest::Client,
    api_key: String,
}

impl CurseForgeClient {
    pub fn new(http: reqwest::Client, api_key: impl Into<String>) -> Self {
        Self {
            http,
            api_key: api_key.into(),
        }
    }

    fn ensure_key(&self) -> Result<&str> {
        if self.api_key.trim().is_empty() {
            return Err(ModpackError::MissingApiKey);
        }
        Ok(&self.api_key)
    }

    async fn get_json(&self, url: &str) -> Result<Value> {
        let key = self.ensure_key()?;
        let resp = self
            .http
            .get(url)
            .header("x-api-key", key)
            .header("Accept", "application/json")
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(ModpackError::Status {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(resp.json::<Value>().await?)
    }

    /// Searches for addons on CurseForge. `class_id` restricts to a specific
    /// content category (mods/modpacks/resource packs/etc); pass `None` for
    /// no filter.
    pub async fn search(
        &self,
        game_id: i32,
        class_id: Option<i32>,
        query: &str,
        page_size: u32,
        page: u32,
    ) -> Result<CfSearchResult> {
        let mut url = format!(
            "{BASE_URL}/v1/mods/search?gameId={game_id}&pageSize={page_size}&index={}&sortField=2&sortOrder=desc",
            page * page_size
        );
        if let Some(cid) = class_id {
            url.push_str(&format!("&classId={cid}"));
        }
        if !query.trim().is_empty() {
            url.push_str(&format!("&searchFilter={}", urlencoding_encode(query)));
        }

        let json = self.get_json(&url).await?;
        let addons = json
            .get("data")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().map(parse_addon).collect())
            .unwrap_or_default();
        let total = json
            .get("pagination")
            .and_then(|p| p.get("totalCount"))
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;

        Ok(CfSearchResult {
            addons,
            total_count: total,
        })
    }

    /// Lists downloadable files for a given mod/addon id.
    pub async fn get_files(&self, mod_id: i32) -> Result<Vec<CfFile>> {
        let url = format!("{BASE_URL}/v1/mods/{mod_id}/files?pageSize=10");
        let json = self.get_json(&url).await?;
        let files = json
            .get("data")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(parse_file).collect())
            .unwrap_or_default();
        Ok(files)
    }

    /// Resolves a `(project_id, file_id)` pair — as found in a CurseForge
    /// modpack `manifest.json` — to a concrete downloadable file. Falls back
    /// to CurseForge's predictable CDN URL scheme when the API doesn't
    /// return a `downloadUrl` (common for "third party" or mature-content
    /// addons that disable direct API downloads).
    pub async fn resolve_manifest_file(
        &self,
        project_id: i32,
        file_id: i32,
    ) -> Result<Option<CfFile>> {
        let url = format!("{BASE_URL}/v1/mods/{project_id}/files/{file_id}");
        let json = self.get_json(&url).await?;
        let Some(item) = json.get("data") else {
            return Ok(None);
        };

        let file_name = item
            .get("fileName")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let length = item.get("fileLength").and_then(Value::as_i64).unwrap_or(0);
        let mut download_url = item
            .get("downloadUrl")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        if download_url.is_empty() && file_id > 0 && !file_name.is_empty() {
            let id_str = file_id.to_string();
            let (a, b) = if id_str.len() > 4 {
                let split = id_str.len() - 3;
                let a = &id_str[..split];
                let b = id_str[split..].trim_start_matches('0');
                (
                    a.to_string(),
                    if b.is_empty() {
                        "0".to_string()
                    } else {
                        b.to_string()
                    },
                )
            } else {
                (id_str.clone(), "0".to_string())
            };
            download_url = format!(
                "https://edge.forgecdn.net/files/{a}/{b}/{}",
                urlencoding_encode(&file_name)
            );
        }

        if download_url.is_empty() {
            return Ok(None);
        }

        Ok(Some(CfFile {
            file_id,
            file_name: file_name.clone(),
            download_url,
            file_length: length,
            display_name: file_name,
        }))
    }
}

fn parse_addon(item: &Value) -> CfAddon {
    let id = item.get("id").and_then(Value::as_i64).unwrap_or(0) as i32;
    let name = html_decode(item.get("name").and_then(Value::as_str).unwrap_or(""));
    let summary = html_decode(item.get("summary").and_then(Value::as_str).unwrap_or(""));
    let download_count = item
        .get("downloadCount")
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as i64;
    let class_id = item.get("classId").and_then(Value::as_i64).unwrap_or(0) as i32;

    let thumbnail_url = item
        .get("logo")
        .and_then(|l| l.get("thumbnailUrl"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let author_name = item
        .get("authors")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|a| a.get("name"))
        .and_then(Value::as_str)
        .map(html_decode)
        .unwrap_or_default();

    let category_name = item
        .get("categories")
        .and_then(Value::as_array)
        .and_then(|c| c.first())
        .and_then(|c| c.get("name"))
        .and_then(Value::as_str)
        .map(html_decode)
        .unwrap_or_default();

    let mut latest_file_id = item
        .get("latestFilesIndexes")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|f| f.get("fileId"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    if latest_file_id == 0 {
        latest_file_id = item.get("mainFileId").and_then(Value::as_i64).unwrap_or(0) as i32;
    }

    CfAddon {
        id,
        name,
        summary,
        thumbnail_url,
        author_name,
        download_count,
        class_id,
        latest_file_id,
        category_name,
    }
}

fn parse_file(item: &Value) -> Option<CfFile> {
    let file_id = item.get("id").and_then(Value::as_i64)? as i32;
    let file_name = item
        .get("fileName")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let download_url = item
        .get("downloadUrl")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if download_url.is_empty() {
        return None;
    }
    let file_length = item.get("fileLength").and_then(Value::as_i64).unwrap_or(0);
    let display_name = item
        .get("displayName")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| file_name.clone());

    Some(CfFile {
        file_id,
        file_name,
        download_url,
        file_length,
        display_name,
    })
}

/// Minimal percent-encoding for query parameters (avoids pulling in a full
/// `url`/`urlencoding` crate dependency for one call site).
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Minimal HTML entity decode covering the handful of entities CurseForge
/// commonly emits in names/summaries (mirrors `System.Net.WebUtility.HtmlDecode`
/// closely enough for display purposes).
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_FIXTURE: &str = r#"{
        "data": [
            {
                "id": 238222,
                "name": "Just Enough Items &amp; Friends",
                "summary": "JEI recipe viewer",
                "downloadCount": 123456789.0,
                "classId": 6,
                "logo": { "thumbnailUrl": "https://example.com/thumb.png" },
                "authors": [{ "name": "mezz" }],
                "categories": [{ "name": "Map and Information" }],
                "latestFilesIndexes": [{ "fileId": 555 }]
            }
        ],
        "pagination": { "totalCount": 1 }
    }"#;

    #[test]
    fn parses_search_addon_fixture() {
        let json: Value = serde_json::from_str(SEARCH_FIXTURE).unwrap();
        let data = json.get("data").unwrap().as_array().unwrap();
        let addon = parse_addon(&data[0]);
        assert_eq!(addon.id, 238222);
        assert_eq!(addon.name, "Just Enough Items & Friends");
        assert_eq!(addon.download_count, 123456789);
        assert_eq!(addon.class_id, 6);
        assert_eq!(addon.thumbnail_url, "https://example.com/thumb.png");
        assert_eq!(addon.author_name, "mezz");
        assert_eq!(addon.category_name, "Map and Information");
        assert_eq!(addon.latest_file_id, 555);

        let total = json
            .get("pagination")
            .and_then(|p| p.get("totalCount"))
            .and_then(Value::as_i64)
            .unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn parses_files_fixture_and_skips_missing_url() {
        let fixture = r#"{
            "data": [
                { "id": 1, "fileName": "a.jar", "downloadUrl": "https://cdn/a.jar", "fileLength": 10, "displayName": "A" },
                { "id": 2, "fileName": "b.jar", "downloadUrl": null, "fileLength": 20 }
            ]
        }"#;
        let json: Value = serde_json::from_str(fixture).unwrap();
        let files: Vec<CfFile> = json
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .filter_map(parse_file)
            .collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_id, 1);
        assert_eq!(files[0].display_name, "A");
    }

    #[test]
    fn builds_forgecdn_fallback_url_for_missing_download_url() {
        // Mirrors CurseForgeService.ResolveManifestFileAsync's fallback URL scheme.
        let file_id = 2452473_i32;
        let file_name = "jei-1.19.2.jar";
        let id_str = file_id.to_string();
        let split = id_str.len() - 3;
        let a = &id_str[..split];
        let b = id_str[split..].trim_start_matches('0');
        let url = format!(
            "https://edge.forgecdn.net/files/{a}/{b}/{}",
            urlencoding_encode(file_name)
        );
        assert_eq!(
            url,
            "https://edge.forgecdn.net/files/2452/473/jei-1.19.2.jar"
        );
    }
}
