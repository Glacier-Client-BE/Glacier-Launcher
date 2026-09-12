//! Launcher settings, theming, localization, stats, and notifications, built
//! on gl-persistence.
//!
//! `LauncherSettings` mirrors the C# `Models/LauncherSettings.cs` shape
//! field-for-field (including its default System.Text.Json PascalCase
//! property names) so the legacy flat `glacier-settings.json` file can be
//! read directly during migration, before being re-saved through
//! `gl_persistence`'s versioned envelope.

pub mod notifications;
pub mod themes;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use notifications::{AppNotification, NotificationService};
pub use themes::{ThemeDefinition, ThemeService};

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
const LEGACY_SETTINGS_FILE: &str = "glacier-settings.json";
const SETTINGS_FILE: &str = "settings.json";

/// `%USERPROFILE%\Glacier Launcher` - matches LauncherUtilityService.LauncherRoot.
pub fn launcher_root() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join("Glacier Launcher")
}

pub fn cache_dir() -> PathBuf {
    launcher_root().join("cache")
}

fn legacy_settings_path() -> PathBuf {
    launcher_root().join(LEGACY_SETTINGS_FILE)
}

fn settings_path() -> PathBuf {
    launcher_root().join(SETTINGS_FILE)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SavedServer {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub address: String,
    #[serde(default = "default_bedrock_port")]
    pub port: i32,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub icon_color: String,
}

fn default_bedrock_port() -> i32 {
    19132
}

/// A saved Microsoft/Xbox → Minecraft sign-in, mirroring `Models.JavaAccount`.
/// Lets the Settings panel switch between multiple signed-in accounts
/// without re-authenticating each time (`XboxProfileService.SwitchAccount`/
/// `PersistAccount`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct JavaAccount {
    /// Minecraft UUID when available, else the account's Xbox XUID — matches
    /// `PersistAccount`'s `id = string.IsNullOrWhiteSpace(mcProfile.Uuid) ?
    /// profile.Xuid : mcProfile.Uuid`.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub gamertag: String,
    #[serde(default)]
    pub xuid: String,
    #[serde(default)]
    pub gamer_picture_url: String,
    #[serde(default)]
    pub minecraft_access_token: String,
    #[serde(default)]
    pub minecraft_access_token_expiry: String,
    #[serde(default)]
    pub minecraft_skin_url: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub last_used_at: String,
}

