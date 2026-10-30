//! Sha1 file hashing, used to match installed mod jars against Modrinth's
//! hash-lookup endpoints (`/version_file/{hash}` and its `/update` variant)
//! for the mod update checker (feature 11).

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha1::{Digest, Sha1};
use sha2::Sha512;

use crate::error::Result;

/// Streams a file through the hashers in `feed`, so large jars are never
/// loaded fully into memory and each file is read exactly once.
fn for_each_chunk(path: impl AsRef<Path>, mut feed: impl FnMut(&[u8])) -> Result<()> {
    let mut reader = BufReader::with_capacity(64 * 1024, File::open(path)?);
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        feed(&buf[..n]);
    }
    Ok(())
}

pub fn sha1_hex_file(path: impl AsRef<Path>) -> Result<String> {
    let mut hasher = Sha1::new();
    for_each_chunk(path, |chunk| hasher.update(chunk))?;
    Ok(hex_encode(&hasher.finalize()))
}

/// Sha512 hex digest of a file, needed for `.mrpack` export (Modrinth's
/// `modrinth.index.json` carries both sha1 and sha512 per file).
pub fn sha512_hex_file(path: impl AsRef<Path>) -> Result<String> {
    let mut hasher = Sha512::new();
    for_each_chunk(path, |chunk| hasher.update(chunk))?;
    Ok(hex_encode(&hasher.finalize()))
}

/// Computes both the sha1 and sha512 hex digests of a file in one read.
pub fn sha1_and_sha512_hex_file(path: impl AsRef<Path>) -> Result<(String, String)> {
    let mut h1 = Sha1::new();
    let mut h512 = Sha512::new();
    for_each_chunk(path, |chunk| {
        h1.update(chunk);
        h512.update(chunk);
    })?;
    Ok((hex_encode(&h1.finalize()), hex_encode(&h512.finalize())))
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
