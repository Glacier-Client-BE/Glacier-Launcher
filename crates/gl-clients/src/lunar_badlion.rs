//! Ported from `LunarBadlionService.cs`: detects local Lunar Client /
//! Badlion Client installs and enumerates Lunar's offline multiversion
//! game files.
//!
//! Deviation from the C# original: `LunarBadlionService.ReadUninstallExe`
//! falls back to scanning the Windows uninstall registry keys
//! (`HKCU`/`HKLM\...\Uninstall`) for a `DisplayIcon` path when the exe
//! isn't found at a well-known location. This crate has no registry
//! dependency in scope (none of the pre-approved crates provide one), so
//! only the well-known-path candidates are checked here. Wire a registry
//! lookup back in at the `gl-integrations` or Tauri host layer if that
//! fallback turns out to matter in practice.
//!
//! Actually launching a detected client (spawning the exe, or the
//! direct-JRE-launch path) is left to the caller — this module only
//! reports what's installed and where.

use std::path::{Path, PathBuf};

/// What was found on disk for Lunar/Badlion. Mirrors the detection fields
/// of `LunarBadlionService`.
#[derive(Debug, Clone, Default)]
pub struct LunarBadlionInfo {
    pub lunar_exe_path: Option<PathBuf>,
    pub badlion_exe_path: Option<PathBuf>,
    pub lunar_offline_root: Option<PathBuf>,
    pub lunar_jre_path: Option<PathBuf>,
    /// Installed Lunar offline version folder names, newest-looking first
    /// (mirrors the C# `OrderByDescending(n => n)` string sort).
    pub lunar_installed_versions: Vec<String>,
}

impl LunarBadlionInfo {
    pub fn lunar_installed(&self) -> bool {
        self.lunar_exe_path.is_some()
    }

    pub fn badlion_installed(&self) -> bool {
        self.badlion_exe_path.is_some()
    }

    /// Mirrors `LunarBadlionService.CanDirectLaunchLunar`.
    pub fn can_direct_launch_lunar(&self) -> bool {
        self.lunar_jre_path.is_some() && !self.lunar_installed_versions.is_empty()
    }
}

/// Runs detection given the user's `LOCALAPPDATA` and `USERPROFILE`
/// directories (passed in explicitly rather than read from the
/// environment, so this is testable without touching the real filesystem
/// layout). Mirrors `LunarBadlionService.Detect`.
pub fn detect(local_app_data: &Path, user_profile: &Path) -> LunarBadlionInfo {
    let mut info = LunarBadlionInfo {
        lunar_exe_path: find_lunar(local_app_data),
        badlion_exe_path: find_badlion(local_app_data),
        ..Default::default()
    };

    let offline_root = user_profile
        .join(".lunarclient")
        .join("offline")
        .join("multiver");
    if offline_root.is_dir() {
        info.lunar_installed_versions = list_version_dirs(&offline_root);
        info.lunar_offline_root = Some(offline_root);
    }

    info.lunar_jre_path = find_lunar_jre(user_profile);
    info
}

