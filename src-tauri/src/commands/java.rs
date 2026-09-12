use gl_java::{EffectiveLaunchConfig, GlobalJavaDefaults, InstanceSettings, JavaInstance, JavaVersion, ModLoaderKind};
use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;

#[derive(Clone, serde::Serialize)]
struct JavaInstallProgressPayload {
    version_id: String,
    stage: String,
    percent: f64,
}

/// Downloads a Java Edition version's client jar, libraries, natives, and
/// assets into the shared versions pool (`<java root>/versions`), the same
/// directory `list_java_versions` scans for `is_installed`/`has_jar`.
/// Mirrors `JavaInstallService.InstallAsync`, reporting progress via the
/// `java-install-progress` event so the frontend can render an inline bar.
#[tauri::command]
pub async fn install_java_version(
    app: AppHandle,
    state: State<'_, AppState>,
    version: JavaVersion,
) -> Result<(), String> {
    let mc_dir = {
        let instances = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        instances.root_dir().to_path_buf()
    };

    let version_id = version.id.clone();
    let installer = gl_java::InstallService::new();
    installer
        .install(&version, &mc_dir, |progress| {
            let _ = app.emit(
                "java-install-progress",
                JavaInstallProgressPayload {
                    version_id: version_id.clone(),
                    stage: progress.stage.clone(),
                    percent: progress.percent,
                },
            );
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_java_versions(state: State<'_, AppState>) -> Result<Vec<JavaVersion>, String> {
    let versions_dir = {
        let instances = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        instances.root_dir().join("versions")
    };
    let mut guard = state.java_versions.lock().await;
    guard
        .get_versions(&versions_dir, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_java_instances(state: State<'_, AppState>) -> Result<Vec<JavaInstance>, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc.instances())
}

#[tauri::command]
pub fn create_java_instance(
    state: State<'_, AppState>,
    name: String,
    version_id: String,
) -> Result<JavaInstance, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.create(&name, &version_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_java_instance(
    state: State<'_, AppState>,
    id: String,
    new_name: String,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.rename(&id, &new_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_java_instance_accent_color(
    state: State<'_, AppState>,
    id: String,
    color: Option<String>,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_accent_color(&id, color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_java_instance(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.delete(&id).map_err(|e| e.to_string())
}

/// Clones an instance's metadata and copies its full directory tree
/// (mods, saves, config, etc.) into a new instance named "<name> Copy".
/// The `InstanceService::duplicate` logic already existed; it just had no
/// `#[tauri::command]` wrapper reaching it from the frontend.
#[tauri::command]
pub fn duplicate_java_instance(
    state: State<'_, AppState>,
    id: String,
) -> Result<JavaInstance, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.duplicate(&id).map_err(|e| e.to_string())
}

/// Imports a `.zip` modpack/instance file into a new instance. Mirrors
/// `Home.Clients.cs::ImportInstanceZip`'s non-`.mrpack` branch
/// (`JavaInstanceService.ImportModpackAsync`): creates an instance named
/// after the zip's filename, then extracts it (auto-detecting a
/// CurseForge/Modrinth `overrides/` root or a MultiMC-style
/// `.minecraft/`/`minecraft/` root, falling back to a flat extract).
/// `import_modpack`/`detect_multimc_root` already existed in
/// `gl_java::modpack_transfer` with no command wrapper reaching them.
#[tauri::command]
pub fn import_java_instance_zip(
    state: State<'_, AppState>,
    zip_path: String,
) -> Result<JavaInstance, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;

    let path = std::path::Path::new(&zip_path);
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported Instance".to_string());

    let instance = svc.create(&name, "").map_err(|e| e.to_string())?;
    if let Err(e) = gl_java::modpack_transfer::import_modpack(path, std::path::Path::new(&instance.directory)) {
        let _ = svc.delete(&instance.id);
        return Err(e.to_string());
    }
    Ok(instance)
}

/// Imports the user's official `%APPDATA%\.minecraft` install into a new
/// "Official .minecraft" instance. Mirrors
/// `Home.Clients.cs::ImportOfficialMinecraft` /
/// `JavaInstanceService.ImportOfficialMinecraftAsync`.
/// `import_official_minecraft` already existed in `gl_java::modpack_transfer`
/// with no command wrapper reaching it.
#[tauri::command]
pub fn import_official_minecraft(state: State<'_, AppState>) -> Result<JavaInstance, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;

    let instance = svc
        .create("Official .minecraft", "")
        .map_err(|e| e.to_string())?;
    if let Err(e) =
        gl_java::modpack_transfer::import_official_minecraft(std::path::Path::new(&instance.directory))
    {
        let _ = svc.delete(&instance.id);
        return Err(e.to_string());
    }
    Ok(instance)
}

#[tauri::command]
pub fn set_java_instance_ram(
    state: State<'_, AppState>,
    id: String,
    min_mb: i32,
    max_mb: i32,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_ram(&id, min_mb, max_mb).map_err(|e| e.to_string())
}

/// Lists the built-in JVM-argument templates/presets ("Vanilla", "Balanced",
/// "Performance", "Low-End") offered when creating an instance or changing
/// its JVM-args preset later. JVM flags only — no mod/API downloads.
#[tauri::command]
pub fn list_jvm_presets() -> Vec<gl_java::JvmPreset> {
    gl_java::JVM_PRESETS
        .iter()
        .map(|p| gl_java::JvmPreset {
            id: p.id,
            label: p.label,
            description: p.description,
            args: p.args,
        })
        .collect()
}

/// Like [`create_java_instance`], but also applies a built-in JVM-args
/// template by id (see [`list_jvm_presets`]). Passing `None`/an unknown id
/// behaves exactly like `create_java_instance`.
#[tauri::command]
pub fn create_java_instance_from_template(
    state: State<'_, AppState>,
    name: String,
    version_id: String,
    template: Option<String>,
) -> Result<JavaInstance, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.create_with_template(&name, &version_id, template.as_deref())
        .map_err(|e| e.to_string())
}

/// Sets an instance's extra JVM args directly, or applies a built-in preset
/// by id — pass `preset` to record which built-in preset these args came
/// from (or `None` for "custom"). Used by the JVM-args-preset dropdown in
/// instance settings.
#[tauri::command]
pub fn set_instance_jvm_args(
    state: State<'_, AppState>,
    id: String,
    extra_jvm_args: Vec<String>,
    preset: Option<String>,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_instance_jvm_args(&id, extra_jvm_args, preset)
        .map_err(|e| e.to_string())
}

/// Applies a built-in JVM-args preset by id (looking up its args), following
/// the exact `set_instance_jvm_args` path. A convenience wrapper so the
/// frontend dropdown doesn't need to know the preset's arg list itself.
#[tauri::command]
pub fn set_instance_jvm_preset(
    state: State<'_, AppState>,
    id: String,
    preset: String,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    let args = gl_java::presets::args_for(&preset);
    svc.set_instance_jvm_args(&id, args, Some(preset))
        .map_err(|e| e.to_string())
}

/// Sets (or clears, with `None`) an instance's group/folder label.
#[tauri::command]
pub fn set_instance_group(
    state: State<'_, AppState>,
    id: String,
    group: Option<String>,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_instance_group(&id, group).map_err(|e| e.to_string())
}

/// Exports an instance's entire directory tree as a zip at `dest_path`
/// (chosen by the frontend via the same `@tauri-apps/plugin-dialog` `save`
/// flow used elsewhere). `zip` is already a workspace dependency (see
/// `crates/gl-java/Cargo.toml`), so this is a real zip export, not a folder
/// copy.
#[tauri::command]
pub fn export_java_instance(
    state: State<'_, AppState>,
    id: String,
    dest_path: String,
) -> Result<(), String> {
    let instance = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        svc.get(&id).ok_or_else(|| format!("Java instance not found: {id}"))?
    };
    gl_java::export_instance(
        std::path::Path::new(&instance.directory),
        std::path::Path::new(&dest_path),
    )
    .map_err(|e| e.to_string())
}

/// Returns an instance's per-instance settings overrides (not the resolved
/// effective config — see [`get_effective_launch_config`] for that).
#[tauri::command]
pub fn get_instance_settings(
    state: State<'_, AppState>,
    id: String,
) -> Result<InstanceSettings, String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    Ok(svc
        .get(&id)
        .ok_or_else(|| format!("Java instance not found: {id}"))?
        .settings)
}

/// Replaces an instance's per-instance settings overrides wholesale. The
/// frontend reads the current value via [`get_instance_settings`], mutates
/// the field the user changed, and sends the whole struct back — every
/// field is independently `Option`, so leaving the rest unset means
/// "inherit from global defaults".
#[tauri::command]
pub fn set_instance_settings(
    state: State<'_, AppState>,
    id: String,
    settings: InstanceSettings,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_instance_settings(&id, settings)
        .map_err(|e| e.to_string())
}

/// Clears a single settings field back to "inherit from default", by name
/// (`"memory"`, `"resolution"`, `"fullscreen"`, `"java_path"`,
/// `"custom_jvm_args"`, `"extra_game_args"`, `"quick_play"`). Unknown names
/// are a no-op.
#[tauri::command]
pub fn reset_instance_setting(
    state: State<'_, AppState>,
    id: String,
    field: String,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    let mut settings = svc
        .get(&id)
        .ok_or_else(|| format!("Java instance not found: {id}"))?
        .settings;
    match field.as_str() {
        "memory" => settings.memory = None,
        "resolution" => {
            settings.resolution = None;
            settings.use_custom_resolution = None;
        }
        "fullscreen" => settings.fullscreen = None,
        "java_path" => settings.java_path = None,
        "custom_jvm_args" => settings.custom_jvm_args = None,
        "extra_game_args" => settings.extra_game_args.clear(),
        "quick_play" => settings.quick_play = None,
        _ => {}
    }
    svc.set_instance_settings(&id, settings)
        .map_err(|e| e.to_string())
}

/// Sets (or clears, with `loader: None`) an instance's configured mod
/// loader. Does not install anything itself — Phase 2's loader installer
/// commands call this once install succeeds.
#[tauri::command]
pub fn set_instance_loader(
    state: State<'_, AppState>,
    id: String,
    loader: Option<ModLoaderKind>,
    loader_version: Option<String>,
    loader_profile_id: Option<String>,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_instance_loader(&id, loader, loader_version, loader_profile_id)
        .map_err(|e| e.to_string())
}

/// Sets (or clears) an instance's preferred account id, referencing
/// `LauncherSettings.java_accounts`. Does not validate the id against the
/// current account list — an id that no longer resolves is treated the
/// same as "use the active global account" at launch time.
#[tauri::command]
pub fn set_instance_account(
    state: State<'_, AppState>,
    id: String,
    account_id: Option<String>,
) -> Result<(), String> {
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    svc.set_instance_account(&id, account_id)
        .map_err(|e| e.to_string())
}

/// Resolves what `launch_java_instance` would actually use for this
/// instance right now — global defaults layered under any per-instance
/// overrides — so the settings UI can show e.g. "4096 MB — from default"
/// instead of guessing.
#[tauri::command]
pub fn get_effective_launch_config(
    state: State<'_, AppState>,
    id: String,
) -> Result<EffectiveLaunchConfig, String> {
    let settings = gl_settings::load().map_err(|e| e.to_string())?;
    let svc = state
        .java_instances
        .lock()
        .map_err(|_| "java instance service lock poisoned".to_string())?;
    let instance = svc
        .get(&id)
        .ok_or_else(|| format!("Java instance not found: {id}"))?;
    let defaults = GlobalJavaDefaults {
        max_ram_mb: settings.java_max_ram_mb,
        min_ram_mb: settings.java_min_ram_mb,
        custom_jvm_args: settings.java_custom_jvm_args.clone(),
        fullscreen: settings.java_fullscreen,
        use_custom_resolution: settings.java_use_custom_resolution,
        window_width: settings.java_window_width,
        window_height: settings.java_window_height,
        server_address: settings.java_server_address.clone(),
        server_port: settings.java_server_port,
        java_runtime_path: settings.java_runtime_path.clone(),
    };
    Ok(gl_java::effective(&instance, &defaults))
}
