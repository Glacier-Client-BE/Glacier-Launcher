//! Port of `Services/JavaInstallService.cs`: downloads a Java Edition
//! version's client jar, libraries, and assets.
//!
//! Library and asset downloads run with bounded concurrency (matching the C#
//! `Parallel.ForEachAsync(..., 6)` intent, with a higher cap for the many small
//! asset objects). There's still no sha1 verification against existing files;
//! replace with `gl_download` (checksum-verified) once available.

use crate::download;
use crate::error::{io_err, JavaError, Result};
use crate::models::JavaVersion;
use crate::reuse::{self, Expected};
use crate::util::safe_join;
use futures_util::StreamExt;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const ASSETS_BASE_URL: &str = "https://resources.download.minecraft.net";
/// Max in-flight HTTP downloads for libraries and assets.
const DOWNLOAD_CONCURRENCY: usize = 12;

#[derive(Debug, Clone, Default)]
pub struct InstallProgress {
    pub stage: String,
    pub percent: f64,
    pub files_done: usize,
    pub files_total: usize,
}

struct DownloadJob {
    url: String,
    dest: PathBuf,
    expected: Option<Expected>,
}

struct AssetJob {
    url: String,
    dest: PathBuf,
    virtual_path: Option<String>,
    expected: Option<Expected>,
}

/// Places a verified copy of `expected` at `dest` from another launcher's
/// folder when one exists. `false` means the caller should download.
async fn try_reuse(roots: &Arc<Vec<PathBuf>>, expected: Option<&Expected>, dest: &Path) -> bool {
    let Some(expected) = expected.cloned() else {
        return false;
    };
    if roots.is_empty() {
        return false;
    }
    let roots = Arc::clone(roots);
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || reuse::reuse_into(&roots, &expected, &dest))
        .await
        .unwrap_or(false)
}

pub struct InstallService {
    client: reqwest::Client,
}

impl Default for InstallService {
    fn default() -> Self {
        Self::new()
    }
}

impl InstallService {
    pub fn new() -> Self {
        Self {
            client: crate::util::http_client(),
        }
    }

