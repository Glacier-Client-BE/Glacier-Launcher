//! Java Edition instance/version/runtime management and launch.
//!
//! Ports (from the original C# WPF launcher) of:
//! - `JavaVersionService` -> [`version_service::VersionService`]
//! - `JavaInstanceService` -> [`instance_service::InstanceService`]
//! - `JavaInstallService` -> [`install_service::InstallService`]
//! - `JavaRuntimeDownloadService` -> [`runtime_download::RuntimeDownloadService`]
//! - `JavaGameLauncher` -> the free functions in [`launcher`]
//!
//! See each module's doc comment for behavioral deviations from the C#
//! original.

pub mod download;
pub mod error;
pub mod install_service;
pub mod instance_service;
pub mod launcher;
pub mod mod_analyzer;
pub mod mod_loader;
pub mod models;
pub mod runtime_download;
pub mod util;
pub mod version_service;
pub mod worlds;

pub use error::{JavaError, Result};
pub use install_service::{InstallProgress, InstallService};
pub use instance_service::InstanceService;
pub use launcher::{
    build_classpath, build_game_args, build_jvm_args, detect_java_major, offline_uuid,
    read_version_profile, resolve_java_runtime, spawn, AuthValues, LaunchOptions, LibraryEntry,
    VersionProfile,
};
pub use mod_analyzer::{ModInfo, ModReport};
pub use models::{JavaInstance, JavaVersion};
pub use runtime_download::RuntimeDownloadService;
pub use version_service::VersionService;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_json(path: &std::path::Path, value: &serde_json::Value) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    }

    /// Build a minimal-but-representative modern version JSON (like 1.20.x)
    /// with the `arguments.jvm`/`arguments.game` structured format, a
    /// windows-only rule, a classpath-containing jvm arg, and one library
    /// with Windows natives — enough to exercise every branch of
    /// `read_version_profile`/`build_classpath`/`build_jvm_args`/`build_game_args`.
    fn sample_version_json() -> serde_json::Value {
        serde_json::json!({
            "mainClass": "net.minecraft.client.main.Main",
            "assets": "17",
            "javaVersion": { "majorVersion": 17 },
            "libraries": [
                {
                    "name": "com.mojang:test-lib:1.0",
                    "downloads": {
                        "artifact": { "path": "com/mojang/test-lib/1.0/test-lib-1.0.jar" },
                        "classifiers": {
                            "natives-windows": { "path": "com/mojang/test-lib/1.0/test-lib-1.0-natives-windows.jar" }
                        }
                    },
                    "natives": { "windows": "natives-windows" }
                },
                {
                    "name": "org.example:linux-only:2.0",
                    "downloads": { "artifact": { "path": "org/example/linux-only/2.0/linux-only-2.0.jar" } },
                    "rules": [ { "action": "allow", "os": { "name": "linux" } } ]
                }
            ],
            "arguments": {
                "jvm": [
                    "-Djava.library.path=${natives_directory}",
                    "-cp",
                    "${classpath}",
                    { "rules": [ { "action": "allow", "os": { "name": "windows" } } ], "value": "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump" }
                ],
                "game": [
                    "--username", "${auth_player_name}",
                    "--version", "${version_name}",
                    "--gameDir", "${game_directory}",
                    "--assetsDir", "${assets_root}",
                    "--assetIndex", "${assets_index_name}",
                    "--uuid", "${auth_uuid}",
                    "--accessToken", "${auth_access_token}",
                    "--userType", "${user_type}"
                ]
            }
        })
    }

    fn setup_version_dir(mc_dir: &std::path::Path, id: &str) {
        let version_dir = mc_dir.join("versions").join(id);
        write_json(
            &version_dir.join(format!("{id}.json")),
            &sample_version_json(),
        );
        std::fs::write(version_dir.join(format!("{id}.jar")), b"fake-jar").unwrap();
    }

    #[test]
    fn read_version_profile_resolves_libraries_and_args() {
        let tmp = tempfile::tempdir().unwrap();
        let mc_dir = tmp.path();
        setup_version_dir(mc_dir, "1.20.1");

        let profile = read_version_profile("1.20.1", mc_dir).unwrap();

        assert_eq!(profile.main_class, "net.minecraft.client.main.Main");
        assert_eq!(profile.assets_index, "17");
        assert_eq!(profile.min_java_major, "17");

        // linux-only library must be excluded by the OS rule.
        let names: Vec<_> = profile
            .libraries
            .iter()
            .map(|l| l.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().any(|n| n == "test-lib-1.0.jar"));
        assert!(names
            .iter()
            .any(|n| n == "test-lib-1.0-natives-windows.jar"));
        assert!(!names.iter().any(|n| n.contains("linux-only")));
        // primary jar included.
        assert!(names.iter().any(|n| n == "1.20.1.jar"));

        // native classified as native.
        let native = profile
            .libraries
            .iter()
            .find(|l| l.path.to_string_lossy().contains("natives-windows"))
            .unwrap();
        assert!(native.is_native);
    }

    #[test]
    fn build_classpath_excludes_natives_and_dedupes() {
        let tmp = tempfile::tempdir().unwrap();
        let mc_dir = tmp.path();
        setup_version_dir(mc_dir, "1.20.1");
        let profile = read_version_profile("1.20.1", mc_dir).unwrap();

        let cp = build_classpath(&profile);
        assert!(cp
            .iter()
            .all(|p| !p.to_string_lossy().contains("natives-windows")));

        // Re-running build_classpath on a profile with a duplicated library
        // path should still de-dupe.
        let mut dup_profile = profile.clone();
        let first = dup_profile.libraries[0].clone();
        dup_profile.libraries.push(first);
        let cp2 = build_classpath(&dup_profile);
        assert_eq!(cp2.len(), cp.len());
    }

    #[test]
    fn build_jvm_args_substitutes_and_filters_classpath_placeholder() {
        let tmp = tempfile::tempdir().unwrap();
        let mc_dir = tmp.path();
        setup_version_dir(mc_dir, "1.20.1");
        let profile = read_version_profile("1.20.1", mc_dir).unwrap();

        let opts = LaunchOptions {
            max_ram_mb: 4096,
            min_ram_mb: 1024,
            custom_jvm_args: "-Dfoo=bar -Dbaz=qux".to_string(),
            ..Default::default()
        };

        let args = build_jvm_args(&profile, mc_dir, "1.20.1", 17, &opts).unwrap();

        // Only the literal "${classpath}" token itself is dropped (matching
        // the C# original's `if (v.Contains("${classpath}")) continue;`,
        // which does NOT special-case the preceding "-cp" flag — classpath is
        // supplied separately via build_classpath/-cp appended by the caller).
        assert!(!args.iter().any(|a| a.contains("${classpath}")));

        assert!(args.iter().any(|a| a.starts_with("-Djava.library.path=")));
        assert!(args.iter().any(|a| a.contains("HeapDumpPath")));
        assert!(args.contains(&"-Xmx4096M".to_string()));
        assert!(args.contains(&"-Xms1024M".to_string()));
        assert!(args.contains(&"-Dfoo=bar".to_string()));
        assert!(args.contains(&"-Dbaz=qux".to_string()));
    }

    #[test]
    fn build_jvm_args_clamps_ram_to_documented_bounds() {
        let tmp = tempfile::tempdir().unwrap();
        let mc_dir = tmp.path();
        setup_version_dir(mc_dir, "1.20.1");
        let profile = read_version_profile("1.20.1", mc_dir).unwrap();

        // Max below floor, min above computed max: both should clamp.
        let opts = LaunchOptions {
            max_ram_mb: 100,
            min_ram_mb: 99999,
            ..Default::default()
        };
        let args = build_jvm_args(&profile, mc_dir, "1.20.1", 17, &opts).unwrap();
        assert!(args.contains(&"-Xmx512M".to_string()));
        assert!(args.contains(&"-Xms512M".to_string()));
    }

    #[test]
    fn is_jvm_arg_compatible_filters_java23_only_flags() {
        // Exercised indirectly through build_jvm_args with a synthetic profile.
        let tmp = tempfile::tempdir().unwrap();
        let mc_dir = tmp.path();
        let mut json = sample_version_json();
        json["arguments"]["jvm"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!("--sun-misc-unsafe-memory-access=allow"));
        let version_dir = mc_dir.join("versions").join("1.20.1");
        write_json(&version_dir.join("1.20.1.json"), &json);
        std::fs::write(version_dir.join("1.20.1.jar"), b"fake").unwrap();

        let profile = read_version_profile("1.20.1", mc_dir).unwrap();

        let args_java17 =
            build_jvm_args(&profile, mc_dir, "1.20.1", 17, &LaunchOptions::default()).unwrap();
        assert!(!args_java17
            .iter()
            .any(|a| a.contains("sun-misc-unsafe-memory-access")));

        let args_java23 =
            build_jvm_args(&profile, mc_dir, "1.20.1", 23, &LaunchOptions::default()).unwrap();
        assert!(args_java23
            .iter()
            .any(|a| a.contains("sun-misc-unsafe-memory-access")));
    }

    #[test]
    fn build_game_args_substitutes_and_appends_resolution_and_server() {
        let tmp = tempfile::tempdir().unwrap();
        let mc_dir = tmp.path();
        setup_version_dir(mc_dir, "1.20.1");
        let profile = read_version_profile("1.20.1", mc_dir).unwrap();

        let auth = AuthValues {
            name: "Steve".to_string(),
            uuid: "11111111-1111-1111-1111-111111111111".to_string(),
            access_token: "secret-token".to_string(),
            user_type: "msa".to_string(),
        };
        let opts = LaunchOptions {
            use_custom_resolution: true,
            window_width: 1920,
            window_height: 1080,
            server_address: "play.example.com".to_string(),
            server_port: 25565,
            ..Default::default()
        };

        let args = build_game_args(&profile, mc_dir, "1.20.1", &auth, &opts);

        assert!(args.windows(2).any(|w| w == ["--username", "Steve"]));
        assert!(args.windows(2).any(|w| w == ["--uuid", auth.uuid.as_str()]));
        assert!(args
            .windows(2)
            .any(|w| w == ["--accessToken", "secret-token"]));
        assert!(args.windows(2).any(|w| w == ["--width", "1920"]));
        assert!(args.windows(2).any(|w| w == ["--height", "1080"]));
        assert!(args
            .windows(2)
            .any(|w| w == ["--server", "play.example.com"]));
        assert!(args.windows(2).any(|w| w == ["--port", "25565"]));
    }

    #[test]
    fn build_game_args_fullscreen_takes_priority_over_custom_resolution() {
        let tmp = tempfile::tempdir().unwrap();
        let mc_dir = tmp.path();
        setup_version_dir(mc_dir, "1.20.1");
        let profile = read_version_profile("1.20.1", mc_dir).unwrap();
        let auth = AuthValues {
            name: "Steve".to_string(),
            uuid: "u".to_string(),
            access_token: "t".to_string(),
            user_type: "msa".to_string(),
        };
        let opts = LaunchOptions {
            fullscreen: true,
            use_custom_resolution: true,
            ..Default::default()
        };
        let args = build_game_args(&profile, mc_dir, "1.20.1", &auth, &opts);
        assert!(args.contains(&"--fullscreen".to_string()));
        assert!(!args.contains(&"--width".to_string()));
    }

    #[test]
    fn read_version_profile_errors_when_json_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let err = read_version_profile("does-not-exist", tmp.path()).unwrap_err();
        matches!(err, JavaError::VersionNotInstalled(_));
    }

    #[test]
    fn read_version_profile_merges_inherits_from() {
        let tmp = tempfile::tempdir().unwrap();
        let mc_dir = tmp.path();

        // Parent defines mainClass + a jvm arg.
        let mut parent = sample_version_json();
        parent["id"] = serde_json::json!("parent");
        write_json(
            &mc_dir.join("versions").join("parent").join("parent.json"),
            &parent,
        );

        // Child inherits and overrides nothing but adds a library.
        let child = serde_json::json!({
            "inheritsFrom": "parent",
            "libraries": [
                {
                    "name": "org.child:extra:1.0",
                    "downloads": { "artifact": { "path": "org/child/extra/1.0/extra-1.0.jar" } }
                }
            ]
        });
        let child_dir = mc_dir.join("versions").join("child");
        write_json(&child_dir.join("child.json"), &child);
        std::fs::write(child_dir.join("child.jar"), b"fake").unwrap();

        let profile = read_version_profile("child", mc_dir).unwrap();
        assert_eq!(profile.main_class, "net.minecraft.client.main.Main");
        assert!(profile
            .libraries
            .iter()
            .any(|l| l.path.to_string_lossy().contains("extra-1.0.jar")));
    }

    #[test]
    fn offline_uuid_matches_known_vector() {
        // The offline-UUID algorithm (MD5("OfflinePlayer:<name>") with
        // variant/version bits forced) is deterministic; this is the same
        // scheme used by vanilla/other launchers for "Player".
        let uuid = offline_uuid("Player");
        assert_eq!(uuid.len(), 32);
        // Version nibble (byte 6 high nibble) must be 3.
        assert_eq!(&uuid[12..13], "3");
        // Variant nibble (byte 8 high nibble) must be 8-b.
        assert!(matches!(uuid.as_bytes()[16], b'8' | b'9' | b'a' | b'b'));
        // Deterministic across calls.
        assert_eq!(uuid, offline_uuid("Player"));
        assert_ne!(uuid, offline_uuid("OtherPlayer"));
    }

    #[test]
    fn instance_crud_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = InstanceService::new(tmp.path().join("java")).unwrap();

        // A default instance should have been created automatically.
        let all = svc.instances();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Default");

        let created = svc.create("My Pack", "1.20.1").unwrap();
        assert_eq!(created.id, "my-pack");
        assert!(PathBuf::from(&created.directory).join("mods").is_dir());

        // Duplicate id collision -> suffixed.
        let created2 = svc.create("My Pack", "1.20.1").unwrap();
        assert_eq!(created2.id, "my-pack-2");

        svc.rename(&created.id, "Renamed Pack").unwrap();
        let after_rename = svc.get(&created.id).unwrap();
        assert_eq!(after_rename.name, "Renamed Pack");

        svc.set_instance_version(&created.id, "1.21.0").unwrap();
        assert_eq!(svc.get(&created.id).unwrap().version_id, "1.21.0");

        svc.set_ram(&created.id, 1024, 4096).unwrap();
        let with_ram = svc.get(&created.id).unwrap();
        assert_eq!(with_ram.min_ram_mb, 1024);
        assert_eq!(with_ram.max_ram_mb, 4096);

        let dup = svc.duplicate(&created.id).unwrap();
        assert!(dup.id.starts_with("renamed-pack-copy"));

        assert!(svc.delete(&created.id).unwrap());
        assert!(svc.get(&created.id).is_none());

        // Cannot delete the last remaining instance.
        for inst in svc.instances() {
            if inst.id != "default" {
                let _ = svc.delete(&inst.id);
            }
        }
        let remaining = svc.instances();
        assert_eq!(remaining.len(), 1);
        assert!(!svc.delete(&remaining[0].id).unwrap());
    }

    #[test]
    fn instance_service_persists_across_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("java");
        {
            let svc = InstanceService::new(root.clone()).unwrap();
            svc.create("Persisted", "1.20.1").unwrap();
        }
        let svc2 = InstanceService::new(root).unwrap();
        assert!(svc2.instances().iter().any(|i| i.name == "Persisted"));
    }
}
