use base64::Engine;
use gl_integrations::models::{Announcement, NewsPost};
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{timeout, Duration};

use crate::state::AppState;

/// Discord's public OAuth client id for this app — not a secret, safe to
/// embed (Discord requires it in the authorize URL, which is necessarily
/// visible to the user's browser anyway).
const DISCORD_CLIENT_ID: &str = "1482726422094024779";
const DISCORD_REDIRECT_URI: &str = "http://localhost:5000/callback";

/// No client secret here by design: this flow uses PKCE (RFC 7636) instead
/// of a confidential-client secret. A secret embedded in a distributed
/// desktop binary isn't actually confidential — anyone can extract it — so
/// baking one in (as this used to) provided no real security and was a
/// real leaked-credential liability. PKCE gives the same protection against
/// authorization-code interception without needing a secret at all. This
/// requires the Discord application to be configured as a "Public Client"
/// in the Discord Developer Portal (OAuth2 → General); a confidential-only
/// app will reject a secret-less token exchange.
fn generate_random_url_safe(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// PKCE `code_challenge` (S256) for a given `code_verifier`, per RFC 7636 §4.2.
fn pkce_code_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

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

    // CSRF guard: only accept a redirect that echoes back this exact
    // per-login token, so another local process (or a malicious page)
    // racing our loopback listener can't inject its own authorization code
    // and get logged in as it instead of the user.
    let csrf_state = generate_random_url_safe(24);
    // PKCE (RFC 7636): proves to Discord that whoever redeems `code` is the
    // same party that started this flow, without needing a client secret.
    let code_verifier = generate_random_url_safe(48);
    let code_challenge = pkce_code_challenge(&code_verifier);

    let authorize_url = url::Url::parse_with_params(
        "https://discord.com/oauth2/authorize",
        &[
            ("client_id", DISCORD_CLIENT_ID),
            ("response_type", "code"),
            ("redirect_uri", DISCORD_REDIRECT_URI),
            ("scope", "identify"),
            ("state", csrf_state.as_str()),
            ("code_challenge", code_challenge.as_str()),
            ("code_challenge_method", "S256"),
        ],
    )
    .map_err(|e| format!("failed to build Discord authorize URL: {e}"))?;

    crate::commands::system::open_url(authorize_url.to_string())
        .map_err(|e| format!("failed to open Discord login page: {e}"))?;

    let (code, returned_state) = timeout(Duration::from_secs(120), accept_oauth_code(&listener))
        .await
        .map_err(|_| "Discord login timed out waiting for the browser redirect.".to_string())??;

    if code.is_empty() {
        return Err("Discord did not return an authorization code.".to_string());
    }
    if returned_state != csrf_state {
        return Err(
            "Discord login redirect failed a security check (state mismatch) — please try again."
                .to_string(),
        );
    }

    let token_response = state
        .http
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", DISCORD_CLIENT_ID),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", DISCORD_REDIRECT_URI),
            ("code_verifier", code_verifier.as_str()),
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

/// The launcher's own window background (`frontend/public/images/bg.jpg`,
/// ~25 KB) and Glacier icon (`src-tauri/icons/128x128.png`), embedded at
/// compile time. This page is served directly off a raw `TcpStream` by a
/// one-shot loopback listener, not through Tauri's asset pipeline or a real
/// web server — there's nothing to serve these files *from* at a URL, so
/// they're inlined as `data:` URIs instead of linked.
const BG_IMAGE_BYTES: &[u8] = include_bytes!("../../../frontend/public/images/bg.jpg");
const ICON_IMAGE_BYTES: &[u8] = include_bytes!("../../icons/128x128.png");

