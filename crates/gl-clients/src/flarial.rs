//! Ported from `FlarialService.cs`: fetches the expected SHA-256 hash for
//! the Flarial client DLL and verifies a local copy against it.

use reqwest::Client;
use serde_json::Value;

use crate::error::{ClientsError, Result};
use crate::hash;

pub const DOWNLOAD_URL: &str = "https://cdn.flarial.xyz/dll/latest.dll";
pub const HASHES_URL: &str = "https://cdn.flarial.xyz/dll_hashes.json";
pub const HASH_KEY: &str = "Release";
pub const FILE_NAME: &str = "Flarial.Client.Release.dll";

/// Fetches `dll_hashes.json` and extracts the `"Release"` hash. Mirrors
/// `FlarialService.GetRemoteHashAsync`.
pub async fn fetch_remote_hash(http: &Client) -> Result<String> {
    let body = http.get(HASHES_URL).send().await?.text().await?;
    parse_remote_hash(&body)
}

/// Parses the `dll_hashes.json` body without a network round trip, so the
/// extraction logic is unit-testable against fixtures.
pub fn parse_remote_hash(body: &str) -> Result<String> {
    let root: Value = serde_json::from_str(body).map_err(ClientsError::from)?;
    root.get(HASH_KEY)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            ClientsError::NotFound(format!("\"{HASH_KEY}\" key missing from dll_hashes.json"))
        })
}

/// Computes the local DLL's SHA-256 hash. Mirrors
/// `FlarialService.GetLocalHashAsync`, except errors are surfaced instead
/// of being swallowed into an empty string — callers wanting the old
/// "silently not up to date" behavior can treat any `Err` as not-up-to-date.
pub fn local_hash(dll_path: &std::path::Path) -> Result<String> {
    hash::sha256_hex_file(dll_path)
}

/// Compares a local hash against the remote expected hash, case-insensitive
/// (matches `FlarialService.IsUpToDateAsync`'s `OrdinalIgnoreCase` compare).
pub fn is_up_to_date(local_hash: &str, remote_hash: &str) -> bool {
    hash::hashes_equal(local_hash, remote_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_hash_from_fixture() {
        let json = r#"{"Release":"AABBCCDDEEFF00112233445566778899AABBCCDDEEFF00112233445566778899","Beta":"1122"}"#;
        let hash = parse_remote_hash(json).unwrap();
        assert_eq!(
            hash,
            "AABBCCDDEEFF00112233445566778899AABBCCDDEEFF00112233445566778899"
        );
    }

    #[test]
    fn missing_release_key_is_an_error() {
        let json = r#"{"Beta":"1122"}"#;
        assert!(parse_remote_hash(json).is_err());
    }

    #[test]
    fn up_to_date_comparison_is_case_insensitive() {
        assert!(is_up_to_date("abcd1234", "ABCD1234"));
        assert!(!is_up_to_date("abcd1234", "ffff0000"));
    }

    #[test]
    fn local_hash_matches_known_file_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, b"abc").unwrap();
        let h = local_hash(&path).unwrap();
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
