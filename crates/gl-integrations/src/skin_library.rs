//! Local skin file library + Mojang username-based skin lookup, ported from
//! `Services/SkinLibraryService.cs`.
//!
//! Deviation: the C# version resolves `SkinsDir` from
//! `LauncherUtilityService.LauncherRoot`, a launcher-wide path helper that
//! doesn't exist in this crate. Callers pass the skins directory in
//! explicitly instead.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Deserialize;

use crate::error::{IntegrationError, Result};
use crate::models::SavedSkin;

/// Lists saved skins in `skins_dir`, newest first, mirroring
/// `SkinLibraryService.List()`. A skin is treated as "slim" if its filename
/// contains `.slim.` (case-insensitive), matching the C# naming convention.
pub fn list_skins(skins_dir: &Path) -> Result<Vec<SavedSkin>> {
    std::fs::create_dir_all(skins_dir)?;

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(skins_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("png"))
            != Some(true)
        {
            continue;
        }
        let metadata = entry.metadata()?;
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
        entries.push((path, modified));
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.1));

    let mut result = Vec::with_capacity(entries.len());
    for (path, modified) in entries {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let stem = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let ticks = modified
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() / 100)
            .unwrap_or(0);
        let slim = file_name.to_lowercase().contains(".slim.");
        let url = format!(
            "https://glacier-files.local/skins/{}?t={}",
            urlencode(&file_name),
            ticks
        );
        result.push(SavedSkin {
            name: stem,
            path: path.to_string_lossy().to_string(),
            url,
            slim,
        });
    }
    Ok(result)
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn sanitize_file_name(name: &str) -> String {
    const INVALID: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];
    name.chars()
        .map(|c| if INVALID.contains(&c) { '_' } else { c })
        .collect()
}

fn unique_path(skins_dir: &Path, base_name: &str, slim: bool) -> PathBuf {
    let base_name = sanitize_file_name(base_name);
    let base_name = if base_name.trim().is_empty() {
        "skin".to_string()
    } else {
        base_name
    };
    let suffix = if slim { ".slim" } else { "" };

    let mut candidate = skins_dir.join(format!("{base_name}{suffix}.png"));
    let mut n = 2;
    while candidate.exists() {
        candidate = skins_dir.join(format!("{base_name}-{n}{suffix}.png"));
        n += 1;
    }
    candidate
}

/// Copies an existing PNG file into the skin library. Mirrors
/// `AddFromFileAsync` (including the 5 MiB size guard).
pub fn add_from_file(skins_dir: &Path, source_path: &Path, slim: bool) -> Result<PathBuf> {
    let metadata = std::fs::metadata(source_path)
        .map_err(|_| IntegrationError::Message("Skin file not found.".to_string()))?;
    if metadata.len() > 5 * 1024 * 1024 {
        return Err(IntegrationError::Message(
            "That PNG is too large to be a skin.".to_string(),
        ));
    }
    let base_name = source_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("skin");
    let dest = unique_path(skins_dir, base_name, slim);
    std::fs::copy(source_path, &dest)?;
    Ok(dest)
}

