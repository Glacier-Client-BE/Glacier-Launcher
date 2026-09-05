//! Thin #[tauri::command] wrappers around `gl_launch`'s Java/Bedrock launch
//! pipelines. Business logic (arg construction, package resolution,
//! activation, injection) lives in `gl_launch`/`gl_java`/`gl_injection`;
//! this module resolves instance config from `AppState` + settings, streams
//! the launched process's output to a second `WebviewWindow`, and maps
//! errors to `String` for the frontend.
//!
//! NOT YET WIRED UP — see the final report for the exact `Cargo.toml`,
//! `commands/mod.rs`, and `lib.rs` edits required before this file is part
//! of the build (it depends on the `gl-launch` crate, which is not yet a
//! `src-tauri` dependency, per this task's constraint against editing
//! `src-tauri/Cargo.toml` directly).

use gl_launch::java::{JavaLaunchRequest, ResolvedAuth};
use gl_launch::{bedrock as gl_bedrock_launch, java as gl_java_launch};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::state::AppState;

const CONSOLE_WINDOW_LABEL: &str = "game-console";
const CONSOLE_LOG_EVENT: &str = "game-console://log";
const CONSOLE_EXIT_EVENT: &str = "game-console://exit";
const CONSOLE_STATUS_EVENT: &str = "game-console://status";
const CONSOLE_TITLE_EVENT: &str = "game-console://title";

/// PID of the most recently launched Java process, if it may still be
/// running. Not stored on `AppState` (kept local to this module): mirrors
/// the original's `_launchCts`/force-close scope, which only ever needs to
/// reach "the thing we just tried to launch," not general app state.
static LAST_JAVA_PID: Mutex<Option<u32>> = Mutex::new(None);

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
    if let Some(pid) = gl_bedrock_launch::find_game_process() {
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
fn stream_child_output(app: AppHandle, mut child: tokio::process::Child) {
    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                emit_log(&app, "stdout", line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                emit_log(&app, "stderr", line);
            }
        });
    }

    tokio::spawn(async move {
        let status = child.wait().await;
        let code = status.ok().and_then(|s| s.code());
        let _ = app.emit(CONSOLE_EXIT_EVENT, ConsoleExitPayload { code });
    });
}

/// Resolves auth for a Java launch from `LauncherSettings`, mirroring
/// `JavaGameLauncher.ResolveAuthValuesAsync` MINUS the live MSA
/// token-refresh call (see `gl_launch::java` module docs — this project's
/// `gl-auth`/Xbox refresh flow isn't wired into the launch path yet). Uses
/// the cached access token if present and unexpired, else falls back to
/// offline mode exactly like the C# fallback-on-refresh-failure path.
fn resolve_java_auth(settings: &gl_settings::LauncherSettings) -> ResolvedAuth {
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
        let expired = chrono_expired(&settings.java_access_token_expiry);
        if !expired {
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

/// RFC 3339 expiry check without pulling in a `chrono` dependency: compares
/// against the current time via `std::time::SystemTime` + a minimal parse.
/// Treats an unparsable/empty expiry as expired (fail closed → falls back to
/// offline rather than sending a stale/garbage token to the JVM).
fn chrono_expired(expiry: &str) -> bool {
    let Ok(parsed) = time_from_rfc3339(expiry) else {
        return true;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    parsed <= now
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

/// Launches a Java Edition instance end to end: resolves the instance's
/// directory + RAM config from `state.java_instances`, resolves auth from
/// settings, runs the full `gl_launch::java::launch` pipeline, and streams
/// the process to the console window. Returns the spawned PID.
#[tauri::command]
pub async fn launch_java_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: Option<String>,
) -> Result<u32, String> {
    let settings = gl_settings::load().map_err(|e| e.to_string())?;

    let (mc_dir, version_id, max_ram_mb, min_ram_mb, glacier_runtime_root) = {
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
        (
            PathBuf::from(inst.directory),
            inst.version_id,
            inst.max_ram_mb,
            inst.min_ram_mb,
            svc.root_dir().join("runtimes"),
        )
    };

    let bundled_runtime_root = mc_dir.join("runtime");
    let explicit_java_path = if settings.java_runtime_path.is_empty() {
        None
    } else {
        Some(PathBuf::from(&settings.java_runtime_path))
    };

    let options = gl_java_launch::LaunchOptions {
        max_ram_mb: if max_ram_mb > 0 {
            max_ram_mb
        } else {
            settings.java_max_ram_mb
        },
        min_ram_mb: if min_ram_mb > 0 {
            min_ram_mb
        } else {
            settings.java_min_ram_mb
        },
        custom_jvm_args: settings.java_custom_jvm_args.clone(),
        extra_jvm_args: Vec::new(),
        fullscreen: settings.java_fullscreen,
        use_custom_resolution: settings.java_use_custom_resolution,
        window_width: settings.java_window_width,
        window_height: settings.java_window_height,
        server_address: settings.java_server_address.clone(),
        server_port: settings.java_server_port,
        auth_xuid: settings.xbox_xuid.clone(),
    };

    let req = JavaLaunchRequest {
        version_id,
        mc_dir,
        explicit_java_path,
        bundled_runtime_root,
        glacier_runtime_root,
        options,
        auth: resolve_java_auth(&settings),
    };

    open_console_window(&app, "Minecraft Java")?;
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
    emit_status(&app, "running", Some(pid), None);
    stream_child_output(app, launched.child);

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
    emit_status(&app, "starting", None, None);

    let result = match gl_bedrock_launch::launch(
        dll_path.map(PathBuf::from),
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

    Ok(result.pid)
}

#[tauri::command]
pub fn is_bedrock_game_running() -> bool {
    gl_bedrock_launch::find_game_process().is_some()
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
