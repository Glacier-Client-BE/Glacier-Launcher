//! Java Edition worlds, world backups and datapacks.
//!
//! Port of the worlds/backups/datapacks half of `JavaInstanceService.cs`
//! (`ListWorlds`:432, `PathForDatapacks`:443, `ListDatapacks`:446,
//! `ImportDatapackFile`:466, `DeleteDatapack`:475, `ListBackups`:488,
//! `BackupWorldAsync`:498, `RestoreBackupAsync`:510, `DeleteBackup`:527,
//! plus the `DirSize`, `ToEntry` and `Slug` helpers).
//!
//! These had no Rust counterpart at all, which is why the "World Backups"
//! panel (`Components/BackupsPanel.razor`) and the Java datapacks tab could
//! not exist in the rewrite.
//!
//! Every listing function creates its directory first, exactly as the C# does
//! (`Directory.CreateDirectory` before enumerating), so a fresh instance shows
//! an empty list rather than an error.

use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{JavaError, Result};

/// Port of `Models/JavaInstance.cs`'s `JavaInstanceFile`. Serialized in
/// PascalCase-free snake_case for the frontend; it is never persisted, so
/// unlike the settings types it has no on-disk compatibility constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInstanceFile {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size_bytes: u64,
    /// ISO-8601 UTC, matching the C# `ToString("o")` used for sorting.
    pub modified_at: String,
    pub is_disabled: bool,
}

/// One entry from `ListWorlds`. The C# returns an anonymous tuple
/// `(Name, Path, Size, ModifiedAt)`; named here so the JSON is legible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaWorld {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: String,
}

/// Port of `DirSize` — sums every file beneath `dir`, and swallows every
/// error to 0 exactly as the C# `catch { return 0; }` pair does. A world
/// folder that is briefly locked by the game must not fail the whole listing.
fn dir_size(dir: &Path) -> u64 {
    fn walk(dir: &Path, total: &mut u64) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => walk(&path, total),
                Ok(ft) if ft.is_file() => {
                    if let Ok(meta) = entry.metadata() {
                        *total += meta.len();
                    }
                }
                _ => {}
            }
        }
    }
    let mut total = 0;
    walk(dir, &mut total);
    total
}

/// ISO-8601 UTC timestamp for a path's mtime, matching C# `ToString("o")`.
/// Sorting is done on this string in the original, so the format has to be
/// lexicographically ordered — which the `o` format and RFC3339 both are.
fn modified_at(path: &Path) -> String {
    let Ok(meta) = fs::metadata(path) else {
        return String::new();
    };
    let Ok(mtime) = meta.modified() else {
        return String::new();
    };
    let dur = match mtime.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };
    format_iso_utc(dur.as_secs(), dur.subsec_nanos())
}

/// Minimal civil-from-days conversion so this crate doesn't need a date
/// dependency for one timestamp format.
fn format_iso_utc(secs: u64, nanos: u32) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };

    format!(
        "{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.{:07}Z",
        nanos / 100
    )
}

/// Port of `ToEntry` for a file.
fn to_entry(kind: &str, path: &Path) -> JavaInstanceFile {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let size_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    JavaInstanceFile {
        is_disabled: name.to_ascii_lowercase().ends_with(".disabled"),
        modified_at: modified_at(path),
        name,
        path: path.to_string_lossy().into_owned(),
        kind: kind.to_string(),
        size_bytes,
    }
}

/// Port of `Slug` (`JavaInstanceService.cs:709`): lowercase, every
/// non-alphanumeric becomes `-`, runs of `-` collapse, ends trimmed, and an
/// empty result becomes `default`.
pub fn slug(value: &str) -> String {
    let raw = if value.trim().is_empty() {
        "default".to_string()
    } else {
        value.trim().to_lowercase()
    };
    let mapped: String = raw
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let mut out = mapped.trim_matches('-').to_string();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    if out.trim().is_empty() {
        "default".to_string()
    } else {
        out
    }
}

