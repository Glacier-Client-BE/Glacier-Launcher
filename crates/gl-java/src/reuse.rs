//! Reuse of game files other launchers already downloaded.
//!
//! Before a library, asset object or client jar is downloaded, the same
//! relative path is looked up in a few well-known folders of other launchers.
//! A candidate is only used when its SHA-1 equals the hash the version/asset
//! JSON pins, so a stale or modified file is never reused.
//!
// Adapted from Horizon launcher (MIT, horizon-rust/horizon): the approach of
// finding another launcher's copy of a pinned file by its SHA-1 and linking it
// into place instead of downloading it (launcher/src/sources.rs,
// launcher/src/other_launchers.rs).

use sha1::{Digest, Sha1};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// A file the version/asset JSON describes: its path relative to a launcher's
/// base folder (`libraries/...`, `assets/objects/xx/...`, `versions/...`) and
/// its SHA-1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    pub rel: String,
    pub sha1: String,
}

impl Expected {
    pub fn new(rel: impl Into<String>, sha1: impl Into<String>) -> Self {
        Self {
            rel: rel.into(),
            sha1: sha1.into(),
        }
    }
}

/// Base folders that other launchers keep `libraries/`, `assets/` and
/// `versions/` in. Only folders that exist are returned, and `mc_dir` itself
/// is left out. Called once per install: it stats a fixed list, never scans.
pub fn discover_roots(mc_dir: &Path) -> Vec<PathBuf> {
    let own = fs::canonicalize(mc_dir).ok();
    candidate_bases()
        .into_iter()
        .filter(|base| base.is_dir())
        .filter(|base| match (&own, fs::canonicalize(base).ok()) {
            (Some(own), Some(base)) => *own != base,
            _ => true,
        })
        .collect()
}

#[cfg(windows)]
fn candidate_bases() -> Vec<PathBuf> {
    let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) else {
        return Vec::new();
    };
    vec![
        // The official launcher.
        appdata.join(".minecraft"),
        // Horizon's own folder.
        appdata.join("horizon-launcher").join("minecraft"),
        // Prism Launcher keeps shared `libraries/` and `assets/` here.
        appdata.join("PrismLauncher"),
        // TLauncher.
        appdata.join(".tlauncher"),
    ]
}

#[cfg(not(windows))]
fn candidate_bases() -> Vec<PathBuf> {
    match std::env::var_os("HOME").map(PathBuf::from) {
        Some(home) => vec![home.join(".minecraft")],
        None => Vec::new(),
    }
}

/// Lowercase hex SHA-1 of a file, read in chunks. `None` if it can't be read.
pub fn sha1_file(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

/// The first of `candidates` that is a file whose SHA-1 equals `sha1`.
pub fn find_matching(candidates: &[PathBuf], sha1: &str) -> Option<PathBuf> {
    if sha1.len() != 40 {
        return None;
    }
    candidates
        .iter()
        .find(|path| {
            path.is_file()
                && sha1_file(path).is_some_and(|actual| actual.eq_ignore_ascii_case(sha1))
        })
        .cloned()
}

/// Looks for a verified copy of `expected` under `roots` and places it at
/// `dest`. Returns `true` only when the file was placed; `false` means the
/// caller should download as usual. Never returns an error, so a bad
/// candidate cannot fail an install.
pub fn reuse_into(roots: &[PathBuf], expected: &Expected, dest: &Path) -> bool {
    if roots.is_empty() {
        return false;
    }
    let candidates: Vec<PathBuf> = roots.iter().map(|r| r.join(&expected.rel)).collect();
    let Some(src) = find_matching(&candidates, &expected.sha1) else {
        return false;
    };
    place(&src, dest).is_ok()
}

/// Places `src` at `dest` atomically: hard-link (or copy when linking fails,
/// e.g. across volumes) to a temp sibling, then rename it into place.
fn place(src: &Path, dest: &Path) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension(match dest.extension() {
        Some(ext) => format!("{}.reuse.part", ext.to_string_lossy()),
        None => "reuse.part".to_string(),
    });
    let _ = fs::remove_file(&tmp);
    if fs::hard_link(src, &tmp).is_err() {
        if let Err(e) = fs::copy(src, &tmp) {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
    }
    fs::rename(&tmp, dest).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // SHA-1 of the bytes b"abc".
    const ABC_SHA1: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";

    fn write(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn sha1_file_matches_known_vector() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.bin");
        write(&file, b"abc");
        assert_eq!(sha1_file(&file).as_deref(), Some(ABC_SHA1));
    }

    #[test]
    fn hash_match_reuses_other_launcher_file() {
        let other = tempfile::tempdir().unwrap();
        let mine = tempfile::tempdir().unwrap();
        write(&other.path().join("libraries/com/x/y/1.0/y-1.0.jar"), b"abc");

        let roots = vec![other.path().to_path_buf()];
        let expected = Expected::new("libraries/com/x/y/1.0/y-1.0.jar", ABC_SHA1);
        let dest = mine.path().join("libraries/com/x/y/1.0/y-1.0.jar");

        assert!(reuse_into(&roots, &expected, &dest));
        assert_eq!(fs::read(&dest).unwrap(), b"abc");
        assert!(!dest.with_extension("jar.reuse.part").exists());
    }

    #[test]
    fn hash_mismatch_falls_back_to_download() {
        let other = tempfile::tempdir().unwrap();
        let mine = tempfile::tempdir().unwrap();
        write(&other.path().join("assets/objects/aa/aabbcc"), b"not the right bytes");

        let roots = vec![other.path().to_path_buf()];
        let expected = Expected::new("assets/objects/aa/aabbcc", ABC_SHA1);
        let dest = mine.path().join("assets/objects/aa/aabbcc");

        assert!(!reuse_into(&roots, &expected, &dest));
        assert!(!dest.exists());
    }

    #[test]
    fn selection_skips_mismatched_candidate_and_takes_matching_one() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        write(&a.path().join("versions/1.21/1.21.jar"), b"wrong");
        write(&b.path().join("versions/1.21/1.21.jar"), b"abc");
        let candidates = vec![
            a.path().join("versions/1.21/1.21.jar"),
            b.path().join("versions/1.21/1.21.jar"),
        ];
        assert_eq!(
            find_matching(&candidates, ABC_SHA1),
            Some(b.path().join("versions/1.21/1.21.jar"))
        );
    }

    #[test]
    fn no_hash_or_no_roots_never_reuses() {
        let other = tempfile::tempdir().unwrap();
        write(&other.path().join("libraries/a.jar"), b"abc");
        let roots = vec![other.path().to_path_buf()];
        let mine = tempfile::tempdir().unwrap();
        let dest = mine.path().join("libraries/a.jar");

        assert!(!reuse_into(&roots, &Expected::new("libraries/a.jar", ""), &dest));
        assert!(!reuse_into(&[], &Expected::new("libraries/a.jar", ABC_SHA1), &dest));
        assert!(!dest.exists());
    }

    #[test]
    fn discover_roots_excludes_own_folder() {
        let own = tempfile::tempdir().unwrap();
        // Whatever the platform's bases are, the launcher's own folder is never returned.
        assert!(!discover_roots(own.path()).contains(&own.path().to_path_buf()));
    }
}
