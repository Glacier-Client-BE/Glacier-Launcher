//! Thin #[tauri::command] wrappers for miscellaneous "System" settings-tab
//! actions ported from Home.razor's Settings panel (Home.Settings.cs):
//! opening folders in Explorer, and export/import of the settings JSON.
//! The frontend now picks the path via tauri-plugin-dialog's native
//! save/open dialogs and passes it through; when no path is given these
//! fall back to the fixed `exports/` folder under the launcher root.

use gl_settings::LauncherSettings;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn get_launcher_root() -> String {
    gl_settings::launcher_root().display().to_string()
}

/// Closes the main launcher window after a successful game launch, when
/// `CloseAfterLaunch` is enabled. Uses `close()` (not `hide()`) — this is an
/// explicit "close after launch" preference, distinct from `MinimizeToTray`,
/// which governs the window's own close (X) button instead.
#[tauri::command]
pub fn close_launcher_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }
}

/// QOL: the actual "yes, quit anyway" side of the close-with-active-downloads
/// confirm — the `CloseRequested` handler in lib.rs prevents the close and
/// asks the frontend first instead of silently killing in-flight downloads;
/// this is what runs once the user confirms.
#[tauri::command]
pub fn force_quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command(async)]
pub fn open_folder(path: String) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Opens an existing file with its default handler - port of the original's
/// `OpenUrl(path)` used by `OpenScreenshot`.
///
/// Deliberately distinct from [`open_folder`], which calls `create_dir_all`:
/// that is right for a folder the user asked to browse, but creating a path
/// that was supposed to be an existing file would silently produce an empty
/// directory instead of reporting that the file is gone.
#[tauri::command(async)]
pub fn open_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(format!("'{path}' no longer exists."));
    }
    // `explorer <file>` launches the file with its default handler, so an
    // executable or script path would run code. Only documents and images
    // are opened here (screenshots, logs, exports).
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    const EXECUTABLE_EXTS: &[&str] = &[
        "exe", "com", "bat", "cmd", "ps1", "msi", "msix", "scr", "vbs", "vbe", "js", "jse",
        "wsf", "wsh", "lnk", "reg", "dll", "jar", "appx",
    ];
    if EXECUTABLE_EXTS.contains(&ext.as_str()) {
        return Err(format!("Refusing to open executable file '{path}'."));
    }
    std::process::Command::new("explorer")
        .arg(&target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Opens an external web link in the user's default browser - port of
/// `OpenUrl` (`Home.razor.cs:670`), which was
/// `Process.Start(new ProcessStartInfo { FileName = url, UseShellExecute = true })`.
///
/// The Credits, News, Servers and Settings panels all need this. `window.open`
/// does not work here: inside the WebView2 host there is no browser chrome to
/// open a new tab in, so those links were silently dead.
///
/// Unlike the C# original this refuses anything that isn't `http`/`https`.
/// `UseShellExecute = true` on an arbitrary string will happily launch an
/// executable or a `file:`/custom-scheme handler; every call site passes a
/// hard-coded web URL, so restricting the scheme costs nothing and keeps the
/// command from becoming a general "run this" primitive reachable from the
/// frontend.
#[tauri::command(async)]
pub fn open_url(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    // Matches the original's `if (!string.IsNullOrEmpty(url))` guard: an empty
    // URL is a no-op, not an error.
    if trimmed.is_empty() {
        return Ok(());
    }
    let lowered = trimmed.to_ascii_lowercase();
    if !(lowered.starts_with("http://") || lowered.starts_with("https://")) {
        return Err(format!(
            "Refusing to open '{trimmed}': only http and https links are allowed."
        ));
    }

    // `rundll32 url.dll,FileProtocolHandler <url>` is a robust way to open URLs
    // on Windows, behaving like `Process.Start` with `UseShellExecute = true`.
    // It properly handles percent-encoded characters and ampersands without
    // the escaping issues of `cmd /c start` or the File Explorer bug of `explorer.exe`.
    let status = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", trimmed])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(format!("start command exited with {status}")) }
}

#[cfg(test)]
mod tests {
    use super::open_url;

    #[test]
    fn empty_url_is_a_no_op_like_the_original() {
        assert!(open_url(String::new()).is_ok());
        assert!(open_url("   ".into()).is_ok());
    }

    #[test]
    fn open_path_refuses_executables_before_spawning() {
        let dir = std::env::temp_dir().join(format!("gl-open-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("payload.EXE");
        std::fs::write(&exe, b"not really").unwrap();
        let err = super::open_path(exe.display().to_string()).unwrap_err();
        assert!(err.contains("Refusing to open executable"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_non_web_schemes() {
        for bad in [
            "file:///C:/Windows/System32/cmd.exe",
            r"C:\Windows\System32\cmd.exe",
            "javascript:alert(1)",
            "ms-settings:",
        ] {
            let err = open_url(bad.into()).unwrap_err();
            assert!(err.contains("only http and https"), "{bad} -> {err}");
        }
    }
}

fn exports_dir() -> PathBuf {
    gl_settings::launcher_root().join("exports")
}

/// (Re)registers the global quick-launch hotkey, unregistering whatever was
/// registered before (there is only ever at most one - a second call always
/// replaces the first rather than stacking). An empty `combo` just clears
/// the existing binding, leaving the hotkey disabled. Also persists the
/// choice to settings so it's restored at the next startup.
#[tauri::command]
pub fn set_quick_launch_hotkey(app: AppHandle, combo: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let manager = app.global_shortcut();
    manager.unregister_all().map_err(|e| e.to_string())?;
    if !combo.is_empty() {
        manager
            .register(combo.as_str())
            .map_err(|e| format!("couldn't register '{combo}': {e}"))?;
    }

    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    settings.quick_launch_hotkey = combo;
    gl_settings::save(&settings).map_err(|e| e.to_string())
}

/// Copies one file to a destination path (creating parent directories as
/// needed) - QOL bulk-export flows (screenshots, etc.) that need "put a
/// copy of this file where the user picked" without a dedicated command per
/// export type. Overwrites an existing file at `dest`.
#[tauri::command(async)]
pub fn copy_file(src: String, dest: String) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(&dest).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
    Ok(())
}

/// Writes arbitrary text to a path the frontend already picked (e.g. via
/// the native save dialog) - QOL exports (mod lists, etc.) that just need
/// "dump this string to a file the user chose" without a dedicated command
/// per export type.
#[tauri::command(async)]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, contents).map_err(|e| e.to_string())
}

fn write_settings_export(path: Option<String>) -> Result<(PathBuf, bool), String> {
    let settings = gl_settings::load().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;

    let explicit_path = path.filter(|p| !p.is_empty());
    let file = if let Some(p) = &explicit_path {
        PathBuf::from(p)
    } else {
        let dir = exports_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        dir.join(format!("glacier-settings-{stamp}.json"))
    };

    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&file, json).map_err(|e| e.to_string())?;
    Ok((file, explicit_path.is_none()))
}

#[tauri::command(async)]
pub fn export_settings(path: Option<String>) -> Result<String, String> {
    let (file, was_auto_path) = write_settings_export(path)?;
    if was_auto_path {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&file)
            .spawn()
            .ok();
    }
    Ok(file.display().to_string())
}

/// QOL: periodic rolling settings backup, separate from the user-triggered
/// "Export settings" button. Called by the frontend on an interval; a no-op
/// (does not touch disk or the timestamp) unless more than 24h have passed
/// since `last_settings_auto_export_unix_ms`, and never opens Explorer the way the
/// manual export does - popping a file-explorer window on an unattended
/// background timer would be jarring. Also prunes the `exports/` folder to
/// the 10 most recent auto-exports so this can't grow unbounded over weeks
/// of use; user-named exports (an explicit path) are never touched by this.
#[tauri::command(async)]
pub fn auto_export_settings_if_due() -> Result<(), String> {
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let due = (now - settings.last_settings_auto_export_unix_ms) > 24 * 60 * 60 * 1000;
    if !due {
        return Ok(());
    }

    write_settings_export(None)?;
    settings.last_settings_auto_export_unix_ms = now;
    gl_settings::save(&settings).map_err(|e| e.to_string())?;

    // Prune to the 10 newest exports so this can't grow unbounded over weeks
    // of use; user-named exports (an explicit path) are never touched here.
    let dir = exports_dir();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| (t, e.path())))
            .collect();
        files.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, path) in files.into_iter().skip(10) {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

/// Imports settings from a JSON file. When `path` is given (from the native
/// open dialog) it's read directly; otherwise falls back to the most
/// recently exported file in the fixed `exports/` folder.
#[tauri::command(async)]
pub fn import_latest_settings(path: Option<String>) -> Result<LauncherSettings, String> {
    if let Some(p) = path.filter(|p| !p.is_empty()) {
        let text = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
        return serde_json::from_str(&text).map_err(|e| e.to_string());
    }

    let dir = exports_dir();
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = std::fs::read_dir(&dir).map_err(|_| "No exported settings found.".to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if newest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                    newest = Some((modified, path));
                }
            }
        }
    }
    let (_, path) = newest.ok_or_else(|| "No exported settings found.".to_string())?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imported: LauncherSettings = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    gl_settings::save(&imported).map_err(|e| e.to_string())?;
    Ok(imported)
}