/// Builds the success page served by the loopback OAuth listener once
/// Discord redirects back with a code — the only thing the user's browser
/// ever shows for this flow. Colors/typography mirror the launcher's own
/// default theme (`frontend/src/app.css`'s `:root` variables) and its
/// window background, so it doesn't look like a generic localhost dev page.
/// Font Awesome icons load from the same CDN `frontend/index.html` already
/// uses — fine here since this renders in the user's real browser, not the
/// Tauri webview's restricted CSP. The "Open Glacier Launcher" button links
/// to the `glacier://` scheme this app already registers
/// (`tauri-plugin-deep-link`, see `lib.rs`) and which the running instance's
/// `tauri_plugin_single_instance` handler answers by showing/focusing the
/// main window — so the button does what it says even though this page has
/// no other way to reach into the desktop app.
fn oauth_success_page() -> String {
    let bg_b64 = base64::engine::general_purpose::STANDARD.encode(BG_IMAGE_BYTES);
    let icon_b64 = base64::engine::general_purpose::STANDARD.encode(ICON_IMAGE_BYTES);
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Discord Connected — Glacier Launcher</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="icon" type="image/png" href="data:image/png;base64,{icon_b64}">
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.5.2/css/all.min.css">
<style>
  :root {{
    --accent: #7289da;
    --accent-hover: #8ea0e0;
    --accent-press: #5b6eb8;
    --accent-glow: rgba(114, 137, 218, 0.42);
    --accent-bg: rgba(114, 137, 218, 0.14);
    --bg: #23272a;
    --bg-panel: #2c2f33;
    --bg-inset: #1f2227;
    --text: #ffffff;
    --text-dim: #99aab5;
    --green: #43b581;
    /* Same overlay stops the real launcher uses over its window
       background when a panel/modal floats above it
       (`.window.panel-open .window-bg`, frontend/src/app.css). */
    --overlay-top: rgba(35, 39, 42, 0.55);
    --overlay-mid: rgba(35, 39, 42, 0.20);
    --overlay-bot: rgba(35, 39, 42, 0.90);
  }}
  * {{
    box-sizing: border-box;
  }}
  html, body {{
    height: 100%;
    margin: 0;
  }}
  body {{
    display: flex;
    flex-direction: column;
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    color: var(--text);
    position: relative;
    overflow: hidden;
  }}
  .page-header, .page-footer {{
    position: relative;
    z-index: 10;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    height: 56px;
    padding: 0 22px;
    background: linear-gradient(to bottom, rgba(31, 34, 39, 0.55), rgba(31, 34, 39, 0.15));
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }}
  .page-footer {{
    justify-content: center;
    background: linear-gradient(to top, rgba(31, 34, 39, 0.55), rgba(31, 34, 39, 0.15));
    border-bottom: none;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }}
  .page-header .brand-row {{
    justify-content: flex-start;
    margin-bottom: 0;
  }}
  .page-header .brand-row img {{
    width: 24px;
    height: 24px;
  }}
  .page-header .status-pill {{
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--green);
    background: rgba(67, 181, 129, 0.14);
    border: 1px solid rgba(67, 181, 129, 0.28);
    padding: 5px 11px;
    border-radius: var(--r-pill, 999px);
  }}
  .page-header .status-pill i {{
    font-size: 7px;
  }}
  .page-footer p {{
    font-size: 11.5px;
    color: var(--text-dim);
    opacity: 0.7;
    max-width: none;
  }}
  .page-footer i {{
    margin-right: 6px;
    opacity: 0.8;
  }}
  .content {{
    position: relative;
    z-index: 5;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 0;
  }}
  .window-bg {{
    position: fixed;
    inset: 0;
    background-image: url('data:image/jpeg;base64,{bg_b64}');
    background-size: cover;
    background-position: center;
    /* Mirrors the launcher's own "panel open over the background" look —
       this page IS that floating panel, so it should feel like one. */
    filter: blur(6px) brightness(0.7) saturate(1.05);
    transform: scale(1.05);
  }}
  .window-bg::after {{
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom,
        var(--overlay-top) 0%, var(--overlay-mid) 45%, var(--overlay-bot) 100%);
  }}
  .card {{
    position: relative;
    width: min(432px, calc(100vw - 48px));
    background: linear-gradient(180deg, rgba(44, 47, 51, 0.82), rgba(31, 34, 39, 0.86));
    backdrop-filter: blur(24px) saturate(160%);
    -webkit-backdrop-filter: blur(24px) saturate(160%);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 22px;
    padding: 30px 34px 28px;
    text-align: center;
    box-shadow:
      0 30px 80px rgba(0, 0, 0, 0.55),
      0 1px 0 rgba(255, 255, 255, 0.06) inset,
      0 0 0 1px rgba(0, 0, 0, 0.2);
    animation: rise 0.5s cubic-bezier(0.22, 1, 0.36, 1);
  }}
  @keyframes rise {{
    from {{ opacity: 0; transform: translateY(14px) scale(0.98); }}
    to   {{ opacity: 1; transform: translateY(0) scale(1); }}
  }}
  .brand-row {{
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    margin-bottom: 20px;
  }}
  .brand-row img {{
    width: 22px;
    height: 22px;
    border-radius: 6px;
  }}
  .brand-row span {{
    font-size: 11.5px;
    font-weight: 700;
    letter-spacing: 0.14em;
    color: var(--text-dim);
    text-transform: uppercase;
  }}
  .status-icon {{
    position: relative;
    width: 76px;
    height: 76px;
    margin: 0 auto 22px;
    border-radius: 50%;
    background: radial-gradient(circle at 50% 40%, rgba(67, 181, 129, 0.28), rgba(67, 181, 129, 0.08));
    display: flex;
    align-items: center;
    justify-content: center;
  }}
  .status-icon::before {{
    content: '';
    position: absolute;
    inset: -8px;
    border-radius: 50%;
    border: 1px solid rgba(67, 181, 129, 0.35);
    animation: pulse-ring 2.2s cubic-bezier(0.22, 1, 0.36, 1) infinite;
  }}
  @keyframes pulse-ring {{
    0%   {{ transform: scale(0.92); opacity: 0.9; }}
    70%  {{ transform: scale(1.18); opacity: 0; }}
    100% {{ transform: scale(1.18); opacity: 0; }}
  }}
  .status-icon i {{
    font-size: 32px;
    color: var(--green);
    filter: drop-shadow(0 2px 10px rgba(67, 181, 129, 0.5));
  }}
  .status-badge {{
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--accent);
    border: 3px solid #262a2d;
    display: flex;
    align-items: center;
    justify-content: center;
  }}
  .status-badge i {{
    font-size: 12px;
    color: #ffffff;
    filter: none;
  }}
  h1 {{
    margin: 0 0 9px;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }}
  p {{
    margin: 0;
    color: var(--text-dim);
    font-size: 14px;
    line-height: 1.55;
    max-width: 30ch;
    margin-inline: auto;
  }}
  .open-btn {{
    margin-top: 26px;
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    background: linear-gradient(180deg, var(--accent-hover), var(--accent));
    color: #ffffff;
    text-decoration: none;
    font-size: 14.5px;
    font-weight: 600;
    padding: 14px 20px;
    border-radius: 13px;
    box-shadow: 0 10px 28px var(--accent-glow), 0 1px 0 rgba(255, 255, 255, 0.18) inset;
    transition: transform 0.15s var(--ease-out, ease), box-shadow 0.15s ease, filter 0.15s ease;
  }}
  .open-btn:hover {{
    filter: brightness(1.06);
    transform: translateY(-1px);
    box-shadow: 0 14px 34px var(--accent-glow), 0 1px 0 rgba(255, 255, 255, 0.2) inset;
  }}
  .open-btn:active {{
    transform: translateY(0);
    background: linear-gradient(180deg, var(--accent), var(--accent-press));
  }}
  .open-btn i {{
    font-size: 15px;
  }}
  .hint {{
    margin-top: 18px;
    font-size: 12.5px;
    color: var(--text-dim);
    opacity: 0.85;
  }}
