//! Per-instance mod provenance: records where each installed jar came from
//! (Modrinth project+version, CurseForge mod+file, or a locally dropped jar)
//! so update-checking and future re-installs don't have to guess from the
//! filename. Natural neighbour of `mod_analyzer.rs`, which owns the same
//! `<instance>/mods` directory but only reads jar metadata, not provenance.
//!
//! Persisted at `<instance>/mods/mods.json` via `gl_persistence`'s atomic
//! envelope writer, same pattern as every other per-instance JSON file in
//! this codebase.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{JavaError, Result};

const SCHEMA_VERSION: u32 = 1;

/// Where one installed mod jar came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModSource {
    Modrinth,
    CurseForge,
    Local,
}

/// One provenance record, keyed by jar filename in `ProvenanceStore::mods`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModProvenance {
    pub filename: String,
    pub source: ModSource,
    /// Modrinth project id / CurseForge numeric mod id as a string. Empty
    /// for `Local`.
    #[serde(default)]
    pub project_id: String,
    /// Modrinth version id / CurseForge numeric file id as a string. Empty
    /// for `Local`.
    #[serde(default)]
    pub version_id: String,
    #[serde(default)]
    pub sha1: String,
}

/// On-disk shape of `mods.json`: a map from filename to its provenance
/// record, so looking up "does this jar have provenance" is O(1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvenanceStore {
    pub mods: HashMap<String, ModProvenance>,
}

fn store_path(mods_dir: &Path) -> std::path::PathBuf {
    mods_dir.join("mods.json")
}

/// Loads `mods.json` next to `mods_dir`. A missing or corrupt file yields an
/// empty store rather than an error — provenance is best-effort metadata,
/// never a hard dependency for a mod to keep working.
pub fn load(mods_dir: &Path) -> ProvenanceStore {
    match gl_persistence::load::<ProvenanceStore>(&store_path(mods_dir)) {
        Ok(Some(envelope)) => envelope.data,
        Ok(None) => ProvenanceStore::default(),
        Err(_) => ProvenanceStore::default(),
    }
}

/// Writes `mods.json` atomically.
pub fn save(mods_dir: &Path, store: &ProvenanceStore) -> Result<()> {
    gl_persistence::save(&store_path(mods_dir), SCHEMA_VERSION, store)
        .map_err(|e| JavaError::Other(e.to_string()))
}

/// Records (or overwrites) one mod's provenance and persists the store.
pub fn record(mods_dir: &Path, record: ModProvenance) -> Result<()> {
    let mut store = load(mods_dir);
    store.mods.insert(record.filename.clone(), record);
    save(mods_dir, &store)
}

/// Removes a filename's provenance record (e.g. on delete/update-replace)
/// and persists the store. A no-op if the filename had no record.
pub fn remove(mods_dir: &Path, filename: &str) -> Result<()> {
    let mut store = load(mods_dir);
    if store.mods.remove(filename).is_some() {
        save(mods_dir, &store)?;
    }
    Ok(())
}

/// Renames a provenance record's key (e.g. when an update replaces
/// `old.jar` with `new.jar`), preserving the source/ids.
pub fn rename(mods_dir: &Path, old_filename: &str, new_filename: &str) -> Result<()> {
    let mut store = load(mods_dir);
    if let Some(mut rec) = store.mods.remove(old_filename) {
        rec.filename = new_filename.to_string();
        store.mods.insert(new_filename.to_string(), rec);
        save(mods_dir, &store)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_store_yields_empty_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = load(dir.path());
        assert!(store.mods.is_empty());
    }

    #[test]
    fn record_load_remove_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        record(
            dir.path(),
            ModProvenance {
                filename: "sodium.jar".into(),
                source: ModSource::Modrinth,
                project_id: "AANobbMI".into(),
                version_id: "abc123".into(),
                sha1: "deadbeef".into(),
            },
        )
        .unwrap();

        let store = load(dir.path());
        assert_eq!(store.mods.len(), 1);
        assert_eq!(store.mods["sodium.jar"].project_id, "AANobbMI");

        remove(dir.path(), "sodium.jar").unwrap();
        assert!(load(dir.path()).mods.is_empty());
    }

    #[test]
    fn rename_preserves_the_record_under_the_new_key() {
        let dir = tempfile::tempdir().unwrap();
        record(
            dir.path(),
            ModProvenance {
                filename: "old.jar".into(),
                source: ModSource::CurseForge,
                project_id: "123".into(),
                version_id: "456".into(),
                sha1: String::new(),
            },
        )
        .unwrap();

        rename(dir.path(), "old.jar", "new.jar").unwrap();
        let store = load(dir.path());
        assert!(!store.mods.contains_key("old.jar"));
        assert_eq!(store.mods["new.jar"].project_id, "123");
    }

    #[test]
    fn corrupt_store_falls_back_to_empty_rather_than_panicking() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(store_path(dir.path()), "not json").unwrap();
        assert!(load(dir.path()).mods.is_empty());
    }
}
