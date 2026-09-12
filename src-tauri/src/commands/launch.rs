//! Thin #[tauri::command] wrappers around `gl_launch`'s Java/Bedrock launch
//! pipelines. Business logic (arg construction, package resolution,
//! activation, injection) lives in `gl_launch`/`gl_java`/`gl_injection`;
//! this module resolves instance config from `AppState` + settings, streams
//! the launched process's output to a second `WebviewWindow`, and maps
//! errors to `String` for the frontend.
//!
//! Also drives Discord Rich Presence transitions (playing/idle) around each
//! launch, best-effort — see [`set_discord_playing`]/[`set_discord_idle`].

use gl_launch::java::{JavaLaunchRequest, ResolvedAuth};
use gl_launch::{bedrock as bedrock_launch, java as gl_java_launch};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::state::AppState;

/// Best-effort: sets Discord Rich Presence to "Playing Minecraft <tag>",
/// silently doing nothing if the Discord service isn't running (RPC only
/// starts when `Settings.DiscordRichPresence` is on).
fn set_discord_playing(app: &AppHandle, version_tag: &str, client_name: Option<&str>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut svc) = state.discord.lock() {
            svc.set_in_game_presence(version_tag, client_name);
        }
    }
}

/// Best-effort: returns Discord Rich Presence to its idle state once the
/// game process exits.
fn set_discord_idle(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut svc) = state.discord.lock() {
            svc.set_idle_presence();
        }
    }
}

const CONSOLE_WINDOW_LABEL: &str = "game-console";
const CONSOLE_LOG_EVENT: &str = "game-console://log";
const CONSOLE_EXIT_EVENT: &str = "game-console://exit";
const CONSOLE_STATUS_EVENT: &str = "game-console://status";
const CONSOLE_TITLE_EVENT: &str = "game-console://title";

/// Phase 4: process-registry events, distinct from the console-window-scoped
/// `CONSOLE_*_EVENT`s above (which are single-slot — one console window, one
/// "current" process). These are keyed per-instance so the Footer badge and
/// LogsPanel live-tail can track *every* currently-running Java instance, not
/// just whichever one most recently opened the console window.
pub const JAVA_LOG_LINE_EVENT: &str = "java-log-line";
pub const JAVA_PROCESS_CHANGED_EVENT: &str = "java-process-changed";

/// Synthetic registry key for `launch_java_version`'s bare-version launches,
/// which have no instance id to key off of.
fn version_registry_key(version_id: &str) -> String {
    format!("version:{version_id}")
}

#[derive(Clone, serde::Serialize)]
struct JavaLogLinePayload<'a> {
    instance_id: &'a str,
    line: String,
    level: &'a str, // "stdout" | "stderr" | "info"
}

#[derive(Clone, serde::Serialize)]
struct JavaProcessChangedPayload<'a> {
    instance_id: &'a str,
    status: &'a str, // "started" | "running" | "exited"
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
}

/// PID of the most recently launched Java process, if it may still be
/// running. Not stored on `AppState` (kept local to this module): mirrors
/// the original's `_launchCts`/force-close scope, which only ever needs to
/// reach "the thing we just tried to launch," not general app state.
static LAST_JAVA_PID: Mutex<Option<u32>> = Mutex::new(None);
// TODO: multi-instance parallel launch needs per-instance process tracking;
// `LAST_JAVA_PID` above (and the equivalent single-slot state backing
// `is_java_game_running`/`cancel_launch`) is a single global slot, not keyed
// by instance id — launching a second instance would silently overwrite the
// first one's tracked PID. Not implemented here; would need a
// `HashMap<instance_id, u32>` on `AppState` plus updates to every reader.

/// The console window's current in-content title (`TitleLabel.Text` in the
/// original). Read by the console page on mount via [`console_content_title`]
/// — a plain `emit` on window creation would race the page's own listener
/// registration, since the webview hasn't loaded yet when the window is
/// built.
static CONSOLE_TITLE: Mutex<String> = Mutex::new(String::new());

