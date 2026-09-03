//! Port of `Services/JavaRuntimeDownloadService.cs`: downloads Adoptium JVM
//! runtimes for a given major version.

use crate::download;
use crate::error::{JavaError, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    link: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    size: i64,
}

#[derive(Debug, Deserialize)]
struct AdoptiumRelease {
    binary: AdoptiumBinary,
}

pub struct RuntimeDownloadService {
    client: reqwest::Client,
    runtimes_root: PathBuf,
}

impl RuntimeDownloadService {
    pub fn new(runtimes_root: PathBuf) -> Self {
        Self {
            client: reqwest::Client::new(),
            runtimes_root,
        }
    }

    /// Default runtimes root: `%USERPROFILE%\Glacier Launcher\runtimes`.
    pub fn default_runtimes_root(home: &Path) -> PathBuf {
        home.join("Glacier Launcher").join("runtimes")
    }

    /// Find an already-downloaded `javaw.exe` for `major`, if any.
    pub fn get_cached_javaw(&self, major: u32) -> Option<PathBuf> {
        Self::cached_javaw_under(&self.runtimes_root, major)
    }

    fn cached_javaw_under(runtimes_root: &Path, major: u32) -> Option<PathBuf> {
        let dir = runtimes_root.join(format!("java-{major}"));
        if !dir.is_dir() {
            return None;
        }
        find_first_named(&dir, "javaw.exe")
    }

    /// Download (or reuse a cached) Adoptium Hotspot JDK for `major` on
    /// Windows x64/aarch64, returning the path to `javaw.exe`.
    pub async fn download(
        &self,
        major: u32,
        mut on_progress: impl FnMut(&str, f64),
    ) -> Result<PathBuf> {
        if let Some(cached) = self.get_cached_javaw(major) {
            return Ok(cached);
        }

        let dest_dir = self.runtimes_root.join(format!("java-{major}"));
        tokio::fs::create_dir_all(&dest_dir)
            .await
            .map_err(crate::error::io_err(&dest_dir))?;

        let arch = if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x64"
        };

        let info_url = format!(
            "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?os=windows&architecture={arch}&image_type=jdk"
        );

        on_progress(&format!("Querying Adoptium for Java {major}…"), 0.0);

        let resp = self.client.get(&info_url).send().await?;
        if !resp.status().is_success() {
            return Err(JavaError::AdoptiumStatus {
                status: resp.status().as_u16(),
                major,
            });
        }
        let releases: Vec<AdoptiumRelease> = resp.json().await?;
        if releases.is_empty() {
            return Err(JavaError::AdoptiumNoPackage(major));
        }

        let mut download_url: Option<String> = None;
        let mut total_size: i64 = 0;
        for release in &releases {
            let pkg = &release.binary.package;
            total_size = pkg.size;
            if pkg.name.to_lowercase().ends_with(".zip") {
                download_url = Some(pkg.link.clone());
                break;
            }
            download_url.get_or_insert_with(|| pkg.link.clone());
        }
        let Some(download_url) = download_url else {
            return Err(JavaError::AdoptiumNoPackage(major));
        };

        let zip_path = dest_dir.join(format!("java-{major}.zip"));
        on_progress(&format!("Downloading Java {major}…"), 5.0);

        // TODO: replace with gl_download once available
        download::download_to_file(&self.client, &download_url, &zip_path).await?;
        let _ = total_size; // reserved for byte-level progress once gl-download lands

        on_progress(&format!("Extracting Java {major}…"), 88.0);
        extract_zip(&zip_path, &dest_dir)?;
        let _ = std::fs::remove_file(&zip_path);

        on_progress(&format!("Java {major} ready."), 100.0);

        self.get_cached_javaw(major)
            .ok_or_else(|| JavaError::JavawNotFoundAfterExtract {
                major,
                dir: dest_dir,
            })
    }
}

fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path).map_err(crate::error::io_err(zip_path))?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(dest_dir)?;
    Ok(())
}

/// Recursively find the first file named `name` under `root`.
fn find_first_named(root: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                return Some(path);
            }
        }
    }
    None
}
