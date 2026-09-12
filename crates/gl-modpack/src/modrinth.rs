//! Modrinth API client (`api.modrinth.com/v2`), mirroring the C#
//! `ModrinthService`.

use serde_json::Value;

use crate::error::{ModpackError, Result};
use crate::models::{MrDependency, MrProject, MrSearchResult, MrVersion};

const BASE_URL: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "GlacierLauncher/1.0 (glacier-launcher)";

pub struct ModrinthClient {
    http: reqwest::Client,
}

impl ModrinthClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    async fn get_json(&self, url: &str) -> Result<Value> {
        let resp = self
            .http
            .get(url)
            .header("User-Agent", USER_AGENT)
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

    async fn post_json(&self, url: &str, body: &Value) -> Result<Value> {
        let resp = self
            .http
            .post(url)
            .header("User-Agent", USER_AGENT)
            .json(body)
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

    /// Searches Modrinth projects, optionally filtered to a single
    /// `project_type` facet (e.g. "mod", "modpack", "resourcepack", "shader").
    pub async fn search(
        &self,
        query: &str,
        facet_type: &str,
        limit: u32,
        offset: u32,
    ) -> Result<MrSearchResult> {
        let mut url = format!("{BASE_URL}/search?limit={limit}&offset={offset}");
        if !query.trim().is_empty() {
            url.push_str(&format!("&query={}", urlencoding_encode(query)));
        }
        if !facet_type.is_empty() {
            url.push_str(&format!("&facets=[[\"project_type:{facet_type}\"]]"));
        }

        let json = self.get_json(&url).await?;
        let projects = json
            .get("hits")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().map(parse_project).collect())
            .unwrap_or_default();
        let total_count = json.get("total_hits").and_then(Value::as_i64).unwrap_or(0) as i32;

        Ok(MrSearchResult {
            projects,
            total_count,
        })
    }