#[tauri::command]
pub fn console_content_title() -> String {
    CONSOLE_TITLE.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Force-kills a process by PID, matching `Process.Kill(entireProcessTree:
/// true)` from the original's `ForceCloseLaunch`. `taskkill /T` recurses to
/// child processes (e.g. a launcher wrapper spawning the real `java.exe`).
fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Force-aborts the in-flight launch: mirrors `ForceCloseLaunch` — kills the
/// most recently spawned Java process (if any) and any running Bedrock game
/// process. Safe to call even if nothing is currently launching (both kills
/// are no-ops when there's no matching PID).
#[tauri::command]
pub fn cancel_launch() {
    if let Ok(mut guard) = LAST_JAVA_PID.lock() {
        if let Some(pid) = guard.take() {
            kill_process_tree(pid);
        }
    }
    if let Some(pid) = bedrock_launch::find_game_process() {
        kill_process_tree(pid);
    }
}

#[derive(Clone, serde::Serialize)]
struct ConsoleLogPayload<'a> {
    stream: &'a str, // "stdout" | "stderr" | "info"
    line: String,
}

#[derive(Clone, serde::Serialize)]
struct ConsoleExitPayload {
    code: Option<i32>,
}

/// Mirrors `GameConsoleWindow.SetStatus`/`MarkRunning`/`MarkFailed`: the
/// console window's status dot + label, driven by launch progress rather
/// than only by process-exit (which `CONSOLE_EXIT_EVENT` already covers).
#[derive(Clone, serde::Serialize)]
struct ConsoleStatusPayload {
    status: &'static str, // "starting" | "running" | "failed"
    pid: Option<u32>,
    message: Option<String>,
}

fn emit_status(app: &AppHandle, status: &'static str, pid: Option<u32>, message: Option<String>) {
    let _ = app.emit(
        CONSOLE_STATUS_EVENT,
        ConsoleStatusPayload {
            status,
            pid,
            message,
        },
    );
}

/// Opens (or focuses) the live console window used to stream stdout/stderr
/// from a launched game process. Simpler than the original's Win32 console
/// window (`GameConsoleWindow.xaml`) — just a second Tauri webview that
/// listens for `CONSOLE_LOG_EVENT`/`CONSOLE_EXIT_EVENT`.
fn open_console_window(app: &AppHandle, content_title: &str) -> Result<(), String> {
    // The OS-level window title stays the static "Game Console" — matches
    // `GameConsoleWindow.xaml`'s `Title="Game Console"`. The per-launch name
    // (e.g. "Minecraft Bedrock · Flarial Client") is the *content* header
    // (`TitleLabel.Text`, set via `SetTitle`), shown below the status dot,
    // not the OS chrome.
    if let Ok(mut guard) = CONSOLE_TITLE.lock() {
        *guard = content_title.to_string();
    }

    if let Some(win) = app.get_webview_window(CONSOLE_WINDOW_LABEL) {
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit_to(CONSOLE_WINDOW_LABEL, CONSOLE_TITLE_EVENT, content_title);
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        CONSOLE_WINDOW_LABEL,
        WebviewUrl::App("console.html".into()),
    )
    .title("Game Console")
    .inner_size(900.0, 560.0)
    .build()
    .map(|_| ())
    .map_err(|e| format!("failed to open console window: {e}"))
}

fn emit_log(app: &AppHandle, stream: &str, line: String) {
    let _ = app.emit(CONSOLE_LOG_EVENT, ConsoleLogPayload { stream, line });
}