// Field-by-field port of Models/LauncherSettings.cs. Defaults are wired via
// `#[serde(default)]` + `Default` below rather than the macro above, since
// per-field default-fn plumbing for ~90 fields is not worth a macro; a plain
// struct + Default impl is more readable and just as migration-safe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct LauncherSettings {
    pub edition: String,
    pub selected_client: String,
    pub discord_rich_presence: bool,
    pub last_used_version: String,
    pub username: String,
    pub user_handle: String,
    pub close_after_launch: bool,
    pub auto_inject: bool,
    pub injection_delay_ms: i32,

    pub accent_color: String,
    pub theme_preset: String,
    pub blur_intensity: i32,
    pub background_opacity: f64,
    pub custom_background_path: String,
    pub compact_mode: bool,
    pub animations_enabled: bool,
    pub animation_speed: f64,
    pub active_theme_id: String,
    pub font_family: String,
    pub ui_scale_pct: i32,
    pub background_fit: String,

    /// Which side of the window panels slide in from/anchor to: "right"
    /// (default, matches the original) or "left".
    pub panel_side: String,
    /// User-dragged panel width in px, from the resize handle beside the
    /// panel's edge. `0.0` means "use the CSS default" (`min(460px, 92vw)`).
    pub panel_width_px: f64,

    pub window_width: f64,
    pub window_height: f64,
    pub remember_window_size: bool,
    pub minimize_to_tray: bool,

    pub show_recently_launched: bool,
    pub recently_launched: Vec<String>,

    pub total_playtime_seconds: i64,
    pub last_played: String,

    pub pinned_versions: Vec<String>,

    pub version_sort_mode: String,
    pub show_only_downloaded: bool,

    pub active_vanilla_version: String,

    pub java_active_version: String,
    pub java_last_used_version: String,
    pub java_active_instance_id: String,

    pub java_use_dot_minecraft: bool,
    pub java_minecraft_dir: String,

    pub java_runtime_path: String,
    pub java_min_ram_mb: i32,
    pub java_max_ram_mb: i32,

    pub java_custom_jvm_args: String,
    pub java_backup_saves_before_launch: bool,
    pub java_fullscreen: bool,
    pub java_window_width: i32,
    pub java_window_height: i32,
    pub java_use_custom_resolution: bool,
    pub java_server_address: String,
    pub java_server_port: i32,

    pub show_launch_console: bool,

    pub java_show_snapshots: bool,
    pub java_show_historical: bool,

    pub java_username: String,
    pub java_uuid: String,
    pub java_access_token: String,

    pub java_offline_mode: bool,
    pub java_offline_username: String,
    pub java_access_token_expiry: String,
    pub java_skin_url: String,

    pub java_skin_slim_model: bool,

    pub skin_viewer_mode: String,
    pub skin_viewer_model: String,
    pub skin_viewer_cape_mode: String,

    pub skin_changed_ticks: i64,

    pub last_applied_skin_path: String,
    pub active_java_account_id: String,
    pub java_accounts: Vec<JavaAccount>,

    pub custom_dll_path: String,

    pub default_custom_client_id: String,

    pub accelerate_loading_ms: i32,

    pub prevent_game_suspend: bool,

    pub verify_injection: bool,

    pub curse_forge_api_key: String,

    pub check_updates_on_startup: bool,
    pub skipped_launcher_version: String,
    pub last_update_check: String,

    pub xbox_gamertag: String,
    pub xbox_xuid: String,
    pub xbox_gamer_picture_url: String,
    pub xbox_gamerscore: String,
    pub xbox_account_tier: String,
    pub xbox_bio: String,

    pub xbox_live_refresh_token: String,

    pub discord_logged_in: bool,
    pub discord_user_id: String,
    pub discord_username: String,
    pub discord_avatar: String,
    pub discord_token: String,

    pub profile_display_mode: String,

    pub saved_servers: Vec<SavedServer>,

    pub bedrock_active_instance_id: String,

    pub onboarding_completed: bool,

    pub accessibility_mode: bool,

    pub language: String,

    pub last_dismissed_announcement_id: String,

    /// Auto-backup-every-N-launches for Bedrock worlds (feature 20). `0`
    /// disables it. Global rather than per-world, matching the scope of the
    /// existing manual "Back up now" action (whole com.mojang snapshot).
    pub bedrock_auto_backup_every_n_launches: i32,
    /// Bedrock launches since the last auto-backup ran. Reset to 0 whenever
    /// an auto-backup fires (manual backups don't reset it).
    pub bedrock_launches_since_backup: i32,

    /// Feature 23: whether the user has dismissed the one-time marketplace
    /// content reminder banner on the Bedrock Worlds/Backups panels.
    pub bedrock_marketplace_reminder_dismissed: bool,

    /// UI polish feature 37: user-rebound keyboard shortcuts, serialized as a
    /// JSON object mapping action name (`"search"`, `"launch"`,
    /// `"fullscreen"`) to a normalized combo string (e.g. `"ctrl+k"`).
    /// `"{}"` (the default) means every action uses its hardcoded default
    /// combo. Kept as an opaque JSON string rather than a typed map since the
    /// set of rebindable actions is a frontend concern (`interop.js`), not
    /// something the Rust side interprets.
    pub keyboard_shortcuts: String,

    /// UI polish feature 42: whether the user has dismissed the first-run
    /// onboarding checklist card shown once after the setup wizard finishes.
    pub onboarding_checklist_dismissed: bool,

    /// Mods & content feature 17: starred/favorited mod project ids
    /// (Modrinth project id, or a CurseForge numeric id as a string) from the
    /// mod browse UI. Local-only, same persistence path as everything else.
    pub starred_mod_ids: Vec<String>,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            edition: "bedrock".into(),
            selected_client: "Latite Client".into(),
            discord_rich_presence: true,
            last_used_version: String::new(),
            username: String::new(),
            user_handle: String::new(),
            close_after_launch: false,
            auto_inject: false,
            injection_delay_ms: 2000,

            accent_color: "#7289da".into(),
            theme_preset: "dark".into(),
            // Defaults to off: backdrop-filter blur is expensive on low-end iGPUs
            // (e.g. Celeron N5100). Users on faster hardware can re-enable it in Settings.
            blur_intensity: 0,
            background_opacity: 0.80,
            custom_background_path: String::new(),
            compact_mode: false,
            animations_enabled: true,
            animation_speed: 1.0,
            active_theme_id: String::new(),
            font_family: String::new(),
            ui_scale_pct: 100,
            background_fit: "cover".into(),

            panel_side: "right".into(),
            panel_width_px: 0.0,

            window_width: 880.0,
            window_height: 600.0,
            remember_window_size: true,
            minimize_to_tray: false,

            show_recently_launched: true,
            recently_launched: Vec::new(),

            total_playtime_seconds: 0,
            last_played: String::new(),

            pinned_versions: Vec::new(),

            version_sort_mode: "newest".into(),
            show_only_downloaded: false,

            active_vanilla_version: String::new(),

            java_active_version: String::new(),
            java_last_used_version: String::new(),
            java_active_instance_id: String::new(),

            java_use_dot_minecraft: true,
            java_minecraft_dir: String::new(),

            java_runtime_path: String::new(),
            java_min_ram_mb: 512,
            java_max_ram_mb: 2048,

            java_custom_jvm_args: String::new(),
            java_backup_saves_before_launch: false,
            java_fullscreen: false,
            java_window_width: 854,
            java_window_height: 480,
            java_use_custom_resolution: false,
            java_server_address: String::new(),
            java_server_port: 25565,

            show_launch_console: true,

            java_show_snapshots: false,
            java_show_historical: false,

            java_username: String::new(),
            java_uuid: String::new(),
            java_access_token: String::new(),

            java_offline_mode: false,
            java_offline_username: "Player".into(),
            java_access_token_expiry: String::new(),
            java_skin_url: String::new(),

            java_skin_slim_model: false,

            skin_viewer_mode: "2d".into(),
            skin_viewer_model: "default".into(),
            skin_viewer_cape_mode: "cape".into(),

            skin_changed_ticks: 0,

            last_applied_skin_path: String::new(),
            active_java_account_id: String::new(),
            java_accounts: Vec::new(),

            custom_dll_path: String::new(),

            default_custom_client_id: String::new(),

            accelerate_loading_ms: 0,

            prevent_game_suspend: false,

            verify_injection: true,

            curse_forge_api_key: String::new(),

            check_updates_on_startup: true,
            skipped_launcher_version: String::new(),
            last_update_check: String::new(),

            xbox_gamertag: String::new(),
            xbox_xuid: String::new(),
            xbox_gamer_picture_url: String::new(),
            xbox_gamerscore: String::new(),
            xbox_account_tier: String::new(),
            xbox_bio: String::new(),

            xbox_live_refresh_token: String::new(),

            discord_logged_in: false,
            discord_user_id: String::new(),
            discord_username: String::new(),
            discord_avatar: String::new(),
            discord_token: String::new(),

            profile_display_mode: "auto".into(),

            saved_servers: Vec::new(),

            bedrock_active_instance_id: String::new(),

            onboarding_completed: false,

            accessibility_mode: false,

            language: "en".into(),

            last_dismissed_announcement_id: String::new(),

            bedrock_auto_backup_every_n_launches: 0,
            bedrock_launches_since_backup: 0,

            bedrock_marketplace_reminder_dismissed: false,

            keyboard_shortcuts: "{}".into(),
            onboarding_checklist_dismissed: false,

            starred_mod_ids: Vec::new(),
        }
    }
}

