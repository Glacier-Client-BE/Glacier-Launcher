//! Resolves a [`JavaInstance`]'s effective launch configuration by layering
//! its per-instance overrides ([`InstanceSettings`], and the legacy
//! `max_ram_mb`/`min_ram_mb`/`extra_jvm_args` fields) on top of the global
//! defaults. This is the single seam that lets the instance-vs-global
//! configuration models coexist: every launch path should build its
//! `gl_java::launcher::LaunchOptions` from [`effective`]'s result rather than
//! reading `LauncherSettings` fields directly.

use crate::launcher::LaunchOptions;
use crate::models::JavaInstance;

/// The subset of `gl_settings::LauncherSettings` that feeds Java launch
/// configuration. Defined here (rather than depending on `gl-settings`
/// directly) so this crate stays settings-agnostic; the Tauri command layer
/// constructs this from the real settings struct.
#[derive(Debug, Clone, Default)]
pub struct GlobalJavaDefaults {
    pub max_ram_mb: i32,
    pub min_ram_mb: i32,
    pub custom_jvm_args: String,
    pub fullscreen: bool,
    pub use_custom_resolution: bool,
    pub window_width: i32,
    pub window_height: i32,
    pub server_address: String,
    pub server_port: i32,
    pub java_runtime_path: String,
}

/// A resolved launch configuration for one instance, plus which fields came
/// from the instance rather than the global defaults (purely informational,
/// for a UI badge like "4096 MB — from default").
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct EffectiveLaunchConfig {
    pub max_ram_mb: i32,
    pub min_ram_mb: i32,
    pub custom_jvm_args: String,
    pub extra_game_args: Vec<String>,
    pub fullscreen: bool,
    pub use_custom_resolution: bool,
    pub window_width: i32,
    pub window_height: i32,
    pub server_address: String,
    pub server_port: i32,
    pub java_path: Option<String>,
    pub quick_play: Option<String>,
    /// Names of fields resolved from the instance's own settings/legacy
    /// fields rather than inherited from `GlobalJavaDefaults`.
    pub overridden: Vec<&'static str>,
}

/// Resolve `instance`'s effective launch configuration against `defaults`.
/// Precedence per field: `instance.settings.<field>` (explicit override) >
/// legacy `instance.max_ram_mb`/`min_ram_mb` (RAM only, when nonzero) >
/// `defaults`.
pub fn effective(instance: &JavaInstance, defaults: &GlobalJavaDefaults) -> EffectiveLaunchConfig {
    let mut overridden = Vec::new();
    let s = &instance.settings;

    let (min_ram_mb, max_ram_mb) = if let Some((min, max)) = s.memory {
        overridden.push("memory");
        (min, max)
    } else if instance.max_ram_mb > 0 || instance.min_ram_mb > 0 {
        overridden.push("memory");
        (instance.min_ram_mb, instance.max_ram_mb)
    } else {
        (defaults.min_ram_mb, defaults.max_ram_mb)
    };

    let (use_custom_resolution, window_width, window_height) =
        if let Some((width, height)) = s.resolution {
            overridden.push("resolution");
            (s.use_custom_resolution.unwrap_or(true), width, height)
        } else if let Some(explicit) = s.use_custom_resolution {
            overridden.push("resolution");
            (explicit, defaults.window_width, defaults.window_height)
        } else {
            (
                defaults.use_custom_resolution,
                defaults.window_width,
                defaults.window_height,
            )
        };

    let fullscreen = if let Some(fs) = s.fullscreen {
        overridden.push("fullscreen");
        fs
    } else {
        defaults.fullscreen
    };

    let custom_jvm_args = if let Some(args) = &s.custom_jvm_args {
        overridden.push("custom_jvm_args");
        args.clone()
    } else {
        defaults.custom_jvm_args.clone()
    };

    let java_path = if let Some(path) = &s.java_path {
        overridden.push("java_path");
        Some(path.clone())
    } else if defaults.java_runtime_path.is_empty() {
        None
    } else {
        Some(defaults.java_runtime_path.clone())
    };

    EffectiveLaunchConfig {
        max_ram_mb,
        min_ram_mb,
        custom_jvm_args,
        extra_game_args: s.extra_game_args.clone(),
        fullscreen,
        use_custom_resolution,
        window_width,
        window_height,
        server_address: defaults.server_address.clone(),
        server_port: defaults.server_port,
        java_path,
        quick_play: s.quick_play.clone(),
        overridden,
    }
}

impl EffectiveLaunchConfig {
    /// Build a `LaunchOptions` from this resolved config, filling in the
    /// remaining fields (`extra_jvm_args`, `auth_xuid`) the caller already
    /// has to hand.
    pub fn into_launch_options(
        self,
        extra_jvm_args: Vec<String>,
        auth_xuid: String,
    ) -> LaunchOptions {
        LaunchOptions {
            max_ram_mb: self.max_ram_mb,
            min_ram_mb: self.min_ram_mb,
            custom_jvm_args: self.custom_jvm_args,
            extra_jvm_args,
            fullscreen: self.fullscreen,
            use_custom_resolution: self.use_custom_resolution,
            window_width: self.window_width,
            window_height: self.window_height,
            server_address: self.server_address,
            server_port: self.server_port,
            auth_xuid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::InstanceSettings;

    fn instance_with_settings(settings: InstanceSettings) -> JavaInstance {
        JavaInstance {
            id: "test".to_string(),
            settings,
            ..Default::default()
        }
    }

    #[test]
    fn all_none_falls_back_to_defaults() {
        let instance = JavaInstance::default();
        let defaults = GlobalJavaDefaults {
            max_ram_mb: 4096,
            min_ram_mb: 1024,
            fullscreen: false,
            window_width: 854,
            window_height: 480,
            ..Default::default()
        };
        let cfg = effective(&instance, &defaults);
        assert_eq!(cfg.max_ram_mb, 4096);
        assert_eq!(cfg.min_ram_mb, 1024);
        assert!(cfg.overridden.is_empty());
    }

    #[test]
    fn explicit_memory_override_wins() {
        let instance = instance_with_settings(InstanceSettings {
            memory: Some((2048, 8192)),
            ..Default::default()
        });
        let defaults = GlobalJavaDefaults {
            max_ram_mb: 4096,
            min_ram_mb: 1024,
            ..Default::default()
        };
        let cfg = effective(&instance, &defaults);
        assert_eq!(cfg.min_ram_mb, 2048);
        assert_eq!(cfg.max_ram_mb, 8192);
        assert!(cfg.overridden.contains(&"memory"));
    }

    #[test]
    fn legacy_ram_fields_still_override_when_no_new_settings() {
        let instance = JavaInstance {
            max_ram_mb: 6144,
            min_ram_mb: 2048,
            ..Default::default()
        };
        let defaults = GlobalJavaDefaults {
            max_ram_mb: 4096,
            min_ram_mb: 1024,
            ..Default::default()
        };
        let cfg = effective(&instance, &defaults);
        assert_eq!(cfg.max_ram_mb, 6144);
        assert_eq!(cfg.min_ram_mb, 2048);
    }

    #[test]
    fn java_path_override() {
        let instance = instance_with_settings(InstanceSettings {
            java_path: Some("C:/custom/javaw.exe".to_string()),
            ..Default::default()
        });
        let defaults = GlobalJavaDefaults {
            java_runtime_path: "C:/global/javaw.exe".to_string(),
            ..Default::default()
        };
        let cfg = effective(&instance, &defaults);
        assert_eq!(cfg.java_path.as_deref(), Some("C:/custom/javaw.exe"));
    }
}