#[tauri::command(async)]
pub fn reset_settings() -> Result<LauncherSettings, String> {
    let current = gl_settings::load().map_err(|e| e.to_string())?;
    // Preserve connected-account state, same as the WPF DoResetSettings().
    let fresh = LauncherSettings {
        discord_logged_in: current.discord_logged_in,
        discord_username: current.discord_username,
        discord_avatar: current.discord_avatar,
        discord_token: current.discord_token,
        discord_user_id: current.discord_user_id,
        xbox_gamertag: current.xbox_gamertag,
        xbox_xuid: current.xbox_xuid,
        xbox_gamer_picture_url: current.xbox_gamer_picture_url,
        xbox_gamerscore: current.xbox_gamerscore,
        xbox_account_tier: current.xbox_account_tier,
        xbox_bio: current.xbox_bio,
        ..Default::default()
    };
    gl_settings::save(&fresh).map_err(|e| e.to_string())?;
    Ok(fresh)
}

/// Recursively removes every file/directory under `dir`, best-effort: one
/// locked or permission-denied entry (a log file still open, an antivirus
/// scan, a running injected client's DLL) does not stop the rest from being
/// deleted, unlike `std::fs::remove_dir_all` which aborts the whole walk on
/// the first error. Returns the paths that could not be removed so the
/// caller can report a partial failure instead of a silent one.
fn force_delete_dir_contents(dir: &std::path::Path) -> Vec<String> {
    let mut failures = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return failures;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry
            .file_type()
            .map(|t| t.is_dir())
            .unwrap_or_else(|_| path.is_dir());
        if is_dir {
            failures.extend(force_delete_dir_contents(&path));
            if std::fs::remove_dir(&path).is_err() {
                // Non-empty because some child failed above, or a fresh
                // permission error - either way, note it and move on.
                if path.read_dir().map(|mut d| d.next().is_some()).unwrap_or(true) {
                    failures.push(path.display().to_string());
                }
            }
        } else if let Err(e) = std::fs::remove_file(&path) {
            log::warn!("delete_all_data: couldn't remove {}: {e}", path.display());
            failures.push(path.display().to_string());
        }
    }
    failures
}