/// Spawns background tasks that pipe a child process's stdout/stderr to the
/// console window line-by-line, and emits `CONSOLE_EXIT_EVENT` once it exits.
/// Never awaited by the caller — launch commands return as soon as the
/// process has started, matching the original's fire-and-forget console
/// attach (`console?.Attach(proc)`).
///
/// `registry` is `Some((registry, registry_key))` for Java launches only
/// (Bedrock doesn't go through this path with a registry key) — when
/// present, every line is *also* pushed into `gl_launch::ProcessRegistry`'s
/// ring buffer and re-emitted as `JAVA_LOG_LINE_EVENT`, and the process is
/// removed from the registry (with `JAVA_PROCESS_CHANGED_EVENT: exited`) once
/// it exits. This taps the existing read loop below; it does not open a
/// second pipe on the child.
fn stream_child_output(
    app: AppHandle,
    mut child: tokio::process::Child,
    registry: Option<(gl_launch::ProcessRegistry, String)>,
) {
    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        let registry = registry.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                emit_log(&app, "stdout", line.clone());
                if let Some((reg, key)) = &registry {
                    reg.push_log_line(key, &line);
                    emit_java_log_line(&app, key, line, "stdout");
                }
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        let registry = registry.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                emit_log(&app, "stderr", line.clone());
                if let Some((reg, key)) = &registry {
                    reg.push_log_line(key, &line);
                    emit_java_log_line(&app, key, line, "stderr");
                }
            }
        });
    }

    tokio::spawn(async move {
        let status = child.wait().await;
        let code = status.ok().and_then(|s| s.code());
        set_discord_idle(&app);
        let _ = app.emit(CONSOLE_EXIT_EVENT, ConsoleExitPayload { code });
        if let Some((reg, key)) = registry {
            reg.mark_exited(&key);
            let _ = app.emit(
                JAVA_PROCESS_CHANGED_EVENT,
                JavaProcessChangedPayload {
                    instance_id: &key,
                    status: "exited",
                    exit_code: code,
                },
            );
        }
    });
}

fn emit_java_log_line(app: &AppHandle, instance_id: &str, line: String, level: &str) {
    let _ = app.emit(
        JAVA_LOG_LINE_EVENT,
        JavaLogLinePayload {
            instance_id,
            line,
            level,
        },
    );
}