pub fn delete_skin(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[derive(Debug, Deserialize)]
struct MojangProfileByName {
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionProfile {
    #[serde(default)]
    properties: Vec<SessionProperty>,
}

#[derive(Debug, Deserialize)]
struct SessionProperty {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct TexturesPayload {
    textures: TexturesBlock,
}

#[derive(Debug, Deserialize)]
struct TexturesBlock {
    #[serde(rename = "SKIN")]
    skin: Option<SkinTexture>,
}

#[derive(Debug, Deserialize)]
struct SkinTexture {
    url: String,
    #[serde(default)]
    metadata: Option<SkinTextureMetadata>,
}

#[derive(Debug, Deserialize)]
struct SkinTextureMetadata {
    model: Option<String>,
}

/// Resolves a Minecraft username to a UUID via `api.mojang.com`, mirroring
/// `ResolveUuidAsync`.
async fn resolve_uuid(client: &reqwest::Client, name: &str) -> Result<String> {
    let url = format!(
        "https://api.mojang.com/users/profiles/minecraft/{}",
        urlencode(name)
    );
    let resp = client.get(&url).send().await.map_err(|_| {
        IntegrationError::Message("No internet connection, or Mojang is unreachable.".to_string())
    })?;

    if resp.status().as_u16() == 204 || resp.status().as_u16() == 404 {
        return Err(IntegrationError::Message(format!(
            "No Minecraft player named '{name}'."
        )));
    }
    if !resp.status().is_success() {
        return Err(IntegrationError::Message(format!(
            "Mojang is unavailable (HTTP {}) - try again shortly.",
            resp.status().as_u16()
        )));
    }
    let profile: MojangProfileByName = resp
        .json()
        .await
        .map_err(|e| IntegrationError::Message(format!("Couldn't parse Mojang response: {e}")))?;
    profile
        .id
        .filter(|id| !id.is_empty())
        .ok_or_else(|| IntegrationError::Message(format!("No Minecraft player named '{name}'.")))
}

/// Resolves the account's skin texture URL + slim flag via
/// `sessionserver.mojang.com`, mirroring `ResolveSkinTextureAsync`.
async fn resolve_skin_texture(
    client: &reqwest::Client,
    uuid: &str,
    name: &str,
) -> Result<(String, bool)> {
    let url = format!("https://sessionserver.mojang.com/session/minecraft/profile/{uuid}");
    let resp = client.get(&url).send().await.map_err(|_| {
        IntegrationError::Message("No internet connection, or Mojang is unreachable.".to_string())
    })?;
    if !resp.status().is_success() {
        return Err(IntegrationError::Message(format!(
            "Mojang profile lookup failed (HTTP {}).",
            resp.status().as_u16()
        )));
    }
    let profile: SessionProfile = resp.json().await.map_err(|_| {
        IntegrationError::Message(format!("Couldn't read the textures for '{name}'."))
    })?;

    let value = profile
        .properties
        .into_iter()
        .find(|p| p.name == "textures")
        .map(|p| p.value)
        .ok_or_else(|| {
            IntegrationError::Message(format!("Couldn't read the textures for '{name}'."))
        })?;

    let decoded_bytes = base64_decode(&value)
        .map_err(|_| IntegrationError::Message(format!("Couldn't read the skin for '{name}'.")))?;
    let decoded = String::from_utf8(decoded_bytes)
        .map_err(|_| IntegrationError::Message(format!("Couldn't read the skin for '{name}'.")))?;
    let payload: TexturesPayload = serde_json::from_str(&decoded)
        .map_err(|_| IntegrationError::Message(format!("Couldn't read the skin for '{name}'.")))?;

    let skin = payload
        .textures
        .skin
        .ok_or_else(|| IntegrationError::Message(format!("'{name}' is using the default skin.")))?;
    if skin.url.is_empty() {
        return Err(IntegrationError::Message(format!(
            "'{name}' is using the default skin."
        )));
    }

    let slim = skin
        .metadata
        .and_then(|m| m.model)
        .map(|m| m.eq_ignore_ascii_case("slim"))
        .unwrap_or(false);

    Ok((skin.url, slim))
}

/// Looks up a player's current skin by username and saves it into the skin
/// library. Mirrors `AddFromUsernameAsync`. Skin textures are small
/// (~few KB), so this crate downloads them directly rather than deferring
/// to `gl-download` (which is reserved for large/bulk downloads).
pub async fn add_from_username(
    client: &reqwest::Client,
    skins_dir: &Path,
    username: &str,
) -> Result<PathBuf> {
    let name = username.trim();
    if name.is_empty() {
        return Err(IntegrationError::Message("Enter a username.".to_string()));
    }

    let uuid = resolve_uuid(client, name).await?;
    let (skin_url, slim) = resolve_skin_texture(client, &uuid, name).await?;
    let skin_url = skin_url.replacen("http://", "https://", 1);

    let bytes = client
        .get(&skin_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|_| {
            IntegrationError::Message(
                "Couldn't download the skin texture - try again shortly.".to_string(),
            )
        })?
        .bytes()
        .await
        .map_err(|_| {
            IntegrationError::Message(
                "Couldn't download the skin texture - try again shortly.".to_string(),
            )
        })?;
    if bytes.len() < 100 {
        return Err(IntegrationError::Message(
            "The downloaded skin looked empty.".to_string(),
        ));
    }

    let dest = unique_path(skins_dir, name, slim);
    std::fs::write(&dest, &bytes)?;
    Ok(dest)
}

/// Minimal base64 (standard alphabet, with padding) decoder so this crate
/// doesn't need to pull in the `base64` crate for one call site.
fn base64_decode(input: &str) -> std::result::Result<Vec<u8>, ()> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut table = [255u8; 256];
    for (i, &b) in ALPHABET.iter().enumerate() {
        table[b as usize] = i as u8;
    }

    let clean: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'\n' && b != b'\r')
        .collect();
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    let mut chunk = [0u8; 4];
    let mut chunk_len = 0usize;

    for &b in &clean {
        if b == b'=' {
            break;
        }
        let v = table[b as usize];
        if v == 255 {
            return Err(());
        }
        chunk[chunk_len] = v;
        chunk_len += 1;
        if chunk_len == 4 {
            out.push((chunk[0] << 2) | (chunk[1] >> 4));
            out.push((chunk[1] << 4) | (chunk[2] >> 2));
            out.push((chunk[2] << 6) | chunk[3]);
            chunk_len = 0;
        }
    }
    match chunk_len {
        0 => {}
        2 => out.push((chunk[0] << 2) | (chunk[1] >> 4)),
        3 => {
            out.push((chunk[0] << 2) | (chunk[1] >> 4));
            out.push((chunk[1] << 4) | (chunk[2] >> 2));
        }
        _ => return Err(()),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip_matches_known_vector() {
        // "textures" payload shape, base64-encoded.
        let encoded = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXguL3MifX19";
        let decoded = base64_decode(encoded).unwrap();
        let text = String::from_utf8(decoded).unwrap();
        assert!(text.contains("SKIN"));
        assert!(text.contains("http://tex./s"));
    }

    #[test]
    fn sanitizes_invalid_filename_chars() {
        assert_eq!(sanitize_file_name("a:b/c*d"), "a_b_c_d");
    }

    #[test]
    fn parses_textures_payload() {
        let json =
            r#"{"textures":{"SKIN":{"url":"https://x/skin.png","metadata":{"model":"slim"}}}}"#;
        let payload: TexturesPayload = serde_json::from_str(json).unwrap();
        let skin = payload.textures.skin.unwrap();
        assert_eq!(skin.url, "https://x/skin.png");
        assert_eq!(skin.metadata.unwrap().model.unwrap(), "slim");
    }
}
