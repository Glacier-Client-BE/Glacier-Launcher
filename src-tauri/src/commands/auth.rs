//! Microsoft/Xbox sign-in, wired through a popup `WebviewWindow` that
//! navigates to `login.live.com`, mirroring `LiveAuthWindow.xaml.cs` +
//! `LiveAuthService.SignInInteractiveAsync`.
//!
//! Tauri's `on_navigation` hook is the equivalent of the C# code's
//! `CoreWebView2.NavigationStarting` interception: it fires synchronously
//! before each navigation and lets us deny it (returning `false`) once we've
//! recognized the redirect back to `REDIRECT_URI` and pulled the
//! authorization code out of its query string.

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;
use url::Url;

use crate::state::AppState;

const AUTH_WINDOW_LABEL: &str = "auth-popup";

fn login_url() -> Url {
    let mut url = Url::parse("https://login.live.com/oauth20_authorize.srf").unwrap();
    url.query_pairs_mut()
        .append_pair("client_id", gl_auth::CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", gl_auth::REDIRECT_URI)
        .append_pair("scope", gl_auth::SCOPE)
        .append_pair("display", "touch")
        // Without this, a cached login.live.com session cookie from an
        // already-signed-in account makes the popup silently re-auth with
        // that same account and redirect immediately - the window "opens
        // and closes" before the user ever sees a form, exactly the
        // symptom hit trying to add a *second* account (the first
        // account's session cookie is still sitting in the webview).
        // `select_account` forces the picker/login form every time,
        // matching the doc comment's claim of `promptAccountPicker: true`
        // that this URL never actually implemented.
        .append_pair("prompt", "select_account");
    url
}

/// Extracts the `code` query param from a redirect URL, if `url` is under
/// `gl_auth::REDIRECT_URI`. Mirrors `LiveAuthWindow`'s redirect-URI-prefix
/// check before reading the authorization code.
fn extract_auth_code(url: &Url) -> Option<String> {
    let redirect = Url::parse(gl_auth::REDIRECT_URI).ok()?;
    if url.scheme() != redirect.scheme()
        || url.host_str() != redirect.host_str()
        || url.path() != redirect.path()
    {
        return None;
    }
    url.query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
}

/// Opens the Microsoft sign-in popup, waits for the redirect, exchanges the
/// resulting authorization code through the full Xbox/Minecraft auth chain,
/// and returns the resolved profile. Mirrors
/// `XboxProfileService.SignInAsync(promptAccountPicker: true)`'s interactive
/// path end-to-end.
#[tauri::command]
pub async fn sign_in(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<gl_auth::SignInResult, String> {
    // Close any stale popup left over from a previous, abandoned attempt.
    if let Some(existing) = app.get_webview_window(AUTH_WINDOW_LABEL) {
        let _ = existing.close();
    }

    let (tx, rx) = oneshot::channel::<Option<String>>();
    let tx = std::sync::Mutex::new(Some(tx));

    let window =
        WebviewWindowBuilder::new(&app, AUTH_WINDOW_LABEL, WebviewUrl::External(login_url()))
            .title("Sign in to Microsoft")
            .inner_size(480.0, 640.0)
            .on_navigation(move |url| {
                if let Some(code) = extract_auth_code(url) {
                    if let Some(sender) = tx.lock().unwrap().take() {
                        let _ = sender.send(Some(code));
                    }
                    return false; // deny the navigation to the redirect URI itself
                }
                true
            })
            .build()
            .map_err(|e| format!("failed to open sign-in window: {e}"))?;

    let code = match rx.await {
        Ok(Some(code)) => code,
        Ok(None) | Err(_) => {
            let _ = window.close();
            return Err("sign-in was cancelled".to_string());
        }
    };
    let _ = window.close();

    gl_auth::exchange_code(&state.http, &code)
        .await
        .map_err(|e| e.to_string())
}

/// Attempts a silent sign-in using a previously persisted refresh token.
/// Mirrors `XboxProfileService.SignInAsync(promptAccountPicker: false)`'s
/// refresh-first path (the interactive fallback is left to the caller: if
/// this returns `Err`, the caller should invoke [`sign_in`] instead).
#[tauri::command]
pub async fn sign_in_with_refresh_token(
    state: State<'_, AppState>,
    refresh_token: String,
) -> Result<gl_auth::SignInResult, String> {
    gl_auth::refresh(&state.http, &refresh_token)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_url_carries_expected_oauth_params() {
        let url = login_url();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(
            pairs.get("client_id"),
            Some(&gl_auth::CLIENT_ID.to_string())
        );
        assert_eq!(pairs.get("response_type"), Some(&"code".to_string()));
        assert_eq!(
            pairs.get("redirect_uri"),
            Some(&gl_auth::REDIRECT_URI.to_string())
        );
        assert_eq!(pairs.get("scope"), Some(&gl_auth::SCOPE.to_string()));
    }

    #[test]
    fn login_url_forces_the_account_picker() {
        // Regression test: without `prompt=select_account`, a cached
        // login.live.com session silently re-authenticates the same
        // account and the popup closes before the user can pick a
        // different one.
        let url = login_url();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(pairs.get("prompt"), Some(&"select_account".to_string()));
    }

    #[test]
    fn extract_auth_code_reads_code_from_redirect_uri() {
        let url =
            Url::parse("https://login.live.com/oauth20_desktop.srf?code=ABC123&lc=1033").unwrap();
        assert_eq!(extract_auth_code(&url).as_deref(), Some("ABC123"));
    }

    #[test]
    fn extract_auth_code_ignores_unrelated_urls() {
        let url = Url::parse("https://login.live.com/some-other-page?code=ABC123").unwrap();
        assert_eq!(extract_auth_code(&url), None);

        let url = Url::parse("https://example.com/?code=ABC123").unwrap();
        assert_eq!(extract_auth_code(&url), None);
    }

    #[test]
    fn extract_auth_code_returns_none_without_code_param() {
        let url =
            Url::parse("https://login.live.com/oauth20_desktop.srf?error=access_denied").unwrap();
        assert_eq!(extract_auth_code(&url), None);
    }
}
