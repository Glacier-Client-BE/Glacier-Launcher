//! Microsoft/Xbox OAuth chain: live.com token exchange, XBL/XSTS, Minecraft profile
//! (ports `LiveAuthService.cs` and `XboxProfileService.cs` from the original WPF launcher).
//!
//! Entry points:
//! - [`exchange_code`] — authorization code -> full resolved [`SignInResult`]
//! - [`refresh`] — refresh token -> full resolved [`SignInResult`]
//! - [`refresh_live_token`] — refresh token -> just a new [`LiveToken`] (mirrors
//!   `LiveAuthService.RefreshAsync`)
//!
//! Callers are expected to obtain the authorization code themselves (e.g. via a
//! WebView2/Tauri popup window) and to persist the returned tokens via `gl-persistence`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Microsoft OAuth client id used by the launcher (Xbox Live "Minecraft" client).
pub const CLIENT_ID: &str = "00000000402b5328";
/// OAuth scope requested at login.live.com.
pub const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
/// OAuth redirect URI (desktop flow).
pub const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";

const LIVE_TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
const XBL_AUTH_ENDPOINT: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_ENDPOINT: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const PROFILE_ENDPOINT: &str = "https://profile.xboxlive.com/users/me/profile/settings";
const MC_AUTH_ENDPOINT: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_ENDPOINT: &str = "https://api.minecraftservices.com/minecraft/profile";

/// The relying party used for the "generic" Xbox Live XSTS token (used to fetch the
/// Xbox profile).
const XSTS_RP_XBOXLIVE: &str = "http://xboxlive.com";
/// The relying party used for the Minecraft-specific XSTS token.
const XSTS_RP_MINECRAFT: &str = "rp://api.minecraftservices.com/";

// ---------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------

/// Errors that can occur anywhere in the auth chain.
#[derive(Debug, Error)]
pub enum AuthError {
    /// Transport-level failure (DNS, TLS, connection reset, timeout, ...).
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// The remote service rejected the credentials/token (bad code, expired refresh
    /// token, Xbox account restriction, etc). `message` is user-facing.
    #[error("{message}")]
    Rejected {
        message: String,
        /// Raw Xbox `XErr` code, when the rejection came from XBL/XSTS.
        x_err: Option<String>,
    },

    /// The response didn't parse into the shape we expected — a sign either the API
    /// changed or we're looking at an error page.
    #[error("unexpected response shape from {endpoint}: {detail}")]
    UnexpectedResponse {
        endpoint: &'static str,
        detail: String,
    },

    /// Sign-in succeeded through XBL/XSTS/Xbox-profile but Minecraft authentication
    /// or profile lookup failed. Mirrors `XboxProfileService`'s partial-success path
    /// (Xbox profile persisted, Minecraft profile null).
    #[error("Xbox sign-in succeeded but Minecraft authentication failed")]
    MinecraftAuthFailed,
}

pub type Result<T> = std::result::Result<T, AuthError>;

/// Maps a raw Xbox `XErr` code to a user-facing message, matching
/// `XboxProfileService.MapXErr`.
fn map_xerr(x_err: &str) -> String {
    match x_err {
        "2148916233" => {
            "This Microsoft account doesn't have an Xbox profile yet. Create one at xbox.com."
                .to_string()
        }
        "2148916235" => "Xbox Live isn't available in your country/region.".to_string(),
        "2148916236" => "This account needs adult verification.".to_string(),
        "2148916237" => {
            "This account needs adult verification (age verification required in your region)."
                .to_string()
        }
        "2148916238" => {
            "This is a child account and needs to be added to a Family group by an adult."
                .to_string()
        }
        other => format!("Xbox couldn't authorize this account (XErr {other})."),
    }
}

// ---------------------------------------------------------------------------------
// Public data shapes
// ---------------------------------------------------------------------------------

/// Result of `login.live.com` token exchange. Mirrors `LiveAuthService.LiveToken`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds, UTC) after which the token should be considered
    /// expired. Computed as `now + expires_in - 60` (60s safety margin), same as
    /// the C# `DateTime.UtcNow.AddSeconds(expiresIn - 60)`.
    pub expires_at_unix: i64,
}

