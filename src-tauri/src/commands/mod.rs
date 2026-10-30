//! Thin #[tauri::command] wrappers. Business logic lives in the gl-* crates;
//! commands here only deserialize args, call into a crate, and map errors.

pub mod ai;
pub mod auth;
pub mod bedrock;
pub mod bedrock_addons;
pub mod client_library;
pub mod clients;
pub mod downloads;
pub mod glacier_client;
pub mod injection;
pub mod integrations;
pub mod java;
pub mod java_loaders;
pub mod java_mods;
pub mod java_packs;
pub mod java_process;
pub mod java_screenshots;
pub mod java_worlds;
pub mod launch;
pub mod launcher_import;
pub mod launchers;
pub mod leveldat;
pub mod levilamina_mods;
pub mod logs;
pub mod modpack;
pub mod notifications;
pub mod servers;
pub mod settings;
pub mod shaders;
pub mod skins;
pub mod sync_packs;
pub mod system;
pub mod theme;
pub mod tweaks;
pub mod update;
pub mod utilities;
pub mod versions;

/// Rejects remote- or frontend-supplied file names that would escape the directory they are joined onto.
pub fn safe_file_name(name: &str) -> Result<&str, String> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':'])
        || name.chars().any(|c| c.is_control());
    if bad {
        Err(format!("Refusing unsafe file name: {name:?}"))
    } else {
        Ok(name)
    }
}

/// Returns the canonical form of `target` only when it is a direct child of
/// `root` (after resolving symlinks and `..`). Used before any command deletes
/// a path that the frontend supplied, so a crafted or stale path can't reach
/// outside the folder the feature actually manages.
pub fn ensure_direct_child(root: &std::path::Path, target: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let refuse = || format!("Refusing to touch {}: it is not inside the expected folder.", target.display());
    let root_c = root.canonicalize().map_err(|_| refuse())?;
    let target_c = target.canonicalize().map_err(|_| refuse())?;
    if target_c.parent() == Some(root_c.as_path()) {
        Ok(target_c)
    } else {
        Err(refuse())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("gl-mod-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("root").join("sub")).unwrap();
        std::fs::write(d.join("root").join("a.json"), "{}").unwrap();
        std::fs::write(d.join("outside.json"), "{}").unwrap();
        d
    }

    #[test]
    fn safe_file_name_rejects_traversal() {
        for bad in ["..", ".", "", "a/b", r"a\b", "C:x", "x\u{0}"] {
            assert!(safe_file_name(bad).is_err(), "{bad:?} should be rejected");
        }
        assert_eq!(safe_file_name("Latite_1.2.dll").unwrap(), "Latite_1.2.dll");
    }

    #[test]
    fn ensure_direct_child_accepts_child_and_rejects_outside() {
        let d = scratch("child");
        let root = d.join("root");
        assert!(ensure_direct_child(&root, &root.join("a.json")).is_ok());
        assert!(ensure_direct_child(&root, &d.join("outside.json")).is_err());
        assert!(ensure_direct_child(&root, &root.join("sub")).is_ok());
        assert!(ensure_direct_child(&root, &root.join("sub").join("..").join("..").join("outside.json")).is_err());
        assert!(ensure_direct_child(&root, &root.join("missing.json")).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
