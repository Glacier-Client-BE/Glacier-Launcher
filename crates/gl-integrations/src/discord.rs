//! Discord Rich Presence integration.
//!
//! Ported from `Services/DiscordRpcService.cs`. Uses a local IPC connection
//! (named pipe on Windows, unix socket elsewhere) to the running Discord
//! client via the `discord-rich-presence` crate rather than hand-rolling the
//! Discord IPC protocol.

use std::time::{SystemTime, UNIX_EPOCH};

use discord_rich_presence::activity::{Activity, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use tracing::warn;

const APPLICATION_ID: &str = "1482726422094024779";

/// Manages a connection to the local Discord client and pushes Rich Presence
/// updates for the launcher's various states (idle, in-game Bedrock, in-game
/// Java).
pub struct DiscordRpcService {
    client: Option<DiscordIpcClient>,
    running: bool,
}

impl Default for DiscordRpcService {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordRpcService {
    pub fn new() -> Self {
        Self {
            client: None,
            running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Connects to the local Discord client and sets the idle presence.
    /// Mirrors `Start()` in the C# service: failures are swallowed (logged)
    /// rather than propagated, since Discord may simply not be running.
    pub fn start(&mut self) {
        if self.running {
            return;
        }

        let mut client = match DiscordIpcClient::new(APPLICATION_ID) {
            Ok(c) => c,
            Err(err) => {
                warn!("[Discord RPC] failed to construct client: {err}");
                return;
            }
        };

        match client.connect() {
            Ok(()) => {
                self.client = Some(client);
                self.running = true;
                self.set_idle_presence();
            }
            Err(err) => {
                warn!("[Discord RPC] {err}");
            }
        }
    }

    /// Clears the presence and closes the IPC connection. Mirrors `Stop()`.
    pub fn stop(&mut self) {
        if !self.running {
            return;
        }
        if let Some(mut client) = self.client.take() {
            let _ = client.clear_activity();
            let _ = client.close();
        }
        self.running = false;
    }

    /// Sets the "In the Launcher / Selecting a version" idle presence.
    pub fn set_idle_presence(&mut self) {
        let Some(client) = self.client.as_mut() else {
            return;
        };
        if !self.running {
            return;
        }

        let activity = Activity::new()
            .details("In the Launcher")
            .state("Selecting a version")
            .assets(
                Assets::new()
                    .large_image("glacier_logo")
                    .large_text("Glacier Launcher"),
            )
            .timestamps(Timestamps::new().start(now_unix()));

        if let Err(err) = client.set_activity(activity) {
            warn!("[Discord RPC] failed to set idle presence: {err}");
        }
    }

    /// Sets the "Playing Minecraft Bedrock" presence.
    ///
    /// `client_name` mirrors the C# switch over the Bedrock client variant
    /// (Flarial, OderSo, Custom DLL, Vanilla, default = Latite).
    pub fn set_in_game_presence(&mut self, version_tag: &str, client_name: Option<&str>) {
        let Some(client) = self.client.as_mut() else {
            return;
        };
        if !self.running {
            return;
        }

        let state = match client_name {
            Some("Flarial Client") => "Using Flarial".to_string(),
            Some("OderSo Client") => {
                if version_tag.is_empty() {
                    "Using OderSo".to_string()
                } else {
                    format!("Using OderSo · {version_tag}")
                }
            }
            Some("Custom DLL") => {
                if version_tag.is_empty() {
                    "Using a custom DLL".to_string()
                } else {
                    format!("Using {version_tag}")
                }
            }
            Some("Vanilla") => "Playing Vanilla".to_string(),
            _ => {
                if version_tag.is_empty() {
                    "Using Latite".to_string()
                } else {
                    format!("Using Latite · {version_tag}")
                }
            }
        };

        let activity = Activity::new()
            .details("Playing Minecraft Bedrock")
            .state(&state)
            .assets(
                Assets::new()
                    .large_image("minecraft_icon")
                    .large_text("Minecraft Bedrock")
                    .small_image("glacier_logo")
                    .small_text("Glacier Launcher"),
            )
            .timestamps(Timestamps::new().start(now_unix()));

        if let Err(err) = client.set_activity(activity) {
            warn!("[Discord RPC] failed to set in-game presence: {err}");
        }
    }

    /// Sets the "Playing Minecraft Java" presence.
    ///
    /// `variant` mirrors the C# switch over Lunar / Badlion / default.
    pub fn set_java_in_game_presence(&mut self, version_id: &str, variant: Option<&str>) {
        let Some(client) = self.client.as_mut() else {
            return;
        };
        if !self.running {
            return;
        }

        let state = match variant {
            Some("Lunar") => {
                if version_id.is_empty() {
                    "Using Lunar Client".to_string()
                } else {
                    format!("Lunar · {version_id}")
                }
            }
            Some("Badlion") => {
                if version_id.is_empty() {
                    "Using Badlion".to_string()
                } else {
                    format!("Badlion · {version_id}")
                }
            }
            _ => {
                if version_id.is_empty() {
                    "In the menu".to_string()
                } else {
                    format!("Playing {version_id}")
                }
            }
        };

        let large_text = format!("Minecraft Java {version_id}");
        let large_text = large_text.trim_end();

        let activity = Activity::new()
            .details("Playing Minecraft Java")
            .state(&state)
            .assets(
                Assets::new()
                    .large_image("minecraft_java")
                    .large_text(large_text)
                    .small_image("glacier_logo")
                    .small_text("Glacier Launcher"),
            )
            .timestamps(Timestamps::new().start(now_unix()));

        if let Err(err) = client.set_activity(activity) {
            warn!("[Discord RPC] failed to set java in-game presence: {err}");
        }
    }
}

impl Drop for DiscordRpcService {
    fn drop(&mut self) {
        self.stop();
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