/// Xbox Live profile fields. Mirrors `Models.XboxProfile`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XboxProfile {
    pub gamertag: String,
    pub xuid: String,
    pub gamer_picture_url: String,
    pub gamerscore: String,
    pub account_tier: String,
    pub bio: String,
}

/// Resolved Minecraft profile (id/name normalized, first skin URL if any). Mirrors
/// the private `XboxProfileService.MinecraftProfile` record struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfile {
    pub uuid: String,
    pub name: String,
    pub skin_url: String,
}

/// Everything a caller needs after a full sign-in: the Xbox profile, the resolved
/// Minecraft profile (if Minecraft auth succeeded), the Minecraft access token +
/// expiry, and the live.com refresh token to persist for next time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignInResult {
    pub xbox_profile: XboxProfile,
    pub minecraft_profile: MinecraftProfile,
    pub minecraft_access_token: String,
    /// Unix timestamp (seconds, UTC); `now + 23h`, matching
    /// `DateTime.UtcNow.AddHours(23)` in the C#.
    pub minecraft_access_token_expiry_unix: i64,
    /// The live.com refresh token to persist (`XboxLiveRefreshToken` in C#
    /// settings), if the provider returned one.
    pub live_refresh_token: Option<String>,
}

// ---------------------------------------------------------------------------------
// live.com token exchange (LiveAuthService)
// ---------------------------------------------------------------------------------

#[derive(Deserialize)]
struct LiveTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct LiveErrorResponse {
    error: Option<String>,
    error_description: Option<String>,
}

fn parse_live_token_response(body: &str) -> Result<LiveToken> {
    let parsed: LiveTokenResponse =
        serde_json::from_str(body).map_err(|e| AuthError::UnexpectedResponse {
            endpoint: "login.live.com",
            detail: e.to_string(),
        })?;
    let expires_in = parsed.expires_in.unwrap_or(3600);
    let now = now_unix();
    Ok(LiveToken {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_at_unix: now + expires_in - 60,
    })
}

fn extract_live_error_message(body: &str) -> Option<String> {
    let parsed: LiveErrorResponse = serde_json::from_str(body).ok()?;
    parsed.error_description.or(parsed.error)
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Exchange an authorization code (obtained via the external login popup) for a
/// live.com access/refresh token. Mirrors `LiveAuthService.ExchangeCodeAsync`.
pub async fn exchange_code_for_live_token(
    client: &reqwest::Client,
    code: &str,
) -> Result<LiveToken> {
    let form = [
        ("client_id", CLIENT_ID),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("scope", SCOPE),
    ];

    let resp = client.post(LIVE_TOKEN_URL).form(&form).send().await?;
    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        let message = extract_live_error_message(&body)
            .unwrap_or_else(|| format!("Token exchange failed ({}).", status.as_u16()));
        return Err(AuthError::Rejected {
            message,
            x_err: None,
        });
    }

    parse_live_token_response(&body)
}

/// Exchange a stored live.com refresh token for a new access token. Mirrors
/// `LiveAuthService.RefreshAsync`. Returns `Ok(None)` on any failure (network,
/// rejection, or malformed response) — same "swallow and return null" behavior as
/// the C# `try { ... } catch { return null; }`.
pub async fn refresh_live_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Option<LiveToken> {
    let form = [
        ("client_id", CLIENT_ID),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("scope", SCOPE),
        ("redirect_uri", REDIRECT_URI),
    ];

    let resp = client.post(LIVE_TOKEN_URL).form(&form).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body = resp.text().await.ok()?;
    parse_live_token_response(&body).ok()
}

// ---------------------------------------------------------------------------------
// XBL / XSTS (XboxProfileService)
// ---------------------------------------------------------------------------------

#[derive(Serialize)]
struct XblAuthProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: &'static str,
    #[serde(rename = "SiteName")]
    site_name: &'static str,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Serialize)]
struct XblAuthRequest {
    #[serde(rename = "RelyingParty")]
    relying_party: &'static str,
    #[serde(rename = "TokenType")]
    token_type: &'static str,
    #[serde(rename = "Properties")]
    properties: XblAuthProperties,
}