/// QOL "copy panel link": a `glacier://...` URL the process was cold-started
/// with, if any, set once in `lib.rs`'s `setup` before any frontend
/// listener could exist to catch an emitted event instead. `take_pending_deep_link`
/// (called once by the frontend at its own startup) is the only consumer.
pub static PENDING_DEEP_LINK: std::sync::OnceLock<std::sync::Mutex<Option<String>>> =
    std::sync::OnceLock::new();

#[tauri::command]
pub fn take_pending_deep_link() -> Option<String> {
    PENDING_DEEP_LINK.get()?.lock().ok()?.take()
}

fn profiles_dir() -> PathBuf {
    gl_settings::launcher_root().join("profiles")
}

/// QOL "config profiles": named settings snapshots the user can save and
/// switch between (e.g. "streaming setup" vs "solo play"), distinct from
/// the single rolling `exports/` folder the manual/auto export use. A thin
/// layer over the same settings JSON shape - `load_config_profile` returns
/// a `LauncherSettings` the frontend applies via the normal `set_settings`
/// path, exactly like importing an export.
#[derive(serde::Serialize)]
pub struct ConfigProfile {
    pub name: String,
    pub path: String,
    pub modified_at: String,
}

#[tauri::command(async)]
pub fn list_config_profiles() -> Vec<ConfigProfile> {
    let dir = profiles_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<ConfigProfile> = entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let modified_at = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default();
            Some(ConfigProfile {
                name,
                path: path.display().to_string(),
                modified_at,
            })
        })
        .collect();
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    out
}

