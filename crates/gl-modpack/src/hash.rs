//! Sha1 file hashing, used to match installed mod jars against Modrinth's
//! hash-lookup endpoints (`/version_file/{hash}` and its `/update` variant)
//! for the mod update checker (feature 11).

use std::path::Path;

use sha1::{Digest, Sha1};
use sha2::Sha512;

use crate::error::Result;

pub fn sha1_hex_file(path: impl AsRef<Path>) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(hex_encode(&hasher.finalize()))
}

/// Sha512 hex digest of a file, needed for `.mrpack` export (Modrinth's
/// `modrinth.index.json` carries both sha1 and sha512 per file).
pub fn sha512_hex_file(path: impl AsRef<Path>) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha512::new();
    hasher.update(&bytes);
    Ok(hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_sha1_of_abc() {
        let tmp = std::env::temp_dir().join(format!("gl-modpack-sha1-test-{}", std::process::id()));
        std::fs::write(&tmp, b"abc").unwrap();
        let hash = sha1_hex_file(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();
        assert_eq!(hash, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }
}
