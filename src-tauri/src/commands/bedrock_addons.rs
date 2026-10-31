//! CurseForge-backed "Browse Addons" panel for Bedrock: search by class id
//! (Addons/Maps/Skins/Texture Packs/Scripts, mirroring the C#
//! `CurseForgeService` constants) and install a selected file into the
//! correct live `com.mojang` folder.
//!
//! Install logic is a simplified port of `CurseForgeService.InstallAddonFileAsync`
//! / `GetDirectoryForClass`: `.mcworld`/`.mctemplate` extract into
//! `minecraftWorlds`, `.mcpack`/`.mcaddon` route each bundled pack by its
//! `manifest.json` module type, and anything else (typically a bare `.zip`)
//! extracts directly into the class's default folder. Every destination is
//! resolved via `gl_bedrock::paths::live_dir_for`, which — unlike a single
//! literal instance root — resolves each managed folder against the correct
//! per-account live root, matching how `BedrockPacksView`/`BedrockWorldsView`
//! already read this data.

use std::path::{Path, PathBuf};

use gl_download::DownloadRequest;
use gl_modpack::curseforge::{
    BEDROCK_CLASS_ADDONS, BEDROCK_CLASS_MAPS, BEDROCK_CLASS_SCRIPTS, BEDROCK_CLASS_SKINS,
    BEDROCK_CLASS_TEXTURE_PACKS, GAME_ID_BEDROCK,
};
use gl_modpack::models::CfSearchResult;
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

/// Searches CurseForge Bedrock content. `class_id` matches one of the
/// `BEDROCK_CLASS_*` constants (Addons/Maps/Skins/Texture Packs/Scripts);
/// `None` searches every Bedrock category.
#[tauri::command]
pub async fn search_bedrock_addons(
    state: State<'_, AppState>,
    api_key: String,
    class_id: Option<i32>,
    query: String,
    page: u32,
) -> Result<CfSearchResult, String> {
    let client = gl_modpack::CurseForgeClient::new(state.http.clone(), api_key);
    client
        .search(GAME_ID_BEDROCK, class_id, &query, 20, page)
        .await
        .map_err(|e| e.to_string())
}

/// Outcome of installing a Bedrock addon file, reported back so the panel can
/// show what landed and where.
#[derive(Debug, Clone, Serialize)]
pub struct BedrockAddonInstallResult {
    pub installed_paths: Vec<String>,
    /// Feature 21 — human-readable conflict warnings collected while
    /// installing, one per pack that collided with an already-enabled pack.
    /// Non-fatal: the install still proceeds, matching "warn, don't block".
    #[serde(default)]
    pub warnings: Vec<String>,
}

fn scratch_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glacier-bedrock-addon-{}-{}",
        std::process::id(),
        nonce()
    ))
}

fn nonce() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Default install folder for a class id, port of `GetDirectoryForClass`:
/// Maps -> worlds, Skins -> skin packs, Scripts -> behavior packs, everything
/// else (Addons/Texture Packs/unknown) -> resource packs.
fn default_dir_name_for_class(class_id: i32) -> &'static str {
    match class_id {
        BEDROCK_CLASS_MAPS => "minecraftWorlds",
        BEDROCK_CLASS_SKINS => "skin_packs",
        BEDROCK_CLASS_SCRIPTS => "behavior_packs",
        BEDROCK_CLASS_ADDONS | BEDROCK_CLASS_TEXTURE_PACKS => "resource_packs",
        _ => "resource_packs",
    }
}

/// Reads `manifest.json`'s `modules[].type` to decide which pack folder a
/// bundle entry belongs in. Mirrors `gl_bedrock::import::pack_module_dir_name`.
fn pack_module_dir_name(pack_dir: &Path) -> &'static str {
    let Ok(text) = std::fs::read_to_string(pack_dir.join("manifest.json")) else {
        return "resource_packs";
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return "resource_packs";
    };
    let types: Vec<String> = json
        .get("modules")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("type").and_then(|t| t.as_str()))
                .map(|s| s.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default();

    if types.iter().any(|t| t == "resources") {
        "resource_packs"
    } else if types.iter().any(|t| t == "data") {
        "behavior_packs"
    } else if types.iter().any(|t| t == "skin_pack") {
        "skin_packs"
    } else {
        "resource_packs"
    }
}

fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "Imported".to_string()
    } else {
        trimmed.to_string()
    }
}

