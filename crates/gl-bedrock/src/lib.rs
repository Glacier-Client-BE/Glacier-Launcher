//! Bedrock Edition instance/world/pack/backup/screenshot/options/shader
//! management, ported from the C# `Services/Bedrock*.cs` + `ShaderService.cs`
//! files.
//!
//! One subsystem from the original C# codebase is **explicitly out of
//! scope** for this crate and is not ported here:
//! - `BedrockPackageService.cs` — MSIX package-family resolution via WinRT.
//!
//! The Windows Update SOAP protocol client (the WU-facing half of the C#
//! `Services/VanillaVersionService.cs`, used to enumerate/download vanilla
//! Bedrock builds from `fe3.delivery.mp.microsoft.com`) **is** ported here,
//! in [`wu`].
//!
//! [`shader::ShaderService`] depends on the same MSIX package resolution
//! (to find the installed Minecraft build's materials folder) and so takes
//! that path as a parameter instead of resolving it itself — see that
//! module's docs for details. Likewise, com.mojang root-folder resolution
//! (the C# `CurseForgeService.ResolveBestRootFor`, which walks per-account
//! package folders) is a cross-cutting concern outside this crate; every
//! service here takes its filesystem roots as constructor parameters so it
//! can be driven by whatever resolves those roots elsewhere, and so it's
//! independently testable against temp directories. The resolver itself now
//! lives in [`paths`] (the port of the C# `CurseForgeService` static path
//! members), so callers that need the *live* game install can get real roots
//! from there instead of joining literal subfolders.

pub mod backup;
pub mod community_db;
pub mod error;
pub mod gdk;
pub mod import;
pub mod instance;
pub mod model;
pub mod options;
pub mod pack;
pub mod paths;
pub mod screenshot;
pub mod shader;
mod util;
pub mod world;
pub mod wu;

