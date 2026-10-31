//! Writes a `.mrpack` (Modrinth modpack) zip from an installed instance,
//! the inverse of `manifest::parse_modrinth_index`. Keeps the same schema
//! that function reads so a round trip (export, then
//! `install_modrinth_modpack` on the resulting file) reproduces the mod list.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::error::{ModpackError, Result};
use crate::export::{read_mod_provenance, ExportOptions, OVERRIDE_DIRS};
use crate::hash::{sha1_and_sha512_hex_file, sha512_hex_file};

/// Writes an `.mrpack` for `instance_dir` to `dest_path`.
///
/// Mods with known provenance (a Modrinth project/version, or a bare
/// `download_url`) are written as `files[]` entries referencing that
/// download; everything else (including all of `config/`, `resourcepacks/`,
/// `shaderpacks/` since those can't generally be resolved to a single
/// canonical download) is copied byte-for-byte into `overrides/`.
pub fn export_mrpack(instance_dir: &Path, dest_path: &Path, opts: &ExportOptions) -> Result<()> {
    let mods_dir = instance_dir.join("mods");
    let provenance = read_mod_provenance(instance_dir);

    let mut files_json = Vec::new();
    let mut override_mod_files: Vec<std::path::PathBuf> = Vec::new();

    if mods_dir.is_dir() {
        for entry in std::fs::read_dir(&mods_dir)?.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext_is_jar = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("jar"))
                .unwrap_or(false);
            if !ext_is_jar {
                continue;
            }
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            let matched = provenance.iter().find(|p| p.filename == filename);
            let url = matched.and_then(|p| p.download_url.clone());

            match url {
                Some(download_url) => {
                    let (sha1, sha512) = match matched.and_then(|p| p.sha1.clone()) {
                        Some(known) => (known, sha512_hex_file(&path)?),
                        None => sha1_and_sha512_hex_file(&path)?,
                    };
                    let size = std::fs::metadata(&path)?.len();
                    files_json.push(json!({
                        "path": format!("mods/{filename}"),
                        "hashes": { "sha1": sha1, "sha512": sha512 },
                        "downloads": [download_url],
                        "fileSize": size,
                        "env": { "client": "required", "server": "required" },
                    }));
                }
                None => {
                    // No known download URL (manual/local jar, or provenance
                    // missing) — fall back to bundling the jar as an override
                    // so the pack still reproduces the instance faithfully.
                    override_mod_files.push(path);
                }
            }
        }
    }

    let mut dependencies = json!({ "minecraft": opts.minecraft_version });
    match opts.loader.as_str() {
        "fabric" => dependencies["fabric-loader"] = json!(opts.loader_version),
        "quilt" => dependencies["quilt-loader"] = json!(opts.loader_version),
        "forge" => dependencies["forge"] = json!(opts.loader_version),
        "neoforge" => dependencies["neoforge"] = json!(opts.loader_version),
        _ => {}
    }

    let index = json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": opts.pack_version,
        "name": opts.pack_name,
        "dependencies": dependencies,
        "files": files_json,
    });

    let file = File::create(dest_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("modrinth.index.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;

    for path in &override_mod_files {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        add_file_to_zip(
            &mut zip,
            path,
            &format!("overrides/mods/{filename}"),
            options,
        )?;
    }

    for dir_name in OVERRIDE_DIRS {
        let dir = instance_dir.join(dir_name);
        if dir.is_dir() {
            add_dir_to_zip(&mut zip, &dir, &format!("overrides/{dir_name}"), options)?;
        }
    }

    zip.finish()?;
    Ok(())
}

fn add_file_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    src: &Path,
    zip_path: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    zip.start_file(zip_path, options)?;
    let mut buf = Vec::new();
    File::open(src)?.read_to_end(&mut buf)?;
    zip.write_all(&buf)?;
    Ok(())
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    dir: &Path,
    zip_prefix: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    for entry in walk_files(dir)? {
        let rel = entry
            .strip_prefix(dir)
            .map_err(|_| ModpackError::Other("bad override path".into()))?;
        let zip_path = format!("{zip_prefix}/{}", rel.to_string_lossy().replace('\\', "/"));
        add_file_to_zip(zip, &entry, &zip_path, options)?;
    }
    Ok(())
}

fn walk_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path.is_file() {
                out.push(path);
            }
        }
        Ok(())
    }
    walk(dir, &mut out)?;
    Ok(out)
}
