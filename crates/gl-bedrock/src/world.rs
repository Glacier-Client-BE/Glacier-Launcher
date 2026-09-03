//! Bedrock world (save) enumeration and export. Mirrors
//! `Services/BedrockWorldService.cs`.

use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::{IoContext, Result};
use crate::model::BedrockWorld;
use crate::util::{directory_size, iso_from_systemtime, sanitize_file_component, walk_files};

const ICON_NAMES: &[&str] = &["world_icon.jpeg", "world_icon.jpg", "world_icon.png"];

pub struct BedrockWorldService {
    worlds_dir: PathBuf,
}

impl BedrockWorldService {
    pub fn new(worlds_dir: impl Into<PathBuf>) -> Self {
        Self {
            worlds_dir: worlds_dir.into(),
        }
    }

    pub fn worlds_dir(&self) -> &Path {
        &self.worlds_dir
    }

    /// Lists worlds under the worlds directory, newest-modified first. A
    /// directory only counts as a world if it has a `db/` folder or a
    /// `level.dat` file, matching the C# filter.
    pub fn list_worlds(&self) -> Result<Vec<BedrockWorld>> {
        let mut result = Vec::new();
        if !self.worlds_dir.is_dir() {
            return Ok(result);
        }

        for entry in std::fs::read_dir(&self.worlds_dir).ctx(&self.worlds_dir)? {
            let entry = entry.ctx(&self.worlds_dir)?;
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }

            let db_path = dir.join("db");
            let level_dat = dir.join("level.dat");
            if !db_path.is_dir() && !level_dat.is_file() {
                continue;
            }

            let name = read_world_name(&dir).unwrap_or_else(|| {
                dir.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            let size = directory_size(&dir);
            let modified = std::fs::metadata(&dir)
                .and_then(|m| m.modified())
                .map(iso_from_systemtime)
                .unwrap_or_default();

            result.push(BedrockWorld {
                name,
                folder_path: dir.to_string_lossy().into_owned(),
                size_bytes: size,
                modified_at: modified,
                icon_path: find_icon(&dir),
            });
        }

        result.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        Ok(result)
    }

    pub fn delete_world(&self, world_path: &Path) -> Result<()> {
        if world_path.is_dir() {
            std::fs::remove_dir_all(world_path).ctx(world_path)?;
        }
        Ok(())
    }

    /// Zips a world folder into `export_dir` as a `.mcworld` archive,
    /// disambiguating the filename with `(n)` suffixes on collision, and
    /// returns the created file's path.
    pub fn export_world(&self, world_path: &Path, export_dir: &Path) -> Result<PathBuf> {
        if !world_path.is_dir() {
            return Err(crate::error::BedrockError::NotFound(
                world_path.to_path_buf(),
            ));
        }
        std::fs::create_dir_all(export_dir).ctx(export_dir)?;

        let folder_name = world_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let world_name = read_world_name(world_path).unwrap_or(folder_name);
        let safe_name = sanitize_file_component(&world_name);

        let mut dest_path = export_dir.join(format!("{safe_name}.mcworld"));
        let mut counter = 1;
        while dest_path.exists() {
            dest_path = export_dir.join(format!("{safe_name} ({counter}).mcworld"));
            counter += 1;
        }

        let file = std::fs::File::create(&dest_path).ctx(&dest_path)?;
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        for entry in walk_files(world_path)? {
            let relative = entry.strip_prefix(world_path).unwrap_or(&entry);
            let name = relative.to_string_lossy().replace('\\', "/");
            archive.start_file(name, options)?;
            let bytes = std::fs::read(&entry).ctx(&entry)?;
            use std::io::Write;
            archive.write_all(&bytes).ctx(&dest_path)?;
        }
        archive.finish()?;

        Ok(dest_path)
    }
}

fn find_icon(world_dir: &Path) -> Option<String> {
    ICON_NAMES
        .iter()
        .map(|name| world_dir.join(name))
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
}

fn read_world_name(world_dir: &Path) -> Option<String> {
    let path = world_dir.join("levelname.txt");
    let name = std::fs::read_to_string(path).ok()?;
    let name = name.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}
