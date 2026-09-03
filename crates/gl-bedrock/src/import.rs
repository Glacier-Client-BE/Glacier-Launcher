//! Importing dropped Bedrock content files (`.mcworld`/`.mctemplate` worlds,
//! `.mcpack` single packs, `.mcaddon` multi-pack bundles) into an instance's
//! `com.mojang`-style folder layout. These are all just zip archives; the
//! extension only tells us how to route the extracted content.

use std::path::{Path, PathBuf};

use crate::error::{BedrockError, IoContext, Result};

/// What kind of content a dropped file turned out to contain, and where it landed.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportOutcome {
    World { name: String, path: String },
    Packs { paths: Vec<String> },
}

/// Imports a dropped file into `instance_dir`, dispatching on its extension.
/// Mirrors the original app's drag-drop-to-import behavior for Bedrock content.
pub fn import_content(source: &Path, instance_dir: &Path) -> Result<ImportOutcome> {
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "mcworld" | "mctemplate" => {
            let path = import_world(source, instance_dir)?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(ImportOutcome::World {
                name,
                path: path.to_string_lossy().into_owned(),
            })
        }
        "mcpack" | "mcaddon" => {
            let paths = import_packs(source, instance_dir)?;
            Ok(ImportOutcome::Packs {
                paths: paths
                    .into_iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
            })
        }
        other => Err(BedrockError::NotFound(PathBuf::from(format!(
            "unsupported Bedrock content extension: .{other}"
        )))),
    }
}

/// Extracts a `.mcworld`/`.mctemplate` archive into `<instance_dir>/minecraftWorlds/<name>`,
/// picking a non-colliding folder name from the archive's `levelname.txt` (falling
/// back to the file's own stem).
fn import_world(source: &Path, instance_dir: &Path) -> Result<PathBuf> {
    let worlds_dir = instance_dir.join("minecraftWorlds");
    std::fs::create_dir_all(&worlds_dir).ctx(&worlds_dir)?;

    let tmp = extract_to_temp(source)?;
    let level_name = std::fs::read_to_string(tmp.path().join("levelname.txt"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let base_name = level_name.unwrap_or_else(|| {
        source
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Imported World".to_string())
    });

    let dest = unique_dir(&worlds_dir, &sanitize_name(&base_name));
    move_dir_contents(tmp.path(), &dest)?;
    Ok(dest)
}

/// Extracts a `.mcpack` (single pack) or `.mcaddon` (bundle of pack folders,
/// or a bundle of nested `.mcpack` files) and routes each pack by its
/// `manifest.json` module type into `resource_packs`/`behavior_packs`/`skin_packs`.
fn import_packs(source: &Path, instance_dir: &Path) -> Result<Vec<PathBuf>> {
    let tmp = extract_to_temp(source)?;
    let mut imported = Vec::new();

    // A pack directory is anywhere under the extracted root that directly
    // contains a manifest.json. `.mcpack` archives usually have the manifest
    // at the root; `.mcaddon` bundles usually have one subfolder per pack.
    let mut pack_dirs = Vec::new();
    if tmp.path().join("manifest.json").is_file() {
        pack_dirs.push(tmp.path().to_path_buf());
    } else if let Ok(entries) = std::fs::read_dir(tmp.path()) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && p.join("manifest.json").is_file() {
                pack_dirs.push(p);
            }
        }
    }

    if pack_dirs.is_empty() {
        return Err(BedrockError::NotFound(PathBuf::from(
            "no manifest.json found in dropped pack archive",
        )));
    }

    for pack_dir in pack_dirs {
        let dir_name = pack_module_dir_name(&pack_dir);
        let dest_root = instance_dir.join(dir_name);
        std::fs::create_dir_all(&dest_root).ctx(&dest_root)?;

        let pack_name = pack_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Imported Pack".to_string());
        let dest = unique_dir(&dest_root, &sanitize_name(&pack_name));
        move_dir_contents(&pack_dir, &dest)?;
        imported.push(dest);
    }

    Ok(imported)
}