fn unique_dir(parent: &Path, base_name: &str) -> PathBuf {
    let candidate = parent.join(base_name);
    if !candidate.exists() {
        return candidate;
    }
    for n in 2.. {
        let candidate = parent.join(format!("{base_name} ({n})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

fn move_dir_contents(src: &Path, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    copy_dir_recursive(src, dest)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Extracts a zip archive into a fresh scratch directory next to the archive
/// itself (cleaned up by the caller along with the rest of the scratch dir —
/// no `tempfile` dependency needed for this one call site).
fn extract_to_temp(zip_path: &Path) -> Result<PathBuf, String> {
    let dir = zip_path.with_extension("extracted");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Extracts a `.mcworld`/`.mctemplate` archive into
/// `minecraftWorlds/<name>`, picking a non-colliding folder name from the
/// archive's `levelname.txt` (falling back to the file's own stem).
fn install_world(zip_path: &Path, file_stem: &str) -> Result<Vec<String>, String> {
    let worlds_dir = gl_bedrock::paths::live_dir_for("minecraftWorlds");
    std::fs::create_dir_all(&worlds_dir).map_err(|e| e.to_string())?;

    let tmp = extract_to_temp(zip_path)?;
    let level_name = std::fs::read_to_string(tmp.join("levelname.txt"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let base_name = level_name.unwrap_or_else(|| file_stem.to_string());

    let dest = unique_dir(&worlds_dir, &sanitize_name(&base_name));
    move_dir_contents(&tmp, &dest)?;
    Ok(vec![dest.to_string_lossy().into_owned()])
}

/// Extracts a `.mcpack` (single pack) or `.mcaddon` (bundle) archive, routing
/// each pack folder by its manifest module type. Runs the feature-21 conflict
/// check against each destination folder before moving the pack into place;
/// any collisions are returned as warnings alongside the installed paths
/// rather than blocking the install.
fn install_packs(zip_path: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let tmp = extract_to_temp(zip_path)?;
    let mut pack_dirs = Vec::new();
    if tmp.join("manifest.json").is_file() {
        pack_dirs.push(tmp.clone());
    } else if let Ok(entries) = std::fs::read_dir(&tmp) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && p.join("manifest.json").is_file() {
                pack_dirs.push(p);
            }
        }
    }

    if pack_dirs.is_empty() {
        return Err("No manifest.json found in the downloaded pack archive.".to_string());
    }

    let mut installed = Vec::new();
    let mut warnings = Vec::new();
    for pack_dir in pack_dirs {
        let dir_name = pack_module_dir_name(&pack_dir);
        let dest_root = gl_bedrock::paths::live_dir_for(dir_name);
        std::fs::create_dir_all(&dest_root).map_err(|e| e.to_string())?;

        let pack_name = pack_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Imported Pack".to_string());

        for conflict in find_pack_conflicts(&pack_dir, &dest_root) {
            warnings.push(format!(
                "'{pack_name}' conflicts with already-enabled pack '{}': {}",
                conflict.existing_pack_name, conflict.reason
            ));
        }

        let dest = unique_dir(&dest_root, &sanitize_name(&pack_name));
        move_dir_contents(&pack_dir, &dest)?;
        installed.push(dest.to_string_lossy().into_owned());
    }
    Ok((installed, warnings))
}

/// Shared conflict scan used both by [`install_packs`] and the
/// `check_bedrock_pack_conflicts` command below.
fn find_pack_conflicts(candidate_dir: &Path, packs_dir: &Path) -> Vec<PackConflict> {
    let candidate = read_pack_identity(candidate_dir);
    if candidate.header_uuid.is_none() && candidate.module_uuids.is_empty() {
        return Vec::new();
    }

    let mut conflicts = Vec::new();
    let Ok(entries) = std::fs::read_dir(packs_dir) else {
        return conflicts;
    };
    for entry in entries.flatten() {
        let existing_dir = entry.path();
        if !existing_dir.is_dir() || existing_dir == candidate_dir {
            continue;
        }
        let existing = read_pack_identity(&existing_dir);
        let name = existing_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        if let (Some(a), Some(b)) = (&candidate.header_uuid, &existing.header_uuid) {
            if a == b {
                conflicts.push(PackConflict {
                    existing_pack_name: name,
                    existing_pack_path: existing_dir.to_string_lossy().into_owned(),
                    reason: "Same pack identifier (header.uuid) — this looks like a duplicate or a fork of an already-enabled pack.".to_string(),
                });
                continue;
            }
        }

        if candidate
            .module_uuids
            .iter()
            .any(|m| existing.module_uuids.contains(m))
        {
            conflicts.push(PackConflict {
                existing_pack_name: name,
                existing_pack_path: existing_dir.to_string_lossy().into_owned(),
                reason: "Shares a module identifier with an already-enabled pack — behavior may conflict.".to_string(),
            });
        }
    }
    conflicts
}

/// Extracts an archive directly into the class's default folder — the
/// fallback path for a bare `.zip` (or any other extension) that isn't a
/// recognized Bedrock content container.
fn install_into_class_dir(
    zip_path: &Path,
    class_id: i32,
    file_stem: &str,
) -> Result<Vec<String>, String> {
    let dir_name = default_dir_name_for_class(class_id);
    let dest_root = gl_bedrock::paths::live_dir_for(dir_name);
    std::fs::create_dir_all(&dest_root).map_err(|e| e.to_string())?;

    let tmp = extract_to_temp(zip_path)?;
    let dest = unique_dir(&dest_root, &sanitize_name(file_stem));
    move_dir_contents(&tmp, &dest)?;
    Ok(vec![dest.to_string_lossy().into_owned()])
}

/// Feature 21 — addon pack conflict detector.
///
/// A pack's `manifest.json` (`header.uuid`) is content-addressed identity:
/// two packs with the same header uuid are the same pack (or a fork of it).
/// A pack can also declare its own module `header.uuid`s — the same field
/// used by `pack_module_dir_name` above. Before applying/enabling a pack,
/// this checks every already-installed pack of the same kind for either
/// matching id, reporting a warning list the frontend can show instead of
/// silently letting two packs stomp on the same namespace.
#[derive(Debug, Clone, Serialize)]
pub struct PackConflict {
    pub existing_pack_name: String,
    pub existing_pack_path: String,
    pub reason: String,
}

#[derive(Debug, Default)]
struct PackIdentity {
    header_uuid: Option<String>,
    module_uuids: Vec<String>,
}

fn read_pack_identity(pack_dir: &Path) -> PackIdentity {
    let Ok(text) = std::fs::read_to_string(pack_dir.join("manifest.json")) else {
        return PackIdentity::default();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return PackIdentity::default();
    };
    let header_uuid = json
        .get("header")
        .and_then(|h| h.get("uuid"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_ascii_lowercase());
    let module_uuids = json
        .get("modules")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("uuid").and_then(|u| u.as_str()))
                .map(|s| s.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default();
    PackIdentity {
        header_uuid,
        module_uuids,
    }
}

/// Checks `candidate_pack_dir` (an extracted pack about to be applied, or
/// an already-installed one) against every other currently-enabled pack of
/// the same `kind` for a `header.uuid`/module `uuid` collision. Reuses the
/// same directory resolution and manifest-reading conventions as the rest
/// of this file rather than re-walking com.mojang independently.
#[tauri::command]
pub async fn check_bedrock_pack_conflicts(
    candidate_pack_dir: String,
    kind: String,
) -> Result<Vec<PackConflict>, String> {
    let candidate_dir = PathBuf::from(&candidate_pack_dir);
    let candidate = read_pack_identity(&candidate_dir);
    if candidate.header_uuid.is_none() && candidate.module_uuids.is_empty() {
        // No identity to compare — nothing to warn about.
        return Ok(Vec::new());
    }

    let parsed_kind = super::bedrock::parse_pack_kind_pub(&kind)?;
    let dir_name = match parsed_kind {
        gl_bedrock::PackKind::Resource => "resource_packs",
        gl_bedrock::PackKind::Behavior => "behavior_packs",
        gl_bedrock::PackKind::Skin => "skin_packs",
        gl_bedrock::PackKind::ResourceDev => "development_resource_packs",
        gl_bedrock::PackKind::BehaviorDev => "development_behavior_packs",
        gl_bedrock::PackKind::SkinDev => "development_skin_packs",
    };
    let packs_dir = gl_bedrock::paths::live_dir_for(dir_name);
    let _ = candidate; // identity check happens inside find_pack_conflicts too
    Ok(find_pack_conflicts(&candidate_dir, &packs_dir))
}

/// Downloads a CurseForge Bedrock addon file (by mod id + file id) and
/// installs it into the correct live `com.mojang` folder for `class_id`.
#[tauri::command]
pub async fn install_bedrock_addon(
    state: State<'_, AppState>,
    api_key: String,
    mod_id: i32,
    file_id: i32,
    class_id: i32,
) -> Result<BedrockAddonInstallResult, String> {
    let cf = gl_modpack::CurseForgeClient::new(state.http.clone(), api_key);
    let resolved = cf
        .resolve_manifest_file(mod_id, file_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Couldn't resolve a download URL for this file.".to_string())?;

    let scratch = scratch_dir();
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    let file_name = if resolved.file_name.trim().is_empty() {
        format!("addon-{file_id}.zip")
    } else {
        resolved.file_name.clone()
    };
    let archive_path = scratch.join(&file_name);

    let req = DownloadRequest::new(resolved.download_url.clone(), archive_path.clone())
        .with_label(resolved.display_name.clone());
    state
        .downloads
        .submit(req)
        .join()
        .await
        .map_err(|e| e.to_string())?;

    let ext = Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let stem = Path::new(&file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported".to_string());

    let result = match ext.as_str() {
        "mcworld" | "mctemplate" => install_world(&archive_path, &stem).map(|p| (p, Vec::new())),
        "mcpack" | "mcaddon" => install_packs(&archive_path),
        _ => install_into_class_dir(&archive_path, class_id, &stem).map(|p| (p, Vec::new())),
    };

    let _ = std::fs::remove_dir_all(&scratch);

    result.map(|(installed_paths, warnings)| BedrockAddonInstallResult {
        installed_paths,
        warnings,
    })
}
