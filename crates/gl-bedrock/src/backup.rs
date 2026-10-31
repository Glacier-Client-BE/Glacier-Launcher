//! Zip-based backup/restore of the managed com.mojang folder set.
//! Mirrors `Services/BedrockBackupService.cs`.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{BedrockError, IoContext, Result};
use crate::model::BedrockBackup;
use crate::util::{
    iso_from_systemtime, now_iso, sanitize_file_component, walk_files, write_atomic,
};

/// The com.mojang subfolders that make up a "full" Bedrock backup/instance
/// snapshot. Mirrors `BedrockBackupService.ManagedFolders`.
pub const MANAGED_FOLDERS: &[&str] = &[
    "minecraftWorlds",
    "resource_packs",
    "behavior_packs",
    "skin_packs",
    "minecraftpe",
    "development_resource_packs",
    "development_behavior_packs",
    "development_skin_packs",
];

pub struct BedrockBackupService {
    backups_dir: PathBuf,
}

/// Where a backup's managed folders are read from or written to.
///
/// The distinction is load-bearing and mirrors the C# `sourceIsLive` /
/// `targetIsLive` flags in `BedrockBackupService`: a managed *instance* keeps
/// every managed folder directly under one directory, whereas the **live**
/// game install resolves each folder independently via
/// `CurseForgeService.LiveRootFor` — pack folders can legitimately live under
/// a different account root than `minecraftWorlds`. Treating the live install
/// as a single root silently backs up (and restores) the wrong directories.
#[derive(Debug, Clone)]
pub enum BedrockRoot {
    /// A managed instance directory; every managed folder sits under it.
    Instance(PathBuf),
    /// The live game install; each managed folder is resolved on its own.
    Live,
}

impl BedrockRoot {
    /// The root that `folder` lives under. Port of
    /// `sourceIsLive ? CurseForgeService.LiveRootFor(folder) : sourceRoot`.
    pub fn root_for(&self, folder: &str) -> PathBuf {
        match self {
            Self::Instance(dir) => dir.clone(),
            Self::Live => crate::paths::live_root_for(folder),
        }
    }

    /// The directory for `folder` itself, i.e. its root joined with its name.
    pub fn dir_for(&self, folder: &str) -> PathBuf {
        self.root_for(folder).join(folder)
    }
}

impl BedrockBackupService {
    pub fn new(backups_dir: impl Into<PathBuf>) -> Self {
        Self {
            backups_dir: backups_dir.into(),
        }
    }

    pub fn backups_dir(&self) -> &Path {
        &self.backups_dir
    }

    /// Lists `*.zip` backups in the backups directory, newest first.
    pub fn list_backups(&self) -> Result<Vec<BedrockBackup>> {
        let mut result = Vec::new();
        if !self.backups_dir.exists() {
            return Ok(result);
        }

        for entry in std::fs::read_dir(&self.backups_dir).ctx(&self.backups_dir)? {
            let entry = entry.ctx(&self.backups_dir)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("zip") {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let created = meta
                .created()
                .or_else(|_| meta.modified())
                .map(iso_from_systemtime)
                .unwrap_or_else(|_| now_iso());
            result.push(BedrockBackup {
                name: path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string(),
                file_path: path.to_string_lossy().into_owned(),
                size_bytes: meta.len(),
                created_at: created,
            });
        }

        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    /// Creates a zip backup of the managed folders under `source_root`,
    /// returning the created archive's path. Errors if none of the managed
    /// folders exist (nothing to back up), mirroring the C# behavior.
    pub fn create_backup_from_folder(
        &self,
        source_root: &Path,
        label: Option<&str>,
    ) -> Result<PathBuf> {
        self.create_backup_from(&BedrockRoot::Instance(source_root.to_path_buf()), label)
    }

    /// Root-aware form of [`Self::create_backup_from_folder`]. Resolves each
    /// managed folder through `source`, so a [`BedrockRoot::Live`] backup
    /// picks up the real per-account directories.
    pub fn create_backup_from(&self, source: &BedrockRoot, label: Option<&str>) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.backups_dir).ctx(&self.backups_dir)?;

        let stamp = timestamp_for_filename();
        let safe_label = match label {
            Some(l) if !l.trim().is_empty() => format!("_{}", sanitize_file_component(l.trim())),
            _ => String::new(),
        };
        let dest_path = self
            .backups_dir
            .join(format!("backup_{stamp}{safe_label}.zip"));

        let file = File::create(&dest_path).ctx(&dest_path)?;
        if let Err(err) = Self::write_managed_folders(file, source, &dest_path) {
            let _ = std::fs::remove_file(&dest_path);
            return Err(err);
        }
        Ok(dest_path)
    }

    fn write_managed_folders(file: File, source: &BedrockRoot, dest_path: &Path) -> Result<()> {
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let mut found_any = false;
        for folder in MANAGED_FOLDERS {
            let src_dir = source.dir_for(folder);
            if !src_dir.is_dir() {
                continue;
            }
            found_any = true;

            for file_path in walk_files(&src_dir)? {
                let relative = file_path.strip_prefix(&src_dir).unwrap_or(&file_path);
                let entry_name =
                    format!("{folder}/{}", relative.to_string_lossy().replace('\\', "/"));
                let mut bytes = Vec::new();
                if File::open(&file_path)
                    .and_then(|mut f| f.read_to_end(&mut bytes))
                    .is_err()
                {
                    continue;
                }
                archive.start_file(entry_name, options)?;
                archive.write_all(&bytes).ctx(dest_path)?;
            }
        }

        if !found_any {
            return Err(BedrockError::NothingToBackUp);
        }

        archive.finish()?;
        Ok(())
    }

