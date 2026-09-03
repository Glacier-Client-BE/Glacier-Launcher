//! Bedrock `options.txt` reader/writer. Mirrors
//! `Services/BedrockOptionsService.cs`.
//!
//! Deviation: the C# `Save` refuses to write while Minecraft is running
//! (`GameLauncher.IsMinecraftRunning()`), since it would otherwise be
//! clobbered when the game exits. Process-liveness detection lives in a
//! different crate, so that guard isn't reproduced here — callers that have
//! access to such a check should perform it before calling [`BedrockOptionsService::save`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{IoContext, Result};
use crate::model::{OptionEntry, OptionKind};
use crate::util::write_atomic;

pub const VSYNC_KEY: &str = "gfx_vsync";

struct Meta {
    label: &'static str,
    group: &'static str,
    hint: &'static str,
}

const fn meta(label: &'static str, group: &'static str, hint: &'static str) -> Meta {
    Meta { label, group, hint }
}

fn known_keys() -> &'static [(&'static str, Meta)] {
    const KEYS: &[(&str, Meta)] = &[
        (
            "gfx_vsync",
            meta(
                "V-Sync",
                "Video",
                "Caps framerate to your monitor. Off can reduce input lag.",
            ),
        ),
        ("gfx_fullscreen", meta("Fullscreen", "Video", "")),
        (
            "gfx_viewdistance",
            meta(
                "Render distance",
                "Video",
                "Measured in blocks — 16 blocks = 1 chunk.",
            ),
        ),
        ("gfx_gamma", meta("Brightness", "Video", "")),
        (
            "gfx_particleviewdistance",
            meta("Particle render distance", "Video", ""),
        ),
        ("gfx_fancygraphics", meta("Fancy graphics", "Video", "")),
        ("gfx_fancyleaves", meta("Fancy leaves", "Video", "")),
        (
            "gfx_transparentleaves",
            meta("Transparent leaves", "Video", ""),
        ),
        ("gfx_smoothlighting", meta("Smooth lighting", "Video", "")),
        (
            "gfx_texel_aa_enabled",
            meta("Texel anti-aliasing", "Video", ""),
        ),
        (
            "gfx_multithreaded_renderer",
            meta(
                "Multithreaded renderer",
                "Video",
                "Usually a win on 4+ core CPUs.",
            ),
        ),
        ("gfx_msaa", meta("Anti-aliasing (MSAA)", "Video", "")),
        ("gfx_hidehand", meta("Hide hand", "Video", "")),
        ("gfx_hidepaperdoll", meta("Hide paper doll", "Video", "")),
        ("gfx_hidegui", meta("Hide GUI", "Video", "")),
        ("gfx_field_of_view", meta("Field of view", "Video", "")),
        ("gfx_fov", meta("Field of view", "Video", "")),
        ("gfx_gui_scale", meta("GUI scale", "Interface", "")),
        ("gui_scale", meta("GUI scale", "Interface", "")),
        (
            "gfx_gui_scale_offset",
            meta("GUI scale offset", "Interface", ""),
        ),
        (
            "gui_scale_offset",
            meta("GUI scale offset", "Interface", ""),
        ),
        (
            "ctrl_sensitivity",
            meta("Mouse sensitivity", "Controls", ""),
        ),
        ("ctrl_rawinput", meta("Raw input", "Controls", "")),
        ("ctrl_autojump", meta("Auto-jump", "Controls", "")),
        ("ctrl_invertmouse", meta("Invert mouse", "Controls", "")),
        ("audio_music", meta("Music volume", "Audio", "")),
        ("audio_sound", meta("Sound volume", "Audio", "")),
        ("dev_enable_debug_hud", meta("Debug HUD", "Advanced", "")),
        ("dev_showhitboxes", meta("Show hitboxes", "Advanced", "")),
        ("game_difficulty", meta("Difficulty", "Gameplay", "")),
        ("game_hidehud", meta("Hide HUD", "Gameplay", "")),
    ];
    KEYS
}

const GROUP_ORDER: &[&str] = &[
    "Video",
    "Interface",
    "Controls",
    "Audio",
    "Gameplay",
    "Advanced",
    "Other",
];

pub fn group_rank(group: &str) -> usize {
    GROUP_ORDER
        .iter()
        .position(|g| *g == group)
        .unwrap_or(GROUP_ORDER.len())
}

fn find_meta(key: &str) -> Option<&'static Meta> {
    known_keys()
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, m)| m)
}

pub struct BedrockOptionsService {
    options_path: PathBuf,
    backup_path: PathBuf,
}

impl BedrockOptionsService {
    pub fn new(options_path: impl Into<PathBuf>) -> Self {
        let options_path = options_path.into();
        let backup_path = append_ext(&options_path, "glacier.bak");
        Self {
            options_path,
            backup_path,
        }
    }

    pub fn options_path(&self) -> &Path {
        &self.options_path
    }

    pub fn exists(&self) -> bool {
        self.options_path.is_file()
    }

    pub fn has_backup(&self) -> bool {
        self.backup_path.is_file()
    }

