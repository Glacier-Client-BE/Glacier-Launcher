//! Minecraft profile skin/cape management via `api.minecraftservices.com`.
//! Ported from `Services/SkinService.cs`.
//!
//! The C# service returns `string?` — `None`/`null` on success, an
//! error-message string on failure. We keep that shape via
//! `Result<(), String>` for a faithful, low-friction port; callers can
//! upgrade to a richer error enum later if needed.

use serde::Deserialize;
use serde_json::json;

use crate::models::CapeInfo;

const SKINS_ENDPOINT: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const ACTIVE_SKIN: &str = "https://api.minecraftservices.com/minecraft/profile/skins/active";
const PROFILE_ENDPOINT: &str = "https://api.minecraftservices.com/minecraft/profile";
const ACTIVE_CAPE: &str = "https://api.minecraftservices.com/minecraft/profile/capes/active";

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    #[serde(rename = "errorMessage")]
    error_message: Option<String>,
}

fn extract_api_message(body: &str) -> String {
    serde_json::from_str::<ApiErrorBody>(body)
        .ok()
        .and_then(|b| b.error_message)
        .unwrap_or_default()
}

/// Uploads a PNG as the active skin. `png_bytes` are the already-read file
/// contents and `file_name` is used for the multipart filename (the C#
/// service reads the file itself; here the caller supplies the bytes so
/// this crate stays free of filesystem policy decisions).
pub async fn upload_skin(
    client: &reqwest::Client,
    token: &str,
    file_name: &str,
    png_bytes: Vec<u8>,
    slim: bool,
) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("Not signed in — sign in with Microsoft first.".to_string());
    }

    let part = reqwest::multipart::Part::bytes(png_bytes)
        .file_name(file_name.to_string())
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .text("variant", if slim { "slim" } else { "classic" })
        .part("file", part);

    let resp = client
        .post(SKINS_ENDPOINT)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        return Ok(());
    }
    if resp.status().as_u16() == 401 {
        return Err("Your session expired — re-sign in with Microsoft.".to_string());
    }
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    let detail = extract_api_message(&body);
    if detail.is_empty() {
        Err(format!("Minecraft rejected the skin (HTTP {status})."))
    } else {
        Err(format!("Minecraft rejected the skin: {detail}"))
    }
}

/// Resets to the default (Steve/Alex) skin.
pub async fn reset_skin(client: &reqwest::Client, token: &str) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("Not signed in.".to_string());
    }
    let resp = client
        .delete(ACTIVE_SKIN)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Minecraft API returned HTTP {}.",
            resp.status().as_u16()
        ))
    }
}

#[derive(Debug, Deserialize)]
struct ProfileResponse {
    #[serde(default)]
    capes: Vec<RawCape>,
}

#[derive(Debug, Deserialize)]
struct RawCape {
    #[serde(default)]
    id: String,
    #[serde(default)]
    alias: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    state: String,
}

/// Lists the account's available capes.
pub async fn get_capes(client: &reqwest::Client, token: &str) -> Vec<CapeInfo> {
    if token.trim().is_empty() {
        return Vec::new();
    }
    let Ok(resp) = client.get(PROFILE_ENDPOINT).bearer_auth(token).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(profile) = resp.json::<ProfileResponse>().await else {
        return Vec::new();
    };

    profile
        .capes
        .into_iter()
        .filter(|c| !c.id.is_empty())
        .map(|c| CapeInfo {
            id: c.id,
            alias: if c.alias.is_empty() {
                "Cape".to_string()
            } else {
                c.alias
            },
            url: c.url.replacen("http://", "https://", 1),
            active: c.state.eq_ignore_ascii_case("ACTIVE"),
        })
        .collect()
}

/// Sets the active cape by id.
pub async fn set_cape(client: &reqwest::Client, token: &str, cape_id: &str) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("Not signed in.".to_string());
    }
    let resp = client
        .put(ACTIVE_CAPE)
        .bearer_auth(token)
        .json(&json!({ "capeId": cape_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Minecraft API returned HTTP {}.",
            resp.status().as_u16()
        ))
    }
}

/// Hides the currently active cape.
pub async fn hide_cape(client: &reqwest::Client, token: &str) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("Not signed in.".to_string());
    }
    let resp = client
        .delete(ACTIVE_CAPE)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Minecraft API returned HTTP {}.",
            resp.status().as_u16()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_api_error_message() {
        let body = r#"{"path":"/x","errorMessage":"Invalid skin dimensions."}"#;
        assert_eq!(extract_api_message(body), "Invalid skin dimensions.");
    }

    #[test]
    fn missing_error_message_is_empty() {
        assert_eq!(extract_api_message("not json"), "");
        assert_eq!(extract_api_message("{}"), "");
    }

    #[test]
    fn parses_profile_capes() {
        let body = r#"{
            "id": "uuid", "name": "Steve",
            "capes": [
                {"id": "c1", "alias": "Migrator", "url": "http://textures/1", "state": "ACTIVE"},
                {"id": "c2", "alias": "", "url": "https://textures/2", "state": "INACTIVE"}
            ]
        }"#;
        let profile: ProfileResponse = serde_json::from_str(body).unwrap();
        assert_eq!(profile.capes.len(), 2);
        assert_eq!(profile.capes[0].alias, "Migrator");
    }
}