    /// Extracts a backup archive into `target_root`, wiping and replacing
    /// each top-level managed folder found in the archive first (mirrors the
    /// "delete-then-extract" behavior of `RestoreBackupIntoFolderAsync`).
    pub fn restore_backup_into_folder(&self, backup_path: &Path, target_root: &Path) -> Result<()> {
        self.restore_backup_into(
            backup_path,
            &BedrockRoot::Instance(target_root.to_path_buf()),
        )
    }

    /// Root-aware form of [`Self::restore_backup_into_folder`], resolving each
    /// top-level managed folder in the archive through `target`.
    pub fn restore_backup_into(&self, backup_path: &Path, target: &BedrockRoot) -> Result<()> {
        if !backup_path.is_file() {
            return Err(BedrockError::BackupNotFound(backup_path.to_path_buf()));
        }

        let file = File::open(backup_path).ctx(backup_path)?;
        let mut archive = ZipArchive::new(file)?;

        // Only managed folder names are honored: an archive entry naming anything
        // else (e.g. `..`) would otherwise resolve to a directory outside the
        // target and be deleted or overwritten.
        let mut top_level_folders = std::collections::BTreeSet::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            if let Some(top) = entry.name().unwrap_or_default().split('/').next() {
                if MANAGED_FOLDERS.contains(&top) {
                    top_level_folders.insert(top.to_string());
                }
            }
        }

        for folder in &top_level_folders {
            let dir = target.dir_for(folder);
            if dir.is_dir() {
                let _ = std::fs::remove_dir_all(&dir);
            }
        }

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.name().unwrap_or_default().ends_with('/') {
                continue;
            }
            let Some(relative) = entry.enclosed_name() else {
                continue;
            };
            // Entry paths are `<managed folder>/<relative path>`; the managed
            // folder decides which root the entry is written under.
            let Some(top) = relative.iter().next().and_then(|c| c.to_str()) else {
                continue;
            };
            if !MANAGED_FOLDERS.contains(&top) {
                continue;
            }
            let dest_path = target.root_for(top).join(&relative);
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent).ctx(parent)?;
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).ctx(&dest_path)?;
            write_atomic(&dest_path, &bytes)?;
        }

        Ok(())
    }

    pub fn delete_backup(&self, backup_path: &Path) -> Result<()> {
        if backup_path.is_file() {
            std::fs::remove_file(backup_path).ctx(backup_path)?;
        }
        Ok(())
    }

    /// QOL "backup diff": compares two backup archives entry-by-entry (name
    /// + uncompressed size, read straight from the zip's central directory -
    /// no extraction needed) and reports what changed between them, newest
    /// (`b`) relative to oldest (`a`).
    pub fn diff_backups(&self, path_a: &Path, path_b: &Path) -> Result<BackupDiff> {
        let entries_a = Self::zip_entry_sizes(path_a)?;
        let entries_b = Self::zip_entry_sizes(path_b)?;

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();
        let mut unchanged_count = 0usize;

        for (name, size_b) in &entries_b {
            match entries_a.get(name) {
                None => added.push(name.clone()),
                Some(size_a) if size_a != size_b => changed.push(name.clone()),
                Some(_) => unchanged_count += 1,
            }
        }
        for name in entries_a.keys() {
            if !entries_b.contains_key(name) {
                removed.push(name.clone());
            }
        }
        added.sort();
        removed.sort();
        changed.sort();

        Ok(BackupDiff {
            added,
            removed,
            changed,
            unchanged_count,
        })
    }

    fn zip_entry_sizes(path: &Path) -> Result<std::collections::HashMap<String, u64>> {
        let file = File::open(path).ctx(path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut out = std::collections::HashMap::with_capacity(archive.len());
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            if entry.name().unwrap_or_default().ends_with('/') {
                continue;
            }
            out.insert(entry.name().unwrap_or_default().to_string(), entry.size());
        }
        Ok(out)
    }
}

/// Result of [`BedrockBackupService::diff_backups`]: file paths (as stored
/// in the archive, `<managed folder>/<relative path>`) added, removed, or
/// changed size between the two backups, plus a count of files identical in
/// both.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct BackupDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    pub unchanged_count: usize,
}

/// `backup_YYYY-MM-DD_HH-MM-SS` style stamp for filenames, derived from the
/// same ISO renderer used elsewhere (avoids a second date-formatting path).
fn timestamp_for_filename() -> String {
    let iso = now_iso(); // e.g. 2026-08-14T12:34:56.789Z
    let date_time = iso.trim_end_matches('Z');
    let (date, time) = date_time.split_once('T').unwrap_or((date_time, "00:00:00"));
    let time = time.split('.').next().unwrap_or(time).replace(':', "-");
    format!("{date}_{time}")
}
