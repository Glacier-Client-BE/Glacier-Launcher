//! Ported from `GlacierClientService.cs`: fetches and parses the Glacier
//! Client version manifest. Actual jar download/verification is delegated
//! to `gl-download` (see the crate-level TODO note); this module covers
//! fetching + parsing the manifest and the installed-size/path helpers.

use std::path::{Path, PathBuf};

use reqwest::Client;

use crate::error::{ClientsError, Result};
use crate::models::{GlacierClientVersion, GlacierManifest};

pub const MANIFEST_URL: &str = "https://cdn.glacierclient.xyz/versions.json";

/// Fetches and parses the Glacier Client version manifest. Mirrors
/// `GlacierClientService.GetManifestAsync` (caching is left to the caller).
pub async fn fetch_manifest(http: &Client) -> Result<GlacierManifest> {
    let body = http.get(MANIFEST_URL).send().await?.text().await?;
    parse_manifest(&body)
}

/// Parses a manifest JSON body without a network round trip.
pub fn parse_manifest(body: &str) -> Result<GlacierManifest> {
    serde_json::from_str(body).map_err(ClientsError::from)
}

/// `~/.glacier`. Mirrors `GlacierClientService.GetGlacierDir`.
pub fn glacier_dir(user_profile: &Path) -> PathBuf {
    user_profile.join(".glacier")
}

/// Sums the size of every installed `.jar` under `~/.glacier`. Mirrors
/// `GlacierClientService.GetInstalledSizeBytes`.
pub fn installed_size_bytes(user_profile: &Path) -> u64 {
    let dir = glacier_dir(user_profile);
    if !dir.is_dir() {
        return 0;
    }
    sum_jar_sizes(&dir)
}

fn sum_jar_sizes(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += sum_jar_sizes(&path);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("jar"))
            .unwrap_or(false)
        {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

/// Mirrors `GlacierClientService.LaunchAsync`'s precondition check (the
/// actual launch is `gl-java`'s concern, out of scope for this crate).
pub fn require_installed(version: &GlacierClientVersion, user_profile: &Path) -> Result<()> {
    if version.is_installed(user_profile) {
        Ok(())
    } else {
        Err(ClientsError::NotFound(format!(
            "Glacier {} is not installed.",
            version.id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST_JSON: &str = r#"
    {
        "schemaVersion": 1,
        "latestRelease": "1.2.0",
        "releaseDate": "2025-01-01",
        "versions": [
            {
                "id": "1.2.0",
                "name": "Glacier 1.2.0",
                "releaseDate": "2025-01-01",
                "tag": "v1.2.0",
                "url": "https://example.com/glacier-1.2.0.jar",
                "sha256": "deadbeef",
                "fabric": true,
                "forge": false,
                "javaVersion": 17,
                "changelog": "Initial"
            }
        ],
        "launcher": {
            "version": "2.0.0",
            "url": "https://example.com/launcher.exe",
            "sha256": "cafebabe"
        }
    }"#;

    #[test]
    fn parses_manifest_camel_case_fields() {
        let manifest = parse_manifest(MANIFEST_JSON).unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.latest_release, "1.2.0");
        assert_eq!(manifest.versions.len(), 1);

        let v = &manifest.versions[0];
        assert_eq!(v.id, "1.2.0");
        assert!(v.fabric);
        assert!(!v.forge);
        assert_eq!(v.java_version, 17);
        assert_eq!(v.loader_label(), "Fabric");
        assert_eq!(v.local_jar_name(), "Glacier-1.2.0.jar");

        let launcher = manifest.launcher.unwrap();
        assert_eq!(launcher.version, "2.0.0");
    }

    #[test]
    fn install_dir_and_jar_path_layout() {
        let v = GlacierClientVersion {
            id: "1.2.0".into(),
            ..Default::default()
        };
        let profile = Path::new("C:/Users/Test");
        assert_eq!(
            v.install_dir(profile),
            profile.join(".glacier").join("versions").join("1.2.0")
        );
        assert_eq!(
            v.jar_path(profile),
            profile
                .join(".glacier")
                .join("versions")
                .join("1.2.0")
                .join("Glacier-1.2.0.jar")
        );
    }

    #[test]
    fn require_installed_errors_when_jar_missing() {
        let v = GlacierClientVersion {
            id: "1.2.0".into(),
            ..Default::default()
        };
        let dir = tempfile::tempdir().unwrap();
        assert!(require_installed(&v, dir.path()).is_err());
    }

    #[test]
    fn require_installed_succeeds_when_jar_present() {
        let v = GlacierClientVersion {
            id: "1.2.0".into(),
            ..Default::default()
        };
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(v.install_dir(dir.path())).unwrap();
        std::fs::write(v.jar_path(dir.path()), b"jar bytes").unwrap();
        assert!(require_installed(&v, dir.path()).is_ok());
    }

    #[test]
    fn installed_size_sums_jars_recursively() {
        let dir = tempfile::tempdir().unwrap();
        let glacier = dir.path().join(".glacier").join("versions").join("a");
        std::fs::create_dir_all(&glacier).unwrap();
        std::fs::write(glacier.join("a.jar"), vec![0u8; 10]).unwrap();
        std::fs::write(glacier.join("readme.txt"), vec![0u8; 999]).unwrap();

        assert_eq!(installed_size_bytes(dir.path()), 10);
    }

    #[test]
    fn installed_size_is_zero_when_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(installed_size_bytes(dir.path()), 0);
    }
}
