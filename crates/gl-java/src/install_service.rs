//! Port of `Services/JavaInstallService.cs`: downloads a Java Edition
//! version's client jar, libraries, and assets.
//!
//! Deviation from the C# original: downloads run sequentially rather than
//! with a bounded-parallelism `Parallel.ForEachAsync(..., 6)`, and there's no
//! sha1 verification against existing files. Both were dropped to keep this
//! placeholder implementation simple — replace with `gl_download` (which is
//! expected to provide parallel, checksum-verified downloads) once available.

use crate::download;
use crate::error::{io_err, JavaError, Result};
use crate::models::JavaVersion;
use serde_json::Value;
use std::path::{Path, PathBuf};

const ASSETS_BASE_URL: &str = "https://resources.download.minecraft.net";

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
}

struct AssetJob {
    url: String,
    dest: PathBuf,
    virtual_path: Option<String>,
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
            client: reqwest::Client::new(),
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

        tick("Downloading client jar…", 4.0, 0, 0, &mut report);
        if let Some(client_dl) = root.get("downloads").and_then(|d| d.get("client")) {
            if let Some(url) = client_dl.get("url").and_then(|v| v.as_str()) {
                let jar_path = version_dir.join(format!("{}.jar", version.id));
                // TODO: replace with gl_download once available
                download::download_to_file(&self.client, url, &jar_path).await?;
            }
        }

        tick("Downloading libraries…", 8.0, 0, 0, &mut report);
        let (lib_jobs, native_jars) = collect_libraries(&root, mc_dir);
        let lib_total = lib_jobs.len();
        for (i, job) in lib_jobs.iter().enumerate() {
            // TODO: replace with gl_download once available
            let _ = download::download_to_file(&self.client, &job.url, &job.dest).await;
            tick(
                &format!("Libraries ({}/{lib_total})", i + 1),
                8.0 + (i + 1) as f64 * 22.0 / lib_total.max(1) as f64,
                i + 1,
                lib_total,
                &mut report,
            );
        }

        tick("Extracting natives…", 32.0, 0, 0, &mut report);
        let natives_dir = version_dir.join("natives");
        tokio::fs::create_dir_all(&natives_dir)
            .await
            .map_err(io_err(&natives_dir))?;
        for nj in &native_jars {
            if !nj.is_file() {
                continue;
            }
            let _ = extract_native_jar(nj, &natives_dir);
        }

        tick("Downloading asset index…", 34.0, 0, 0, &mut report);
        let (asset_jobs, is_legacy, is_virtual, asset_index_name) =
            self.resolve_assets(&root, mc_dir).await?;
        let asset_total = asset_jobs.len();
        tick("Downloading assets…", 36.0, 0, asset_total, &mut report);
        for (i, job) in asset_jobs.iter().enumerate() {
            // TODO: replace with gl_download once available
            let _ = download::download_to_file(&self.client, &job.url, &job.dest).await;
            tick(
                &format!("Assets ({}/{asset_total})", i + 1),
                36.0 + (i + 1) as f64 * 60.0 / asset_total.max(1) as f64,
                i + 1,
                asset_total,
                &mut report,
            );
        }

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
                let dest = virt_root.join(vpath);
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
        let index_path = mc_dir
            .join("assets")
            .join("indexes")
            .join(format!("{index_id}.json"));

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
                if hash.len() < 2 {
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
                jobs.push(AssetJob {
                    url,
                    dest,
                    virtual_path,
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
                    let dest = mc_dir
                        .join("libraries")
                        .join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
                    jobs.push(DownloadJob {
                        url: url.to_string(),
                        dest,
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
                        let dest = mc_dir
                            .join("libraries")
                            .join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
                        jobs.push(DownloadJob {
                            url: url.to_string(),
                            dest: dest.clone(),
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