/// Resolves auth for a Java launch from `LauncherSettings`, mirroring
/// `JavaGameLauncher.ResolveAuthValuesAsync` — now WITH the proactive live
/// MSA token-refresh call NoRiskClient's `minecraft_auth.rs` does before
/// launch (a 12h buffer via `gl_auth::token_needs_refresh`), where earlier
/// this project only used whatever was cached in settings. If the cached
/// token is expired or inside that buffer and a `xbox_live_refresh_token` is
/// on file, attempts `gl_auth::refresh` and persists the refreshed values
/// back to settings on success. A refresh failure never makes things worse
/// than before this change: a still-valid (if soon-to-expire) cached token
/// is still used, and only a *hard*-expired token with no usable refresh
/// falls back to offline mode, exactly like the prior fallback-on-failure
/// path.
async fn resolve_java_auth(
    http: &reqwest::Client,
    settings: &mut gl_settings::LauncherSettings,
) -> ResolvedAuth {
    if settings.java_offline_mode {
        let name = if settings.java_offline_username.trim().is_empty() {
            "Player".to_string()
        } else {
            settings.java_offline_username.trim().to_string()
        };
        return ResolvedAuth::Offline { username: name };
    }

    if !settings.java_access_token.is_empty()
        && !settings.java_uuid.is_empty()
        && !settings.java_username.is_empty()
    {
        let expiry_unix = time_from_rfc3339(&settings.java_access_token_expiry)
            .map(|s| s as i64)
            .unwrap_or(0);
        let now = now_unix();
        let needs_refresh = gl_auth::token_needs_refresh(expiry_unix, now);

        if needs_refresh && !settings.xbox_live_refresh_token.is_empty() {
            match gl_auth::refresh(http, &settings.xbox_live_refresh_token).await {
                Ok(result) => {
                    settings.java_username = result.minecraft_profile.name.clone();
                    settings.java_uuid = result.minecraft_profile.uuid.clone();
                    settings.java_access_token = result.minecraft_access_token.clone();
                    settings.java_access_token_expiry =
                        unix_to_rfc3339(result.minecraft_access_token_expiry_unix);
                    if let Some(refresh_token) = result.live_refresh_token.clone() {
                        settings.xbox_live_refresh_token = refresh_token;
                    }
                    // Best-effort persist — a failed save here shouldn't block
                    // launch; the next launch just refreshes again.
                    let _ = gl_settings::save(settings);
                    return ResolvedAuth::Msa {
                        username: settings.java_username.clone(),
                        uuid: settings.java_uuid.clone(),
                        access_token: settings.java_access_token.clone(),
                    };
                }
                Err(_) => {
                    // Proactive refresh failed (offline, revoked, transient
                    // error). Fall through to the cached-token check below —
                    // same "swallow and keep going" behavior as before this
                    // change, just reached via a different branch.
                }
            }
        }

        if expiry_unix > now {
            return ResolvedAuth::Msa {
                username: settings.java_username.clone(),
                uuid: settings.java_uuid.clone(),
                access_token: settings.java_access_token.clone(),
            };
        }
    }

    let name = if settings.username.trim().is_empty() {
        "Player".to_string()
    } else {
        settings.username.trim().to_string()
    };
    ResolvedAuth::Offline { username: name }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Inverse of [`time_from_rfc3339`]: Unix-seconds → `YYYY-MM-DDTHH:MM:SSZ`,
/// via the standard `civil_from_days` algorithm (Howard Hinnant's), so a
/// refreshed token's expiry round-trips through settings in the same format
/// this launcher already writes (and `time_from_rfc3339` already parses).
fn unix_to_rfc3339(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let secs_of_day = secs.rem_euclid(86400);
    let (hour, min, sec) = (secs_of_day / 3600, (secs_of_day / 60) % 60, secs_of_day % 60);

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Minimal RFC 3339 UTC (`YYYY-MM-DDTHH:MM:SSZ`) → Unix-seconds parser,
/// sufficient for the `DateTimeOffset`-formatted timestamps this launcher
/// writes to settings itself.
fn time_from_rfc3339(s: &str) -> Result<u64, ()> {
    let s = s.trim();
    if s.len() < 19 {
        return Err(());
    }
    let year: i64 = s.get(0..4).and_then(|v| v.parse().ok()).ok_or(())?;
    let month: i64 = s.get(5..7).and_then(|v| v.parse().ok()).ok_or(())?;
    let day: i64 = s.get(8..10).and_then(|v| v.parse().ok()).ok_or(())?;
    let hour: i64 = s.get(11..13).and_then(|v| v.parse().ok()).ok_or(())?;
    let min: i64 = s.get(14..16).and_then(|v| v.parse().ok()).ok_or(())?;
    let sec: i64 = s.get(17..19).and_then(|v| v.parse().ok()).ok_or(())?;

    // Days-since-epoch via a standard civil-from-days algorithm (Howard
    // Hinnant's `days_from_civil`), avoiding a chrono dependency.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days_since_epoch = era * 146097 + doe - 719468;

    let secs = days_since_epoch * 86400 + hour * 3600 + min * 60 + sec;
    if secs < 0 {
        Err(())
    } else {
        Ok(secs as u64)
    }
}

// TODO: scheduled/auto-launch (e.g. "launch every day at 6pm") is not
// implemented — no task-scheduling infra exists in this crate/app to hook
// into; `launch_java_instance` below would be the natural entry point for it.

/// Launches a Java Edition instance end to end: resolves the instance's
/// directory + RAM config from `state.java_instances`, resolves auth from
/// settings, runs the full `gl_launch::java::launch` pipeline, and streams
/// the process to the console window. Returns the spawned PID.
#[tauri::command]
// `extra_jvm_args`: extra raw JVM args appended after the built-in ones,
// e.g. the Glacier Client's `-javaagent:<jarPath>`
// (`GlacierClientService.LaunchAsync`).
pub async fn launch_java_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: Option<String>,
    extra_jvm_args: Option<Vec<String>>,
) -> Result<u32, String> {
    let settings = gl_settings::load().map_err(|e| e.to_string())?;

    let (mc_dir, version_id, effective_config, instance_extra_jvm_args, glacier_runtime_root, registry_key) = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        let instances = svc.instances();
        let inst = match &instance_id {
            Some(id) => instances
                .iter()
                .find(|i| i.id.eq_ignore_ascii_case(id))
                .cloned()
                .ok_or_else(|| format!("Java instance not found: {id}"))?,
            None => instances
                .iter()
                .find(|i| i.id == settings.java_active_instance_id)
                .or_else(|| instances.first())
                .cloned()
                .ok_or_else(|| "No Java instance configured.".to_string())?,
        };
        let defaults = gl_java::GlobalJavaDefaults {
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
        let effective_config = gl_java::effective(&inst, &defaults);
        (
            PathBuf::from(inst.directory),
            inst.version_id,
            effective_config,
            inst.extra_jvm_args,
            svc.root_dir().join("runtimes"),
            inst.id,
        )
    };

    // Instance-level JVM-args preset/custom args (feature 3) come before the
    // caller-supplied `extra_jvm_args` (e.g. the Glacier Client's
    // `-javaagent:...`), which must stay last so it can't be shadowed.
    let mut combined_extra_jvm_args = instance_extra_jvm_args;
    combined_extra_jvm_args.extend(extra_jvm_args.unwrap_or_default());

    launch_java_with(
        app,
        state.java_processes.clone(),
        registry_key,
        state.http.clone(),
        settings,
        mc_dir,
        version_id,
        effective_config,
        glacier_runtime_root,
        combined_extra_jvm_args,
    )
    .await
}

/// Pre-launch, non-blocking health check (feature 1): Java-version match for
/// the instance's pinned version, configured heap vs currently available
/// RAM, and free disk space on the instance's drive. Returns warnings only —
/// never an error that would block Launch — mirroring
/// `gl_java::prelaunch::check`'s own contract. `id: None` checks whatever
/// instance `launch_java_instance` would resolve to (active instance).
#[tauri::command]
pub fn java_prelaunch_check(
    state: State<'_, AppState>,
    id: Option<String>,
) -> Result<Vec<gl_java::PrelaunchWarning>, String> {
    let settings = gl_settings::load().map_err(|e| e.to_string())?;

    let (instance, mc_dir, runtimes_root) = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        let instances = svc.instances();
        let inst = match &id {
            Some(id) => instances
                .iter()
                .find(|i| i.id.eq_ignore_ascii_case(id))
                .cloned()
                .ok_or_else(|| format!("Java instance not found: {id}"))?,
            None => instances
                .iter()
                .find(|i| i.id == settings.java_active_instance_id)
                .or_else(|| instances.first())
                .cloned()
                .ok_or_else(|| "No Java instance configured.".to_string())?,
        };
        let mc_dir = PathBuf::from(&inst.directory);
        let runtimes_root = svc.root_dir().join("runtimes");
        (inst, mc_dir, runtimes_root)
    };

    let configured_max_ram_mb = if instance.max_ram_mb > 0 {
        instance.max_ram_mb
    } else {
        settings.java_max_ram_mb
    };

    Ok(gl_java::prelaunch::check(
        &instance,
        &mc_dir,
        &settings.java_runtime_path,
        &runtimes_root,
        configured_max_ram_mb,
        available_ram_mb(),
        free_disk_bytes(&mc_dir),
    ))
}

