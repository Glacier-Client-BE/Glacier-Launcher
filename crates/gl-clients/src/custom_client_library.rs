//! Ported from `CustomClientLibraryService.cs`: manages the user's
//! "custom client library" (`library.json` + copied `.dll`/`.exe` files
//! under `%USERPROFILE%\Glacier Launcher\clients`).
//!
//! Actual file download for `add_from_url` is out of scope here — see the
//! crate-level `// TODO: wire to gl-download` note; this module covers
//! parsing/saving the index, path/name helpers, and the pure
//! hash-and-size-based "does this need an update" decision logic.

use std::path::{Path, PathBuf};

use crate::error::{ClientsError, Result};
use crate::hash;
use crate::models::CustomClientEntry;

/// `%USERPROFILE%\Glacier Launcher\clients`. Mirrors
/// `CustomClientLibraryService.LibraryDir`.
pub fn library_dir(user_profile: &Path) -> PathBuf {
    user_profile.join("Glacier Launcher").join("clients")
}

pub fn index_path(library_dir: &Path) -> PathBuf {
    library_dir.join("library.json")
}

pub fn path_of(library_dir: &Path, entry: &CustomClientEntry) -> PathBuf {
    library_dir.join(&entry.file_name)
}

/// Parses a `library.json` body into the entry list. Mirrors
/// `CustomClientLibraryService.All`'s deserialize step (pruning entries
/// whose backing file is missing is a caller concern, since that needs
/// filesystem access alongside the parsed list).
pub fn parse_index(body: &str) -> Result<Vec<CustomClientEntry>> {
    serde_json::from_str(body).map_err(ClientsError::from)
}

/// Serializes the entry list the same way the C# `Save()` does: indented
/// JSON with the entries' `PascalCase` field names (see
/// `CustomClientEntry`'s serde renames).
pub fn serialize_index(entries: &[CustomClientEntry]) -> Result<String> {
    serde_json::to_string_pretty(entries).map_err(ClientsError::from)
}

/// Drops entries whose backing file no longer exists under `library_dir`.
/// Returns whether anything was removed (so the caller knows to re-save).
/// Mirrors the prune step in `CustomClientLibraryService.All`.
pub fn prune_missing_files(entries: &mut Vec<CustomClientEntry>, library_dir: &Path) -> bool {
    let before = entries.len();
    entries.retain(|e| path_of(library_dir, e).is_file());
    entries.len() != before
}

/// Picks a filename that doesn't collide with an existing file in
/// `library_dir`, appending " (1)", " (2)", ... before the extension.
/// Mirrors `CustomClientLibraryService.UniqueFileName`.
pub fn unique_file_name(library_dir: &Path, desired: &str) -> String {
    let path = Path::new(desired);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(desired)
        .to_string();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    let mut candidate = format!("{stem}{ext}");
    let mut n = 1;
    while library_dir.join(&candidate).exists() {
        candidate = format!("{stem} ({n}){ext}");
        n += 1;
    }
    candidate
}

/// Validates a source file extension is one of the allowed custom-client
/// kinds, returning the normalized `Kind` string. Mirrors the extension
/// check in `AddFromFile`/`AddFromUrlAsync`.
pub fn kind_for_extension(file_name: &str) -> Result<&'static str> {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "exe" => Ok("exe"),
        "dll" => Ok("dll"),
        _ => Err(ClientsError::InvalidInput(
            "Only .dll and .exe clients can be added.".to_string(),
        )),
    }
}

/// Computes the SHA-256 hex hash + byte length metadata stamped onto a
/// newly registered entry. Mirrors `CustomClientLibraryService.Register`'s
/// hashing step.
pub fn hash_and_size(path: &Path) -> Result<(String, u64)> {
    let hash = hash::sha256_hex_file(path)?;
    let size = std::fs::metadata(path)?.len();
    Ok((hash, size))
}

/// The outcome of checking a web-sourced entry for updates, mirroring the
/// three branches of `CustomClientLibraryService.UpdateIfChangedAsync`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateDecision {
    /// The remote `Content-Length` matches the recorded size — treated as
    /// up to date without downloading the body.
    UpToDateBySize,
    /// The downloaded bytes' hash matches the recorded hash — up to date,
    /// but the recorded size should be refreshed to `new_size`.
    UpToDateByHash { refreshed_size: u64 },
    /// The hash differs — the entry should be replaced with new content.
    NeedsUpdate { new_hash: String, new_size: u64 },
}

