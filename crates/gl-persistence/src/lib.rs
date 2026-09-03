//! Atomic JSON persistence with corrupt-file quarantine and schema versioning
//! (JsonStore.cs equivalent).

use std::path::Path;

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

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

/// Write `contents` to `path` atomically: write to a sibling temp file, then
/// rename over the destination. Mirrors JsonStore.WriteAtomic.
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }

    let tmp = append_extension(path, &format!("{}.tmp", uuid_like_suffix()));

    let write_result = std::fs::write(&tmp, contents);
    let result = match write_result {
        Ok(()) => std::fs::rename(&tmp, path).map_err(PersistError::from),
        Err(e) => Err(PersistError::from(e)),
    };

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
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path)?;
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