#[derive(Serialize)]
struct XstsAuthProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: &'static str,
    #[serde(rename = "UserTokens")]
    user_tokens: [String; 1],
}

#[derive(Serialize)]
struct XstsAuthRequest {
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: &'static str,
    #[serde(rename = "Properties")]
    properties: XstsAuthProperties,
}

#[derive(Deserialize)]
struct XboxAuthResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Deserialize)]
struct DisplayClaims {
    xui: Vec<XuiEntry>,
}

#[derive(Deserialize)]
struct XuiEntry {
    uhs: String,
}

#[derive(Deserialize)]
struct XboxErrorResponse {
    #[serde(rename = "XErr")]
    x_err: Option<serde_json::Value>,
}

fn xbox_error_to_xerr_string(body: &str) -> Option<String> {
    let parsed: XboxErrorResponse = serde_json::from_str(body).ok()?;
    match parsed.x_err? {
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::String(s) => Some(s),
        _ => None,
    }
}

fn xbl_auth_body(live_access_token: &str) -> XblAuthRequest {
    XblAuthRequest {
        relying_party: "http://auth.xboxlive.com",
        token_type: "JWT",
        properties: XblAuthProperties {
            auth_method: "RPS",
            site_name: "user.auth.xboxlive.com",
            rps_ticket: format!("t={live_access_token}"),
        },
    }
}

fn xsts_auth_body(xbl_token: &str, relying_party: &str) -> XstsAuthRequest {
    XstsAuthRequest {
        relying_party: relying_party.to_string(),
        token_type: "JWT",
        properties: XstsAuthProperties {
            sandbox_id: "RETAIL",
            user_tokens: [xbl_token.to_string()],
        },
    }
}

/// POST an XBL/XSTS auth request and extract `(Token, uhs)`. Mirrors
/// `XboxProfileService.PostXboxAuthAsync`.
async fn post_xbox_auth<B: Serialize>(
    client: &reqwest::Client,
    endpoint: &'static str,
    body: &B,
    contract_version: &str,
) -> Result<(String, String)> {
    let resp = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .header("x-xbl-contract-version", contract_version)
        .json(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body_text = resp.text().await.unwrap_or_default();
        let x_err = xbox_error_to_xerr_string(&body_text);
        let message = match &x_err {
            Some(code) => map_xerr(code),
            None => format!("Xbox auth call failed ({status})."),
        };
        return Err(AuthError::Rejected { message, x_err });
    }

    let text = resp.text().await?;
    let parsed: XboxAuthResponse =
        serde_json::from_str(&text).map_err(|e| AuthError::UnexpectedResponse {
            endpoint,
            detail: e.to_string(),
        })?;
    let uhs = parsed
        .display_claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or_else(|| AuthError::UnexpectedResponse {
            endpoint,
            detail: "missing DisplayClaims.xui[0].uhs".to_string(),
        })?;
    Ok((parsed.token, uhs))
}

/// Exchange a live.com access token for an XBL token + user hash. Mirrors
/// `XboxProfileService.ExchangeForXblTokenAsync`.
async fn exchange_for_xbl_token(
    client: &reqwest::Client,
    live_access_token: &str,
) -> Result<(String, String)> {
    let body = xbl_auth_body(live_access_token);
    post_xbox_auth(client, XBL_AUTH_ENDPOINT, &body, "1").await
}

/// Exchange an XBL token for an XSTS token + user hash against the given relying
/// party. Mirrors `XboxProfileService.ExchangeForXstsTokenAsync`.
async fn exchange_for_xsts_token(
    client: &reqwest::Client,
    xbl_token: &str,
    relying_party: &str,
) -> Result<(String, String)> {
    let body = xsts_auth_body(xbl_token, relying_party);
    post_xbox_auth(client, XSTS_AUTH_ENDPOINT, &body, "1").await
}

// ---------------------------------------------------------------------------------
// Xbox profile
// ---------------------------------------------------------------------------------

#[derive(Deserialize)]
struct ProfileSettingsResponse {
    #[serde(rename = "profileUsers")]
    profile_users: Vec<ProfileUser>,
}

