//! Bedrock RenderDragon `.material.bin` shader-pack apply/restore. Wires
//! `gl_bedrock::shader::ShaderService` (a straight port of
//! `Services/ShaderService.cs` — that service is a pure bulk-file-copy
//! operation, not a binary-format patcher: it copies pre-built
//! `.material.bin` files from a source zip/mcpack/folder over the ones in
//! the installed game's `data/renderer/materials`, keeping a backup of
//! whatever it overwrites so "restore" can put stock files back) to the
//! installed Bedrock package resolved via `gl_launch::bedrock::resolve()`
//! (the same PowerShell `Get-AppxPackage` based resolver already used for
//! launching).
//!
//! Deviation from the C# original: `BedrockPackageInfo` here doesn't carry
//! `IsDevelopmentMode` (that needs a WinRT `PackageManager` call the launch
//! crate deliberately avoids). Instead of that up-front check, this module
//! probes the `materials` directory with a real write-then-delete — Store
//! installs live under an ACL Windows won't let any app write to, so the
//! probe fails there with the same practical outcome (and a clear message)
//! as the original's `IsSideloaded` check.

use std::path::{Path, PathBuf};

use gl_bedrock::shader::ShaderService;
use serde::Serialize;

fn user_profile() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn shader_root() -> PathBuf {
    user_profile().join("Glacier Launcher").join("shaders")
}

fn service() -> ShaderService {
    ShaderService::new(shader_root())
}

fn is_bedrock_running() -> bool {
    gl_injection::find_process_by_name("Minecraft.Windows.exe").is_some()
}

fn check_writable(dir: &Path) -> Result<(), String> {
    let probe = dir.join(".glacier-write-test");
    match std::fs::write(&probe, b"x") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(_) => Err(
            "Shaders need a sideloaded install — the Store/Xbox copy of Minecraft lives in a \
             folder Windows won't let any app modify. Install a version from the Versions tab \
             (Developer Mode), then come back."
                .to_string(),
        ),
    }
}

/// Mirrors `ShaderService.UnavailableReason()` / `MaterialsDir`.
async fn resolve_materials_dir() -> Result<PathBuf, String> {
    let pkg = gl_launch::bedrock::resolve()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Minecraft Bedrock isn't installed.".to_string())?;

    let materials = pkg
        .installed_path
        .join("data")
        .join("renderer")
        .join("materials");
    if !materials.is_dir() {
        return Err(
            "This build has no data/renderer/materials folder — RenderDragon shaders aren't \
             supported on it."
                .to_string(),
        );
    }

    if is_bedrock_running() {
        return Err("Close Minecraft first — the game holds its material files open.".to_string());
    }

    check_writable(&materials)?;
    Ok(materials)
}

/// Returns `null` when shaders are usable right now, or the reason they
/// aren't (mirrors `ShaderService.UnavailableReason()`).
#[tauri::command]
pub async fn shaders_unavailable_reason() -> Option<String> {
    resolve_materials_dir().await.err()
}

#[derive(Debug, Clone, Serialize)]
pub struct AppliedShaderView {
    pub file_name: String,
    pub source_name: String,
    pub applied_iso: String,
    pub size_bytes: u64,
}

#[tauri::command]
pub fn list_applied_shaders() -> Vec<AppliedShaderView> {
    service()
        .load_applied()
        .into_iter()
        .map(|a| AppliedShaderView {
            file_name: a.file_name,
            source_name: a.source_name,
            applied_iso: a.applied_iso,
            size_bytes: a.size_bytes,
        })
        .collect()
}

/// Applies `.material.bin` files pulled from `source_paths` (loose `.bin`
/// files, folders of them, or `.zip`/`.mcpack` bundles) over the installed
/// game's materials, backing up any stock file the first time it's
/// overwritten. Returns the installed file names.
#[tauri::command]
pub async fn apply_shaders(source_paths: Vec<String>) -> Result<Vec<String>, String> {
    let materials = resolve_materials_dir().await?;
    let sources: Vec<PathBuf> = source_paths.into_iter().map(PathBuf::from).collect();
    service()
        .apply(&materials, &sources)
        .map_err(|e| e.to_string())
}

/// Reverts every applied shader material back to its stock backup (or
/// deletes it if it didn't exist before). Returns the number reverted.
#[tauri::command]
pub async fn restore_shaders() -> Result<usize, String> {
    let materials = resolve_materials_dir().await?;
    service().restore(&materials).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_stock_shader_materials() -> Result<Vec<String>, String> {
    let materials = resolve_materials_dir().await?;
    Ok(service().list_stock_materials(&materials))
}

#[tauri::command]
pub async fn open_shaders_materials_folder() -> Result<(), String> {
    let materials = resolve_materials_dir().await?;
    std::process::Command::new("explorer")
        .arg(&materials)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
