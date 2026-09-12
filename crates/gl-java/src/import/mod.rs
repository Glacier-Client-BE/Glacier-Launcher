//! External launcher import (Phase 5 of the NoRiskClient port).
//!
//! Detects existing MultiMC/Prism Launcher, CurseForge app, ATLauncher, and
//! Modrinth App installs on disk and copies their instances into Glacier as
//! new `JavaInstance`s, without touching the source installation.
//!
//! Module layout:
//! - [`detect`] — find likely install roots for each supported launcher.
//! - `mmc` / `curseforge_app` / `atlauncher` / `modrinth_app` — one adapter
//!   per launcher format, each producing an [`ImportCandidate`].
//! - [`loader_map`] — map each launcher's own loader-naming convention to
//!   [`crate::models::ModLoaderKind`].
//! - [`preview`] — build an `ImportCandidate` (mod/world counts, size) for a
//!   single detected instance without copying anything.
//! - [`pipeline`] — stage, copy, finalize (set loader), and roll back a
//!   single import.
//! - [`safety`] — pre-flight checks (id collision, free disk space).
//!
//! **Verification status:** these adapters are implemented from each
//! launcher's publicly documented on-disk format, not tested against a real
//! install of that launcher (none is available in this sandbox). See
//! `PARITY.md` for exactly which parts are verified vs best-effort.

pub mod atlauncher;
mod common;
pub mod curseforge_app;
pub mod detect;
pub mod loader_map;
pub mod mmc;
pub mod modrinth_app;
pub mod pipeline;
pub mod preview;
pub mod safety;

use crate::models::ModLoaderKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Which external launcher an [`ImportCandidate`] came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceLauncher {
    /// MultiMC and Prism Launcher share the same on-disk instance format.
    MultiMcOrPrism,
    CurseForgeApp,
    ATLauncher,
    ModrinthApp,
}

impl SourceLauncher {
    pub fn label(&self) -> &'static str {
        match self {
            SourceLauncher::MultiMcOrPrism => "MultiMC / Prism Launcher",
            SourceLauncher::CurseForgeApp => "CurseForge app",
            SourceLauncher::ATLauncher => "ATLauncher",
            SourceLauncher::ModrinthApp => "Modrinth App",
        }
    }
}

/// One instance found in an external launcher, ready to be previewed and
/// imported. Common shape produced by every adapter in this module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportCandidate {
    pub source_launcher: SourceLauncher,
    pub name: String,
    pub mc_version: String,
    #[serde(default)]
    pub loader: Option<ModLoaderKind>,
    #[serde(default)]
    pub loader_version: Option<String>,
    pub instance_path: PathBuf,
    #[serde(default)]
    pub mod_count: usize,
    #[serde(default)]
    pub world_count: usize,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub icon_path: Option<PathBuf>,
}

/// A detected external-launcher install root, as returned by [`detect::detect_all`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedLauncher {
    pub launcher: SourceLauncher,
    pub root_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_service::InstanceService;
    use std::fs;

    fn write(path: &std::path::Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn mmc_adapter_parses_a_fabric_instance() {
        let tmp = tempfile::tempdir().unwrap();
        let instances_root = tmp.path().join("instances");
        let inst_dir = instances_root.join("My Pack");
        write(&inst_dir.join("instance.cfg"), "InstanceType=OneSix\nname=My Pack\niconKey=default\n");
        write(
            &inst_dir.join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.fabricmc.fabric-loader","version":"0.15.7"}]}"#,
        );
        write(&inst_dir.join(".minecraft").join("mods").join("a.jar"), "x");

        let found = mmc::scan(&instances_root);
        assert_eq!(found.len(), 1);
        let c = &found[0];
        assert_eq!(c.name, "My Pack");
        assert_eq!(c.mc_version, "1.20.1");
        assert_eq!(c.loader, Some(ModLoaderKind::Fabric));
        assert_eq!(c.loader_version.as_deref(), Some("0.15.7"));
        assert_eq!(c.mod_count, 1);
    }

    #[test]
    fn curseforge_app_adapter_parses_forge_instance() {
        let tmp = tempfile::tempdir().unwrap();
        let instances_root = tmp.path().join("Instances");
        let inst_dir = instances_root.join("SkyFactory");
        write(
            &inst_dir.join("minecraftinstance.json"),
            r#"{"name":"SkyFactory","gameVersion":"1.20.1","baseModLoader":{"name":"forge-47.2.0"}}"#,
        );

        let found = curseforge_app::scan(&instances_root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].loader, Some(ModLoaderKind::Forge));
        assert_eq!(found[0].loader_version.as_deref(), Some("47.2.0"));
    }

    #[test]
    fn atlauncher_adapter_parses_neoforge_instance() {
        let tmp = tempfile::tempdir().unwrap();
        let instances_root = tmp.path().join("instances");
        let inst_dir = instances_root.join("Pack1");
        write(
            &inst_dir.join("instance.json"),
            r#"{"launcher":{"name":"Pack1","loaderVersion":{"type":"NeoForge","version":"21.1.0"}},"minecraftVersion":"1.21.1"}"#,
        );

        let found = atlauncher::scan(&instances_root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].mc_version, "1.21.1");
        assert_eq!(found[0].loader, Some(ModLoaderKind::NeoForge));
    }

    #[test]
    fn modrinth_app_adapter_parses_quilt_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_root = tmp.path().join("profiles");
        let inst_dir = profiles_root.join("My Profile");
        write(
            &inst_dir.join("profile.json"),
            r#"{"metadata":{"name":"My Profile","game_version":"1.20.4","loader":"quilt","loader_version":{"id":"0.23.1"}}}"#,
        );

        let found = modrinth_app::scan(&profiles_root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].loader, Some(ModLoaderKind::Quilt));
        assert_eq!(found[0].loader_version.as_deref(), Some("0.23.1"));
    }

    #[test]
    fn pipeline_rejects_name_collision_and_never_creates_a_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = InstanceService::new(tmp.path().join("java")).unwrap();
        // The default instance is named "Default".
        let source_dir = tmp.path().join("external-src");
        fs::create_dir_all(&source_dir).unwrap();
        let candidate = ImportCandidate {
            source_launcher: SourceLauncher::MultiMcOrPrism,
            name: "Default".to_string(),
            mc_version: "1.20.1".to_string(),
            loader: None,
            loader_version: None,
            instance_path: source_dir,
            mod_count: 0,
            world_count: 0,
            size_bytes: 0,
            icon_path: None,
        };
        let before = svc.instances().len();
        let err = pipeline::import_one(&svc, &candidate).unwrap_err();
        assert!(err.to_string().contains("already exists"));
        assert_eq!(svc.instances().len(), before);
    }

    #[test]
    fn pipeline_imports_and_copies_files() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = InstanceService::new(tmp.path().join("java")).unwrap();
        let source_dir = tmp.path().join("external-src");
        write(&source_dir.join("mods").join("cool.jar"), "jar-bytes");
        let candidate = ImportCandidate {
            source_launcher: SourceLauncher::ModrinthApp,
            name: "Imported One".to_string(),
            mc_version: "1.20.1".to_string(),
            loader: Some(ModLoaderKind::Fabric),
            loader_version: Some("0.15.7".to_string()),
            instance_path: source_dir,
            mod_count: 1,
            world_count: 0,
            size_bytes: 9,
            icon_path: None,
        };
        let created = pipeline::import_one(&svc, &candidate).expect("import should succeed");
        assert_eq!(created.loader, Some(ModLoaderKind::Fabric));
        let copied = std::path::Path::new(&created.directory).join("mods").join("cool.jar");
        assert!(copied.is_file());
    }
}