/// Available physical RAM in MB, best-effort. Windows-only raw `kernel32`
/// call, matching the exact pattern already used in
/// `commands::utilities::win::memory` / `commands::versions::available_free_space`
/// (no new `sysinfo`/`windows` crate dependency for one call).
#[cfg(windows)]
fn available_ram_mb() -> Option<u64> {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct MEMORYSTATUSEX {
        dwLength: u32,
        dwMemoryLoad: u32,
        ullTotalPhys: u64,
        ullAvailPhys: u64,
        ullTotalPageFile: u64,
        ullAvailPageFile: u64,
        ullTotalVirtual: u64,
        ullAvailVirtual: u64,
        ullAvailExtendedVirtual: u64,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MEMORYSTATUSEX) -> i32;
    }
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        return None;
    }
    Some(status.ullAvailPhys / 1_048_576)
}

#[cfg(not(windows))]
fn available_ram_mb() -> Option<u64> {
    None
}

/// Free disk space (bytes) on `path`'s drive, best-effort. Same raw
/// `GetDiskFreeSpaceExW` pattern as `commands::versions::available_free_space`.
#[cfg(windows)]
fn free_disk_bytes(path: &std::path::Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            dir_name: *const u16,
            free_bytes_avail: *mut u64,
            total_bytes: *mut u64,
            total_free_bytes: *mut u64,
        ) -> i32;
    }
    let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let wide: Vec<u16> = absolute
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut free = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free) };
    if ok != 0 {
        Some(free)
    } else {
        None
    }
}

#[cfg(not(windows))]
fn free_disk_bytes(_path: &std::path::Path) -> Option<u64> {
    None
}

