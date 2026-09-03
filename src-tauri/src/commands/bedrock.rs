use std::path::{Path, PathBuf};

use gl_bedrock::{
    BedrockBackup, BedrockBackupService, BedrockInstance, BedrockPack, BedrockPackService,
    BedrockRoot, BedrockWorld, BedrockWorldService, ImportOutcome, PackKind, ScreenshotFile,
};
use tauri::State;

use crate::state::AppState;

/// Imports a dropped Bedrock content file (`.mcworld`/`.mctemplate`/`.mcpack`/
/// `.mcaddon`) into the given instance's folder. Mirrors the original app's
/// drag-drop import handling for Bedrock content.
#[tauri::command]
pub fn import_bedrock_content(
    source_path: String,
    instance_dir: String,
) -> Result<ImportOutcome, String> {
    gl_bedrock::import_content(Path::new(&source_path), Path::new(&instance_dir))
        .map_err(|e| e.to_string())
}

fn parse_pack_kind(kind: &str) -> Result<PackKind, String> {
    match kind {
        "resource" => Ok(PackKind::Resource),
        "behavior" => Ok(PackKind::Behavior),
        "skin" => Ok(PackKind::Skin),
        "resource-dev" => Ok(PackKind::ResourceDev),
        "behavior-dev" => Ok(PackKind::BehaviorDev),
        "skin-dev" => Ok(PackKind::SkinDev),
        other => Err(format!("unknown pack kind: {other}")),
    }
}

fn backups_dir() -> std::path::PathBuf {
    gl_settings::launcher_root().join("Backups")
}

/// The root a Bedrock content folder lives under.
///
/// The frontend sends the selected managed instance's directory, or an empty
/// string when no instance is selected — which means the **live** game
/// install. Live roots must be resolved per folder via
/// `gl_bedrock::paths` (the port of C# `CurseForgeService.LiveRootFor`),
/// because pack folders can sit under a different per-account root than
/// `minecraftWorlds`. Joining a literal subfolder onto an empty instance dir
/// is what made every Bedrock panel read from a path that does not exist.
fn bedrock_root(instance_dir: &str) -> BedrockRoot {
    if instance_dir.trim().is_empty() {
        BedrockRoot::Live
    } else {
        BedrockRoot::Instance(PathBuf::from(instance_dir))
    }
}

/// The directory for `folder_name` under the selected instance, or under the
/// live install when no instance is selected.
fn bedrock_dir(instance_dir: &str, folder_name: &str) -> PathBuf {
    bedrock_root(instance_dir).dir_for(folder_name)
}

