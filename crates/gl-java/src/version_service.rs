//! Port of `Services/JavaVersionService.cs`: fetches and caches the Mojang
//! piston-meta version manifest, and annotates versions with local install
//! state.

use crate::download;
use crate::error::Result;
use crate::models::JavaVersion;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Default, Deserialize)]
struct ManifestLatest {
    #[serde(default)]
    release: Option<String>,
    #[serde(default)]
    snapshot: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ManifestVersionEntry {
    #[serde(default)]
    id: String,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    sha1: String,
    #[serde(default, rename = "releaseTime")]
    release_time: String,
    #[serde(default)]
    time: String,
    #[serde(default, rename = "complianceLevel")]
    compliance_level: i32,
}

#[derive(Debug, Default, Deserialize)]
struct ManifestRoot {
    #[serde(default)]
    latest: ManifestLatest,
    #[serde(default)]
    versions: Vec<ManifestVersionEntry>,
}

/// Cache-file shape (mirrors the anonymous object serialized by the C# code).
#[derive(Debug, Serialize, Deserialize)]
struct CachedVersion {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "Url")]
    url: String,
    #[serde(rename = "Sha1")]
    sha1: String,
    #[serde(rename = "ReleaseTime")]
    release_time: String,
    #[serde(rename = "Time")]
    time: String,
    #[serde(rename = "ComplianceLevel")]
    compliance_level: i32,
}

pub struct VersionService {
    client: reqwest::Client,
    cache_file: PathBuf,
    pub last_error: Option<String>,
    pub latest_release: Option<String>,
    pub latest_snapshot: Option<String>,
}

impl VersionService {
    pub fn new(cache_file: PathBuf) -> Self {
        Self {
            client: crate::util::http_client(),
            cache_file,
            last_error: None,
            latest_release: None,
            latest_snapshot: None,
        }
    }

    /// Default cache file location: `%USERPROFILE%\Glacier Launcher\cache\java-versions.json`.
    pub fn default_cache_file() -> PathBuf {
        let home = dirs_home();
        home.join("Glacier Launcher")
            .join("cache")
            .join("java-versions.json")
    }

    /// Fetch the manifest (falling back to the on-disk cache on failure),
    /// then annotate each entry against `versions_dir` and `active_version_id`.
    pub async fn get_versions(
        &mut self,
        versions_dir: &Path,
        active_version_id: Option<&str>,
    ) -> Result<Vec<JavaVersion>> {
        self.last_error = None;

        // All disk I/O (cache read/write, per-version stat calls) runs on the
        // blocking pool so it never stalls the async runtime.
        let mut versions = match self.fetch_manifest().await {
            Ok(v) => {
                if let Ok(json) = serde_json::to_string(&cache_entries(&v)) {
                    let path = self.cache_file.clone();
                    let _ = tokio::task::spawn_blocking(move || persist_cache(&path, &json)).await;
                }
                v
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to fetch Mojang manifest: {e}"));
                let path = self.cache_file.clone();
                let cached = tokio::task::spawn_blocking(move || load_cache_file(&path))
                    .await
                    .unwrap_or_default();
                if !cached.is_empty() {
                    if let Some(err) = &mut self.last_error {
                        err.push_str(" (showing cached data)");
                    }
                }
                cached
            }
        };

        let versions_dir = versions_dir.to_path_buf();
        let active = active_version_id.map(str::to_string);
        versions = tokio::task::spawn_blocking(move || {
            annotate_installed(&mut versions, &versions_dir, active.as_deref());
            versions
        })
        .await
        .map_err(|e| {
            crate::error::JavaError::Other(format!("version annotate task failed: {e}"))
        })?;

        Ok(versions)
    }

    /// Scan `versions_dir` for version folders not present in `known_ids`
    /// (e.g. modloader profiles), mirroring `ScanCustomInstalledVersions`.
    pub fn scan_custom_installed_versions(
        &self,
        versions_dir: &Path,
        known_ids: &[String],
        active_version_id: Option<&str>,
    ) -> Vec<JavaVersion> {
        let mut results = Vec::new();
        let Ok(entries) = std::fs::read_dir(versions_dir) else {
            return results;
        };

        let known: std::collections::HashSet<String> =
            known_ids.iter().map(|s| s.to_lowercase()).collect();

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(id) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if id.is_empty() || known.contains(&id.to_lowercase()) {
                continue;
            }
            let json_path = path.join(format!("{id}.json"));
            if !json_path.is_file() {
                continue;
            }
            let has_jar = path.join(format!("{id}.jar")).is_file();
            let is_active = active_version_id
                .map(|a| a.eq_ignore_ascii_case(id))
                .unwrap_or(false);
            results.push(JavaVersion {
                id: id.to_string(),
                kind: "custom".to_string(),
                is_installed: true,
                has_jar,
                is_active,
                ..Default::default()
            });
        }

        results
    }

    async fn fetch_manifest(&mut self) -> Result<Vec<JavaVersion>> {
        let body = download::get_string(&self.client, MANIFEST_URL).await?;
        let root: ManifestRoot = serde_json::from_str(&body)?;

        self.latest_release = root.latest.release;
        self.latest_snapshot = root.latest.snapshot;

        Ok(root
            .versions
            .into_iter()
            .map(|v| JavaVersion {
                id: v.id,
                kind: v.kind,
                url: v.url,
                sha1: v.sha1,
                release_time: v.release_time,
                time: v.time,
                compliance_level: v.compliance_level,
                ..Default::default()
            })
            .collect())
    }
}

fn cache_entries(versions: &[JavaVersion]) -> Vec<CachedVersion> {
    versions
        .iter()
        .map(|v| CachedVersion {
            id: v.id.clone(),
            kind: v.kind.clone(),
            url: v.url.clone(),
            sha1: v.sha1.clone(),
            release_time: v.release_time.clone(),
            time: v.time.clone(),
            compliance_level: v.compliance_level,
        })
        .collect()
}

fn persist_cache(cache_file: &Path, json: &str) {
    if let Some(parent) = cache_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Atomic so a crash mid-write can't leave a truncated cache that
    // then fails to parse on every subsequent launch.
    let _ = crate::util::write_atomic(cache_file, json);
}

fn load_cache_file(cache_file: &Path) -> Vec<JavaVersion> {
    let Ok(text) = std::fs::read_to_string(cache_file) else {
        return Vec::new();
    };
    let Ok(cached) = serde_json::from_str::<Vec<CachedVersion>>(&text) else {
        return Vec::new();
    };
    cached
        .into_iter()
        .map(|c| JavaVersion {
            id: c.id,
            kind: c.kind,
            url: c.url,
            sha1: c.sha1,
            release_time: c.release_time,
            time: c.time,
            compliance_level: c.compliance_level,
            ..Default::default()
        })
        .collect()
}

fn annotate_installed(versions: &mut [JavaVersion], versions_dir: &Path, active: Option<&str>) {
    for v in versions.iter_mut() {
        let dir = versions_dir.join(&v.id);
        v.is_installed = dir.join(format!("{}.json", v.id)).is_file();
        v.has_jar = dir.join(format!("{}.jar", v.id)).is_file();
        v.is_active = Some(v.id.as_str()) == active;
    }
}

fn dirs_home() -> PathBuf {
    // Avoid pulling in a `dirs` crate dependency; mirror
    // Environment.SpecialFolder.UserProfile via USERPROFILE/HOME.
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
