use std::path::{Path, PathBuf};

use gl_bedrock::{
    BedrockBackup, BedrockBackupService, BedrockInstance, BedrockPack, BedrockPackService,
    BedrockRoot, BedrockWorld, BedrockWorldService, ImportOutcome, PackKind, ScreenshotFile,
};
use tauri::State;

use crate::state::AppState;

/// Resolves the live `com.mojang` folder (resource packs, worlds, behaviour
/// packs) for the Settings panel's "Open Minecraft folder" button. Reuses
/// the same per-account root resolution the world/pack/screenshot commands
/// already depend on, rather than a fresh lookup.
#[tauri::command]
pub async fn resolve_com_mojang_folder() -> String {
    gl_bedrock::com_mojang_root().display().to_string()
}

/// Imports a dropped Bedrock content file (`.mcworld`/`.mctemplate`/`.mcpack`/
/// `.mcaddon`) into the given instance's folder. Mirrors the original app's
/// drag-drop import handling for Bedrock content.
#[tauri::command]
pub async fn import_bedrock_content(
    source_path: String,
    instance_dir: String,
) -> Result<ImportOutcome, String> {
    gl_bedrock::import_content(Path::new(&source_path), Path::new(&instance_dir))
        .map_err(|e| e.to_string())
}

/// Public alias so other command modules (e.g. the conflict detector in
/// `bedrock_addons.rs`) can reuse the same kind-string parsing instead of
/// duplicating it.
pub fn parse_pack_kind_pub(kind: &str) -> Result<PackKind, String> {
    parse_pack_kind(kind)
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

/// The live com.mojang directory for `folder_name`.
///
/// The browsing panels read the LIVE folder, never an instance's archive -
/// `BedrockWorldService.WorldsDir` and `BedrockPackService.DirFor` are static
/// paths in the original, with no instance parameter. The instance system
/// *swaps* data between the live folder and an instance directory
/// (`SwitchToAsync`), so live always holds the active instance's data and an
/// instance directory is a dormant snapshot. Reading from the snapshot means a
/// user who has never switched instances sees nothing, because the default
/// instance's archive is empty.
fn live_dir(folder_name: &str) -> PathBuf {
    BedrockRoot::Live.dir_for(folder_name)
}

#[tauri::command]
pub async fn list_bedrock_worlds(_instance_dir: String) -> Result<Vec<BedrockWorld>, String> {
    // `_instance_dir` is accepted and ignored - see `live_dir`. Kept in the
    // signature so the panels don't all need changing for a backend-side fix.
    let svc = BedrockWorldService::new(live_dir("minecraftWorlds"));
    svc.list_worlds().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_bedrock_world(world_path: String) -> Result<(), String> {
    let worlds = live_dir("minecraftWorlds");
    let target = super::ensure_direct_child(&worlds, Path::new(&world_path))?;
    let svc = BedrockWorldService::new(&worlds);
    svc.delete_world(&target).map_err(|e| e.to_string())
}

/// Exports a world as a `.mcworld`.
///
/// `export_dir` is optional: the C# `BedrockWorldService.ExportWorldAsync`
/// chooses the destination itself rather than taking one from the caller,
/// always writing to `%USERPROFILE%\Glacier Launcher\exported-worlds`. That
/// default lives here so callers don't have to guess it.
#[tauri::command]
pub async fn export_bedrock_world(
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
pub async fn bedrock_backups_dir() -> String {
    backups_dir().to_string_lossy().into_owned()
}

/// The live Screenshots directory. Always the live install, never a managed
/// instance - the original's screenshots path is a static property.
#[tauri::command]
pub async fn bedrock_screenshots_dir() -> String {
    gl_bedrock::screenshots_dir().to_string_lossy().into_owned()
}

/// The live `minecraftWorlds` directory - port of the
/// `BedrockWorldService.WorldsDir` static, used by the Worlds panel's
/// "Open folder" action.
#[tauri::command]
pub async fn bedrock_worlds_dir() -> String {
    gl_bedrock::worlds_dir().to_string_lossy().into_owned()
}

#[tauri::command]
pub async fn list_bedrock_packs(
    _instance_dir: String,
    kind: String,
) -> Result<Vec<BedrockPack>, String> {
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
    svc.list_packs(&live_dir(dir_name), kind)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_bedrock_pack(pack_path: String) -> Result<(), String> {
    // A pack folder is a direct child of one of the six pack directories.
    let target = Path::new(&pack_path);
    let target = [
        "resource_packs",
        "behavior_packs",
        "skin_packs",
        "development_resource_packs",
        "development_behavior_packs",
        "development_skin_packs",
    ]
    .iter()
    .find_map(|dir| super::ensure_direct_child(&live_dir(dir), target).ok())
    .ok_or_else(|| format!("Refusing to delete {pack_path}: not an installed Bedrock pack."))?;
    let svc = BedrockPackService::new();
    svc.delete_pack(&target).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_bedrock_backups() -> Result<Vec<BedrockBackup>, String> {
    let svc = BedrockBackupService::new(backups_dir());
    svc.list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_bedrock_backup(
    instance_dir: String,
    label: Option<String>,
) -> Result<String, String> {
    let svc = BedrockBackupService::new(backups_dir());
    svc.create_backup_from(&bedrock_root(&instance_dir), label.as_deref())
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_bedrock_backup(
    backup_path: String,
    instance_dir: String,
) -> Result<(), String> {
    let svc = BedrockBackupService::new(backups_dir());
    svc.restore_backup_into(Path::new(&backup_path), &bedrock_root(&instance_dir))
        .map_err(|e| e.to_string())
}

/// QOL "restore to new world/instance": extracts a backup into a brand-new
/// Bedrock instance instead of overwriting the live install / an existing
/// instance. Lets you look at (or cherry-pick from) an old snapshot without
/// touching what you're currently playing, or recover from a bad restore by
/// just deleting the new instance. Mirrors `restore_bedrock_backup` but
/// targets a freshly-created instance directory via
/// `restore_backup_into_folder` (the same instance-agnostic extraction path
/// `restore_backup_into` uses under the hood).
#[tauri::command]
pub async fn restore_bedrock_backup_to_new_instance(
    state: State<'_, AppState>,
    backup_path: String,
    name: Option<String>,
) -> Result<BedrockInstance, String> {
    let instance = {
        let mut instances = state.bedrock_instances.lock().map_err(|e| e.to_string())?;
        instances
            .create(name.as_deref().unwrap_or("Restored backup"), "")
            .map_err(|e| e.to_string())?
    };

    let svc = BedrockBackupService::new(backups_dir());
    svc.restore_backup_into_folder(Path::new(&backup_path), Path::new(&instance.directory))
        .map_err(|e| e.to_string())?;

    Ok(instance)
}

/// QOL "backup diff": compares two backups' contents (added/removed/changed
/// files by name+size) so you can see what actually differs before
/// deciding which one to restore, instead of just eyeballing two timestamps
/// and total sizes.
#[tauri::command]
pub async fn diff_bedrock_backups(
    path_a: String,
    path_b: String,
) -> Result<gl_bedrock::BackupDiff, String> {
    let svc = BedrockBackupService::new(backups_dir());
    svc.diff_backups(Path::new(&path_a), Path::new(&path_b))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_bedrock_backup(backup_path: String) -> Result<(), String> {
    let root = backups_dir();
    let target = super::ensure_direct_child(&root, Path::new(&backup_path))?;
    let svc = BedrockBackupService::new(root);
    svc.delete_backup(&target).map_err(|e| e.to_string())
}

/// Feature 20 — Bedrock world backup scheduler. Called right before a
/// Bedrock launch: bumps the launches-since-backup counter and, if it has
/// reached the configured `bedrock_auto_backup_every_n_launches` threshold,
/// fires an automatic backup by reusing the exact same
/// `BedrockBackupService::create_backup_from` the manual "Back up now" button
/// calls (no separate zip/copy logic). `0` disables the feature, matching the
/// convention used elsewhere in this settings struct for "off".
///
/// Returns `Some(backup_path)` when an auto-backup ran, `None` otherwise.
/// Never fails the launch: a backup error is logged by the caller and
/// swallowed here, same as other best-effort pre-launch steps
/// (`accelerate_loading_ms`, Discord presence, etc).
pub fn maybe_run_bedrock_auto_backup() -> Result<Option<String>, String> {
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    if settings.bedrock_auto_backup_every_n_launches <= 0 {
        return Ok(None);
    }

    settings.bedrock_launches_since_backup += 1;

    let should_backup =
        settings.bedrock_launches_since_backup >= settings.bedrock_auto_backup_every_n_launches;

    let result = if should_backup {
        settings.bedrock_launches_since_backup = 0;
        let svc = BedrockBackupService::new(backups_dir());
        Some(
            svc.create_backup_from(&BedrockRoot::Live, Some("Auto-backup"))
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    gl_settings::save(&settings).map_err(|e| e.to_string())?;
    Ok(result)
}

/// Feature 23 — whether the one-time marketplace-content reminder banner has
/// been dismissed already.
#[tauri::command]
pub async fn bedrock_marketplace_reminder_dismissed() -> Result<bool, String> {
    Ok(gl_settings::load()
        .map_err(|e| e.to_string())?
        .bedrock_marketplace_reminder_dismissed)
}

/// Persists the dismissal so the banner doesn't come back.
#[tauri::command]
pub async fn dismiss_bedrock_marketplace_reminder() -> Result<(), String> {
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    settings.bedrock_marketplace_reminder_dismissed = true;
    gl_settings::save(&settings).map_err(|e| e.to_string())
}

/// Reads the auto-backup-every-N-launches setting for the Backups panel.
#[tauri::command]
pub async fn bedrock_auto_backup_interval() -> Result<i32, String> {
    Ok(gl_settings::load()
        .map_err(|e| e.to_string())?
        .bedrock_auto_backup_every_n_launches)
}

/// Sets the auto-backup-every-N-launches setting. `0` disables it.
#[tauri::command]
pub async fn set_bedrock_auto_backup_interval(every_n_launches: i32) -> Result<(), String> {
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    settings.bedrock_auto_backup_every_n_launches = every_n_launches.max(0);
    // Restarting the count on a settings change avoids an immediate backup
    // firing on the very next launch just because the old counter had
    // already accumulated toward a since-lowered threshold.
    settings.bedrock_launches_since_backup = 0;
    gl_settings::save(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_bedrock_screenshots(
    _instance_dir: String,
) -> Result<Vec<ScreenshotFile>, String> {
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
pub async fn delete_bedrock_screenshot(path: String) -> Result<(), String> {
    let shots = gl_bedrock::screenshots_dir();
    let captures = dirs_captures_dir();
    let target = super::ensure_direct_child(&shots, Path::new(&path))
        .or_else(|_| super::ensure_direct_child(&captures, Path::new(&path)))?;
    let svc = gl_bedrock::BedrockScreenshotService::new(
        target.parent().unwrap_or(Path::new(".")),
        captures,
    );
    svc.delete_file(&target).map_err(|e| e.to_string())?;
    // A deleted file can't stay favorited - drop it from the list too rather
    // than leaving a dangling path that would just silently never match
    // anything list_bedrock_screenshots returns again.
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    settings.bedrock_favorite_screenshots.retain(|p| p != &path);
    gl_settings::save(&settings).map_err(|e| e.to_string())
}

/// The set of screenshot paths the user has starred, for the Photos panel to
/// mark tiles and sort/filter by.
#[tauri::command]
pub async fn list_favorite_screenshots() -> Result<Vec<String>, String> {
    Ok(gl_settings::load()
        .map_err(|e| e.to_string())?
        .bedrock_favorite_screenshots)
}

/// Toggles one screenshot's favorite state and returns the new state, so the
/// frontend doesn't need a separate read-then-write round trip.
#[tauri::command]
pub async fn toggle_favorite_screenshot(path: String) -> Result<bool, String> {
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    let now_favorited = if let Some(idx) = settings
        .bedrock_favorite_screenshots
        .iter()
        .position(|p| p == &path)
    {
        settings.bedrock_favorite_screenshots.remove(idx);
        false
    } else {
        settings.bedrock_favorite_screenshots.push(path);
        true
    };
    gl_settings::save(&settings).map_err(|e| e.to_string())?;
    Ok(now_favorited)
}

fn dirs_captures_dir() -> std::path::PathBuf {
    std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
        .join("Videos")
        .join("Captures")
}

#[tauri::command]
pub async fn list_bedrock_instances(
    state: State<'_, AppState>,
) -> Result<Vec<BedrockInstance>, String> {
    let svc = state
        .bedrock_instances
        .lock()
        .map_err(|_| "bedrock instance service lock poisoned".to_string())?;
    Ok(svc.instances())
}

#[tauri::command]
pub async fn create_bedrock_instance(
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
pub async fn rename_bedrock_instance(
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

#[tauri::command]
pub async fn set_bedrock_instance_accent_color(
    state: State<'_, AppState>,
    id: String,
    color: Option<String>,
) -> Result<(), String> {
    let mut svc = state
        .bedrock_instances
        .lock()
        .map_err(|_| "bedrock instance service lock poisoned".to_string())?;
    svc.set_accent_color(&id, color).map_err(|e| e.to_string())
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
pub async fn switch_bedrock_instance(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
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
pub async fn save_active_bedrock_instance(state: State<'_, AppState>) -> Result<(), String> {
    let live_root = gl_bedrock::com_mojang_root();
    let svc = state.bedrock_instances.lock().map_err(|e| e.to_string())?;
    svc.save_active(live_root).map_err(|e| e.to_string())
}

/// Sets the game version tag recorded against an instance.
#[tauri::command]
pub async fn set_bedrock_instance_version(
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
pub async fn active_bedrock_instance_id(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let svc = state.bedrock_instances.lock().map_err(|e| e.to_string())?;
    Ok(svc.active_instance_id().map(str::to_string))
}

#[tauri::command]
pub async fn delete_bedrock_instance(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut svc = state
        .bedrock_instances
        .lock()
        .map_err(|_| "bedrock instance service lock poisoned".to_string())?;
    svc.delete(&id).map_err(|e| e.to_string())
}