/// Load settings, migrating from the legacy flat (no-envelope, PascalCase)
/// `glacier-settings.json` on first run if the new envelope-based
/// `settings.json` doesn't exist yet.
pub fn load() -> gl_persistence::Result<LauncherSettings> {
    if let Some(envelope) = gl_persistence::load::<LauncherSettings>(&settings_path())? {
        return Ok(envelope.data);
    }

    // No new-format file yet - try the legacy flat file written by the
    // C# JsonStore-backed SettingsService.
    let legacy_path = legacy_settings_path();
    if legacy_path.exists() {
        let contents = std::fs::read_to_string(&legacy_path)?;
        match serde_json::from_str::<LauncherSettings>(&contents) {
            Ok(settings) => {
                // Persist immediately in the new envelope format; never
                // touch/delete the original legacy file.
                save(&settings)?;
                return Ok(settings);
            }
            Err(e) => {
                gl_persistence::quarantine_corrupt(&legacy_path);
                return Err(e.into());
            }
        }
    }

    Ok(LauncherSettings::default())
}

pub fn save(settings: &LauncherSettings) -> gl_persistence::Result<()> {
    gl_persistence::save(&settings_path(), SETTINGS_SCHEMA_VERSION, settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_round_trip_through_persistence() {
        let dir = std::env::temp_dir().join(format!("gl-settings-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        let settings = LauncherSettings::default();
        gl_persistence::save(&path, SETTINGS_SCHEMA_VERSION, &settings).unwrap();
        let loaded = gl_persistence::load::<LauncherSettings>(&path)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.data, settings);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn legacy_flat_pascal_case_json_deserializes() {
        let legacy_json = r#"{
            "Edition": "java",
            "SelectedClient": "Custom",
            "DiscordRichPresence": false,
            "InjectionDelayMs": 5000,
            "WindowWidth": 1024.0,
            "AnimationSpeed": 2.5,
            "RecentlyLaunched": ["1.20.1", "1.19.4"]
        }"#;

        let settings: LauncherSettings = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(settings.edition, "java");
        assert_eq!(settings.selected_client, "Custom");
        assert!(!settings.discord_rich_presence);
        assert_eq!(settings.injection_delay_ms, 5000);
        assert_eq!(settings.window_width, 1024.0);
        assert_eq!(settings.recently_launched, vec!["1.20.1", "1.19.4"]);
        // Fields absent from the legacy JSON fall back to defaults.
        assert_eq!(settings.accent_color, "#7289da");
    }
}
