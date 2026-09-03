//! Shared data models ported from `Models/AppNotification.cs`,
//! `Models/TweakPack.cs`, and `Models/PlaySession.cs`, plus the small
//! response/record types that lived inline in the various C# services.

use serde::{Deserialize, Serialize};
use uuid_like::new_id;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppNotification {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub message: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "now_iso")]
    pub time_iso: String,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub url: String,
}

fn default_kind() -> String {
    "info".to_string()
}

fn default_icon() -> String {
    "fa-solid fa-circle-info".to_string()
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Minimal RFC3339-ish stamp; callers that need real formatting should
    // use gl-persistence/chrono once wired in.
    format!("{secs}")
}

impl Default for AppNotification {
    fn default() -> Self {
        Self {
            id: new_id(),
            title: String::new(),
            message: String::new(),
            kind: default_kind(),
            icon: default_icon(),
            time_iso: now_iso(),
            read: false,
            url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TweakPack {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category_id: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub icon_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TweakCategory {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub packs: Vec<TweakPack>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PlaySession {
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub seconds: i64,
    #[serde(default)]
    pub edition: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Announcement {
    pub id: String,
    pub title: String,
    pub message: String,
    pub kind: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewsPost {
    pub icon: String,
    pub title: String,
    pub subtitle: String,
    pub url: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapeInfo {
    pub id: String,
    pub alias: String,
    pub url: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerStatus {
    pub online: bool,
    pub players_online: i32,
    pub players_max: i32,
    pub motd: String,
    pub version: String,
    pub latency_ms: i64,
}

impl ServerStatus {
    pub const OFFLINE: ServerStatus = ServerStatus {
        online: false,
        players_online: 0,
        players_max: 0,
        motd: String::new(),
        version: String::new(),
        latency_ms: 0,
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedSkin {
    pub name: String,
    pub path: String,
    pub url: String,
    pub slim: bool,
}

/// Tiny helper module so we don't need to pull in the `uuid` crate just for
/// generating a random-ish notification id (mirrors `Guid.NewGuid("N")`
/// closely enough for a local-only identifier).
mod uuid_like {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn new_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{nanos:032x}")
    }
}