    /// Fetches the newest version of a project (used both for "latest mod
    /// version" and "modpack file to install").
    pub async fn get_latest_version(&self, project_id: &str) -> Result<Option<MrVersion>> {
        let url = format!("{BASE_URL}/project/{project_id}/version?limit=1");
        let json = self.get_json(&url).await?;
        let Some(versions) = json.as_array() else {
            return Ok(None);
        };
        for ver in versions {
            if let Some(v) = parse_version(ver) {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    /// Fetches a project's display title (feature 12: naming dependency
    /// warnings by a human-readable title rather than a bare project id).
    pub async fn get_project_title(&self, project_id: &str) -> Result<Option<String>> {
        let url = format!("{BASE_URL}/project/{project_id}");
        let json = self.get_json(&url).await?;
        Ok(json
            .get("title")
            .and_then(Value::as_str)
            .map(|s| html_decode(s)))
    }

    /// Looks up the exact version matching a jar's sha1 hash (feature 11:
    /// used to identify which installed jar corresponds to which Modrinth
    /// version before checking for an update).
    pub async fn get_version_by_hash(&self, sha1_hex: &str) -> Result<Option<MrVersion>> {
        let url = format!("{BASE_URL}/version_file/{sha1_hex}?algorithm=sha1");
        match self.get_json(&url).await {
            Ok(json) => Ok(parse_version(&json)),
            Err(ModpackError::Status { status: 404, .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Given an installed jar's sha1 hash, asks Modrinth for the newest
    /// version compatible with the given loaders/game versions (feature 11:
    /// mod update checker). Empty `loaders`/`game_versions` mean "no filter".
    pub async fn get_update_for_hash(
        &self,
        sha1_hex: &str,
        loaders: &[String],
        game_versions: &[String],
    ) -> Result<Option<MrVersion>> {
        let url = format!("{BASE_URL}/version_file/{sha1_hex}/update?algorithm=sha1");
        let body = serde_json::json!({
            "loaders": loaders,
            "game_versions": game_versions,
        });
        match self.post_json(&url, &body).await {
            Ok(json) => Ok(parse_version(&json)),
            Err(ModpackError::Status { status: 404, .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

fn parse_project(item: &Value) -> MrProject {
    let id = item
        .get("project_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let slug = item
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let title = html_decode(item.get("title").and_then(Value::as_str).unwrap_or(""));
    let description = html_decode(
        item.get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let icon_url = item
        .get("icon_url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let author = item
        .get("author")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let downloads = item.get("downloads").and_then(Value::as_i64).unwrap_or(0);
    let project_type = item
        .get("project_type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let category_name = item
        .get("categories")
        .and_then(Value::as_array)
        .and_then(|c| c.first())
        .and_then(Value::as_str)
        .map(str::to_string);

    MrProject {
        id,
        slug,
        title,
        description,
        icon_url,
        author,
        downloads,
        project_type,
        category_name,
    }
}

fn parse_version(ver: &Value) -> Option<MrVersion> {
    let files = ver.get("files").and_then(Value::as_array)?;
    if files.is_empty() {
        return None;
    }
    let primary = files
        .iter()
        .find(|f| f.get("primary").and_then(Value::as_bool).unwrap_or(false))
        .unwrap_or(&files[0]);

    let file_name = primary.get("filename").and_then(Value::as_str)?.to_string();
    let url = primary.get("url").and_then(Value::as_str)?.to_string();
    let size = primary.get("size").and_then(Value::as_i64).unwrap_or(0);
    let sha1 = primary
        .get("hashes")
        .and_then(|h| h.get("sha1"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let name = ver
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| file_name.clone());
    let id = ver
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let changelog = ver
        .get("changelog")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let dependencies = ver
        .get("dependencies")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|d| MrDependency {
                    project_id: d
                        .get("project_id")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    version_id: d
                        .get("version_id")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    dependency_type: d
                        .get("dependency_type")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Some(MrVersion {
        id,
        name,
        file_name,
        url,
        size,
        sha1,
        changelog,
        dependencies,
    })
}

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
        "hits": [
            {
                "project_id": "AANobbMI",
                "slug": "sodium",
                "title": "Sodium",
                "description": "A modern rendering engine",
                "icon_url": "https://cdn.modrinth.com/data/AANobbMI/icon.png",
                "author": "jellysquid3",
                "downloads": 50000000,
                "project_type": "mod",
                "categories": ["optimization"]
            }
        ],
        "total_hits": 1
    }"#;

    #[test]
    fn parses_search_project_fixture() {
        let json: Value = serde_json::from_str(SEARCH_FIXTURE).unwrap();
        let hits = json.get("hits").unwrap().as_array().unwrap();
        let project = parse_project(&hits[0]);
        assert_eq!(project.id, "AANobbMI");
        assert_eq!(project.slug, "sodium");
        assert_eq!(project.title, "Sodium");
        assert_eq!(project.downloads, 50_000_000);
        assert_eq!(project.project_type, "mod");
        assert_eq!(project.category_name.as_deref(), Some("optimization"));
    }

    const VERSION_FIXTURE: &str = r#"[
        {
            "id": "abc123",
            "name": "Sodium 0.5.8",
            "files": [
                { "filename": "sodium-fabric-0.5.8.jar", "url": "https://cdn.modrinth.com/data/AANobbMI/versions/abc/sodium.jar", "size": 123456, "primary": true },
                { "filename": "sodium-fabric-0.5.8-sources.jar", "url": "https://cdn.modrinth.com/x.jar", "size": 999, "primary": false }
            ]
        }
    ]"#;

    #[test]
    fn parses_latest_version_and_prefers_primary_file() {
        let json: Value = serde_json::from_str(VERSION_FIXTURE).unwrap();
        let arr = json.as_array().unwrap();
        let v = parse_version(&arr[0]).unwrap();
        assert_eq!(v.id, "abc123");
        assert_eq!(v.file_name, "sodium-fabric-0.5.8.jar");
        assert_eq!(v.size, 123456);
    }

    #[test]
    fn returns_none_when_no_files() {
        let json = serde_json::json!({ "id": "x", "files": [] });
        assert!(parse_version(&json).is_none());
    }
}