/// Pure decision logic for whether a fetched-by-URL update is needed,
/// given the just-fetched bytes and the entry's recorded metadata. This
/// mirrors the post-HEAD-check branch of `UpdateIfChangedAsync` — the HEAD
/// short-circuit itself is a caller/network concern.
pub fn decide_update(remote_bytes: &[u8], recorded_hash: &str) -> UpdateDecision {
    let remote_hash = hash::sha256_hex_bytes(remote_bytes);
    let new_size = remote_bytes.len() as u64;
    if hash::hashes_equal(&remote_hash, recorded_hash) {
        UpdateDecision::UpToDateByHash {
            refreshed_size: new_size,
        }
    } else {
        UpdateDecision::NeedsUpdate {
            new_hash: remote_hash,
            new_size,
        }
    }
}

/// Mirrors the HEAD-based short circuit in `UpdateIfChangedAsync`: if the
/// remote `Content-Length` equals the recorded size, skip the full fetch.
pub fn head_matches_recorded_size(remote_content_length: Option<u64>, recorded_size: u64) -> bool {
    matches!(remote_content_length, Some(len) if len == recorded_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_index_json() {
        let entries = vec![CustomClientEntry {
            id: "abc123".into(),
            name: "My DLL".into(),
            file_name: "my.dll".into(),
            kind: "dll".into(),
            source_url: "".into(),
            sha256: "deadbeef".into(),
            size_bytes: 1234,
            added_iso: "2024-01-01T00:00:00Z".into(),
            updated_iso: "2024-01-01T00:00:00Z".into(),
            auto_update: true,
        }];
        let json = serialize_index(&entries).unwrap();
        assert!(json.contains("\"Id\": \"abc123\""));
        let parsed = parse_index(&json).unwrap();
        assert_eq!(parsed, entries);
    }

    #[test]
    fn unique_file_name_appends_counter_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("client.dll"), b"").unwrap();
        std::fs::write(dir.path().join("client (1).dll"), b"").unwrap();

        let name = unique_file_name(dir.path(), "client.dll");
        assert_eq!(name, "client (2).dll");
    }

    #[test]
    fn unique_file_name_passes_through_when_no_collision() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(unique_file_name(dir.path(), "fresh.dll"), "fresh.dll");
    }

    #[test]
    fn kind_for_extension_accepts_dll_and_exe_only() {
        assert_eq!(kind_for_extension("client.dll").unwrap(), "dll");
        assert_eq!(kind_for_extension("client.EXE").unwrap(), "exe");
        assert!(kind_for_extension("client.zip").is_err());
    }

    #[test]
    fn decide_update_detects_matching_and_changed_hash() {
        let bytes = b"new content";
        let matching_hash = hash::sha256_hex_bytes(bytes);

        match decide_update(bytes, &matching_hash) {
            UpdateDecision::UpToDateByHash { refreshed_size } => {
                assert_eq!(refreshed_size, bytes.len() as u64)
            }
            other => panic!("expected UpToDateByHash, got {other:?}"),
        }

        match decide_update(
            bytes,
            "0000000000000000000000000000000000000000000000000000000000000000",
        ) {
            UpdateDecision::NeedsUpdate { new_hash, new_size } => {
                assert_eq!(new_hash, matching_hash);
                assert_eq!(new_size, bytes.len() as u64);
            }
            other => panic!("expected NeedsUpdate, got {other:?}"),
        }
    }

    #[test]
    fn head_short_circuit_matches_on_equal_length() {
        assert!(head_matches_recorded_size(Some(100), 100));
        assert!(!head_matches_recorded_size(Some(100), 99));
        assert!(!head_matches_recorded_size(None, 100));
    }

    #[test]
    fn prune_missing_files_removes_absent_entries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("present.dll"), b"").unwrap();

        let mut entries = vec![
            CustomClientEntry {
                file_name: "present.dll".into(),
                ..blank_entry()
            },
            CustomClientEntry {
                file_name: "missing.dll".into(),
                ..blank_entry()
            },
        ];
        let changed = prune_missing_files(&mut entries, dir.path());
        assert!(changed);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "present.dll");
    }

    fn blank_entry() -> CustomClientEntry {
        CustomClientEntry {
            id: String::new(),
            name: String::new(),
            file_name: String::new(),
            kind: "dll".into(),
            source_url: String::new(),
            sha256: String::new(),
            size_bytes: 0,
            added_iso: String::new(),
            updated_iso: String::new(),
            auto_update: true,
        }
    }
}