/// Reads `manifest.json`'s `modules[].type` to decide which folder a pack
/// belongs in. Prefers `resources` > `data` > `skin_pack`, matching the most
/// common single-purpose-pack case; addons with both a resource and behavior
/// module are rare enough that this pass only files them under one folder.
fn pack_module_dir_name(pack_dir: &Path) -> &'static str {
    let Ok(text) = std::fs::read_to_string(pack_dir.join("manifest.json")) else {
        return "resource_packs";
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return "resource_packs";
    };
    let types: Vec<String> = json
        .get("modules")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("type").and_then(|t| t.as_str()))
                .map(|s| s.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default();

    if types.iter().any(|t| t == "resources") {
        "resource_packs"
    } else if types.iter().any(|t| t == "data") {
        "behavior_packs"
    } else if types.iter().any(|t| t == "skin_pack") {
        "skin_packs"
    } else {
        "resource_packs"
    }
}

fn extract_to_temp(source: &Path) -> Result<tempfile::TempDir> {
    let file = std::fs::File::open(source).ctx(source)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let dir = tempfile::tempdir().ctx(source)?;
    archive.extract(dir.path())?;
    Ok(dir)
}

fn move_dir_contents(src: &Path, dest: &Path) -> Result<()> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    // Cross-device rename (temp dir on a different volume than the destination)
    // falls back to a recursive copy.
    copy_dir_recursive(src, dest)?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest).ctx(dest)?;
    for entry in std::fs::read_dir(src).ctx(src)? {
        let entry = entry.ctx(src)?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).ctx(&from)?;
        }
    }
    Ok(())
}

fn unique_dir(parent: &Path, base_name: &str) -> PathBuf {
    let candidate = parent.join(base_name);
    if !candidate.exists() {
        return candidate;
    }
    for n in 2.. {
        let candidate = parent.join(format!("{base_name} ({n})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "Imported".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_zip(path: &Path, entries: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn imports_mcworld_using_levelname() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("dropped.mcworld");
        write_zip(
            &zip_path,
            &[("levelname.txt", "My Cool World"), ("level.dat", "x")],
        );

        let instance_dir = dir.path().join("instance");
        let outcome = import_content(&zip_path, &instance_dir).unwrap();

        match outcome {
            ImportOutcome::World { name, path } => {
                assert_eq!(name, "My Cool World");
                assert!(Path::new(&path).join("level.dat").is_file());
            }
            _ => panic!("expected World outcome"),
        }
    }

    #[test]
    fn imports_mcpack_by_resource_module_type() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("dropped.mcpack");
        let manifest = r#"{"modules":[{"type":"resources"}]}"#;
        write_zip(
            &zip_path,
            &[("manifest.json", manifest), ("pack_icon.png", "x")],
        );

        let instance_dir = dir.path().join("instance");
        let outcome = import_content(&zip_path, &instance_dir).unwrap();

        match outcome {
            ImportOutcome::Packs { paths } => {
                assert_eq!(paths.len(), 1);
                assert!(paths[0].contains("resource_packs"));
            }
            _ => panic!("expected Packs outcome"),
        }
    }

    #[test]
    fn imports_mcaddon_bundle_routes_by_module_type() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("dropped.mcaddon");
        write_zip(
            &zip_path,
            &[
                ("RP/manifest.json", r#"{"modules":[{"type":"resources"}]}"#),
                ("BP/manifest.json", r#"{"modules":[{"type":"data"}]}"#),
            ],
        );

        let instance_dir = dir.path().join("instance");
        let outcome = import_content(&zip_path, &instance_dir).unwrap();

        match outcome {
            ImportOutcome::Packs { paths } => {
                assert_eq!(paths.len(), 2);
                assert!(paths.iter().any(|p| p.contains("resource_packs")));
                assert!(paths.iter().any(|p| p.contains("behavior_packs")));
            }
            _ => panic!("expected Packs outcome"),
        }
    }

    #[test]
    fn rejects_unsupported_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dropped.zip");
        std::fs::write(&path, b"not a real zip").unwrap();
        assert!(import_content(&path, &dir.path().join("instance")).is_err());
    }
}
