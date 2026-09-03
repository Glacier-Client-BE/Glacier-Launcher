//! RenderDragon `.material.bin` shader pack management. Mirrors
//! `Services/ShaderService.cs`.
//!
//! Deviation: the C# `GetInstall()`/`UnavailableReason()` resolve the
//! installed Minecraft package (and whether it's sideloaded) via WinRT's
//! `PackageManager` — that's the same MSIX package-family resolution as
//! `BedrockPackageService.cs`, which is explicitly out of scope for this
//! crate. Callers must resolve the install's `data/renderer/materials`
//! directory themselves and pass it in; this service only manages files
//! within that directory once known.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::error::{IoContext, Result};
use crate::model::AppliedShader;
use crate::util::{now_iso, write_atomic};

pub struct ShaderService {
    shader_root: PathBuf,
}

impl ShaderService {
    pub fn new(shader_root: impl Into<PathBuf>) -> Self {
        Self {
            shader_root: shader_root.into(),
        }
    }

    pub fn shader_root(&self) -> &Path {
        &self.shader_root
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.shader_root.join("stock-backup")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.shader_root.join("applied.json")
    }

    pub fn load_applied(&self) -> Vec<AppliedShader> {
        let path = self.manifest_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Vec::new();
        };
        match serde_json::from_str(&text) {
            Ok(list) => list,
            Err(_) => {
                let quarantined = path.with_extension("json.corrupt");
                let _ = std::fs::rename(&path, quarantined);
                Vec::new()
            }
        }
    }

    fn save_applied(&self, list: &[AppliedShader]) -> Result<()> {
        let json = serde_json::to_string_pretty(list)?;
        write_atomic(&self.manifest_path(), json.as_bytes())
    }

    pub fn has_applied_shaders(&self) -> bool {
        !self.load_applied().is_empty()
    }

    /// Extracts `.material.bin` files referenced by `source_paths` (loose
    /// files, directories, or `.zip`/`.mcpack` bundles) into `materials_dir`,
    /// backing up any stock file being overwritten for the first time.
    /// Returns the installed file names.
    pub fn apply(&self, materials_dir: &Path, source_paths: &[PathBuf]) -> Result<Vec<String>> {
        std::fs::create_dir_all(self.backup_dir()).ctx(self.backup_dir())?;

        let staging = self
            .shader_root
            .join(format!("staging-{}", std::process::id()));
        std::fs::create_dir_all(&staging).ctx(&staging)?;

        let mut collected: Vec<(PathBuf, String)> = Vec::new();

        let result = (|| -> Result<Vec<String>> {
            for src in source_paths {
                if src.is_dir() {
                    let bundle_name = src
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    for f in crate::util::walk_files(src)? {
                        if f.to_string_lossy().ends_with(".material.bin") {
                            collected.push((f, bundle_name.clone()));
                        }
                    }
                    continue;
                }
                if !src.is_file() {
                    continue;
                }

                let ext = src
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "bin" {
                    let name = src
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    collected.push((src.clone(), name));
                } else if ext == "zip" || ext == "mcpack" {
                    let bundle_name = src
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let dest = staging.join(src.file_stem().unwrap_or_default());
                    std::fs::create_dir_all(&dest).ctx(&dest)?;

                    let file = std::fs::File::open(src).ctx(src)?;
                    let mut zip = ZipArchive::new(file)?;
                    for i in 0..zip.len() {
                        let mut entry = zip.by_index(i)?;
                        let entry_name = entry.name().to_string();
                        if !entry_name.to_lowercase().ends_with(".material.bin") {
                            continue;
                        }
                        let file_name = Path::new(&entry_name)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or(entry_name.clone());
                        let out_path = dest.join(&file_name);
                        let mut bytes = Vec::new();
                        entry.read_to_end(&mut bytes).ctx(&out_path)?;
                        std::fs::write(&out_path, &bytes).ctx(&out_path)?;
                        collected.push((out_path, bundle_name.clone()));
                    }
                }
            }

            if collected.is_empty() {
                return Err(crate::error::BedrockError::NotFound(
                    self.shader_root.clone(),
                ));
            }

            let mut applied = self.load_applied();
            let mut installed_names = Vec::new();

            for (path, source_name) in &collected {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let target_path = materials_dir.join(&name);
                let backup_path = self.backup_dir().join(&name);
                let had_stock = target_path.is_file();

                if had_stock && !backup_path.is_file() {
                    let _ = std::fs::copy(&target_path, &backup_path);
                }
                std::fs::copy(path, &target_path).ctx(&target_path)?;

                applied.retain(|a| !a.file_name.eq_ignore_ascii_case(&name));
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                applied.push(AppliedShader {
                    file_name: name.clone(),
                    source_name: source_name.clone(),
                    applied_iso: now_iso(),
                    size_bytes: size,
                    had_stock,
                });
                installed_names.push(name);
            }

            self.save_applied(&applied)?;
            Ok(installed_names)
        })();

        let _ = std::fs::remove_dir_all(&staging);
        result
    }

    /// Reverts every applied shader material back to its stock backup (or
    /// deletes it if it didn't exist before), returning the number reverted.
    pub fn restore(&self, materials_dir: &Path) -> Result<usize> {
        let mut applied = self.load_applied();
        if applied.is_empty() {
            return Ok(0);
        }

        let mut reverted = 0;
        let mut remaining = Vec::new();

        for entry in applied.drain(..) {
            let target_path = materials_dir.join(&entry.file_name);
            let backup_path = self.backup_dir().join(&entry.file_name);

            let did_revert = if backup_path.is_file() {
                let ok = std::fs::copy(&backup_path, &target_path).is_ok();
                if ok {
                    let _ = std::fs::remove_file(&backup_path);
                }
                ok
            } else if !entry.had_stock && target_path.is_file() {
                std::fs::remove_file(&target_path).is_ok()
            } else {
                false
            };

            if did_revert {
                reverted += 1;
            } else {
                remaining.push(entry);
            }
        }

        self.save_applied(&remaining)?;
        Ok(reverted)
    }

    pub fn list_stock_materials(&self, materials_dir: &Path) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(materials_dir) else {
            return Vec::new();
        };
        let mut names: HashSet<String> = HashSet::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("bin") {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name.ends_with(".material.bin") {
                names.insert(name);
            }
        }
        let mut list: Vec<String> = names.into_iter().collect();
        list.sort_by_key(|n| n.to_lowercase());
        list
    }
}