    pub fn load(&self) -> Result<Vec<OptionEntry>> {
        let mut result = Vec::new();
        if !self.exists() {
            return Ok(result);
        }

        let text = match std::fs::read_to_string(&self.options_path) {
            Ok(t) => t,
            Err(_) => return Ok(result),
        };

        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some(sep) = line.find(':') else { continue };
            if sep == 0 {
                continue;
            }
            let key = line[..sep].trim();
            let value = line[sep + 1..].trim();
            if key.is_empty() {
                continue;
            }

            let meta = find_meta(key);
            result.push(OptionEntry {
                key: key.to_string(),
                value: value.to_string(),
                kind: infer_kind(key, value),
                label: meta
                    .map(|m| m.label.to_string())
                    .unwrap_or_else(|| prettify(key)),
                group: meta
                    .map(|m| m.group.to_string())
                    .unwrap_or_else(|| "Other".to_string()),
                hint: meta.map(|m| m.hint.to_string()).unwrap_or_default(),
                known: meta.is_some(),
            });
        }

        Ok(result)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .load()?
            .into_iter()
            .find(|e| e.key.eq_ignore_ascii_case(key))
            .map(|e| e.value))
    }

    pub fn is_vsync_enabled(&self) -> Result<bool> {
        Ok(matches!(
            self.get(VSYNC_KEY)?.as_deref(),
            Some("1") | Some("true") | Some("True")
        ))
    }

    /// Merges `changes` into `options.txt`, appending unknown keys, and
    /// takes a one-time `.glacier.bak` snapshot of the pre-change file (only
    /// if no backup already exists yet, matching the C# "first write wins"
    /// backup semantics).
    pub fn save(&self, changes: &HashMap<String, String>) -> Result<()> {
        if changes.is_empty() {
            return Ok(());
        }

        let dir = self
            .options_path
            .parent()
            .ok_or_else(|| crate::error::BedrockError::NotFound(self.options_path.clone()))?;
        if !dir.is_dir() {
            return Err(crate::error::BedrockError::NotFound(dir.to_path_buf()));
        }

        let mut existing: Vec<String> = if self.options_path.is_file() {
            std::fs::read_to_string(&self.options_path)
                .ctx(&self.options_path)?
                .lines()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        if self.options_path.is_file() && !self.backup_path.is_file() {
            let _ = std::fs::copy(&self.options_path, &self.backup_path);
        }

        // Track pending changes keyed by lowercased key so matching against
        // existing lines is case-insensitive, same as the C# `OrdinalIgnoreCase`
        // dictionary.
        let mut pending: HashMap<String, String> = changes
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.clone()))
            .collect();

        for line in existing.iter_mut() {
            let Some(sep) = line.find(':') else { continue };
            if sep == 0 {
                continue;
            }
            let key = line[..sep].trim().to_string();
            if let Some(new_value) = pending.remove(&key.to_lowercase()) {
                *line = format!("{key}:{new_value}");
            }
        }

        // Anything left in `pending` wasn't present in the file yet — append
        // it using the caller's original key casing.
        for (key, value) in changes {
            if pending.remove(&key.to_lowercase()).is_some() {
                existing.push(format!("{key}:{value}"));
            }
        }

        let mut content = existing.join("\r\n");
        content.push_str("\r\n");

        write_atomic(&self.options_path, content.as_bytes())
    }

    pub fn save_one(&self, key: &str, value: &str) -> Result<()> {
        let mut changes = HashMap::new();
        changes.insert(key.to_string(), value.to_string());
        self.save(&changes)
    }

    pub fn set_vsync(&self, enabled: bool) -> Result<()> {
        self.save_one(VSYNC_KEY, if enabled { "1" } else { "0" })
    }

    pub fn restore_backup(&self) -> Result<bool> {
        if !self.backup_path.is_file() {
            return Ok(false);
        }
        let content = std::fs::read(&self.backup_path).ctx(&self.backup_path)?;
        write_atomic(&self.options_path, &content)?;
        Ok(true)
    }
}

fn append_ext(path: &Path, ext: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".");
    s.push(ext);
    PathBuf::from(s)
}

fn infer_kind(key: &str, value: &str) -> OptionKind {
    if value == "0" || value == "1" {
        if let Some(m) = find_meta(key) {
            if matches!(
                m.group,
                "Video" | "Controls" | "Interface" | "Advanced" | "Gameplay"
            ) {
                let k = key.to_lowercase();
                if k.contains("distance")
                    || k.contains("scale")
                    || k.contains("gamma")
                    || k.contains("sensitiv")
                    || k.contains("msaa")
                    || k.contains("fov")
                    || k.contains("field_of_view")
                {
                    return OptionKind::Integer;
                }
            }
        }
        return OptionKind::Toggle;
    }
    if value.parse::<i64>().is_ok() {
        return OptionKind::Integer;
    }
    if value.parse::<f64>().is_ok() {
        return OptionKind::Decimal;
    }
    OptionKind::Text
}

fn prettify(key: &str) -> String {
    let s = key.replace('_', " ");
    let s = s.trim();
    if s.is_empty() {
        return key.to_string();
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