</style>
</head>
<body>
  <div class="window-bg"></div>

  <header class="page-header">
    <div class="brand-row">
      <img src="data:image/png;base64,{icon_b64}" alt="">
      <span>Glacier Launcher</span>
    </div>
    <div class="status-pill"><i class="fa-solid fa-circle"></i> Connected</div>
  </header>

  <div class="content">
    <div class="card">
      <div class="status-icon">
        <i class="fa-solid fa-check"></i>
        <div class="status-badge"><i class="fa-brands fa-discord"></i></div>
      </div>
      <h1>Discord account connected</h1>
      <p>You're all set — this tab is done.</p>
      <a class="open-btn" href="glacier://">
        <i class="fa-solid fa-arrow-right-to-bracket"></i>
        Open Glacier Launcher
      </a>
      <div class="hint">You can also just close this tab</div>
    </div>
  </div>

  <footer class="page-footer">
    <p><i class="fa-solid fa-snowflake"></i>Glacier Launcher · Account Integrations</p>
  </footer>
</body>
</html>"##
    )
}

/// Accepts exactly one connection on `listener`, reads the request line,
/// pulls `code` and `state` out of the query string, and writes back a
/// minimal success page before the connection (and the listener, once the
/// caller drops it) closes. Returns `(code, state)` — the caller is
/// responsible for verifying `state` matches what it generated before
/// trusting `code`.
async fn accept_oauth_code(listener: &TcpListener) -> Result<(String, String), String> {
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
    // e.g. "GET /callback?code=XYZ&state=YYY HTTP/1.1"
    let path = request_line.split_whitespace().nth(1).unwrap_or("");
    let query = path.split_once('?').map(|(_, query)| query).unwrap_or("");
    let find_param = |name: &str| {
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix(name).map(|v| v.to_string()))
            .unwrap_or_default()
    };
    let code = find_param("code=");
    let returned_state = find_param("state=");
    // Query strings from a redirect the browser itself constructed don't
    // carry characters that need percent-decoding here (Discord's `code`/
    // `state` are URL-safe by construction — we generate `state` ourselves
    // with a URL-safe alphabet), so no decoding step is needed.

    let body = oauth_success_page();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;

    Ok((code, returned_state))
}

/// Ports `DisconnectDiscord` (`Home.Panels.cs:939-949`): clears the persisted
/// Discord fields and stops Rich Presence if it was running.
#[tauri::command]
pub async fn logout_discord(state: State<'_, AppState>) -> Result<(), String> {
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
    // `get_news` already HTML-decodes title/subtitle exactly once (and caches
    // the decoded posts). Decoding again here would turn "&amp;lt;" into "<".
    Ok(gl_integrations::news::get_news(&state.http).await)
}

#[tauri::command]
pub async fn start_discord_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.start();
    Ok(())
}

#[tauri::command]
pub async fn stop_discord_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.stop();
    Ok(())
}

#[tauri::command]
pub async fn set_discord_in_game_presence(
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
pub async fn set_discord_idle_presence(state: State<'_, AppState>) -> Result<(), String> {
    let mut svc = state
        .discord
        .lock()
        .map_err(|_| "discord service lock poisoned".to_string())?;
    svc.set_idle_presence();
    Ok(())
}
