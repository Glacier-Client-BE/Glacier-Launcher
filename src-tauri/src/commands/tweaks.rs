//! Thin #[tauri::command] wrappers around `gl_integrations::tweaks`.
//! Business logic (category fetch/flatten, blob resolution, manifest
//! building) lives in the crate; this module adds the byte-downloading /
//! file-assembly glue that the crate intentionally leaves to callers.
#![allow(dead_code)]

use std::io::Write;
use std::path::Path;

use gl_download::DownloadRequest;
use gl_integrations::models::{TweakCategory, TweakPack};
use gl_integrations::tweaks;
use serde::Serialize;
use serde_json::Value;
use tauri::State;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::state::AppState;

/// Fetches an image server-side and returns its raw bytes to the frontend.
///
/// Some external hosts (confirmed: vanillatweaks.net) hotlink-block based on
/// the `Referer` header and return 403 to any browser-context request whose
/// origin isn't their own site — this breaks `<img src>` tags pointed
/// directly at their icon URLs in both a WebView2 host and this app's
/// webview. `reqwest` here sends no `Referer` at all, so proxying the fetch
/// through the backend sidesteps the block entirely.
#[tauri::command]
pub async fn fetch_image_bytes(state: State<'_, AppState>, url: String) -> Result<Vec<u8>, String> {
    let bytes = state
        .http
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    Ok(bytes.to_vec())
}

#[tauri::command]
pub fn tweaks_java_versions() -> Vec<&'static str> {
    tweaks::JAVA_VERSIONS.to_vec()
}

#[tauri::command]
pub async fn list_vanilla_tweaks(
    state: State<'_, AppState>,
    version: String,
) -> Result<Vec<TweakCategory>, String> {
    tweaks::get_vanilla_resource_packs(&state.http, &version)
        .await
        .map_err(|e| e.to_string())
}

/// Requests the VanillaTweaks zip for `selection`, downloads it, and writes
/// it into `<minecraft_dir>/resourcepacks/`. Returns the absolute path of
/// the written file.
#[tauri::command]
pub async fn build_vanilla_tweaks_pack(
    state: State<'_, AppState>,
    minecraft_dir: String,
    version: String,
    selection: std::collections::HashMap<String, Vec<String>>,
) -> Result<String, String> {
    let url = tweaks::request_vanilla_resource_pack_zip(&state.http, &selection, &version)
        .await
        .map_err(|e| e.to_string())?;

    let file_name = url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("VanillaTweaks.zip");
    let dest = Path::new(&minecraft_dir)
        .join("resourcepacks")
        .join(file_name);

    let req = DownloadRequest::new(url, dest.clone()).with_label("VanillaTweaks pack");
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().into_owned())
}

#[derive(Debug, Serialize)]
pub struct BedrockTweaksIndexDto {
    pub categories: Vec<TweakCategory>,
    pub engine_version: [i32; 3],
    pub version_label: String,
}

#[tauri::command]
pub async fn list_bedrock_tweaks(
    state: State<'_, AppState>,
) -> Result<BedrockTweaksIndexDto, String> {
    let idx = tweaks::get_bedrock_resource_packs(&state.http)
        .await
        .map_err(|e| e.to_string())?;
    Ok(BedrockTweaksIndexDto {
        categories: idx.categories,
        engine_version: idx.engine_version,
        version_label: idx.version_label,
    })
}

/// Downloads the selected BedrockTweaks packs' files and assembles them into
/// a single `.mcpack` at `<dest_dir>/<file_name>`, mirroring
/// `TweaksService.BuildBedrockResourcePackAsync`. Returns the written path.
#[tauri::command]
pub async fn build_bedrock_tweaks_pack(
    state: State<'_, AppState>,
    dest_dir: String,
    engine_version: [i32; 3],
    selection: Vec<TweakPack>,
) -> Result<String, String> {
    if selection.is_empty() {
        return Err("Select at least one tweak first.".to_string());
    }

    const BT_REPO: &str = "BedrockTweaks/Files";
    const BT_BRANCH: &str = "devel";
    let tree_url =
        format!("https://api.github.com/repos/{BT_REPO}/git/trees/{BT_BRANCH}?recursive=1");
    let tree_json: Value = state
        .http
        .get(&tree_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let entries = tree_json
        .get("tree")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let blobs = tweaks::resolve_bedrock_blobs(&entries, &selection);
    if blobs.is_empty() {
        return Err("BedrockTweaks returned no files for that selection.".to_string());
    }

    // Download every blob into a scratch dir, preserving the in-pack relative path.
    let scratch =
        std::env::temp_dir().join(format!("glacier-bedrocktweaks-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;

    let mut reqs = Vec::new();
    let mut rel_paths = Vec::new();
    for (prefix, path) in &blobs {
        let relative = path.strip_prefix(prefix.as_str()).unwrap_or(path);
        let raw_url = format!("https://raw.githubusercontent.com/{BT_REPO}/{BT_BRANCH}/{path}");
        let local = scratch.join(sanitize_for_fs(path));
        reqs.push(DownloadRequest::new(raw_url, local.clone()).with_label("BedrockTweaks file"));
        rel_paths.push((local, relative.to_string()));
    }

    let outcomes = state.downloads.download_all(reqs).await;
    for outcome in &outcomes {
        if let Err(e) = outcome {
            let _ = std::fs::remove_dir_all(&scratch);
            return Err(e.to_string());
        }
    }

    let name = if selection.len() == 1 {
        selection[0].name.clone()
    } else {
        format!("BedrockTweaks ({})", selection.len())
    };
    let file_name = format!("{}.mcpack", sanitize_for_fs(&name));
    let dest = Path::new(&dest_dir).join(&file_name);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let manifest =
        tweaks::build_bedrock_manifest(&selection, engine_version, new_uuid_v4(), new_uuid_v4())
            .map_err(|e| e.to_string())?;

    let file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(manifest.as_bytes())
        .map_err(|e| e.to_string())?;

    for (local, relative) in &rel_paths {
        let Ok(bytes) = std::fs::read(local) else {
            continue;
        };
        let entry_name = relative.replace('\\', "/");
        if zip.start_file(entry_name, options).is_err() {
            continue;
        }
        let _ = zip.write_all(&bytes);
    }
    zip.finish().map_err(|e| e.to_string())?;

    let _ = std::fs::remove_dir_all(&scratch);

    Ok(dest.to_string_lossy().into_owned())
}

fn sanitize_for_fs(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Minimal random-ish UUIDv4-shaped string generator (avoids pulling in the
/// `uuid` crate for two manifest ids; only needs to be unique-looking, not
/// cryptographically random).
fn new_uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    let mix = nanos ^ (pid << 64);
    let bytes = mix.to_be_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6] & 0x0f,
        (bytes[7] & 0x3f) | 0x80, bytes[8],
        bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14]
    )
}