    /// Install `version` into `mc_dir` (client jar, libraries, natives,
    /// asset index + objects). Mirrors `InstallAsync`.
    pub async fn install(
        &self,
        version: &JavaVersion,
        mc_dir: &Path,
        mut report: impl FnMut(&InstallProgress),
    ) -> Result<()> {
        if version.url.is_empty() {
            return Err(JavaError::Other(format!(
                "version {} is missing a manifest URL — refresh the Java versions list",
                version.id
            )));
        }

        let mut progress = InstallProgress::default();
        let mut tick = |stage: &str,
                        percent: f64,
                        done: usize,
                        total: usize,
                        report: &mut dyn FnMut(&InstallProgress)| {
            progress.stage = stage.to_string();
            progress.percent = percent;
            progress.files_done = done;
            progress.files_total = total;
            report(&progress);
        };

        let version_dir = mc_dir.join("versions").join(&version.id);
        tokio::fs::create_dir_all(&version_dir)
            .await
            .map_err(io_err(&version_dir))?;

        tick("Downloading version metadata…", 1.0, 0, 0, &mut report);
        let version_json_path = version_dir.join(format!("{}.json", version.id));
        // TODO: replace with gl_download once available
        let version_json_text = download::get_string(&self.client, &version.url).await?;
        tokio::fs::write(&version_json_path, &version_json_text)
            .await
            .map_err(io_err(&version_json_path))?;

        let root: Value = serde_json::from_str(&version_json_text)?;

        // Other launchers' folders are looked at once per install.
        let roots = Arc::new(reuse::discover_roots(mc_dir));

        tick("Downloading client jar…", 4.0, 0, 0, &mut report);
        if let Some(client_dl) = root.get("downloads").and_then(|d| d.get("client")) {
            if let Some(url) = client_dl.get("url").and_then(|v| v.as_str()) {
                let jar_path = version_dir.join(format!("{}.jar", version.id));
                let expected = client_dl
                    .get("sha1")
                    .and_then(|v| v.as_str())
                    .map(|sha1| Expected::new(format!("versions/{0}/{0}.jar", version.id), sha1));
                if !try_reuse(&roots, expected.as_ref(), &jar_path).await {
                    // TODO: replace with gl_download once available
                    download::download_to_file(&self.client, url, &jar_path).await?;
                }
            }
        }

        tick("Downloading libraries…", 8.0, 0, 0, &mut report);
        let (lib_jobs, native_jars) = collect_libraries(&root, mc_dir);
        let lib_total = lib_jobs.len();
        let client = &self.client;
        let roots_ref = &roots;
        // Owned (url, dest, expected) tuples: a closure over `&Job` trips a
        // higher-ranked lifetime error once this future is awaited inside a
        // Tauri command.
        let pairs: Vec<(String, PathBuf, Option<Expected>)> = lib_jobs
            .iter()
            .map(|j| (j.url.clone(), j.dest.clone(), j.expected.clone()))
            .collect();
        let mut lib_stream =
            futures_util::stream::iter(pairs.into_iter().map(|(url, dest, expected)| {
                async move {
                    if !dest.is_file() && !try_reuse(roots_ref, expected.as_ref(), &dest).await {
                        // TODO: replace with gl_download once available
                        download::download_to_file(client, &url, &dest).await?;
                    }
                    Ok::<(), JavaError>(())
                }
            }))
            .buffer_unordered(DOWNLOAD_CONCURRENCY);
        let mut lib_done = 0usize;
        while let Some(res) = lib_stream.next().await {
            res?;
            lib_done += 1;
            tick(
                &format!("Libraries ({lib_done}/{lib_total})"),
                8.0 + lib_done as f64 * 22.0 / lib_total.max(1) as f64,
                lib_done,
                lib_total,
                &mut report,
            );
        }
        drop(lib_stream);

        tick("Extracting natives…", 32.0, 0, 0, &mut report);
        let natives_dir = version_dir.join("natives");
        tokio::fs::create_dir_all(&natives_dir)
            .await
            .map_err(io_err(&natives_dir))?;
        // Zip extraction is blocking CPU/IO; keep it off the async runtime.
        let extract_dir = natives_dir.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            for nj in &native_jars {
                if !nj.is_file() {
                    continue;
                }
                extract_native_jar(nj, &extract_dir)?;
            }
            Ok(())
        })
        .await
        .map_err(|e| JavaError::Other(format!("native extraction task failed: {e}")))??;

        tick("Downloading asset index…", 34.0, 0, 0, &mut report);
        let (asset_jobs, is_legacy, is_virtual, asset_index_name) =
            self.resolve_assets(&root, mc_dir).await?;
        let asset_total = asset_jobs.len();
        tick("Downloading assets…", 36.0, 0, asset_total, &mut report);
        // Owned (url, dest) pairs: a closure over `&Job` trips a higher-ranked
        // lifetime error once this future is awaited inside a Tauri command.
        let pairs: Vec<(String, PathBuf, Option<Expected>)> = asset_jobs
            .iter()
            .map(|j| (j.url.clone(), j.dest.clone(), j.expected.clone()))
            .collect();
        let mut asset_stream =
            futures_util::stream::iter(pairs.into_iter().map(|(url, dest, expected)| {
                async move {
                    if !dest.is_file() && !try_reuse(roots_ref, expected.as_ref(), &dest).await {
                        // TODO: replace with gl_download once available
                        download::download_to_file(client, &url, &dest).await?;
                    }
                    Ok::<(), JavaError>(())
                }
            }))
            .buffer_unordered(DOWNLOAD_CONCURRENCY);
        let mut asset_done = 0usize;
        while let Some(res) = asset_stream.next().await {
            res?;
            asset_done += 1;
            tick(
                &format!("Assets ({asset_done}/{asset_total})"),
                36.0 + asset_done as f64 * 60.0 / asset_total.max(1) as f64,
                asset_done,
                asset_total,
                &mut report,
            );
        }
        drop(asset_stream);

        if is_legacy || is_virtual {
            tick("Linking legacy/virtual assets…", 96.0, 0, 0, &mut report);
            let virt_root = if is_legacy {
                mc_dir.join("assets").join("virtual").join("legacy")
            } else {
                mc_dir
                    .join("assets")
                    .join("virtual")
                    .join(&asset_index_name)
            };
            tokio::fs::create_dir_all(&virt_root)
                .await
                .map_err(io_err(&virt_root))?;
            for job in &asset_jobs {
                let Some(vpath) = &job.virtual_path else {
                    continue;
                };
                // Object names come from the asset index JSON; never let one
                // write outside the virtual asset root.
                let Some(dest) = safe_join(&virt_root, vpath) else {
                    continue;
                };
                if let Some(parent) = dest.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                if !dest.exists() {
                    let _ = tokio::fs::copy(&job.dest, &dest).await;
                }
            }
        }

        tick("Done", 100.0, 0, 0, &mut report);
        Ok(())
    }

    async fn resolve_assets(
        &self,
        version_root: &Value,
        mc_dir: &Path,
    ) -> Result<(Vec<AssetJob>, bool, bool, String)> {
        let Some(idx) = version_root.get("assetIndex") else {
            return Ok((Vec::new(), false, false, "legacy".to_string()));
        };
        let url = idx
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let index_id = idx
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("legacy")
            .to_string();
        let index_path = safe_join(
            &mc_dir.join("assets").join("indexes"),
            &format!("{index_id}.json"),
        )
        .ok_or_else(|| {
            JavaError::Other(format!(
                "asset index id {index_id:?} is not a safe file name"
            ))
        })?;

        // TODO: replace with gl_download once available
        download::download_to_file(&self.client, &url, &index_path).await?;

        let index_text = tokio::fs::read_to_string(&index_path)
            .await
            .map_err(io_err(&index_path))?;
        let index_root: Value = serde_json::from_str(&index_text)?;

        let is_legacy = index_root
            .get("map_to_resources")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_virtual = index_root
            .get("virtual")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut jobs = Vec::new();
        if let Some(objects) = index_root.get("objects").and_then(|v| v.as_object()) {
            let objects_dir = mc_dir.join("assets").join("objects");
            for (name, obj) in objects {
                let Some(hash) = obj.get("hash").and_then(|v| v.as_str()) else {
                    continue;
                };
                if hash.len() < 2 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                    continue;
                }
                let sub = &hash[..2];
                let dest = objects_dir.join(sub).join(hash);
                let url = format!("{ASSETS_BASE_URL}/{sub}/{hash}");
                let virtual_path = if is_legacy || is_virtual {
                    Some(name.clone())
                } else {
                    None
                };
                // Asset objects are named by their SHA-1.
                let expected = Some(Expected::new(format!("assets/objects/{sub}/{hash}"), hash));
                jobs.push(AssetJob {
                    url,
                    dest,
                    virtual_path,
                    expected,
                });
            }
        }

        Ok((jobs, is_legacy, is_virtual, index_id))
    }
}

