//! Saved multiplayer server list + ping, ported from
//! `Services/ServerPingService.cs` and the `SavedServer` model. The list
//! itself is persisted as a small JSON file via `gl_persistence`, mirroring
//! the atomic-write-with-corrupt-quarantine pattern used elsewhere.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use gl_integrations::models::ServerStatus;
use gl_integrations::server_ping;

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SavedServer {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub icon_color: String,
}

fn default_port() -> u16 {
    19132
}

fn servers_path() -> std::path::PathBuf {
    gl_settings::launcher_root().join("servers.json")
}

fn load_servers() -> Vec<SavedServer> {
    match gl_persistence::load::<Vec<SavedServer>>(&servers_path()) {
        Ok(Some(envelope)) => envelope.data,
        // Missing file or (already-quarantined) corrupt file both fall back
        // to an empty list rather than surfacing an error to the UI.
        Ok(None) | Err(_) => Vec::new(),
    }
}

fn save_servers(servers: &[SavedServer]) -> Result<(), String> {
    gl_persistence::save(&servers_path(), SCHEMA_VERSION, &servers.to_vec())
        .map_err(|e| e.to_string())
}

fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:032x}")
}

#[tauri::command]
pub fn list_saved_servers() -> Vec<SavedServer> {
    load_servers()
}

#[tauri::command]
pub fn add_saved_server(
    name: String,
    address: String,
    port: u16,
    icon: Option<String>,
    icon_color: Option<String>,
) -> Result<SavedServer, String> {
    let mut servers = load_servers();
    let entry = SavedServer {
        id: new_id(),
        name,
        address,
        port,
        icon: icon.unwrap_or_default(),
        icon_color: icon_color.unwrap_or_default(),
    };
    servers.push(entry.clone());
    save_servers(&servers)?;
    Ok(entry)
}

#[tauri::command]
pub fn update_saved_server(server: SavedServer) -> Result<(), String> {
    let mut servers = load_servers();
    match servers.iter_mut().find(|s| s.id == server.id) {
        Some(existing) => *existing = server,
        None => return Err(format!("no saved server with id {}", server.id)),
    }
    save_servers(&servers)
}

#[tauri::command]
pub fn remove_saved_server(id: String) -> Result<(), String> {
    let mut servers = load_servers();
    servers.retain(|s| s.id != id);
    save_servers(&servers)
}

/// Pings an arbitrary host/port (not necessarily saved), mirroring
/// `ServerPingService.PingAsync`'s port-based Java/Bedrock dispatch.
#[tauri::command]
pub async fn ping_server(host: String, port: u16) -> ServerStatus {
    server_ping::ping(&host, port, 2500).await
}