/// Port of `ListWorlds` — every directory under `<instance>/saves`, newest
/// first by mtime.
pub fn list_worlds(minecraft_dir: &Path) -> Result<Vec<JavaWorld>> {
    let saves = minecraft_dir.join("saves");
    fs::create_dir_all(&saves).map_err(|e| JavaError::Other(e.to_string()))?;

    let mut worlds: Vec<JavaWorld> = fs::read_dir(&saves)
        .map_err(|e| JavaError::Other(e.to_string()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .map(|p| JavaWorld {
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size: dir_size(&p),
            modified_at: modified_at(&p),
            path: p.to_string_lossy().into_owned(),
        })
        .collect();
    worlds.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(worlds)
}

/// Port of `ListBackups` — every `*.zip` under `<instance>/backups`.
pub fn list_backups(minecraft_dir: &Path) -> Result<Vec<JavaInstanceFile>> {
    let backups = minecraft_dir.join("backups");
    fs::create_dir_all(&backups).map_err(|e| JavaError::Other(e.to_string()))?;

    let mut files: Vec<JavaInstanceFile> = fs::read_dir(&backups)
        .map_err(|e| JavaError::Other(e.to_string()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .map(|e| e.eq_ignore_ascii_case("zip"))
                    .unwrap_or(false)
        })
        .map(|p| to_entry("backups", &p))
        .collect();
    files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(files)
}

/// Port of `PathForDatapacks`.
pub fn datapacks_dir(minecraft_dir: &Path, world_name: &str) -> PathBuf {
    minecraft_dir
        .join("saves")
        .join(world_name)
        .join("datapacks")
}

/// Port of `ListDatapacks` — `*.zip` files AND loose folders, both newest
/// first. Folders are a real datapack form, which is why the C# lists them.
pub fn list_datapacks(minecraft_dir: &Path, world_name: &str) -> Result<Vec<JavaInstanceFile>> {
    let dir = datapacks_dir(minecraft_dir, world_name);
    fs::create_dir_all(&dir).map_err(|e| JavaError::Other(e.to_string()))?;

    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|e| JavaError::Other(e.to_string()))?
        .flatten()
    {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
        {
            out.push(to_entry("datapacks", &path));
        } else if path.is_dir() {
            out.push(JavaInstanceFile {
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                size_bytes: dir_size(&path),
                modified_at: modified_at(&path),
                path: path.to_string_lossy().into_owned(),
                kind: "datapacks".to_string(),
                is_disabled: false,
            });
        }
    }
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(out)
}

/// Port of `ImportDatapackFile` — copies into the world's datapacks folder,
/// overwriting a same-named file.
pub fn import_datapack(minecraft_dir: &Path, world_name: &str, zip_path: &Path) -> Result<PathBuf> {
    if !zip_path.is_file() {
        return Err(JavaError::Other("Datapack file not found.".into()));
    }
    let dir = datapacks_dir(minecraft_dir, world_name);
    fs::create_dir_all(&dir).map_err(|e| JavaError::Other(e.to_string()))?;
    let name = zip_path
        .file_name()
        .ok_or_else(|| JavaError::Other("Datapack file has no name.".into()))?;
    let dest = dir.join(name);
    fs::copy(zip_path, &dest).map_err(|e| JavaError::Other(e.to_string()))?;
    Ok(dest)
}

/// Port of `DeleteDatapack` — handles both the file and the folder form.
pub fn delete_datapack(path: &Path) -> Result<()> {
    if path.is_file() {
        fs::remove_file(path).map_err(|e| JavaError::Other(e.to_string()))?;
    } else if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| JavaError::Other(e.to_string()))?;
    }
    Ok(())
}

