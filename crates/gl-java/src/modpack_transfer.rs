//! Modpack overrides extraction/export/import and official `.minecraft`
//! import — the remaining half of `Services/JavaInstanceService.cs` that had
//! no Rust counterpart (`ExtractOverridesAsync`:198, `ExportModpackAsync`:320,
//! `ImportModpackAsync`:341, `DetectMultiMcRoot`:387,
//! `ImportOfficialMinecraftAsync`:409).
//!
//! `instance_service::InstanceService` owns instance CRUD/layout; these free
//! functions operate on a given instance directory and are wired up by
//! callers (e.g. the modpack install flow, or a "duplicate instance"-style
//! command) that already hold an `InstanceService` + resolved instance.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{JavaError, Result};

fn err(e: impl std::fmt::Display) -> JavaError {
    JavaError::Other(e.to_string())
}

/// Extract a modpack zip's `overrides/` or `client-overrides/` entries into
/// `instance_dir`, mirroring `ExtractOverridesAsync`.
pub fn extract_overrides(zip_path: &Path, instance_dir: &Path) -> Result<()> {
    let file = fs::File::open(zip_path).map_err(err)?;
    let mut archive = zip::ZipArchive::new(file).map_err(err)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(err)?;
        if entry.is_dir() {
            continue;
        }
        // `enclosed_name` rejects `..` and absolute-path entries, so a
        // malicious or corrupt modpack zip cannot write outside
        // `instance_dir` (zip-slip) regardless of the prefix-stripping below.
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        let full = enclosed.to_string_lossy().replace('\\', "/");

        let rel = if let Some(r) = full.strip_prefix("overrides/") {
            r
        } else if let Some(r) = full.strip_prefix("client-overrides/") {
            r
        } else {
            continue;
        };
        if rel.is_empty() {
            continue;
        }

        let dest = instance_dir.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(err)?;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(err)?;
        fs::write(&dest, buf).map_err(err)?;
    }
    Ok(())
}

/// Export an instance's `mods`/`config`/`resourcepacks`/`shaderpacks`
/// directories plus a minimal `manifest.json` into a modpack-style zip under
/// `<root_dir>/exports/`, mirroring `ExportModpackAsync`. Returns the zip's
/// path.
pub fn export_modpack(
    root_dir: &Path,
    instance_dir: &Path,
    instance_id: &str,
    instance_name: &str,
    version_id: &str,
    timestamp: &str,
) -> Result<PathBuf> {
    let exports = root_dir.join("exports");
    fs::create_dir_all(&exports).map_err(err)?;
    let zip_path = exports.join(format!("{instance_id}-{timestamp}.zip"));

    let file = fs::File::create(&zip_path).map_err(err)?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for (sub, prefix) in [
        ("mods", "overrides/mods"),
        ("config", "overrides/config"),
        ("resourcepacks", "overrides/resourcepacks"),
        ("shaderpacks", "overrides/shaderpacks"),
    ] {
        add_dir_to_zip(&mut zip, &instance_dir.join(sub), prefix, options)?;
    }

    let manifest = serde_json::json!({
        "minecraft": { "version": version_id },
        "manifestType": "minecraftModpack",
        "manifestVersion": 1,
        "name": instance_name,
        "version": "1.0.0",
        "files": [],
        "overrides": "overrides",
    });
    zip.start_file("manifest.json", options).map_err(err)?;
    use std::io::Write;
    zip.write_all(serde_json::to_string_pretty(&manifest).map_err(err)?.as_bytes())
        .map_err(err)?;
    zip.finish().map_err(err)?;
    Ok(zip_path)
}

/// Export an entire instance directory tree (saves, mods, config, screenshots,
/// everything) as a plain zip at `dest_zip_path`, preserving relative paths.
/// Unlike [`export_modpack`] (which produces a CurseForge/Modrinth-style
/// `overrides/` pack for redistribution), this is a straight backup/transfer
/// of one instance — the `zip` crate is already a workspace dependency here
/// (see `crates/gl-java/Cargo.toml`), so no folder-copy fallback is needed.
pub fn export_instance(instance_dir: &Path, dest_zip_path: &Path) -> Result<()> {
    if let Some(parent) = dest_zip_path.parent() {
        fs::create_dir_all(parent).map_err(err)?;
    }
    let file = fs::File::create(dest_zip_path).map_err(err)?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    add_dir_to_zip(&mut zip, instance_dir, ".", options)?;
    zip.finish().map_err(err)?;
    Ok(())
}

fn add_dir_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in walk_files(dir) {
        let rel = entry.strip_prefix(dir).expect("walked under dir");
        let entry_name = format!("{prefix}/{}", rel.to_string_lossy().replace('\\', "/"));
        let Ok(mut file) = fs::File::open(&entry) else { continue };
        if zip.start_file(&entry_name, options).is_err() {
            continue;
        }
        std::io::copy(&mut file, zip).map_err(err)?;
    }
    Ok(())
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}