pub use backup::{BackupDiff, BedrockBackupService, BedrockRoot, MANAGED_FOLDERS};
pub use error::{BedrockError, Result};
pub use import::{import_content, ImportOutcome};
pub use instance::BedrockInstanceService;
pub use model::{
    AppliedShader, BedrockBackup, BedrockInstance, BedrockPack, BedrockWorld, OptionEntry,
    OptionKind, PackKind, ScreenshotFile,
};
pub use options::BedrockOptionsService;
pub use pack::{BedrockPackService, PackDirs};
pub use paths::{
    com_mojang_root, live_dir_for, live_root_for, resolve_best_root_for, resolve_best_root_in,
    screenshots_dir, users_root, worlds_dir,
};
pub use screenshot::BedrockScreenshotService;
pub use shader::ShaderService;
pub use world::BedrockWorldService;
pub use wu::WuVersion;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gl-bedrock-test-{name}-{}-{}",
            std::process::id(),
            now_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn now_nanos() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    // ---- instance service ----

    #[test]
    fn instance_create_list_rename_delete_roundtrip() {
        let root = temp_dir("instances");

        let mut svc = BedrockInstanceService::new(&root, None).unwrap();
        // A "Default" instance is created automatically.
        assert_eq!(svc.instances().len(), 1);
        assert_eq!(svc.active_instance().unwrap().name, "Default");

        let second = svc.create("My Second Instance", "1.21.0").unwrap();
        assert_eq!(second.id, "my-second-instance");
        assert_eq!(svc.instances().len(), 2);

        svc.rename(&second.id, "Renamed").unwrap();
        assert_eq!(svc.instances()[1].name, "Renamed");

        svc.set_version(&second.id, "1.22.0").unwrap();
        assert_eq!(svc.instances()[1].version_tag, "1.22.0");

        // Can't delete the active instance.
        let default_id = svc.instances()[0].id.clone();
        assert!(!svc.delete(&default_id).unwrap());

        // Can delete a non-active instance.
        assert!(svc.delete(&second.id).unwrap());
        assert_eq!(svc.instances().len(), 1);

        // Can't delete the last remaining instance.
        assert!(!svc.delete(&default_id).unwrap());

        // Reload from disk picks up the same state.
        let reloaded = BedrockInstanceService::new(&root, None).unwrap();
        assert_eq!(reloaded.instances().len(), 1);
    }

    #[test]
    fn instance_ids_deduplicate_on_slug_collision() {
        let root = temp_dir("instance-dedup");
        let mut svc = BedrockInstanceService::new(&root, None).unwrap();

        let a = svc.create("Test", "").unwrap();
        let b = svc.create("Test", "").unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(a.id, "test");
        assert_eq!(b.id, "test-2");
    }

    #[test]
    fn instance_switch_swaps_live_folders() {
        let root = temp_dir("instance-switch");
        let live = temp_dir("instance-switch-live");

        // Seed the "live" com.mojang root with a world.
        let live_world_dir = live.join("minecraftWorlds").join("MyWorld");
        fs::create_dir_all(&live_world_dir).unwrap();
        fs::write(live_world_dir.join("level.dat"), b"live-data").unwrap();

        let mut svc = BedrockInstanceService::new(&root, Some(&live)).unwrap();
        // Default instance should have captured the seeded world.
        let default_instance = svc.active_instance().unwrap();
        let captured = std::path::Path::new(&default_instance.directory)
            .join("minecraftWorlds")
            .join("MyWorld")
            .join("level.dat");
        assert!(captured.is_file());

        let second = svc.create("Second", "").unwrap();
        assert!(svc.switch_to(&second.id, &live).unwrap());
        assert_eq!(svc.active_instance().unwrap().id, second.id);

        // Live root's world folder should now reflect the (empty) second
        // instance — the old world was archived into the default instance,
        // not left behind in the live root.
        assert!(!live_world_dir.exists());

        // Switching back restores it.
        assert!(svc.switch_to(&default_instance.id, &live).unwrap());
        assert!(live_world_dir.join("level.dat").is_file());
    }

    // ---- world service ----

    #[test]
    fn world_service_lists_and_exports() {
        let worlds_dir = temp_dir("worlds");
        let world_dir = worlds_dir.join("world1");
        fs::create_dir_all(world_dir.join("db")).unwrap();
        fs::write(world_dir.join("levelname.txt"), "My Cool World").unwrap();
        fs::write(world_dir.join("db").join("chunk.bin"), b"chunkdata").unwrap();

        // A directory with neither db/ nor level.dat isn't a world.
        fs::create_dir_all(worlds_dir.join("not-a-world")).unwrap();

        let svc = BedrockWorldService::new(&worlds_dir);
        let worlds = svc.list_worlds().unwrap();
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].name, "My Cool World");
        assert!(worlds[0].size_bytes > 0);

        let export_dir = temp_dir("worlds-export");
        let exported = svc.export_world(&world_dir, &export_dir).unwrap();
        assert!(exported.is_file());
        assert_eq!(
            exported.file_name().unwrap().to_str().unwrap(),
            "My Cool World.mcworld"
        );

        // Exporting again disambiguates with a numeric suffix.
        let exported2 = svc.export_world(&world_dir, &export_dir).unwrap();
        assert_ne!(exported, exported2);

        svc.delete_world(&world_dir).unwrap();
        assert!(!world_dir.exists());
    }

    // ---- pack service ----

    #[test]
    fn pack_service_lists_by_manifest_name() {
        let dir = temp_dir("packs");
        let pack_dir = dir.join("some_pack_folder");
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(
            pack_dir.join("manifest.json"),
            r#"{"header":{"name":"Cool Resource Pack"}}"#,
        )
        .unwrap();
        fs::write(pack_dir.join("pack.png"), vec![0u8; 10]).unwrap();

        let svc = BedrockPackService::new();
        let packs = svc.list_packs(&dir, PackKind::Resource).unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].name, "Cool Resource Pack");
        assert_eq!(packs[0].kind, "resource");

        svc.delete_pack(&pack_dir).unwrap();
        assert!(!pack_dir.exists());
    }

    // ---- backup service ----

    #[test]
    fn backup_create_and_restore_roundtrip() {
        let backups_dir = temp_dir("backups");
        let source_root = temp_dir("backup-source");
        let world_dir = source_root.join("minecraftWorlds").join("W1");
        fs::create_dir_all(&world_dir).unwrap();
        fs::write(world_dir.join("level.dat"), b"hello").unwrap();

        let svc = BedrockBackupService::new(&backups_dir);
        let backup_path = svc
            .create_backup_from_folder(&source_root, Some("label"))
            .unwrap();
        assert!(backup_path.is_file());

        let listed = svc.list_backups().unwrap();
        assert_eq!(listed.len(), 1);

        // Restoring into a fresh root should recreate the file.
        let restore_root = temp_dir("backup-restore");
        svc.restore_backup_into_folder(&backup_path, &restore_root)
            .unwrap();
        let restored_file = restore_root
            .join("minecraftWorlds")
            .join("W1")
            .join("level.dat");
        assert_eq!(fs::read(restored_file).unwrap(), b"hello");

        svc.delete_backup(&backup_path).unwrap();
        assert!(!backup_path.exists());
    }

    #[test]
    fn backup_with_nothing_present_errors() {
        let backups_dir = temp_dir("backups-empty");
        let source_root = temp_dir("backup-empty-source");
        let svc = BedrockBackupService::new(&backups_dir);
        let err = svc
            .create_backup_from_folder(&source_root, None)
            .unwrap_err();
        assert!(matches!(err, BedrockError::NothingToBackUp));
    }

    #[test]
    fn restore_never_overwrites_without_a_prior_backup_safety_copy() {
        // Safety property: restoring into a live folder always goes through
        // BedrockBackupService, which the instance service also uses to
        // snapshot the *current* state (via `backup_instance`) before a
        // destructive switch. Here we verify that a backup created from an
        // instance's own directory can be restored back into that same
        // directory without data loss, i.e. the backup-then-restore path
        // is safe to use as a "back up before mutation" step.
        let root = temp_dir("safety-instances");
        let svc = BedrockInstanceService::new(&root, None).unwrap();
        let instance = svc.active_instance().unwrap();

        let world_dir = std::path::Path::new(&instance.directory)
            .join("minecraftWorlds")
            .join("W");
        fs::create_dir_all(&world_dir).unwrap();
        fs::write(world_dir.join("level.dat"), b"precious").unwrap();

        let backups_dir = temp_dir("safety-backups");
        let backups = BedrockBackupService::new(&backups_dir);
        let backup_path = svc.backup_instance(&instance, &backups).unwrap();
        assert!(backup_path.is_file());

        // Simulate destructive mutation.
        fs::write(world_dir.join("level.dat"), b"corrupted").unwrap();

        svc.restore_into_instance(&backup_path, &instance, &backups)
            .unwrap();
        assert_eq!(fs::read(world_dir.join("level.dat")).unwrap(), b"precious");
    }

    // ---- screenshot service ----

    #[test]
    fn screenshot_service_lists_from_both_sources() {
        let com_mojang_shots = temp_dir("shots-cm");
        fs::write(com_mojang_shots.join("a.jpeg"), vec![1u8; 4]).unwrap();
        fs::write(com_mojang_shots.join("ignored.txt"), vec![1u8; 4]).unwrap();

        let captures = temp_dir("shots-captures");
        fs::write(captures.join("2024-minecraft-01.png"), vec![1u8; 4]).unwrap();
        fs::write(captures.join("other-game.png"), vec![1u8; 4]).unwrap();

        let svc = BedrockScreenshotService::new(&com_mojang_shots, &captures);
        let shots = svc.list_screenshots().unwrap();
        assert_eq!(shots.len(), 2);
        assert!(shots.iter().any(|s| s.name == "a.jpeg"));
        assert!(shots.iter().any(|s| s.name == "2024-minecraft-01.png"));

        let target = com_mojang_shots.join("a.jpeg");
        svc.delete_file(&target).unwrap();
        assert!(!target.exists());
    }

    // ---- options service ----

    #[test]
    fn options_service_load_save_and_restore() {
        let dir = temp_dir("options");
        let options_path = dir.join("options.txt");
        fs::write(
            &options_path,
            "gfx_vsync:0\ngfx_viewdistance:16\ncustom_key:hello\n",
        )
        .unwrap();

        let svc = BedrockOptionsService::new(&options_path);
        let entries = svc.load().unwrap();
        assert_eq!(entries.len(), 3);
        let vsync = entries.iter().find(|e| e.key == "gfx_vsync").unwrap();
        assert_eq!(vsync.label, "V-Sync");
        assert!(matches!(vsync.kind, OptionKind::Toggle));
        assert!(!vsync.is_on());

        svc.set_vsync(true).unwrap();
        assert!(svc.is_vsync_enabled().unwrap());
        assert!(svc.has_backup());

        let mut changes = HashMap::new();
        changes.insert("brand_new_key".to_string(), "42".to_string());
        svc.save(&changes).unwrap();
        assert_eq!(svc.get("brand_new_key").unwrap().as_deref(), Some("42"));

        // Restoring the backup brings back the pre-change vsync value.
        assert!(svc.restore_backup().unwrap());
        assert_eq!(svc.get("gfx_vsync").unwrap().as_deref(), Some("0"));
    }

    // ---- shader service ----

    #[test]
    fn shader_service_apply_and_restore_roundtrip() {
        let shader_root = temp_dir("shaders");
        let materials_dir = temp_dir("shaders-materials");

        let stock_material = materials_dir.join("water.material.bin");
        fs::write(&stock_material, b"stock-bytes").unwrap();

        let source_dir = temp_dir("shaders-source");
        fs::write(source_dir.join("water.material.bin"), b"shader-bytes").unwrap();

        let svc = ShaderService::new(&shader_root);
        let installed = svc
            .apply(&materials_dir, std::slice::from_ref(&source_dir))
            .unwrap();
        assert_eq!(installed, vec!["water.material.bin".to_string()]);
        assert_eq!(fs::read(&stock_material).unwrap(), b"shader-bytes");
        assert!(svc.has_applied_shaders());

        let reverted = svc.restore(&materials_dir).unwrap();
        assert_eq!(reverted, 1);
        assert_eq!(fs::read(&stock_material).unwrap(), b"stock-bytes");
        assert!(!svc.has_applied_shaders());
    }

    #[test]
    fn shader_service_lists_stock_materials() {
        let shader_root = temp_dir("shaders-list");
        let materials_dir = temp_dir("shaders-list-materials");
        fs::write(materials_dir.join("a.material.bin"), b"1").unwrap();
        fs::write(materials_dir.join("b.material.bin"), b"2").unwrap();
        fs::write(materials_dir.join("not-a-material.txt"), b"3").unwrap();

        let svc = ShaderService::new(&shader_root);
        let list = svc.list_stock_materials(&materials_dir);
        assert_eq!(
            list,
            vec!["a.material.bin".to_string(), "b.material.bin".to_string()]
        );
    }
}