/// Port of `BackupWorldAsync` — zips the world folder to
/// `<instance>/backups/<slug>-yyyyMMdd-HHmmss.zip`, including the world
/// directory itself as the archive's top-level entry (the C# passes
/// `includeBaseDirectory: true`, which `RestoreBackupAsync` then relies on to
/// find the target folder name).
pub fn backup_world(minecraft_dir: &Path, world_dir: &Path, timestamp: &str) -> Result<PathBuf> {
    if !world_dir.is_dir() {
        return Err(JavaError::Other("World folder not found.".into()));
    }
    let backups = minecraft_dir.join("backups");
    fs::create_dir_all(&backups).map_err(|e| JavaError::Other(e.to_string()))?;

    let name = world_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "world".to_string());
    let zip_path = backups.join(format!("{}-{}.zip", slug(&name), timestamp));

    let file = fs::File::create(&zip_path).map_err(|e| JavaError::Other(e.to_string()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // `includeBaseDirectory: true` — entries are `<worldName>/...`.
    add_dir_to_zip(&mut zip, world_dir, &name, options)?;
    zip.finish().map_err(|e| JavaError::Other(e.to_string()))?;
    Ok(zip_path)
}

fn add_dir_to_zip<W: Write + Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(dir)
        .map_err(|e| JavaError::Other(e.to_string()))?
        .flatten()
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let entry_name = format!("{prefix}/{file_name}");
        if path.is_dir() {
            add_dir_to_zip(zip, &path, &entry_name, options)?;
        } else if path.is_file() {
            // A file the game has locked must not abort the whole backup.
            let Ok(bytes) = fs::read(&path) else { continue };
            if zip.start_file(&entry_name, options).is_err() {
                continue;
            }
            zip.write_all(&bytes)
                .map_err(|e| JavaError::Other(e.to_string()))?;
        }
    }
    Ok(())
}

/// Port of `RestoreBackupAsync`.
///
/// Faithful and destructive: it takes the archive's first top-level path
/// segment as the world folder name and **deletes that folder** before
/// extracting, so restoring replaces the current world rather than merging
/// into it. That is what the panel's "Replaces the current world with this
/// backup" tooltip promises, so the behaviour is kept — the confirmation step
/// lives in the UI, as in the original.
pub fn restore_backup(minecraft_dir: &Path, zip_path: &Path) -> Result<()> {
    if !zip_path.is_file() {
        return Err(JavaError::Other("Backup not found.".into()));
    }
    let saves = minecraft_dir.join("saves");
    fs::create_dir_all(&saves).map_err(|e| JavaError::Other(e.to_string()))?;

    let file = fs::File::open(zip_path).map_err(|e| JavaError::Other(e.to_string()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| JavaError::Other(e.to_string()))?;

    let mut top: Option<String> = None;
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        let full = entry.name().replace('\\', "/");
        if let Some(first) = full.split('/').next() {
            if !first.is_empty() {
                top = Some(first.to_string());
                break;
            }
        }
    }
    let top = top.unwrap_or_else(|| {
        zip_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "world".to_string())
    });

    let target = saves.join(&top);
    if target.is_dir() {
        fs::remove_dir_all(&target).map_err(|e| JavaError::Other(e.to_string()))?;
    }

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| JavaError::Other(e.to_string()))?;
        // `enclosed_name` rejects `..` and absolute paths, so a malicious or
        // corrupt archive cannot write outside `saves` (zip-slip).
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let out_path = saves.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| JavaError::Other(e.to_string()))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| JavaError::Other(e.to_string()))?;
        }
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| JavaError::Other(e.to_string()))?;
        fs::write(&out_path, buf).map_err(|e| JavaError::Other(e.to_string()))?;
    }
    Ok(())
}