#[derive(Deserialize)]
struct ProfileUser {
    id: Option<String>,
    settings: Vec<ProfileSetting>,
}

#[derive(Deserialize)]
struct ProfileSetting {
    id: String,
    value: String,
}

/// Fetch the Xbox Live profile using the "generic" XSTS token. Mirrors
/// `XboxProfileService.FetchProfileAsync`.
async fn fetch_xbox_profile(
    client: &reqwest::Client,
    xsts_token: &str,
    user_hash: &str,
) -> Result<Option<XboxProfile>> {
    let url = format!(
        "{PROFILE_ENDPOINT}?settings=Gamertag,GameDisplayPicRaw,Gamerscore,AccountTier,XboxOneRep,PreferredColor,RealName,Bio"
    );

    let resp = client
        .get(&url)
        .header(
            "Authorization",
            format!("XBL3.0 x={user_hash};{xsts_token}"),
        )
        .header("x-xbl-contract-version", "3")
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        return Err(AuthError::Rejected {
            message: format!("Failed to fetch Xbox profile ({}).", status.as_u16()),
            x_err: None,
        });
    }

    let text = resp.text().await?;
    let parsed: ProfileSettingsResponse =
        serde_json::from_str(&text).map_err(|e| AuthError::UnexpectedResponse {
            endpoint: "profile.xboxlive.com",
            detail: e.to_string(),
        })?;

    let Some(user) = parsed.profile_users.into_iter().next() else {
        return Ok(None);
    };

    let mut profile = XboxProfile::default();
    for setting in user.settings {
        match setting.id.as_str() {
            "Gamertag" => profile.gamertag = setting.value,
            "GameDisplayPicRaw" => profile.gamer_picture_url = setting.value,
            "Gamerscore" => profile.gamerscore = setting.value,
            "AccountTier" => profile.account_tier = setting.value,
            "Bio" => profile.bio = setting.value,
            _ => {}
        }
    }
    if let Some(xuid) = user.id {
        profile.xuid = xuid;
    }

    if profile.gamertag.is_empty() {
        Ok(None)
    } else {
        Ok(Some(profile))
    }
}

// ---------------------------------------------------------------------------------
// Minecraft services
// ---------------------------------------------------------------------------------

#[derive(Serialize)]
struct McAuthRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Deserialize)]
struct McAuthResponse {
    access_token: Option<String>,
}

/// Exchange an XSTS token/user-hash (Minecraft relying party) for a Minecraft
/// services access token. Mirrors `XboxProfileService.AuthenticateWithMinecraftAsync`.
async fn authenticate_with_minecraft(
    client: &reqwest::Client,
    xsts_token: &str,
    user_hash: &str,
) -> Result<Option<String>> {
    let body = McAuthRequest {
        identity_token: format!("XBL3.0 x={user_hash};{xsts_token}"),
    };

    let resp = client
        .post(MC_AUTH_ENDPOINT)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let text = resp.text().await?;
    let parsed: McAuthResponse =
        serde_json::from_str(&text).map_err(|e| AuthError::UnexpectedResponse {
            endpoint: "api.minecraftservices.com/authentication/login_with_xbox",
            detail: e.to_string(),
        })?;
    Ok(parsed.access_token)
}

#[derive(Deserialize)]
struct McProfileResponse {
    id: Option<String>,
    name: Option<String>,
    skins: Option<Vec<McSkin>>,
}

#[derive(Deserialize)]
struct McSkin {
    url: Option<String>,
}

/// Normalize a bare 32-char hex UUID into dashed form, matching the C#
/// `id[..8]-id[8..12]-id[12..16]-id[16..20]-id[20..]` slicing.
fn normalize_uuid(id: &str) -> String {
    if id.len() == 32 && !id.contains('-') {
        format!(
            "{}-{}-{}-{}-{}",
            &id[0..8],
            &id[8..12],
            &id[12..16],
            &id[16..20],
            &id[20..32]
        )
    } else {
        id.to_string()
    }
}