#[tauri::command]
pub fn list_bedrock_worlds(instance_dir: String) -> Result<Vec<BedrockWorld>, String> {
    let svc = BedrockWorldService::new(bedrock_dir(&instance_dir, "minecraftWorlds"));
    svc.list_worlds().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_bedrock_world(world_path: String) -> Result<(), String> {
    let svc = BedrockWorldService::new(Path::new(&world_path).parent().unwrap_or(Path::new(".")));
    svc.delete_world(Path::new(&world_path))
        .map_err(|e| e.to_string())
}

/// Exports a world as a `.mcworld`.
///
/// `export_dir` is optional: the C# `BedrockWorldService.ExportWorldAsync`
/// chooses the destination itself rather than taking one from the caller,
/// always writing to `%USERPROFILE%\Glacier Launcher\exported-worlds`. That
/// default lives here so callers don't have to guess it.
#[tauri::command]
pub fn export_bedrock_world(
    world_path: String,
    export_dir: Option<String>,
) -> Result<String, String> {
    let dest = match export_dir {
        Some(d) if !d.trim().is_empty() => PathBuf::from(d),
        _ => gl_settings::launcher_root().join("exported-worlds"),
    };
    let svc = BedrockWorldService::new(Path::new(&world_path).parent().unwrap_or(Path::new(".")));
    svc.export_world(Path::new(&world_path), &dest)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

/// The launcher's Bedrock backups directory, so the Backups panel's
/// "Open folder" action works before any backup has been listed.
#[tauri::command]
pub fn bedrock_backups_dir() -> String {
    backups_dir().to_string_lossy().into_owned()
}

/// The live Screenshots directory. Always the live install, never a managed
/// instance - the original's screenshots path is a static property.
#[tauri::command]
pub fn bedrock_screenshots_dir() -> String {
    gl_bedrock::screenshots_dir().to_string_lossy().into_owned()
}

/// The live `minecraftWorlds` directory - port of the
/// `BedrockWorldService.WorldsDir` static, used by the Worlds panel's
/// "Open folder" action.
#[tauri::command]
pub fn bedrock_worlds_dir() -> String {
    gl_bedrock::worlds_dir().to_string_lossy().into_owned()
}

#[tauri::command]
pub fn list_bedrock_packs(instance_dir: String, kind: String) -> Result<Vec<BedrockPack>, String> {
    let kind = parse_pack_kind(&kind)?;
    let dir_name = match kind {
        PackKind::Resource => "resource_packs",
        PackKind::Behavior => "behavior_packs",
        PackKind::Skin => "skin_packs",
        PackKind::ResourceDev => "development_resource_packs",
        PackKind::BehaviorDev => "development_behavior_packs",
        PackKind::SkinDev => "development_skin_packs",
    };
    let svc = BedrockPackService::new();
    svc.list_packs(&bedrock_dir(&instance_dir, dir_name), kind)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_bedrock_pack(pack_path: String) -> Result<(), String> {
    let svc = BedrockPackService::new();
    svc.delete_pack(Path::new(&pack_path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_bedrock_backups() -> Result<Vec<BedrockBackup>, String> {
    let svc = BedrockBackupService::new(backups_dir());
    svc.list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_bedrock_backup(
    instance_dir: String,
    label: Option<String>,
) -> Result<String, String> {
    let svc = BedrockBackupService::new(backups_dir());
    svc.create_backup_from(&bedrock_root(&instance_dir), label.as_deref())
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_bedrock_backup(backup_path: String, instance_dir: String) -> Result<(), String> {
    let svc = BedrockBackupService::new(backups_dir());
    svc.restore_backup_into(Path::new(&backup_path), &bedrock_root(&instance_dir))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_bedrock_backup(backup_path: String) -> Result<(), String> {
    let svc = BedrockBackupService::new(backups_dir());
    svc.delete_backup(Path::new(&backup_path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_bedrock_screenshots(_instance_dir: String) -> Result<Vec<ScreenshotFile>, String> {
    // C# `BedrockScreenshotService.ComMojangScreenshotsDir` is a static
    // property over `CurseForgeService.ComMojangRoot` — screenshots always
    // come from the live install, never from a managed instance folder. The
    // parameter is kept so the frontend's existing call shape still binds.
    let svc = gl_bedrock::BedrockScreenshotService::new(
        gl_bedrock::screenshots_dir(),
        dirs_captures_dir(),
    );
    svc.list_screenshots().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_bedrock_screenshot(path: String) -> Result<(), String> {
    let svc = gl_bedrock::BedrockScreenshotService::new(
        Path::new(&path).parent().unwrap_or(Path::new(".")),
        dirs_captures_dir(),
    );
    svc.delete_file(Path::new(&path)).map_err(|e| e.to_string())
}

fn dirs_captures_dir() -> std::path::PathBuf {
    std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
        .join("Videos")
        .join("Captures")
}

#[tauri::command]
pub fn list_bedrock_instances(state: State<'_, AppState>) -> Result<Vec<BedrockInstance>, String> {
    let svc = state
        .bedrock_instances
        .lock()
        .map_err(|_| "bedrock instance service lock poisoned".to_string())?;
    Ok(svc.instances())
}

#[tauri::command]
pub fn create_bedrock_instance(
    state: State<'_, AppState>,
    name: String,
    version_tag: String,
) -> Result<BedrockInstance, String> {
    let mut svc = state
        .bedrock_instances
        .lock()
        .map_err(|_| "bedrock instance service lock poisoned".to_string())?;
    svc.create(&name, &version_tag).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_bedrock_instance(
    state: State<'_, AppState>,
    id: String,
    new_name: String,
) -> Result<(), String> {
    let mut svc = state
        .bedrock_instances
        .lock()
        .map_err(|_| "bedrock instance service lock poisoned".to_string())?;
    svc.rename(&id, &new_name).map_err(|e| e.to_string())
}

/// Switches the active Bedrock instance, swapping the managed com.mojang
/// folders between the live install and the target instance, then switching
/// the installed Minecraft build to the instance's recorded version tag. Port
/// of `BedrockInstanceService.SwitchToAsync`.
///
/// This moves real game data on disk, so it resolves the live root the same
/// way every other Bedrock command now does rather than trusting a path from
/// the frontend.
///
/// Returns the original's **warning string**, not a bool: empty means fully
/// successful, and a non-empty value means the data swap succeeded but the
/// game version did not change. Both halves are reported because the data swap
/// is the destructive part and must not be reported as a failure just because
/// the version switch afterwards didn't take.
#[tauri::command]
pub fn switch_bedrock_instance(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let live_root = gl_bedrock::com_mojang_root();

    let version_tag = {
        let mut svc = state.bedrock_instances.lock().map_err(|e| e.to_string())?;
        // A `false` here means the requested instance was already active, so
        // the C# returns early with no warning and no version switch.
        if !svc.switch_to(&id, live_root).map_err(|e| e.to_string())? {
            return Ok(String::new());
        }
        svc.instances()
            .into_iter()
            .find(|i| i.id.eq_ignore_ascii_case(&id))
            .map(|i| i.version_tag)
            .unwrap_or_default()
    };

    // Port of `SwitchToAsync:158` — no tag recorded means nothing to switch.
    if version_tag.trim().is_empty() {
        return Ok(String::new());
    }

    let manifest = crate::commands::versions::vanilla_bedrock_version_dir(&version_tag)
        .join("AppxManifest.xml");
    if !manifest.is_file() {
        return Ok(format!(
            "Switched instance data, but Minecraft {version_tag} isn't downloaded — game version left as-is."
        ));
    }

    match gl_bedrock::wu::unregister_current()
        .and_then(|_| gl_bedrock::wu::register_version(&manifest))
    {
        Ok(()) => Ok(String::new()),
        Err(e) => Ok(format!(
            "Switched instance data, but couldn't switch game version: {e}"
        )),
    }
}

/// Archives the live com.mojang state into the active instance without
/// switching. Port of `SaveActiveAsync`.
#[tauri::command]
pub fn save_active_bedrock_instance(state: State<'_, AppState>) -> Result<(), String> {
    let live_root = gl_bedrock::com_mojang_root();
    let svc = state.bedrock_instances.lock().map_err(|e| e.to_string())?;
    svc.save_active(live_root).map_err(|e| e.to_string())
}

/// Sets the game version tag recorded against an instance.
#[tauri::command]
pub fn set_bedrock_instance_version(
    state: State<'_, AppState>,
    id: String,
    version_tag: String,
) -> Result<(), String> {
    let mut svc = state.bedrock_instances.lock().map_err(|e| e.to_string())?;
    svc.set_version(&id, &version_tag)
        .map_err(|e| e.to_string())
}

/// The id of the currently active Bedrock instance, if any.
#[tauri::command]
pub fn active_bedrock_instance_id(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let svc = state.bedrock_instances.lock().map_err(|e| e.to_string())?;
    Ok(svc.active_instance_id().map(str::to_string))
}

#[tauri::command]
pub fn delete_bedrock_instance(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let mut svc = state
        .bedrock_instances
        .lock()
        .map_err(|_| "bedrock instance service lock poisoned".to_string())?;
    svc.delete(&id).map_err(|e| e.to_string())
}