fn collect_libraries(root: &Value, mc_dir: &Path) -> (Vec<DownloadJob>, Vec<PathBuf>) {
    let mut jobs = Vec::new();
    let mut natives = Vec::new();

    let Some(libs) = root.get("libraries").and_then(|v| v.as_array()) else {
        return (jobs, natives);
    };

    for lib in libs {
        if !rules_allow(lib) {
            continue;
        }
        let Some(dl) = lib.get("downloads") else {
            continue;
        };

        if let Some(art) = dl.get("artifact") {
            if let (Some(url), Some(path)) = (
                art.get("url").and_then(|v| v.as_str()),
                art.get("path").and_then(|v| v.as_str()),
            ) {
                if !url.is_empty() {
                    // `path` comes from the version JSON: refuse anything that
                    // would land outside `libraries/`.
                    let Some(dest) = safe_join(&mc_dir.join("libraries"), path) else {
                        continue;
                    };
                    let expected = art
                        .get("sha1")
                        .and_then(|v| v.as_str())
                        .map(|sha1| Expected::new(format!("libraries/{path}"), sha1));
                    jobs.push(DownloadJob {
                        url: url.to_string(),
                        dest,
                        expected,
                    });
                }
            }
        }

        if let (Some(classifiers), Some(key)) = (
            dl.get("classifiers").filter(|c| c.is_object()),
            lib.get("natives")
                .and_then(|n| n.get("windows"))
                .and_then(|v| v.as_str()),
        ) {
            let key = key.replace("${arch}", "64");
            if let Some(native) = classifiers.get(&key) {
                if let (Some(url), Some(path)) = (
                    native.get("url").and_then(|v| v.as_str()),
                    native.get("path").and_then(|v| v.as_str()),
                ) {
                    if !url.is_empty() {
                        let Some(dest) = safe_join(&mc_dir.join("libraries"), path) else {
                            continue;
                        };
                        let expected = native
                            .get("sha1")
                            .and_then(|v| v.as_str())
                            .map(|sha1| Expected::new(format!("libraries/{path}"), sha1));
                        jobs.push(DownloadJob {
                            url: url.to_string(),
                            dest: dest.clone(),
                            expected,
                        });
                        natives.push(dest);
                    }
                }
            }
        }
    }

    (jobs, natives)
}

fn rules_allow(el: &Value) -> bool {
    let Some(rules) = el.get("rules").and_then(|v| v.as_array()) else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        let action = rule
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("allow");
        if let Some(os) = rule.get("os") {
            if let Some(name) = os.get("name").and_then(|v| v.as_str()) {
                if !name.eq_ignore_ascii_case("windows") {
                    continue;
                }
            }
        }
        allowed = action == "allow";
    }
    allowed
}

fn extract_native_jar(jar_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(jar_path).map_err(io_err(jar_path))?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.starts_with("META-INF/") {
            continue;
        }
        let Some(file_name) = Path::new(&name).file_name() else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let dest = dest_dir.join(file_name);
        let mut out = std::fs::File::create(&dest).map_err(io_err(&dest))?;
        std::io::copy(&mut entry, &mut out).map_err(io_err(&dest))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_paths_escaping_libraries_dir_are_skipped() {
        let root = serde_json::json!({
            "libraries": [
                {
                    "downloads": { "artifact": {
                        "path": "../../../evil.jar",
                        "url": "https://libraries.minecraft.net/evil.jar",
                        "sha1": "a9993e364706816aba3e25717850c26c9cd0d89d"
                    }}
                },
                {
                    "downloads": { "artifact": {
                        "path": "com/ok/ok-1.0.jar",
                        "url": "https://libraries.minecraft.net/ok.jar"
                    }}
                }
            ]
        });
        let mc = Path::new("mc");
        let (jobs, _) = collect_libraries(&root, mc);
        assert_eq!(jobs.len(), 1);
        assert_eq!(
            jobs[0].dest,
            mc.join("libraries")
                .join("com")
                .join("ok")
                .join("ok-1.0.jar")
        );
    }
}
