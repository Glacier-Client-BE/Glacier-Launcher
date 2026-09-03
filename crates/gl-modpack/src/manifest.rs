//! Parses modpack archive manifests (`manifest.json` for CurseForge,
//! `modrinth.index.json` for Modrinth/`.mrpack`) into a source-agnostic
//! [`ModpackPlan`], mirroring `ModpackInstallService.ParseCurseForgeManifest`
//! / `ParseModrinthIndex`.

use std::io::Read;

use serde_json::Value;

use crate::error::{ModpackError, Result};
use crate::models::{CurseForgeFileRef, ModLoader, ModpackFile, ModpackPlan};

/// Reads `manifest.json` out of a CurseForge modpack zip (already opened via
/// the `zip` crate) and turns it into a [`ModpackPlan`]. File entries carry a
/// `curseforge_ref` that must be resolved via [`crate::curseforge::CurseForgeClient`]
/// to get an actual download URL.
pub fn parse_curseforge_manifest<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    fallback_name: &str,
) -> Result<ModpackPlan> {
    let json = read_zip_json(archive, "manifest.json")?.ok_or_else(|| {
        ModpackError::NotFound("Not a CurseForge modpack (no manifest.json).".into())
    })?;

    let mut plan = ModpackPlan {
        name: fallback_name.to_string(),
        ..Default::default()
    };

    if let Some(n) = json.get("name").and_then(Value::as_str) {
        plan.name = n.to_string();
    }

    if let Some(mc) = json.get("minecraft") {
        if let Some(v) = mc.get("version").and_then(Value::as_str) {
            plan.minecraft_version = v.to_string();
        }
        if let Some(loaders) = mc.get("modLoaders").and_then(Value::as_array) {
            if let Some(first) = loaders.first() {
                let id = first.get("id").and_then(Value::as_str).unwrap_or("");
                let (loader, version) = parse_loader_id(id);
                plan.loader = loader;
                plan.loader_version = version;
            }
        }
    }

    if let Some(files) = json.get("files").and_then(Value::as_array) {
        for f in files {
            let project_id = f.get("projectID").and_then(Value::as_i64).unwrap_or(0) as i32;
            let file_id = f.get("fileID").and_then(Value::as_i64).unwrap_or(0) as i32;
            if project_id > 0 && file_id > 0 {
                plan.files.push(ModpackFile {
                    curseforge_ref: Some(CurseForgeFileRef {
                        project_id,
                        file_id,
                    }),
                    ..Default::default()
                });
            }
        }
    }

    Ok(plan)
}

