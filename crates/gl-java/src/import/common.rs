//! Shared helpers used by every launcher adapter: directory size and
//! mod/world counting. Kept intentionally simple (non-recursive-error,
//! best-effort) since these numbers are informational (UI badges), not used
//! for correctness.

use std::path::Path;

/// Total size in bytes of every file under `dir`, recursing into
/// subdirectories. Returns 0 if `dir` doesn't exist or can't be read.
pub fn dir_size_bytes(dir: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

/// Number of `.jar` files directly inside `instance_dir/mods` (loader-managed
/// mods are always flat in that folder across every launcher this module
/// supports).
pub fn count_mods(instance_dir: &Path) -> usize {
    count_files_with_ext(&instance_dir.join("mods"), "jar")
}

/// Number of world folders (each containing a `level.dat`) under
/// `instance_dir/saves`.
pub fn count_worlds(instance_dir: &Path) -> usize {
    let saves = instance_dir.join("saves");
    let Ok(entries) = std::fs::read_dir(&saves) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir() && e.path().join("level.dat").is_file())
        .count()
}

fn count_files_with_ext(dir: &Path, ext: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case(ext))
                    .unwrap_or(false)
        })
        .count()
}