fn find_lunar(local_app_data: &Path) -> Option<PathBuf> {
    let candidates = [
        local_app_data
            .join("Programs")
            .join("lunarclient")
            .join("Lunar Client.exe"),
        local_app_data
            .join("Programs")
            .join("Lunar Client")
            .join("Lunar Client.exe"),
        local_app_data.join("lunarclient").join("Lunar Client.exe"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

fn find_badlion(local_app_data: &Path) -> Option<PathBuf> {
    let candidates = [
        local_app_data
            .join("Programs")
            .join("Badlion Client")
            .join("Badlion Client.exe"),
        local_app_data
            .join("Badlion Client")
            .join("Badlion Client.exe"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

fn list_version_dirs(offline_root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(offline_root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    names.sort_by(|a, b| b.cmp(a));
    names
}

fn find_lunar_jre(user_profile: &Path) -> Option<PathBuf> {
    let jre_root = user_profile.join(".lunarclient").join("jre");
    if !jre_root.is_dir() {
        return None;
    }
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in walk_files(&jre_root) {
        if entry
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case("javaw.exe"))
            .unwrap_or(false)
        {
            if let Ok(meta) = std::fs::metadata(&entry) {
                if let Ok(modified) = meta.modified() {
                    if best.as_ref().map(|(_, t)| modified > *t).unwrap_or(true) {
                        best = Some((entry, modified));
                    }
                }
            }
        }
    }
    best.map(|(p, _)| p)
}

fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_files(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// Finds the `.jar` files and `natives` directory for a Lunar offline
/// version, if that version is present. Mirrors the file-discovery half of
/// `LunarBadlionService.TryDirectLaunch` (spawning the JVM process itself
/// is a `gl-java`/host concern, not this crate's).
pub fn resolve_direct_launch(
    info: &LunarBadlionInfo,
    version_id: &str,
) -> Option<DirectLaunchFiles> {
    let root = info.lunar_offline_root.as_ref()?;
    let jre = info.lunar_jre_path.as_ref()?;

    let version_dir = root.join(version_id);
    if !version_dir.is_dir() {
        return None;
    }

    let jars: Vec<PathBuf> = std::fs::read_dir(&version_dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("jar"))
                .unwrap_or(false)
        })
        .collect();
    if jars.is_empty() {
        return None;
    }

    Some(DirectLaunchFiles {
        jre_path: jre.clone(),
        version_dir: version_dir.clone(),
        natives_dir: version_dir.join("natives"),
        jars,
    })
}

#[derive(Debug, Clone)]
pub struct DirectLaunchFiles {
    pub jre_path: PathBuf,
    pub version_dir: PathBuf,
    pub natives_dir: PathBuf,
    pub jars: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_lunar_exe_at_known_path() {
        let dir = tempfile::tempdir().unwrap();
        let local_app_data = dir.path().join("LocalAppData");
        let user_profile = dir.path().join("Profile");
        let lunar_dir = local_app_data.join("Programs").join("lunarclient");
        std::fs::create_dir_all(&lunar_dir).unwrap();
        std::fs::write(lunar_dir.join("Lunar Client.exe"), b"").unwrap();

        let info = detect(&local_app_data, &user_profile);
        assert!(info.lunar_installed());
        assert!(!info.badlion_installed());
    }

    #[test]
    fn lists_offline_versions_sorted_descending() {
        let dir = tempfile::tempdir().unwrap();
        let local_app_data = dir.path().join("LocalAppData");
        let user_profile = dir.path().join("Profile");
        let multiver = user_profile
            .join(".lunarclient")
            .join("offline")
            .join("multiver");
        std::fs::create_dir_all(multiver.join("1.8")).unwrap();
        std::fs::create_dir_all(multiver.join("1.16")).unwrap();
        std::fs::create_dir_all(multiver.join("1.7")).unwrap();

        let info = detect(&local_app_data, &user_profile);
        assert_eq!(info.lunar_installed_versions, vec!["1.8", "1.7", "1.16"]);
    }

    #[test]
    fn no_installs_found_when_nothing_present() {
        let dir = tempfile::tempdir().unwrap();
        let info = detect(&dir.path().join("lad"), &dir.path().join("profile"));
        assert!(!info.lunar_installed());
        assert!(!info.badlion_installed());
        assert!(!info.can_direct_launch_lunar());
    }

    #[test]
    fn resolves_direct_launch_files_when_version_present() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("multiver");
        let version_dir = root.join("1.16");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(version_dir.join("game.jar"), b"").unwrap();

        let jre = dir.path().join("javaw.exe");
        std::fs::write(&jre, b"").unwrap();

        let info = LunarBadlionInfo {
            lunar_offline_root: Some(root),
            lunar_jre_path: Some(jre.clone()),
            lunar_installed_versions: vec!["1.16".into()],
            ..Default::default()
        };

        let launch = resolve_direct_launch(&info, "1.16").unwrap();
        assert_eq!(launch.jars.len(), 1);
        assert_eq!(launch.jre_path, jre);
    }

    #[test]
    fn resolve_direct_launch_none_when_version_missing() {
        let info = LunarBadlionInfo::default();
        assert!(resolve_direct_launch(&info, "1.16").is_none());
    }
}