/// Port of `DeleteBackup` — swallows failures, as the C# `try/catch {}` does.
pub fn delete_backup(zip_path: &Path) {
    if zip_path.is_file() {
        let _ = fs::remove_file(zip_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_matches_the_csharp_rules() {
        assert_eq!(slug("My World"), "my-world");
        assert_eq!(slug("  New   World!!  "), "new-world");
        assert_eq!(slug("///"), "default");
        assert_eq!(slug(""), "default");
        assert_eq!(slug("   "), "default");
        assert_eq!(slug("a--b"), "a-b");
        assert_eq!(slug("Test123"), "test123");
    }

    #[test]
    fn listing_creates_its_directory_and_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(list_worlds(root).unwrap().is_empty());
        assert!(root.join("saves").is_dir());
        assert!(list_backups(root).unwrap().is_empty());
        assert!(root.join("backups").is_dir());
    }

    #[test]
    fn backup_then_restore_round_trips_a_world() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let world = root.join("saves").join("My World");
        fs::create_dir_all(world.join("region")).unwrap();
        fs::write(world.join("level.dat"), b"hello").unwrap();
        fs::write(world.join("region").join("r.0.0.mca"), b"chunk").unwrap();

        let zip = backup_world(root, &world, "20260101-120000").unwrap();
        assert!(zip.is_file());
        assert_eq!(zip.file_name().unwrap(), "my-world-20260101-120000.zip");

        let listed = list_backups(root).unwrap();
        assert_eq!(listed.len(), 1);

        // Corrupt the live world, then restore over it.
        fs::write(world.join("level.dat"), b"corrupted").unwrap();
        fs::write(world.join("stray.txt"), b"should not survive").unwrap();
        restore_backup(root, &zip).unwrap();

        assert_eq!(fs::read(world.join("level.dat")).unwrap(), b"hello");
        assert_eq!(
            fs::read(world.join("region").join("r.0.0.mca")).unwrap(),
            b"chunk"
        );
        assert!(
            !world.join("stray.txt").exists(),
            "restore replaces the world folder, it does not merge into it"
        );
    }

    #[test]
    fn backup_of_a_missing_world_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = backup_world(dir.path(), &dir.path().join("nope"), "x").unwrap_err();
        assert!(err.to_string().contains("World folder not found"));
    }

    #[test]
    fn restore_of_a_missing_backup_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = restore_backup(dir.path(), &dir.path().join("nope.zip")).unwrap_err();
        assert!(err.to_string().contains("Backup not found"));
    }

    #[test]
    fn delete_backup_never_panics_on_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        delete_backup(&dir.path().join("nope.zip"));
    }

    #[test]
    fn datapacks_list_files_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let dp = datapacks_dir(root, "My World");
        fs::create_dir_all(dp.join("loose-pack")).unwrap();
        fs::write(dp.join("loose-pack").join("pack.mcmeta"), b"{}").unwrap();
        fs::write(dp.join("packed.zip"), b"zip").unwrap();
        fs::write(dp.join("notes.txt"), b"ignored").unwrap();

        let listed = list_datapacks(root, "My World").unwrap();
        let names: Vec<&str> = listed.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"packed.zip"));
        assert!(names.contains(&"loose-pack"));
        assert!(
            !names.contains(&"notes.txt"),
            "only .zip files and folders are datapacks"
        );
    }

    #[test]
    fn import_then_delete_a_datapack() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let src = root.join("incoming.zip");
        fs::write(&src, b"data").unwrap();

        let dest = import_datapack(root, "World", &src).unwrap();
        assert!(dest.is_file());
        assert_eq!(list_datapacks(root, "World").unwrap().len(), 1);

        delete_datapack(&dest).unwrap();
        assert!(list_datapacks(root, "World").unwrap().is_empty());
    }

    #[test]
    fn importing_a_missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = import_datapack(dir.path(), "World", &dir.path().join("nope.zip")).unwrap_err();
        assert!(err.to_string().contains("Datapack file not found"));
    }

    #[test]
    fn iso_timestamps_sort_lexicographically() {
        // The C# sorts on the string form, so the format must be ordered.
        let early = format_iso_utc(1_700_000_000, 0);
        let late = format_iso_utc(1_800_000_000, 0);
        assert!(early < late, "{early} should sort before {late}");
        assert!(early.starts_with("2023-11-"), "unexpected: {early}");
    }
}
