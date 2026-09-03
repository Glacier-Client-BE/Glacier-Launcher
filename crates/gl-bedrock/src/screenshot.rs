//! Bedrock screenshot enumeration. Mirrors
//! `Services/BedrockScreenshotService.cs`.

use std::path::{Path, PathBuf};

use crate::error::{IoContext, Result};
use crate::model::ScreenshotFile;
use crate::util::{iso_from_systemtime, walk_files};

pub struct BedrockScreenshotService {
    /// The Xbox Game Bar-style `Captures` folder (Windows' `Videos\Captures`),
    /// scanned for `*minecraft*.png` screenshots.
    captures_dir: PathBuf,
    /// `<com.mojang>/Screenshots`, scanned for `*.jpeg`.
    com_mojang_screenshots_dir: PathBuf,
}

impl BedrockScreenshotService {
    pub fn new(
        com_mojang_screenshots_dir: impl Into<PathBuf>,
        captures_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            captures_dir: captures_dir.into(),
            com_mojang_screenshots_dir: com_mojang_screenshots_dir.into(),
        }
    }

    /// Lists screenshots from both source folders, newest-modified first.
    pub fn list_screenshots(&self) -> Result<Vec<ScreenshotFile>> {
        let mut result = Vec::new();

        if self.com_mojang_screenshots_dir.is_dir() {
            for file in walk_files(&self.com_mojang_screenshots_dir)? {
                if file
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("jpeg"))
                    != Some(true)
                {
                    continue;
                }
                if let Some(entry) = screenshot_entry(&file) {
                    result.push(entry);
                }
            }
        }

        if self.captures_dir.is_dir() {
            for entry in std::fs::read_dir(&self.captures_dir).ctx(&self.captures_dir)? {
                let entry = entry.ctx(&self.captures_dir)?;
                let file = entry.path();
                if !file.is_file() {
                    continue;
                }
                if file
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("png"))
                    != Some(true)
                {
                    continue;
                }
                let name = file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !name.to_lowercase().contains("minecraft") {
                    continue;
                }
                if let Some(entry) = screenshot_entry(&file) {
                    result.push(entry);
                }
            }
        }

        result.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        Ok(result)
    }

    pub fn delete_file(&self, path: &Path) -> Result<()> {
        if path.is_file() {
            std::fs::remove_file(path).ctx(path)?;
        }
        Ok(())
    }
}

fn screenshot_entry(file: &Path) -> Option<ScreenshotFile> {
    let meta = std::fs::metadata(file).ok()?;
    Some(ScreenshotFile {
        name: file.file_name()?.to_string_lossy().into_owned(),
        path: file.to_string_lossy().into_owned(),
        kind: "screenshot".to_string(),
        size_bytes: meta.len(),
        modified_at: meta.modified().map(iso_from_systemtime).unwrap_or_default(),
    })
}
