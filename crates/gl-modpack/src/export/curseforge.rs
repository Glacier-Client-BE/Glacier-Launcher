//! Writes a CurseForge modpack zip from an installed instance, the inverse
//! of `manifest::parse_curseforge_manifest`.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::error::{ModpackError, Result};
use crate::export::{read_mod_provenance, ExportOptions, OVERRIDE_DIRS};

fn loader_id(opts: &ExportOptions) -> String {
    if opts.loader.is_empty() {
        String::new()
    } else {
        format!("{}-{}", opts.loader, opts.loader_version)
    }
}

/// Writes a CurseForge modpack zip for `instance_dir` to `dest_path`.
///
/// Mods with known CurseForge provenance (`cf_project_id`/`cf_file_id`) are
/// written as `files[]` project+file references; everything else is bundled
/// as `overrides/mods/<file>` since CurseForge's manifest format has no
/// direct-URL fallback for arbitrary files.
pub fn export_curseforge(instance_dir: &Path, dest_path: &Path, opts: &ExportOptions) -> Result<()> {
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
            match matched.and_then(|p| p.cf_project_id.zip(p.cf_file_id)) {
                Some((project_id, file_id)) => {
                    files_json.push(json!({
                        "projectID": project_id,
                        "fileID": file_id,
                        "required": true,
                    }));
                }
                None => override_mod_files.push(path),
            }
        }
    }

    let manifest = json!({
        "minecraft": {
            "version": opts.minecraft_version,
            "modLoaders": if opts.loader.is_empty() {
                json!([])
            } else {
                json!([{ "id": loader_id(opts), "primary": true }])
            },
        },
        "manifestType": "minecraftModpack",
        "manifestVersion": 1,
        "name": opts.pack_name,
        "version": opts.pack_version,
        "author": "",
        "files": files_json,
        "overrides": "overrides",
    });

    let file = File::create(dest_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    for path in &override_mod_files {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        add_file_to_zip(&mut zip, path, &format!("overrides/mods/{filename}"), options)?;
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
