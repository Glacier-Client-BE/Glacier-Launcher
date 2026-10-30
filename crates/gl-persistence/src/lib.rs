//! Atomic JSON persistence with corrupt-file quarantine and schema versioning
//! (JsonStore.cs equivalent).

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Serializes every `write_atomic` call process-wide. Without this, two
/// concurrent saves to the *same* path (e.g. two Tauri commands saving
/// settings near-simultaneously) each write their own temp file safely, but
/// the final `rename`s can land in either order — a last-writer-wins race
/// that silently drops one save. A single process-wide lock is simpler than
/// a per-path lock table (and correct: this crate's callers are infrequent,
/// small JSON writes, not a hot path where serializing all of them together
/// would be a real bottleneck) and also prevents a write and a concurrent
/// `load`/`quarantine_corrupt` on the same path from ever observing a
/// half-renamed state.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Error)]
pub enum PersistError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PersistError>;

/// A versioned envelope wrapping persisted data, allowing schema migrations
/// over time without breaking older on-disk files.
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct Envelope<T> {
    pub schema_version: u32,
    pub data: T,
}

/// Write `contents` to `path` atomically: write to a sibling temp file,
/// `fsync` it so the bytes are actually on disk (not just in the OS page
/// cache), then rename over the destination. Mirrors JsonStore.WriteAtomic —
/// extended with the `fsync` step, since `write`+`rename` alone is atomic
/// against torn writes but not durable against a crash/power loss between
/// the write and the rename: without flushing, a crash can still leave the
/// destination truncated, missing, or pointing at stale/zero-length data
/// once the OS actually writes the page cache back.
///
/// Serialized process-wide via `WRITE_LOCK` so two concurrent calls for the
/// same path can't have their renames land in a nondeterministic order and
/// silently drop one save.
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }

    let tmp = append_extension(path, &format!("{}.tmp", uuid_like_suffix()));

    let write_result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(contents.as_bytes())?;
        // Flush the temp file's actual content to disk before the rename —
        // otherwise the rename can be durable while the data behind it
        // isn't, on a filesystem/OS that doesn't itself guarantee write
        // ordering across a rename.
        file.sync_all()?;
        Ok(())
    })();

    let result = match write_result {
        Ok(()) => std::fs::rename(&tmp, path).map_err(PersistError::from),
        Err(e) => Err(PersistError::from(e)),
    };

    // Best-effort: fsync the parent directory too, so the rename's directory
    // entry update itself is durable, not just the file content. Unix
    // filesystems need this explicitly; Windows has no portable std
    // equivalent (opening a directory as a `File` isn't supported there),
    // but NTFS journals metadata operations like rename on its own, so this
    // is skipped rather than faked on that platform.
    #[cfg(unix)]
    if result.is_ok() {
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                if let Ok(dir_file) = std::fs::File::open(dir) {
                    let _ = dir_file.sync_all();
                }
            }
        }
    }

    // Best-effort cleanup of the temp file if the rename didn't consume it.
    if tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
    }

    result
}

/// Move a corrupt file aside as `<path>.corrupt` so it doesn't keep failing
/// to parse on every subsequent launch. Mirrors JsonStore.QuarantineCorrupt.
pub fn quarantine_corrupt(path: &Path) {
    if path.exists() {
        let quarantined = append_extension(path, "corrupt");
        let _ = std::fs::rename(path, quarantined);
    }
}

/// Write a value as a versioned JSON envelope, atomically.
pub fn save<T: Serialize>(path: &Path, schema_version: u32, data: &T) -> Result<()> {
    let envelope = Envelope {
        schema_version,
        data,
    };
    let contents = serde_json::to_string_pretty(&envelope)?;
    write_atomic(path, &contents)
}

/// Load a versioned JSON envelope. On parse failure, quarantines the corrupt
/// file and returns the error so the caller can fall back to defaults.
pub fn load<T: DeserializeOwned>(path: &Path) -> Result<Option<Envelope<T>>> {
    // Single read instead of exists() + read: avoids an extra metadata
    // syscall on every load, and a missing file is still reported as None.
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match serde_json::from_str::<Envelope<T>>(&contents) {
        Ok(envelope) => Ok(Some(envelope)),
        Err(e) => {
            quarantine_corrupt(path);
            Err(PersistError::from(e))
        }
    }
}

fn append_extension(path: &Path, ext: &str) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".");
    s.push(ext);
    std::path::PathBuf::from(s)
}

/// Lightweight random suffix generator, avoiding a dependency on the `uuid`
/// crate for a single temp-file-naming use case.
fn uuid_like_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    format!("{nanos:x}-{pid:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Sample {
        name: String,
        count: u32,
    }

    #[test]
    fn round_trips_through_atomic_write_and_load() {
        let dir = std::env::temp_dir().join(format!("gl-persistence-test-{}", uuid_like_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        let value = Sample {
            name: "glacier".into(),
            count: 42,
        };
        save(&path, 1, &value).unwrap();

        let loaded = load::<Sample>(&path).unwrap().unwrap();
        assert_eq!(loaded.schema_version, 1);
        assert_eq!(loaded.data, value);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_file_returns_none() {
        let path = std::env::temp_dir().join("gl-persistence-does-not-exist.json");
        assert!(load::<Sample>(&path).unwrap().is_none());
    }

    #[test]
    fn corrupt_file_is_quarantined_on_load_failure() {
        let dir =
            std::env::temp_dir().join(format!("gl-persistence-corrupt-{}", uuid_like_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(&path, "{ this is not valid json").unwrap();

        let result = load::<Sample>(&path);
        assert!(result.is_err());
        assert!(!path.exists());
        assert!(append_extension(&path, "corrupt").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_leftover_temp_file_after_write() {
        let dir = std::env::temp_dir().join(format!("gl-persistence-tmp-{}", uuid_like_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        write_atomic(&path, "{}").unwrap();

        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path() != path)
            .collect();
        assert!(leftovers.is_empty(), "leftover files: {leftovers:?}");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