/// Launches a bare Java version id directly against the shared versions
/// pool (`<java root>/versions`, the same directory `install_java_version`
/// installs into and `list_java_versions` scans), with no instance's
/// mods/saves/config attached. Mirrors `JavaLauncherService.LaunchAsync(v.Id)`
/// — the original's version-scoped launch path, distinct from the
/// instance-scoped `launch_java_instance` this app otherwise uses
/// everywhere (see the Glacier Client launch command for the same
/// adaptation). Used by the flat Versions panel's "Launch" action, which
/// has no instance selector.
#[tauri::command]
pub async fn launch_java_version(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
) -> Result<u32, String> {
    let settings = gl_settings::load().map_err(|e| e.to_string())?;

    let (mc_dir, glacier_runtime_root) = {
        let svc = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        (svc.root_dir().to_path_buf(), svc.root_dir().join("runtimes"))
    };

    // No instance is involved for a bare version launch, so the "effective"
    // config is just the global defaults with nothing overridden.
    let defaults = gl_java::GlobalJavaDefaults {
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
    let effective_config = gl_java::effective(&gl_java::JavaInstance::default(), &defaults);
    let registry_key = version_registry_key(&version_id);

    launch_java_with(
        app,
        state.java_processes.clone(),
        registry_key,
        state.http.clone(),
        settings,
        mc_dir,
        version_id,
        effective_config,
        glacier_runtime_root,
        Vec::new(),
    )
    .await
}

/// Shared tail of both Java launch commands, once `mc_dir`/`version_id` are
/// resolved: builds the launch request, opens the console window, runs
/// `gl_launch::java::launch`, and wires up Discord presence / suspend-guard
/// / output streaming exactly the same way for either entry point.
#[allow(clippy::too_many_arguments)]
async fn launch_java_with(
    app: AppHandle,
    processes: gl_launch::ProcessRegistry,
    registry_key: String,
    http: reqwest::Client,
    mut settings: gl_settings::LauncherSettings,
    mc_dir: PathBuf,
    version_id: String,
    effective_config: gl_java::EffectiveLaunchConfig,
    glacier_runtime_root: PathBuf,
    extra_jvm_args: Vec<String>,
) -> Result<u32, String> {
    let bundled_runtime_root = mc_dir.join("runtime");
    let explicit_java_path = effective_config
        .java_path
        .clone()
        .map(PathBuf::from);
    let extra_game_args = effective_config.extra_game_args.clone();
    let auth = resolve_java_auth(&http, &mut settings).await;
    let options = effective_config.into_launch_options(extra_jvm_args, settings.xbox_xuid.clone());

    let discord_version_tag = version_id.clone();
    let req = JavaLaunchRequest {
        version_id,
        mc_dir,
        explicit_java_path,
        bundled_runtime_root,
        glacier_runtime_root,
        options,
        auth,
        extra_game_args,
    };

    if settings.show_launch_console {
        open_console_window(&app, "Minecraft Java")?;
    }
    emit_status(&app, "starting", None, None);
    emit_log(
        &app,
        "info",
        format!(".minecraft = {}", req.mc_dir.display()),
    );

    let launched = match gl_java_launch::launch(req).await {
        Ok(l) => l,
        Err(e) => {
            let msg = e.to_string();
            emit_status(&app, "failed", None, Some(msg.clone()));
            return Err(msg);
        }
    };

    emit_log(&app, "info", format!("Started PID {}", launched.pid));
    emit_log(&app, "info", launched.command_line.clone());

    let pid = launched.pid;
    if let Ok(mut guard) = LAST_JAVA_PID.lock() {
        *guard = Some(pid);
    }
    set_discord_playing(&app, &discord_version_tag, None);
    emit_status(&app, "running", Some(pid), None);
    if settings.prevent_game_suspend {
        spawn_suspend_guard(pid);
    }

    processes.register(&registry_key, pid);
    let _ = app.emit(
        JAVA_PROCESS_CHANGED_EVENT,
        JavaProcessChangedPayload {
            instance_id: &registry_key,
            status: "started",
            exit_code: None,
        },
    );
    stream_child_output(app, launched.child, Some((processes, registry_key)));

    Ok(pid)
}

/// `PreventGameSuspend`: holds `ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED` for
/// as long as `pid` is alive, then releases it. Runs on a blocking thread —
/// the wait is a plain `WaitForSingleObject`, not awaitable.
fn spawn_suspend_guard(pid: u32) {
    tauri::async_runtime::spawn_blocking(move || {
        gl_injection::set_prevent_suspend(true);
        gl_injection::wait_for_process_exit(pid);
        gl_injection::set_prevent_suspend(false);
    });
}

/// Launches Bedrock connected directly to a server via the
/// `minecraft://?addExternalServer=` deep link. Mirrors `LaunchServerAsync`:
/// runs the normal launch first (so the client DLL, if any, is already
/// injected), then fires the deep link as a follow-up. Best-effort — a
/// deep-link failure doesn't fail the launch itself, matching the original's
/// swallow-and-continue behaviour.
#[tauri::command]
pub async fn launch_bedrock_server(
    app: AppHandle,
    dll_path: Option<String>,
    server_name: String,
    host: String,
    port: u16,
) -> Result<u32, String> {
    let pid = launch_bedrock_instance(app.clone(), dll_path).await?;

    if let Err(e) = bedrock_launch::connect_external_server(&server_name, &host, port).await {
        emit_log(
            &app,
            "warn",
            format!("Launch & connect: deep link failed: {e}"),
        );
    }

    Ok(pid)
}

/// Launches Bedrock (vanilla, or with an optional client DLL injected after
/// `Settings.InjectionDelayMs`). Returns the game process PID.
#[tauri::command]
pub async fn launch_bedrock_instance(
    app: AppHandle,
    dll_path: Option<String>,
) -> Result<u32, String> {
    let settings = gl_settings::load().map_err(|e| e.to_string())?;

    // Feature 20: auto-backup-every-N-launches, checked right before launch.
    // Best-effort — a failed backup must never block the game from starting.
    match crate::commands::bedrock::maybe_run_bedrock_auto_backup() {
        Ok(Some(path)) => emit_log(&app, "info", format!("Auto-backup created: {path}")),
        Ok(None) => {}
        Err(e) => emit_log(&app, "warn", format!("Auto-backup skipped: {e}")),
    }

    if settings.show_launch_console {
        open_console_window(
            &app,
            &format!(
                "Minecraft Bedrock · {}",
                dll_path
                    .as_deref()
                    .map(|p| {
                        std::path::Path::new(p)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| p.to_string())
                    })
                    .unwrap_or_else(|| "Vanilla".to_string())
            ),
        )?;
    }
    emit_status(&app, "starting", None, None);

    // `AccelerateLoadingMs`: stop RuntimeBroker after this delay to shorten
    // Bedrock's UWP loading-screen throttle. Fire-and-forget — best-effort,
    // never blocks or fails the launch.
    if settings.accelerate_loading_ms > 0 {
        let delay = settings.accelerate_loading_ms.max(0) as u64;
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            gl_injection::kill_processes_by_name("RuntimeBroker.exe");
        });
    }

    let result = match bedrock_launch::launch(
        dll_path.as_ref().map(PathBuf::from),
        settings.injection_delay_ms.max(0) as u64,
        settings.verify_injection,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            emit_status(&app, "failed", None, Some(msg.clone()));
            return Err(msg);
        }
    };

    emit_log(
        &app,
        "info",
        format!(
            "Bedrock package: {}!{}{}",
            result.package.family_name,
            result.package.application_id,
            if result.package.is_preview {
                " (Preview)"
            } else {
                ""
            }
        ),
    );
    emit_log(
        &app,
        "info",
        if result.was_already_running {
            format!("Minecraft already running (PID {}). Reusing.", result.pid)
        } else {
            format!("Minecraft started (PID {}).", result.pid)
        },
    );
    emit_status(&app, "running", Some(result.pid), None);
    if settings.prevent_game_suspend {
        spawn_suspend_guard(result.pid);
    }

    let client_name = dll_path.as_deref().map(|p| {
        std::path::Path::new(p)
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.to_string())
    });
    set_discord_playing(&app, "Bedrock", client_name.as_deref());
    spawn_discord_idle_on_exit(app.clone(), result.pid);
    if settings.show_launch_console {
        spawn_console_exit_on_pid(app.clone(), result.pid);
    }

    Ok(result.pid)
}