/// Import an arbitrary modpack/instance zip into `instance_dir`, mirroring
/// `ImportModpackAsync` + `DetectMultiMcRoot`: recognizes CurseForge/Modrinth
/// `overrides/`/`client-overrides/` layouts, MultiMC-style
/// `.minecraft/`/`minecraft/` roots (detected via an `instance.cfg` entry),
/// and otherwise extracts every entry verbatim.
pub fn import_modpack(zip_path: &Path, instance_dir: &Path) -> Result<()> {
    if !zip_path.is_file() {
        return Err(JavaError::Other("Modpack file not found.".into()));
    }
    let file = fs::File::open(zip_path).map_err(err)?;
    let mut archive = zip::ZipArchive::new(file).map_err(err)?;

    let mut has_overrides = false;
    let mut has_cfg = false;
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else { continue };
        let full = entry.name().replace('\\', "/");
        if full.to_ascii_lowercase().starts_with("overrides/")
            || full.to_ascii_lowercase().starts_with("client-overrides/")
        {
            has_overrides = true;
        }
        if full.to_ascii_lowercase().ends_with("instance.cfg") {
            has_cfg = true;
        }
    }

    let mmc_root = if has_cfg {
        detect_multimc_root(&mut archive)
    } else {
        None
    };

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(err)?;
        if entry.is_dir() {
            continue;
        }
        // `enclosed_name` rejects `..` and absolute-path entries, so a
        // malicious or corrupt zip cannot escape `instance_dir` (zip-slip)
        // no matter what prefix-stripping logic runs below — the detection
        // pass above (has_overrides/has_cfg/mmc_root) still reads raw entry
        // names since it only classifies the archive, but every actual
        // destination path here is derived from the sanitized name.
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        let full = enclosed.to_string_lossy().replace('\\', "/");

        let rel: Option<String> = if has_overrides {
            if let Some(r) = full.strip_prefix("overrides/") {
                Some(r.to_string())
            } else {
                full.strip_prefix("client-overrides/").map(|r| r.to_string())
            }
        } else if let Some(root) = &mmc_root {
            full.strip_prefix(root.as_str()).map(|r| r.to_string())
        } else {
            Some(full.clone())
        };

        let Some(rel) = rel else { continue };
        if rel.is_empty() {
            continue;
        }

        let dest = instance_dir.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(err)?;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(err)?;
        fs::write(&dest, buf).map_err(err)?;
    }
    Ok(())
}

/// Port of `DetectMultiMcRoot`: finds the `.minecraft/`/`minecraft/` prefix
/// used by MultiMC-family instance zips.
fn detect_multimc_root<R: Read + std::io::Seek>(archive: &mut zip::ZipArchive<R>) -> Option<String> {
    for candidate in [".minecraft/", "minecraft/"] {
        for i in 0..archive.len() {
            let Ok(entry) = archive.by_index(i) else { continue };
            let full = entry.name().replace('\\', "/");
            let lower = full.to_ascii_lowercase();
            if lower.ends_with(candidate)
                || lower.contains(&format!("/{candidate}"))
                || lower.starts_with(candidate)
            {
                if let Some(idx) = full.to_ascii_lowercase().find(candidate) {
                    return Some(full[..idx + candidate.len()].to_string());
                }
            }
        }
    }
    None
}

/// Import the user's official `%APPDATA%\.minecraft` install into
/// `instance_dir`, mirroring `ImportOfficialMinecraftAsync`. Copies
/// `mods`/`config`/`saves`/`resourcepacks`/`shaderpacks`/`screenshots`
/// directories and `options.txt`/`servers.dat`, swallowing per-item errors
/// exactly as the C# `try/catch {}` does.
pub fn import_official_minecraft(instance_dir: &Path) -> Result<()> {
    let appdata = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| JavaError::Other("could not resolve %APPDATA%".to_string()))?;
    let official = appdata.join(".minecraft");
    if !official.is_dir() {
        return Err(JavaError::Other(
            "No official .minecraft folder found in %APPDATA%.".to_string(),
        ));
    }

    for sub in [
        "mods",
        "config",
        "saves",
        "resourcepacks",
        "shaderpacks",
        "screenshots",
        "options.txt",
        "servers.dat",
    ] {
        let src = official.join(sub);
        let dst = instance_dir.join(sub);
        let result = if src.is_dir() {
            copy_dir_recursive(&src, &dst)
        } else if src.is_file() {
            fs::copy(&src, &dst).map(|_| ()).map_err(err)
        } else {
            Ok(())
        };
        // Mirrors the C# `catch {}` swallow — one bad item must not abort
        // the whole import.
        let _ = result;
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target).map_err(err)?;
    for entry in walk_files(source) {
        let rel = entry.strip_prefix(source).expect("walked under source");
        let dest = target.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(err)?;
        }
        fs::copy(&entry, &dest).map_err(err)?;
    }
    Ok(())
}
