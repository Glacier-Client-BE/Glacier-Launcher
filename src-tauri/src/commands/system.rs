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

#[tauri::command]
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
#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(format!("'{path}' no longer exists."));
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
#[tauri::command]
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

#[tauri::command]
pub fn export_settings(path: Option<String>) -> Result<String, String> {
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
    if explicit_path.is_none() {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&file)
            .spawn()
            .ok();
    }
    Ok(file.display().to_string())
}

/// Imports settings from a JSON file. When `path` is given (from the native
/// open dialog) it's read directly; otherwise falls back to the most
/// recently exported file in the fixed `exports/` folder.
#[tauri::command]
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

#[tauri::command]
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