/// Waits (off the async runtime, since this is a blocking `WaitForSingleObject`)
/// for `pid` to exit, then resets Discord Rich Presence to idle. Separate from
/// `spawn_suspend_guard`, which only runs when `PreventGameSuspend` is on —
/// this always runs so the presence doesn't stay stuck on "Playing" after the
/// game closes.
fn spawn_discord_idle_on_exit(app: AppHandle, pid: u32) {
    tauri::async_runtime::spawn_blocking(move || {
        gl_injection::wait_for_process_exit(pid);
        set_discord_idle(&app);
    });
}

/// Bedrock has no `tokio::process::Child` to `.wait()` on — it's a UWP app
/// tracked only by PID — so unlike the Java path (`stream_child_output`),
/// nothing emits `CONSOLE_EXIT_EVENT` once it closes and the console window
/// is left open forever. Waits for `pid` to exit (same primitive
/// `spawn_discord_idle_on_exit` uses) and emits the same exit event so
/// `GameConsoleApp.svelte`'s auto-close behaves identically for both editions.
fn spawn_console_exit_on_pid(app: AppHandle, pid: u32) {
    tauri::async_runtime::spawn_blocking(move || {
        gl_injection::wait_for_process_exit(pid);
        let _ = app.emit(CONSOLE_EXIT_EVENT, ConsoleExitPayload { code: None });
    });
}

