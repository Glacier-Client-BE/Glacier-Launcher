use gl_integrations::models::{Announcement, NewsPost};
use serde::Serialize;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{timeout, Duration};

use crate::state::AppState;

const DISCORD_CLIENT_ID: &str = "1482726422094024779";
const DISCORD_CLIENT_SECRET: &str = "zwvBIpvo18qHSLUzlvG1AKfJKAkMLujc";
const DISCORD_REDIRECT_URI: &str = "http://localhost:5000/callback";
const DISCORD_AUTHORIZE_URL: &str = "https://discord.com/oauth2/authorize?client_id=1482726422094024779&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A5000%2Fcallback&scope=identify";

/// Minimal profile returned to the frontend after a successful login, so it
/// can toast/update without re-reading settings. Port of the display fields
/// `OpenDiscordOAuth` (`Home.Panels.cs:852`) pulled from `/users/@me`.
#[derive(Debug, Clone, Serialize)]
pub struct DiscordProfile {
    pub username: String,
    pub avatar: String,
}

/// Ports `OpenDiscordOAuth` (`Home.Panels.cs:852-920`): opens the system
/// browser to Discord's authorize URL, waits on a one-shot local HTTP
/// listener for the `code` redirect, exchanges it for an access token, then
/// fetches `/users/@me` and persists the result to settings.
///
/// Unlike the C# original (which used `HttpListener` and swallowed all
/// errors in an empty `catch {}`), this returns a descriptive `Err` on
/// failure and bounds the wait for the browser redirect with a 2-minute
/// timeout so the command can't hang forever if the user closes the tab.
#[tauri::command]
pub async fn login_discord(state: State<'_, AppState>) -> Result<DiscordProfile, String> {
    // Bind before opening the browser, so the redirect always has a listener
    // waiting for it.
    let listener = TcpListener::bind("127.0.0.1:5000")
        .await
        .map_err(|e| format!("failed to start local OAuth listener on port 5000: {e}"))?;

    crate::commands::system::open_url(DISCORD_AUTHORIZE_URL.to_string())
        .map_err(|e| format!("failed to open Discord login page: {e}"))?;

    let code = timeout(Duration::from_secs(120), accept_oauth_code(&listener))
        .await
        .map_err(|_| "Discord login timed out waiting for the browser redirect.".to_string())??;

    if code.is_empty() {
        return Err("Discord did not return an authorization code.".to_string());
    }

    let token_response = state
        .http
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", DISCORD_CLIENT_ID),
            ("client_secret", DISCORD_CLIENT_SECRET),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", DISCORD_REDIRECT_URI),
        ])
        .send()
        .await
        .map_err(|e| format!("Discord token exchange failed: {e}"))?;

    let token_json: serde_json::Value = token_response
        .json()
        .await
        .map_err(|e| format!("Discord token response was not valid JSON: {e}"))?;
    let access_token = token_json
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if access_token.is_empty() {
        return Err("Discord did not return an access token.".to_string());
    }

    let user_response = state
        .http
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("Discord profile fetch failed: {e}"))?;

    let user_json: serde_json::Value = user_response
        .json()
        .await
        .map_err(|e| format!("Discord profile response was not valid JSON: {e}"))?;
    let user_id = user_json
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let username = user_json
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let avatar_hash = user_json
        .get("avatar")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let avatar_url = if avatar_hash.is_empty() {
        String::new()
    } else {
        format!("https://cdn.discordapp.com/avatars/{user_id}/{avatar_hash}.png")
    };

    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    settings.discord_logged_in = true;
    settings.discord_user_id = user_id;
    settings.discord_username = username.clone();
    settings.discord_avatar = avatar_url.clone();
    settings.discord_token = access_token;
    gl_settings::save(&settings).map_err(|e| e.to_string())?;

    Ok(DiscordProfile {
        username,
        avatar: avatar_url,
    })
}

/// Accepts exactly one connection on `listener`, reads the request line,
/// pulls `code` out of the query string, and writes back a minimal success
/// page before the connection (and the listener, once the caller drops it)
/// closes.
async fn accept_oauth_code(listener: &TcpListener) -> Result<String, String> {
    let (mut stream, _) = listener
        .accept()
        .await
        .map_err(|e| format!("failed to accept OAuth redirect: {e}"))?;

    let mut buf = [0u8; 8192];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("failed to read OAuth redirect: {e}"))?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let request_line = request.lines().next().unwrap_or("");
    // e.g. "GET /callback?code=XYZ HTTP/1.1"
    let path = request_line.split_whitespace().nth(1).unwrap_or("");
    let code = path
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or("")
        .split('&')
        .find_map(|pair| pair.strip_prefix("code=").map(|v| v.to_string()))
        .unwrap_or_default();
    // Query strings from a redirect the browser itself constructed don't
    // carry characters that need percent-decoding here (Discord's `code` is
    // URL-safe), so no decoding step is needed.

    let body = "Signed in - you can close this tab.";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;

    Ok(code)
}

/// Ports `DisconnectDiscord` (`Home.Panels.cs:939-949`): clears the persisted
/// Discord fields and stops Rich Presence if it was running.
#[tauri::command]
pub fn logout_discord(state: State<'_, AppState>) -> Result<(), String> {
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    settings.discord_logged_in = false;
    settings.discord_user_id = String::new();
    settings.discord_username = String::new();
    settings.discord_avatar = String::new();
    settings.discord_token = String::new();
    gl_settings::save(&settings).map_err(|e| e.to_string())?;

    if let Ok(mut svc) = state.discord.lock() {
        svc.stop();
    }

    Ok(())
}

#[tauri::command]
pub async fn get_announcement(state: State<'_, AppState>) -> Result<Option<Announcement>, String> {
    Ok(gl_integrations::announcements::get_announcement(&state.http).await)
}

#[tauri::command]
pub async fn get_news(state: State<'_, AppState>) -> Result<Option<Vec<NewsPost>>, String> {
    Ok(gl_integrations::news::get_news(&state.http)
        .await
        .map(gl_integrations::news::decode))
}

#[tauri::command]
pub fn start_discord_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.start();
    Ok(())
}

#[tauri::command]
pub fn stop_discord_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.stop();
    Ok(())
}

#[tauri::command]
pub fn set_discord_in_game_presence(
    state: State<'_, AppState>,
    version_tag: String,
    client_name: Option<String>,
) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.set_in_game_presence(&version_tag, client_name.as_deref());
    Ok(())
}

#[tauri::command]
pub fn set_discord_idle_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.set_idle_presence();
    Ok(())
}
