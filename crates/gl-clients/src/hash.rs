//! SHA-256 hashing helpers shared by FlarialService (DLL hash verification)
//! and CustomClientLibraryService (custom client integrity checks).

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

use crate::error::Result;

/// Lower-case hex SHA-256 of a byte slice.
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Lower-case hex SHA-256 of a file's contents, streamed in chunks so large
/// DLLs don't need to be loaded fully into memory.
pub fn sha256_hex_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Case-insensitive hash comparison, matching the C# services' use of
/// `StringComparison.OrdinalIgnoreCase` when comparing hex hash strings.
pub fn hashes_equal(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_sha256_of_empty_string() {
        assert_eq!(
            sha256_hex_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn known_sha256_of_abc() {
        assert_eq!(
            sha256_hex_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_comparison_is_case_insensitive() {
        assert!(hashes_equal("ABCDEF0123456789", "abcdef0123456789"));
        assert!(!hashes_equal("abc", "abd"));
    }

    #[test]
    fn file_hash_matches_bytes_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bin");
        std::fs::write(&path, b"hello world").unwrap();
        assert_eq!(
            sha256_hex_file(&path).unwrap(),
            sha256_hex_bytes(b"hello world")
        );
    }
}
