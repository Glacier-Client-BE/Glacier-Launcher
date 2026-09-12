//! Java Edition resourcepacks + shaderpacks: list/import/delete, modelled
//! directly on `worlds.rs`'s backups shape (same `JavaInstanceFile` return
//! type, same atomic/defensive-I/O style, same directory-created-on-list
//! behaviour so a fresh instance shows an empty list rather than an error).
//!
//! Both families are near-identical `*.zip` (or loose-folder, for
//! resourcepacks/shaderpacks that ship unpacked) directories, so the actual
//! listing logic is shared via `list_pack_dir`/`import_pack`/`delete_pack`
//! and the two families differ only in their subdirectory name.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{JavaError, Result};
use crate::worlds::JavaInstanceFile;

/// The two supported Java content-pack kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackKind {
    ResourcePack,
    ShaderPack,
}

impl PackKind {
    fn dir_name(&self) -> &'static str {
        match self {
            PackKind::ResourcePack => "resourcepacks",
            PackKind::ShaderPack => "shaderpacks",
        }
    }
}

fn pack_dir(minecraft_dir: &Path, kind: PackKind) -> PathBuf {
    minecraft_dir.join(kind.dir_name())
}

/// Copied from `worlds.rs::dir_size` (private there) — sums every file
/// beneath `dir`, swallowing errors to 0 so a locked/briefly-missing folder
/// doesn't fail the whole listing.
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

use crate::worlds::modified_at;

fn to_entry(kind: &str, path: &Path, size_bytes: u64) -> JavaInstanceFile {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    JavaInstanceFile {
        is_disabled: name.to_ascii_lowercase().ends_with(".disabled"),
        modified_at: modified_at(path),
        name,
        path: path.to_string_lossy().into_owned(),
        kind: kind.to_string(),
        size_bytes,
    }
}

/// Lists every `*.zip` file and loose folder directly under the pack
/// directory, newest first. Creates the directory first so a fresh instance
/// returns an empty list rather than an error, matching `worlds.rs`.
pub fn list_packs(minecraft_dir: &Path, kind: PackKind) -> Result<Vec<JavaInstanceFile>> {
    let dir = pack_dir(minecraft_dir, kind);
    fs::create_dir_all(&dir).map_err(|e| JavaError::Other(e.to_string()))?;

    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|e| JavaError::Other(e.to_string()))?
        .flatten()
    {
        let path = entry.path();
        if path.is_file() {
            let is_zip = path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("zip"))
                .unwrap_or(false);
            let is_disabled = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_ascii_lowercase().ends_with(".zip.disabled"))
                .unwrap_or(false);
            if is_zip || is_disabled {
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                out.push(to_entry(kind.dir_name(), &path, size));
            }
        } else if path.is_dir() {
            out.push(to_entry(kind.dir_name(), &path, dir_size(&path)));
        }
    }
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(out)
}

/// Copies a pack file (zip or folder) into the pack directory, overwriting a
/// same-named entry.
pub fn import_pack(minecraft_dir: &Path, kind: PackKind, source: &Path) -> Result<PathBuf> {
    if !source.exists() {
        return Err(JavaError::Other("Pack file not found.".into()));
    }
    let dir = pack_dir(minecraft_dir, kind);
    fs::create_dir_all(&dir).map_err(|e| JavaError::Other(e.to_string()))?;
    let name = source
        .file_name()
        .ok_or_else(|| JavaError::Other("Pack file has no name.".into()))?;
    let dest = dir.join(name);

    if source.is_dir() {
        copy_dir_recursive(source, &dest)?;
    } else {
        fs::copy(source, &dest).map_err(|e| JavaError::Other(e.to_string()))?;
    }
    Ok(dest)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).map_err(|e| JavaError::Other(e.to_string()))?;
    for entry in fs::read_dir(src)
        .map_err(|e| JavaError::Other(e.to_string()))?
        .flatten()
    {
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| JavaError::Other(e.to_string()))?;
        }
    }
    Ok(())
}

/// Deletes a pack (file or folder). Handles both forms like
/// `worlds::delete_datapack`.
pub fn delete_pack(path: &Path) -> Result<()> {
    if path.is_file() {
        fs::remove_file(path).map_err(|e| JavaError::Other(e.to_string()))?;
    } else if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| JavaError::Other(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_creates_its_directory_and_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_packs(dir.path(), PackKind::ResourcePack).unwrap().is_empty());
        assert!(dir.path().join("resourcepacks").is_dir());
        assert!(list_packs(dir.path(), PackKind::ShaderPack).unwrap().is_empty());
        assert!(dir.path().join("shaderpacks").is_dir());
    }

    #[test]
    fn import_list_delete_a_zip_pack() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("incoming.zip");
        fs::write(&src, b"data").unwrap();

        let dest = import_pack(dir.path(), PackKind::ResourcePack, &src).unwrap();
        assert!(dest.is_file());
        let listed = list_packs(dir.path(), PackKind::ResourcePack).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "incoming.zip");

        delete_pack(&dest).unwrap();
        assert!(list_packs(dir.path(), PackKind::ResourcePack).unwrap().is_empty());
    }

    #[test]
    fn import_a_loose_shaderpack_folder() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("MyShader");
        fs::create_dir_all(src.join("shaders")).unwrap();
        fs::write(src.join("shaders").join("final.fsh"), b"x").unwrap();

        let dest = import_pack(dir.path(), PackKind::ShaderPack, &src).unwrap();
        assert!(dest.is_dir());
        let listed = list_packs(dir.path(), PackKind::ShaderPack).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "MyShader");
        assert!(listed[0].size_bytes > 0);
    }

    #[test]
    fn importing_a_missing_source_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err =
            import_pack(dir.path(), PackKind::ResourcePack, &dir.path().join("nope.zip"))
                .unwrap_err();
        assert!(err.to_string().contains("Pack file not found"));
    }

    #[test]
    fn non_zip_files_are_not_listed() {
        let dir = tempfile::tempdir().unwrap();
        let rp = dir.path().join("resourcepacks");
        fs::create_dir_all(&rp).unwrap();
        fs::write(rp.join("notes.txt"), b"ignored").unwrap();
        assert!(list_packs(dir.path(), PackKind::ResourcePack).unwrap().is_empty());
    }
}