/// Saves the current settings as a named profile. Overwrites a profile with
/// the same name - saving under an existing name is how you'd update it.
#[tauri::command(async)]
pub fn save_config_profile(name: String) -> Result<String, String> {
    let dir = profiles_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Sanitize to a safe filename - profile names come from free-text input.
    let safe_name: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let safe_name = safe_name.trim();
    if safe_name.is_empty() {
        return Err("Profile name can't be empty.".to_string());
    }
    let path = dir.join(format!("{safe_name}.json"));
    let settings = gl_settings::load().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command(async)]
pub fn load_config_profile(path: String) -> Result<LauncherSettings, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn delete_config_profile(path: String) -> Result<(), String> {
    // Only profiles written by `save_config_profile` may be deleted; the
    // frontend passes an arbitrary string here.
    let target = super::ensure_direct_child(&profiles_dir(), std::path::Path::new(&path))?;
    std::fs::remove_file(&target).map_err(|e| e.to_string())
}

/// Deletes everything under the launcher's data root (`%USERPROFILE%\Glacier
/// Launcher` - settings, cache, downloads, instances, backups, skins,
/// themes, notifications, everything `gl_settings::launcher_root()` and its
/// subsystems write into), then relaunches the launcher into a clean state.
///
/// This is the nuclear "Delete all data" option: unlike `reset_settings`, it
/// does not preserve connected-account state, and unlike a normal uninstall
/// it does not touch the installed application files - only this user's
/// data. It always attempts the delete regardless of partial failures (see
/// `force_delete_dir_contents`); it only returns `Err` when it could not
/// remove *anything* on a locked/denied root, so the caller can tell "wiped
/// clean" from "the folder is entirely inaccessible" apart from "wiped
/// almost everything, a few files were in use."
#[tauri::command(async)]
pub fn delete_all_data(app: AppHandle) -> Result<(), String> {
    let root = gl_settings::launcher_root();
    if !root.exists() {
        app.restart();
    }

    let failures = force_delete_dir_contents(&root);
    let _ = std::fs::remove_dir(&root);

    if root.exists() && root.read_dir().map(|mut d| d.next().is_none()).unwrap_or(false) {
        // Directory itself survives (Windows sometimes needs a beat to
        // release a just-emptied directory handle) but is empty - fine.
    }

    if !failures.is_empty() && root.exists() {
        return Err(format!(
            "Deleted most data, but {} item(s) were locked and skipped: {}",
            failures.len(),
            failures.join(", ")
        ));
    }

    // Nothing left to run on - relaunch fresh rather than continuing with
    // an AppState built from data that no longer exists on disk.
    app.restart();
}
