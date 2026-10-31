//! Peeks at a `.mrpack` or CurseForge modpack zip's manifest without
//! extracting or downloading anything, so the UI can show a confirm dialog
//! before committing to an install.

use serde::Serialize;
use serde_json::Value;
use std::fs::File;
use std::path::Path;

use crate::error::{ModpackError, Result};
use crate::manifest::{parse_curseforge_manifest, parse_modrinth_index};
use crate::models::ModLoader;

const MAX_MOD_NAMES: usize = 25;

#[derive(Debug, Clone, Default, Serialize)]
pub struct PackPreview {
    pub name: String,
    pub mc_version: String,
    pub loader: Option<String>,
    pub loader_version: String,
    pub mod_count: usize,
    pub mod_names: Vec<String>,
    pub overrides_present: bool,
}

/// Parses just the manifest entry of `zip_path` (either a `.mrpack` or a
/// CurseForge modpack zip) and summarizes it. Does not extract files or
/// contact any network API — `mod_names` for CurseForge packs are literal
/// `projectID` file names when derivable from the manifest and otherwise a
/// placeholder, since resolving real titles requires the CurseForge API.
pub fn preview_pack(zip_path: &Path) -> Result<PackPreview> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let overrides_present = archive.by_name("overrides/").is_ok()
        || (0..archive.len()).any(|i| {
            archive
                .by_index(i)
                .map(|e| e.name().starts_with("overrides/"))
                .unwrap_or(false)
        });

    let fallback_name = zip_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("pack")
        .to_string();

    if let Ok(plan) = parse_modrinth_index(&mut archive, &fallback_name) {
        let mod_names: Vec<String> = plan
            .files
            .iter()
            .map(|f| {
                f.relative_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&f.relative_path)
                    .to_string()
            })
            .take(MAX_MOD_NAMES)
            .collect();
        return Ok(PackPreview {
            name: plan.name,
            mc_version: plan.minecraft_version,
            loader: plan.loader.map(|l| l.as_str().to_string()),
            loader_version: plan.loader_version,
            mod_count: plan.files.len(),
            mod_names,
            overrides_present,
        });
    }

    if let Ok(plan) = parse_curseforge_manifest(&mut archive, &fallback_name) {
        // CurseForge manifest entries only carry project/file ids, not
        // names; surface what we can without a network call.
        let mod_names: Vec<String> = plan
            .files
            .iter()
            .filter_map(|f| f.curseforge_ref.as_ref())
            .map(|r| format!("project {} (file {})", r.project_id, r.file_id))
            .take(MAX_MOD_NAMES)
            .collect();
        return Ok(PackPreview {
            name: plan.name,
            mc_version: plan.minecraft_version,
            loader: plan.loader.map(|l| l.as_str().to_string()),
            loader_version: plan.loader_version,
            mod_count: plan.files.len(),
            mod_names,
            overrides_present,
        });
    }

    Err(ModpackError::Unsupported(
        "not a recognized .mrpack or CurseForge modpack archive".into(),
    ))
}

#[allow(dead_code)]
fn loader_from_value(v: &Value) -> Option<ModLoader> {
    v.as_str().and_then(ModLoader::parse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_mrpack(path: &Path) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("modrinth.index.json", options).unwrap();
        zip.write_all(
            br#"{
                "formatVersion": 1,
                "game": "minecraft",
                "name": "Test Pack",
                "dependencies": { "minecraft": "1.20.1", "fabric-loader": "0.15.0" },
                "files": [
                    { "path": "mods/sodium.jar", "hashes": {}, "downloads": ["https://x/sodium.jar"], "fileSize": 1 }
                ]
            }"#,
        )
        .unwrap();
        zip.start_file("overrides/config/foo.json", options)
            .unwrap();
        zip.write_all(b"{}").unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn previews_mrpack_without_extracting() {
        let tmp = std::env::temp_dir().join(format!("preview-test-{}.mrpack", std::process::id()));
        write_mrpack(&tmp);
        let preview = preview_pack(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(preview.name, "Test Pack");
        assert_eq!(preview.mc_version, "1.20.1");
        assert_eq!(preview.loader.as_deref(), Some("fabric"));
        assert_eq!(preview.mod_count, 1);
        assert_eq!(preview.mod_names, vec!["sodium.jar".to_string()]);
        assert!(preview.overrides_present);
    }
}