/// Reads `modrinth.index.json` out of an `.mrpack` zip and turns it into a
/// [`ModpackPlan`]. Files with `env.client == "unsupported"` are skipped, as
/// they must not be installed on the client.
pub fn parse_modrinth_index<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    fallback_name: &str,
) -> Result<ModpackPlan> {
    let json = read_zip_json(archive, "modrinth.index.json")?.ok_or_else(|| {
        ModpackError::NotFound("Not a Modrinth pack (no modrinth.index.json).".into())
    })?;

    let mut plan = ModpackPlan {
        name: fallback_name.to_string(),
        ..Default::default()
    };

    if let Some(n) = json.get("name").and_then(Value::as_str) {
        plan.name = n.to_string();
    }

    if let Some(deps) = json.get("dependencies") {
        if let Some(mc) = deps.get("minecraft").and_then(Value::as_str) {
            plan.minecraft_version = mc.to_string();
        }
        for (key, loader) in [
            ("fabric-loader", ModLoader::Fabric),
            ("quilt-loader", ModLoader::Quilt),
            ("forge", ModLoader::Forge),
            ("neoforge", ModLoader::NeoForge),
        ] {
            if let Some(lv) = deps.get(key).and_then(Value::as_str) {
                plan.loader = Some(loader);
                plan.loader_version = lv.to_string();
                break;
            }
        }
    }

    if let Some(files) = json.get("files").and_then(Value::as_array) {
        for f in files {
            let path = f.get("path").and_then(Value::as_str).unwrap_or("");
            if path.is_empty() {
                continue;
            }

            let unsupported = f
                .get("env")
                .and_then(|e| e.get("client"))
                .and_then(Value::as_str)
                .map(|c| c.eq_ignore_ascii_case("unsupported"))
                .unwrap_or(false);
            if unsupported {
                continue;
            }

            let url = f
                .get("downloads")
                .and_then(Value::as_array)
                .and_then(|d| d.first())
                .and_then(Value::as_str)
                .unwrap_or("");
            if url.is_empty() {
                continue;
            }

            let sha1 = f
                .get("hashes")
                .and_then(|h| h.get("sha1"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let sha512 = f
                .get("hashes")
                .and_then(|h| h.get("sha512"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let size = f.get("fileSize").and_then(Value::as_i64).unwrap_or(-1);

            plan.files.push(ModpackFile {
                url: url.to_string(),
                relative_path: path.to_string(),
                sha1,
                sha512,
                size,
                curseforge_ref: None,
            });
        }
    }

    Ok(plan)
}

/// Splits a CurseForge `modLoaders[].id` like `"forge-47.2.0"` into
/// `(Some(Forge), "47.2.0")`.
fn parse_loader_id(id: &str) -> (Option<ModLoader>, String) {
    if id.is_empty() {
        return (None, String::new());
    }
    match id.split_once('-') {
        Some((loader, version)) => (ModLoader::parse(loader), version.to_string()),
        None => (ModLoader::parse(id), String::new()),
    }
}

/// The install profile id to select after a Fabric/Quilt install, matching
/// `ModpackInstallService.LoaderProfileId`. Forge/NeoForge installers write
/// their own profile id, so those return the bare Minecraft version.
pub fn loader_profile_id(plan: &ModpackPlan) -> String {
    match plan.loader {
        Some(ModLoader::Fabric) => format!(
            "fabric-loader-{}-{}",
            plan.loader_version, plan.minecraft_version
        ),
        Some(ModLoader::Quilt) => format!(
            "quilt-loader-{}-{}",
            plan.loader_version, plan.minecraft_version
        ),
        _ => plan.minecraft_version.clone(),
    }
}

fn read_zip_json<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    entry_name: &str,
) -> Result<Option<Value>> {
    let mut entry = match archive.by_name(entry_name) {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(ModpackError::from(e)),
    };
    let mut contents = String::new();
    entry.read_to_string(&mut contents)?;
    Ok(Some(serde_json::from_str(&contents)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn zip_with_entry(name: &str, contents: &str) -> zip::ZipArchive<Cursor<Vec<u8>>> {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file(name, options).unwrap();
            writer.write_all(contents.as_bytes()).unwrap();
            writer.finish().unwrap();
        }
        zip::ZipArchive::new(Cursor::new(buf)).unwrap()
    }

    const CF_MANIFEST: &str = r#"{
        "minecraft": {
            "version": "1.19.2",
            "modLoaders": [{ "id": "forge-43.2.0", "primary": true }]
        },
        "name": "All the Mods 8",
        "version": "1.0.0",
        "files": [
            { "projectID": 238222, "fileID": 4593548, "required": true },
            { "projectID": 0, "fileID": 0 }
        ]
    }"#;

    #[test]
    fn parses_curseforge_manifest_fixture() {
        let mut archive = zip_with_entry("manifest.json", CF_MANIFEST);
        let plan = parse_curseforge_manifest(&mut archive, "fallback").unwrap();
        assert_eq!(plan.name, "All the Mods 8");
        assert_eq!(plan.minecraft_version, "1.19.2");
        assert_eq!(plan.loader, Some(ModLoader::Forge));
        assert_eq!(plan.loader_version, "43.2.0");
        assert_eq!(plan.files.len(), 1);
        assert_eq!(
            plan.files[0].curseforge_ref,
            Some(CurseForgeFileRef {
                project_id: 238222,
                file_id: 4593548
            })
        );
    }

    #[test]
    fn errors_when_manifest_missing() {
        let mut archive = zip_with_entry("other.json", "{}");
        let err = parse_curseforge_manifest(&mut archive, "fallback").unwrap_err();
        assert!(matches!(err, ModpackError::NotFound(_)));
    }

    const MR_INDEX: &str = r#"{
        "formatVersion": 1,
        "game": "minecraft",
        "name": "Fabulously Optimized",
        "dependencies": { "minecraft": "1.20.1", "fabric-loader": "0.15.0" },
        "files": [
            {
                "path": "mods/sodium.jar",
                "hashes": { "sha1": "aaa", "sha512": "bbb" },
                "downloads": ["https://cdn.modrinth.com/sodium.jar"],
                "fileSize": 12345
            },
            {
                "path": "mods/server-only.jar",
                "env": { "client": "unsupported", "server": "required" },
                "hashes": {},
                "downloads": ["https://cdn.modrinth.com/server-only.jar"],
                "fileSize": 999
            }
        ]
    }"#;

    #[test]
    fn parses_modrinth_index_fixture_and_skips_unsupported_client_files() {
        let mut archive = zip_with_entry("modrinth.index.json", MR_INDEX);
        let plan = parse_modrinth_index(&mut archive, "fallback").unwrap();
        assert_eq!(plan.name, "Fabulously Optimized");
        assert_eq!(plan.minecraft_version, "1.20.1");
        assert_eq!(plan.loader, Some(ModLoader::Fabric));
        assert_eq!(plan.loader_version, "0.15.0");
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.files[0].relative_path, "mods/sodium.jar");
        assert_eq!(plan.files[0].sha512, "bbb");
        assert_eq!(plan.files[0].size, 12345);
    }

    #[test]
    fn loader_profile_id_matches_fabric_and_forge_conventions() {
        let fabric_plan = ModpackPlan {
            loader: Some(ModLoader::Fabric),
            loader_version: "0.15.0".into(),
            minecraft_version: "1.20.1".into(),
            ..Default::default()
        };
        assert_eq!(
            loader_profile_id(&fabric_plan),
            "fabric-loader-0.15.0-1.20.1"
        );

        let forge_plan = ModpackPlan {
            loader: Some(ModLoader::Forge),
            minecraft_version: "1.19.2".into(),
            ..Default::default()
        };
        assert_eq!(loader_profile_id(&forge_plan), "1.19.2");
    }
}
