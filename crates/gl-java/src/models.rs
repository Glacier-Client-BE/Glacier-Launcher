use serde::{Deserialize, Serialize};

/// A single Java Edition version as reported by the Mojang piston-meta manifest,
/// annotated with local install-state flags. Mirrors the C# `JavaVersion` model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct JavaVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub release_time: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub compliance_level: i32,

    // Not present in Mojang's manifest JSON (hence `default` for
    // deserializing it), but very much needed by frontend consumers of this
    // struct over Tauri IPC, so these are NOT skip_serializing.
    #[serde(default)]
    pub is_installed: bool,
    #[serde(default)]
    pub has_jar: bool,
    #[serde(default)]
    pub is_active: bool,
}

impl JavaVersion {
    /// Human label mirroring the C# `TypeLabel` computed property.
    pub fn type_label(&self) -> &str {
        match self.kind.as_str() {
            "release" => "Release",
            "snapshot" => "Snapshot",
            "old_beta" => "Beta",
            "old_alpha" => "Alpha",
            other => other,
        }
    }
}

/// A managed Java Edition install instance (a `.minecraft`-shaped directory tree).
/// Mirrors the C# `JavaInstance` model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct JavaInstance {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub directory: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub max_ram_mb: i32,
    #[serde(default)]
    pub min_ram_mb: i32,
    /// Extra JVM args applied at launch (in addition to the global custom
    /// args), populated either from a built-in template chosen at instance
    /// creation (see [`crate::presets`]) or a later per-instance JVM-args
    /// preset change. `#[serde(default)]` so instances persisted before this
    /// field existed keep deserializing.
    #[serde(default)]
    pub extra_jvm_args: Vec<String>,
    /// Name of the JVM-args preset currently applied (`"vanilla"`,
    /// `"balanced"`, `"performance"`, `"low-end"`), if any — purely
    /// informational for the UI; `extra_jvm_args` is what launch actually
    /// reads. `None` means "custom" (user-edited args that don't match a
    /// preset, or never had one applied).
    #[serde(default)]
    pub jvm_args_preset: Option<String>,
    /// Optional group/folder name for organizing instances in the UI.
    #[serde(default)]
    pub group: Option<String>,
    /// Optional per-instance accent hex color (e.g. "#7289da") shown as a
    /// small dot/border on the instance card. `None` falls back to the
    /// global theme accent. `#[serde(default)]` so instances persisted
    /// before this field existed keep deserializing.
    #[serde(default)]
    pub accent_color: Option<String>,
    /// Configured mod loader, if any. `None`/`Vanilla` both mean "no loader".
    #[serde(default)]
    pub loader: Option<ModLoaderKind>,
    #[serde(default)]
    pub loader_version: Option<String>,
    /// The `versions/<id>` directory name the loader installer produced
    /// (distinct from `version_id`, which stays the vanilla base version).
    #[serde(default)]
    pub loader_profile_id: Option<String>,
    /// Per-instance overrides for launch configuration. See
    /// [`InstanceSettings`] and `crate::effective::effective`.
    #[serde(default)]
    pub settings: InstanceSettings,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub banner_path: Option<String>,
    /// Whether this instance shares the common assets/libraries cache
    /// (`InstanceService::shared_assets_dir`/`shared_libraries_dir`).
    /// Defaults to `true`, matching today's behaviour for every existing
    /// instance.
    #[serde(default = "default_true")]
    pub use_shared_minecraft_folder: bool,
    /// Optional bound `gl_settings::JavaAccount` id (opaque here — gl-java
    /// has no dependency on gl-settings). `None` falls back to the global
    /// active account.
    #[serde(default)]
    pub preferred_account_id: Option<String>,
    #[serde(default)]
    pub playtime_seconds: i64,
    /// Sync-pack ids applied to this instance (Phase 7 / gl-sync).
    #[serde(default)]
    pub sync_pack_ids: Vec<String>,
}

/// Which mod loader (if any) an instance is configured for.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModLoaderKind {
    #[default]
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl ModLoaderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModLoaderKind::Vanilla => "vanilla",
            ModLoaderKind::Fabric => "fabric",
            ModLoaderKind::Quilt => "quilt",
            ModLoaderKind::Forge => "forge",
            ModLoaderKind::NeoForge => "neoforge",
        }
    }
}

fn default_true() -> bool {
    true
}

/// Per-instance overrides for launch configuration. Every field is
/// `Option`/empty-by-default, so an instance with no overrides behaves
/// exactly as it did before this struct existed — the global
/// `LauncherSettings` values (or the instance's legacy `max_ram_mb`/
/// `min_ram_mb`/`extra_jvm_args` fields) remain the fallback. See
/// `crate::effective::effective`, the single place these are resolved.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InstanceSettings {
    /// Explicit `javaw.exe`/`java.exe` path for this instance only.
    #[serde(default)]
    pub java_path: Option<String>,
    /// `(min_mb, max_mb)` heap override. Distinct from the legacy
    /// `JavaInstance::min_ram_mb`/`max_ram_mb` fields (kept for
    /// backward-compatible reads); new writes should go through this.
    #[serde(default)]
    pub memory: Option<(i32, i32)>,
    /// `(width, height)` window resolution override.
    #[serde(default)]
    pub resolution: Option<(i32, i32)>,
    #[serde(default)]
    pub use_custom_resolution: Option<bool>,
    #[serde(default)]
    pub fullscreen: Option<bool>,
    /// Additional raw game args, appended (not a replacement for the
    /// instance's `extra_jvm_args`, which are JVM-side).
    #[serde(default)]
    pub extra_game_args: Vec<String>,
    #[serde(default)]
    pub custom_jvm_args: Option<String>,
    /// Reserved for a future quick-play launch target (world/server);
    /// stored but not yet consumed by `gl_java::launcher::build_game_args`.
    #[serde(default)]
    pub quick_play: Option<String>,
}