/// Fetch the Minecraft profile (uuid/name/skin) for a Minecraft services access
/// token. Mirrors `XboxProfileService.FetchMinecraftProfileAsync`.
async fn fetch_minecraft_profile(
    client: &reqwest::Client,
    mc_access_token: &str,
) -> Result<Option<MinecraftProfile>> {
    let resp = client
        .get(MC_PROFILE_ENDPOINT)
        .header("Authorization", format!("Bearer {mc_access_token}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let text = resp.text().await?;
    let parsed: McProfileResponse =
        serde_json::from_str(&text).map_err(|e| AuthError::UnexpectedResponse {
            endpoint: "api.minecraftservices.com/minecraft/profile",
            detail: e.to_string(),
        })?;

    let (Some(id), Some(name)) = (parsed.id, parsed.name) else {
        return Ok(None);
    };
    if id.is_empty() || name.is_empty() {
        return Ok(None);
    }

    let skin_url = parsed
        .skins
        .unwrap_or_default()
        .into_iter()
        .find_map(|s| s.url)
        .unwrap_or_default();

    Ok(Some(MinecraftProfile {
        uuid: normalize_uuid(&id),
        name,
        skin_url,
    }))
}

// ---------------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------------

/// Run the full XBL -> XSTS -> Xbox profile -> Minecraft auth -> Minecraft profile
/// chain given an already-obtained live.com access token. Mirrors the body of
/// `XboxProfileService.SignInAsync` after the live token is acquired.
async fn resolve_sign_in(client: &reqwest::Client, live_token: &LiveToken) -> Result<SignInResult> {
    let (xbl_token, _) = exchange_for_xbl_token(client, &live_token.access_token).await?;

    let (xsts_token, user_hash) =
        exchange_for_xsts_token(client, &xbl_token, XSTS_RP_XBOXLIVE).await?;
    let (mc_xsts_token, mc_user_hash) =
        exchange_for_xsts_token(client, &xbl_token, XSTS_RP_MINECRAFT).await?;

    let xbox_profile = fetch_xbox_profile(client, &xsts_token, &user_hash)
        .await?
        .ok_or_else(|| AuthError::UnexpectedResponse {
            endpoint: "profile.xboxlive.com",
            detail: "could not load Xbox profile".to_string(),
        })?;

    let mc_token = authenticate_with_minecraft(client, &mc_xsts_token, &mc_user_hash).await?;
    let Some(mc_token) = mc_token else {
        return Err(AuthError::MinecraftAuthFailed);
    };

    let mc_profile = fetch_minecraft_profile(client, &mc_token)
        .await?
        .ok_or(AuthError::MinecraftAuthFailed)?;

    Ok(SignInResult {
        xbox_profile,
        minecraft_profile: mc_profile,
        minecraft_access_token: mc_token,
        minecraft_access_token_expiry_unix: now_unix() + 23 * 3600,
        live_refresh_token: live_token.refresh_token.clone(),
    })
}

/// Full sign-in from an authorization code (obtained externally, e.g. via a
/// WebView2/Tauri login popup). Mirrors `XboxProfileService.SignInAsync` when
/// called with a fresh interactive login.
pub async fn exchange_code(client: &reqwest::Client, code: &str) -> Result<SignInResult> {
    let live_token = exchange_code_for_live_token(client, code).await?;
    resolve_sign_in(client, &live_token).await
}

/// Full sign-in from a stored live.com refresh token (refresh-before-launch path).
/// Mirrors `XboxProfileService.SignInAsync` when called non-interactively
/// (`_liveAuth.RefreshAsync()`).
pub async fn refresh(client: &reqwest::Client, refresh_token: &str) -> Result<SignInResult> {
    let live_token = refresh_live_token(client, refresh_token)
        .await
        .ok_or_else(|| AuthError::Rejected {
            message: "Refresh token is invalid or expired; interactive sign-in required."
                .to_string(),
            x_err: None,
        })?;
    resolve_sign_in(client, &live_token).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xbl_auth_body_matches_csharp_shape() {
        let body = xbl_auth_body("EwAAA...token");
        let json = serde_json::to_value(&body).expect("serialize");
        assert_eq!(json["RelyingParty"], "http://auth.xboxlive.com");
        assert_eq!(json["TokenType"], "JWT");
        assert_eq!(json["Properties"]["AuthMethod"], "RPS");
        assert_eq!(json["Properties"]["SiteName"], "user.auth.xboxlive.com");
        assert_eq!(json["Properties"]["RpsTicket"], "t=EwAAA...token");
    }

    #[test]
    fn xsts_auth_body_matches_csharp_shape_for_xboxlive() {
        let body = xsts_auth_body("xbl-token", XSTS_RP_XBOXLIVE);
        let json = serde_json::to_value(&body).expect("serialize");
        assert_eq!(json["RelyingParty"], "http://xboxlive.com");
        assert_eq!(json["TokenType"], "JWT");
        assert_eq!(json["Properties"]["SandboxId"], "RETAIL");
        assert_eq!(
            json["Properties"]["UserTokens"],
            serde_json::json!(["xbl-token"])
        );
    }

    #[test]
    fn xsts_auth_body_matches_csharp_shape_for_minecraft() {
        let body = xsts_auth_body("xbl-token", XSTS_RP_MINECRAFT);
        let json = serde_json::to_value(&body).expect("serialize");
        assert_eq!(json["RelyingParty"], "rp://api.minecraftservices.com/");
    }

    #[test]
    fn mc_auth_request_matches_csharp_shape() {
        let body = McAuthRequest {
            identity_token: "XBL3.0 x=hash;token".to_string(),
        };
        let json = serde_json::to_value(&body).expect("serialize");
        assert_eq!(json["identityToken"], "XBL3.0 x=hash;token");
    }

    #[test]
    fn parse_live_token_response_full() {
        let body = r#"{"access_token":"acc123","refresh_token":"ref456","expires_in":3600}"#;
        let token = parse_live_token_response(body).expect("parse");
        assert_eq!(token.access_token, "acc123");
        assert_eq!(token.refresh_token.as_deref(), Some("ref456"));
        let now = now_unix();
        assert!(token.expires_at_unix > now + 3000 && token.expires_at_unix <= now + 3540);
    }

    #[test]
    fn parse_live_token_response_missing_expires_in_defaults_3600() {
        let body = r#"{"access_token":"acc123"}"#;
        let token = parse_live_token_response(body).expect("parse");
        assert_eq!(token.refresh_token, None);
        let now = now_unix();
        assert!(token.expires_at_unix > now + 3000 && token.expires_at_unix <= now + 3540);
    }

    #[test]
    fn parse_live_token_response_rejects_garbage() {
        let body = r#"{"not_a_token_field": true}"#;
        assert!(parse_live_token_response(body).is_err());
    }

    #[test]
    fn extract_live_error_message_prefers_description() {
        let body =
            r#"{"error":"invalid_grant","error_description":"The provided code is invalid."}"#;
        assert_eq!(
            extract_live_error_message(body).as_deref(),
            Some("The provided code is invalid.")
        );
    }

    #[test]
    fn extract_live_error_message_falls_back_to_error() {
        let body = r#"{"error":"invalid_grant"}"#;
        assert_eq!(
            extract_live_error_message(body).as_deref(),
            Some("invalid_grant")
        );
    }

    #[test]
    fn xbox_auth_response_parses_token_and_uhs() {
        let body = r#"{
            "IssueInstant": "2024-01-01T00:00:00Z",
            "NotAfter": "2024-01-01T16:00:00Z",
            "Token": "xbl-token-value",
            "DisplayClaims": { "xui": [ { "uhs": "user-hash-value" } ] }
        }"#;
        let parsed: XboxAuthResponse = serde_json::from_str(body).expect("parse");
        assert_eq!(parsed.token, "xbl-token-value");
        assert_eq!(parsed.display_claims.xui[0].uhs, "user-hash-value");
    }

    #[test]
    fn xbox_error_response_extracts_numeric_xerr() {
        let body = r#"{"Identity":"0","XErr":2148916233,"Message":"","Redirect":""}"#;
        assert_eq!(
            xbox_error_to_xerr_string(body).as_deref(),
            Some("2148916233")
        );
    }

    #[test]
    fn xbox_error_response_extracts_string_xerr() {
        let body = r#"{"XErr":"2148916233"}"#;
        assert_eq!(
            xbox_error_to_xerr_string(body).as_deref(),
            Some("2148916233")
        );
    }

    #[test]
    fn map_xerr_known_codes() {
        assert!(map_xerr("2148916233").contains("doesn't have an Xbox profile"));
        assert!(map_xerr("2148916238").contains("Family group"));
        assert!(map_xerr("9999999999").contains("9999999999"));
    }

    #[test]
    fn profile_settings_response_parses_and_maps_fields() {
        let body = r#"{
            "profileUsers": [
                {
                    "id": "2535000000000000",
                    "settings": [
                        {"id": "Gamertag", "value": "TestGamer"},
                        {"id": "GameDisplayPicRaw", "value": "https://images-eds.xboxlive.com/pic.png"},
                        {"id": "Gamerscore", "value": "12345"},
                        {"id": "AccountTier", "value": "Gold"},
                        {"id": "Bio", "value": "hello world"}
                    ]
                }
            ]
        }"#;
        let parsed: ProfileSettingsResponse = serde_json::from_str(body).expect("parse");
        let user = &parsed.profile_users[0];
        assert_eq!(user.id.as_deref(), Some("2535000000000000"));
        assert_eq!(user.settings.len(), 5);
    }

    #[test]
    fn normalize_uuid_adds_dashes_to_bare_hex() {
        assert_eq!(
            normalize_uuid("069a79f444e94726a5befca90e38aaaf"),
            "069a79f4-44e9-4726-a5be-fca90e38aaaf"
        );
    }

    #[test]
    fn normalize_uuid_leaves_dashed_uuid_alone() {
        let dashed = "069a79f4-44e9-4726-a5be-fca90e38aaf9";
        assert_eq!(normalize_uuid(dashed), dashed);
    }

    #[test]
    fn mc_profile_response_parses_with_skin() {
        let body = r#"{
            "id": "069a79f444e94726a5befca90e38aaf",
            "name": "Notch",
            "skins": [
                {"id": "abc", "state": "ACTIVE", "url": "https://textures.minecraft.net/texture/abc", "variant": "CLASSIC"}
            ],
            "capes": []
        }"#;
        let parsed: McProfileResponse = serde_json::from_str(body).expect("parse");
        assert_eq!(
            parsed.id.as_deref(),
            Some("069a79f444e94726a5befca90e38aaf")
        );
        assert_eq!(parsed.name.as_deref(), Some("Notch"));
        let skin_url = parsed.skins.unwrap()[0].url.clone().unwrap();
        assert_eq!(skin_url, "https://textures.minecraft.net/texture/abc");
    }

    #[test]
    fn mc_profile_response_handles_no_skins() {
        let body =
            r#"{"id":"069a79f444e94726a5befca90e38aaf","name":"Notch","skins":[],"capes":[]}"#;
        let parsed: McProfileResponse = serde_json::from_str(body).expect("parse");
        assert_eq!(parsed.skins.unwrap().len(), 0);
    }

    #[test]
    fn mc_auth_response_parses_access_token() {
        let body = r#"{"username":"...","roles":[],"access_token":"mc-token","token_type":"Bearer","expires_in":86400}"#;
        let parsed: McAuthResponse = serde_json::from_str(body).expect("parse");
        assert_eq!(parsed.access_token.as_deref(), Some("mc-token"));
    }

    #[test]
    fn constants_match_csharp_literals() {
        assert_eq!(CLIENT_ID, "00000000402b5328");
        assert_eq!(SCOPE, "service::user.auth.xboxlive.com::MBI_SSL");
        assert_eq!(REDIRECT_URI, "https://login.live.com/oauth20_desktop.srf");
    }

    // ---- live network integration tests (not run in CI) ----

    #[tokio::test]
    #[ignore = "hits live login.live.com endpoint; requires a real authorization code"]
    async fn exchange_code_against_live_endpoint() {
        let client = reqwest::Client::new();
        let result = exchange_code_for_live_token(&client, "fake-code").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "hits live Xbox Live / Minecraft services endpoints; requires a real token chain"]
    async fn full_sign_in_against_live_endpoints() {
        let client = reqwest::Client::new();
        let result = refresh(&client, "fake-refresh-token").await;
        assert!(result.is_err());
    }
}