#[tauri::command]
pub fn is_bedrock_game_running() -> bool {
    bedrock_launch::find_game_process().is_some()
}

/// Java's `tokio::process::Child` isn't shared out to the frontend, so this
/// checks `LAST_JAVA_PID` (the PID of the most recently launched Java
/// process) against the actual process table, rather than name-matching
/// `java.exe`/`javaw.exe` — that would false-positive on any other running
/// Java application. Bedrock's `is_bedrock_game_running` gets away with
/// name-matching because it's UWP-scoped by package family.
#[tauri::command]
pub fn is_java_game_running() -> bool {
    let Ok(guard) = LAST_JAVA_PID.lock() else {
        return false;
    };
    match *guard {
        Some(pid) => gl_injection::process_is_alive(pid),
        None => false,
    }
}

/// Port of `GameConsoleWindow.SaveBtn_Click`: writes the console's full text
/// to a path the user already chose via a save dialog.
#[tauri::command]
pub fn save_console_log(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

/// Port of `GameConsoleWindow.ShareBtn_Click`: uploads the console's full
/// text to mclo.gs and returns the share URL.
#[tauri::command]
pub async fn upload_console_log_to_mclogs(content: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.mclo.gs/1/log")
        .form(&[("content", content.as_str())])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("mclo.gs upload failed (HTTP {status}): {body}"));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    parsed["url"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "mclo.gs response had no url".to_string())
}

/// Port of `GameConsoleWindow.StopBtn_Click`: kills the process the console
/// window is currently attached to. An alias of [`cancel_launch`] under the
/// console's own name — same underlying kill, different caller.
#[tauri::command]
pub fn console_stop_game() {
    cancel_launch();
}

#[cfg(test)]
mod token_refresh_tests {
    use super::*;

    #[test]
    fn unix_to_rfc3339_round_trips_through_time_from_rfc3339() {
        // A handful of real timestamps spanning a leap day and a century
        // boundary, since the civil-date algorithms are easy to get subtly
        // wrong at those edges.
        let samples: &[i64] = &[
            0,             // 1970-01-01T00:00:00Z
            1_700_000_000, // 2023-11-14T22:13:20Z
            951_782_400,   // 2000-02-29T00:00:00Z (leap day)
            1_893_456_000, // 2030-01-01T00:00:00Z
        ];
        for &secs in samples {
            let formatted = unix_to_rfc3339(secs);
            let parsed = time_from_rfc3339(&formatted).expect("must parse what we just formatted");
            assert_eq!(parsed as i64, secs, "round-trip failed for {formatted}");
        }
    }

    #[test]
    fn unix_to_rfc3339_matches_expected_format() {
        assert_eq!(unix_to_rfc3339(0), "1970-01-01T00:00:00Z");
    }
}
