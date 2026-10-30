//! Custom theme storage and CSS-variable generation - port of
//! `Services/ThemeService.cs` (193 lines) and `Models/ThemeDefinition.cs`
//! (118 lines).
//!
//! Neither had any counterpart in the rewrite, which is why the Theme Studio
//! panel could not exist and the Settings panel's two Theme Studio entry
//! points navigated nowhere.
//!
//! # File format
//!
//! `%USERPROFILE%\Glacier Launcher\themes.json`, a bare JSON array with
//! PascalCase keys - byte-compatible with what `System.Text.Json` wrote, so
//! an existing install's saved themes load unchanged. As with the
//! notification feed, this uses `gl_persistence`'s atomic writer directly
//! rather than `save()`, whose versioned envelope would change the shape on
//! disk.
//!
//! Applying a theme is a frontend concern (it calls `setTheme`,
//! `setThemeVars`, `setFont`, ... in `interop.js`), so the C# `ApplyAsync` /
//! `ClearAsync` methods have no Rust counterpart by design; this module owns
//! persistence and the CSS-variable computation those calls consume.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// One saved theme. Field names match the C# `ThemeDefinition` properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ThemeDefinition {
    pub schema_version: i32,
    pub id: String,
    pub name: String,
    pub base_preset: String,

    pub accent: String,
    pub bg: String,
    pub bg_panel: String,
    pub text: String,
    pub text_dim: String,
    pub red: String,
    pub green: String,
    pub orange: String,

    pub radius_sm: i32,
    pub radius_md: i32,
    pub blur: i32,

    pub overlay_strength: f64,

    pub font_family: String,
    pub animation_speed: f64,

    pub background_image: String,
    pub background_fit: String,
    pub background_opacity: f64,

    pub custom_css: String,
}

impl Default for ThemeDefinition {
    fn default() -> Self {
        Self {
            schema_version: 1,
            id: new_id(),
            name: "Custom Theme".into(),
            base_preset: "dark".into(),
            accent: "#7289da".into(),
            bg: "#23272a".into(),
            bg_panel: "#2c2f33".into(),
            text: "#ffffff".into(),
            text_dim: "#99aab5".into(),
            red: "#ff5c5c".into(),
            green: "#43b581".into(),
            orange: "#faa61a".into(),
            radius_sm: 8,
            radius_md: 12,
            blur: 14,
            overlay_strength: 1.0,
            font_family: String::new(),
            animation_speed: 1.0,
            background_image: String::new(),
            background_fit: "cover".into(),
            background_opacity: 1.0,
            custom_css: String::new(),
        }
    }
}

/// 32-char lowercase hex, matching `Guid.NewGuid().ToString("N")`.
fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:016x}{:016x}", nanos as u64, seq ^ (nanos >> 64) as u64)
}

/// `.ToString("0.###")` - up to 3 decimals, trailing zeros trimmed.
fn fmt3(v: f64) -> String {
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Port of `ThemeDefinition.ParseColor`. Accepts `#rgb`, `#rrggbb`,
/// `#rrggbbaa` and `rgb()`/`rgba()`. Any parse failure falls back to
/// `(114, 137, 218, 1.0)`, exactly as the C# `catch` does - a bad colour
/// must never break rendering.
pub fn parse_color(color: &str) -> (i32, i32, i32, f64) {
    const FALLBACK: (i32, i32, i32, f64) = (114, 137, 218, 1.0);
    let s = color.trim();

    if s.len() >= 3 && s[..3].eq_ignore_ascii_case("rgb") {
        let (Some(open), Some(close)) = (s.find('('), s.find(')')) else {
            return FALLBACK;
        };
        if close <= open {
            return FALLBACK;
        }
        let parts: Vec<&str> = s[open + 1..close].split(',').map(str::trim).collect();
        if parts.len() < 3 {
            return FALLBACK;
        }
        let (Ok(r), Ok(g), Ok(b)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<i32>(),
            parts[2].parse::<i32>(),
        ) else {
            return FALLBACK;
        };
        let a = if parts.len() > 3 {
            parts[3].parse::<f64>().unwrap_or(1.0)
        } else {
            1.0
        };
        return (r, g, b, a);
    }

    let mut h = s.trim_start_matches('#').to_string();
    if h.len() == 3 {
        let c: Vec<char> = h.chars().collect();
        h = format!("{0}{0}{1}{1}{2}{2}", c[0], c[1], c[2]);
    }
    if h.len() < 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return FALLBACK;
    }
    let (Ok(r), Ok(g), Ok(b)) = (
        i32::from_str_radix(&h[0..2], 16),
        i32::from_str_radix(&h[2..4], 16),
        i32::from_str_radix(&h[4..6], 16),
    ) else {
        return FALLBACK;
    };
    let a = if h.len() >= 8 {
        i32::from_str_radix(&h[6..8], 16)
            .map(|v| v as f64 / 255.0)
            .unwrap_or(1.0)
    } else {
        1.0
    };
    (r, g, b, a)
}

/// Port of `ThemeDefinition.Lighten`: moves each channel toward 255 by
/// `amount`. Returns hex when opaque, `rgba()` otherwise.
pub fn lighten(color: &str, amount: f64) -> String {
    let (r, g, b, a) = parse_color(color);
    let l = |c: i32| ((c as f64 + (255.0 - c as f64) * amount) as i32).clamp(0, 255);
    let (lr, lg, lb) = (l(r), l(g), l(b));
    if a >= 0.999 {
        format!("#{lr:02x}{lg:02x}{lb:02x}")
    } else {
        format!("rgba({lr},{lg},{lb},{})", fmt3(a))
    }
}

impl ThemeDefinition {
    /// Port of `BuildCssVars`. The map feeds `interop.js`'s `setThemeVars`.
    ///
    /// A `BTreeMap` is used so the output order is stable - the C# dictionary
    /// order was incidental, and stability makes the result diffable.
    pub fn build_css_vars(&self) -> BTreeMap<String, String> {
        let (ar, ag, ab, _) = parse_color(&self.accent);
        let (br, bg2, bb, _) = parse_color(&self.bg);
        let (tr, tg, tb, _) = parse_color(&self.text);
        let s = self.overlay_strength.clamp(0.0, 1.0);
        // Light text sits on a dark ground, so item overlays are subtler.
        let light_text = (tr + tg + tb) as f64 / 3.0 > 128.0;

        let mut m = BTreeMap::new();
        m.insert("--accent".into(), self.accent.clone());
        m.insert("--accent-hover".into(), lighten(&self.accent, 0.12));
        m.insert("--accent-glow".into(), format!("rgba({ar},{ag},{ab},0.42)"));
        m.insert("--accent-bg".into(), format!("rgba({ar},{ag},{ab},0.10)"));
        m.insert("--bg".into(), self.bg.clone());
        m.insert("--bg-panel".into(), self.bg_panel.clone());
        m.insert(
            "--bg-item".into(),
            format!(
                "rgba({tr},{tg},{tb},{})",
                if light_text { "0.04" } else { "0.05" }
            ),
        );
        m.insert(
            "--bg-item-hover".into(),
            format!(
                "rgba({tr},{tg},{tb},{})",
                if light_text { "0.075" } else { "0.09" }
            ),
        );
        m.insert("--text".into(), self.text.clone());
        m.insert("--text-dim".into(), self.text_dim.clone());
        m.insert("--red".into(), self.red.clone());
        m.insert("--green".into(), self.green.clone());
        m.insert("--orange".into(), self.orange.clone());
        m.insert("--r-sm".into(), format!("{}px", self.radius_sm));
        m.insert("--r-md".into(), format!("{}px", self.radius_md));
        m.insert(
            "--overlay-top".into(),
            format!("rgba({br},{bg2},{bb},{})", fmt3(0.55 * s)),
        );
        m.insert(
            "--overlay-mid".into(),
            format!("rgba({br},{bg2},{bb},{})", fmt3(0.20 * s)),
        );
        m.insert(
            "--overlay-bot".into(),
            format!("rgba({br},{bg2},{bb},{})", fmt3(0.85 * s)),
        );
        m
    }

    /// Port of `Clone()`: a copy with a fresh id.
    pub fn duplicated(&self) -> Self {
        Self {
            id: new_id(),
            ..self.clone()
        }
    }
}

/// One entry of the C# `PresetSeeds` table.
pub struct PresetSeed {
    pub preset: &'static str,
    pub label: &'static str,
    pub bg: &'static str,
    pub bg_panel: &'static str,
    pub text: &'static str,
    pub text_dim: &'static str,
}

/// Verbatim copy of `ThemeService.PresetSeeds`.
pub const PRESET_SEEDS: &[PresetSeed] = &[
    PresetSeed {
        preset: "dark",
        label: "Dark",
        bg: "#0a0b14",
        bg_panel: "#121526",
        text: "#eaecf6",
        text_dim: "#9aa1b8",
    },
    PresetSeed {
        preset: "legacy",
        label: "Legacy",
        bg: "#23272a",
        bg_panel: "#2c2f33",
        text: "#ffffff",
        text_dim: "#99aab5",
    },
    PresetSeed {
        preset: "darker",
        label: "Darker",
        bg: "#1e2124",
        bg_panel: "#26292c",
        text: "#ffffff",
        text_dim: "#99aab5",
    },
    PresetSeed {
        preset: "midnight",
        label: "Midnight",
        bg: "#141520",
        bg_panel: "#1c1e30",
        text: "#ffffff",
        text_dim: "#99aab5",
    },
    PresetSeed {
        preset: "slate",
        label: "Slate",
        bg: "#2a2d35",
        bg_panel: "#33373f",
        text: "#ffffff",
        text_dim: "#99aab5",
    },
    PresetSeed {
        preset: "ocean",
        label: "Ocean",
        bg: "#0e1f2a",
        bg_panel: "#13293a",
        text: "#ffffff",
        text_dim: "#99aab5",
    },
    PresetSeed {
        preset: "forest",
        label: "Forest",
        bg: "#16241a",
        bg_panel: "#1d2f22",
        text: "#ffffff",
        text_dim: "#99aab5",
    },
    PresetSeed {
        preset: "sunset",
        label: "Sunset",
        bg: "#2a1a26",
        bg_panel: "#33212f",
        text: "#ffffff",
        text_dim: "#99aab5",
    },
    PresetSeed {
        preset: "light",
        label: "Light",
        bg: "#f5f6f8",
        bg_panel: "#ffffff",
        text: "#0e1116",
        text_dim: "#5b6470",
    },
];

/// Port of `ThemeService.SeedFrom`: a new theme seeded from a base preset.
/// An unknown preset falls back to the first seed (Dark), as in the original.
pub fn seed_from(preset: &str, accent: &str) -> ThemeDefinition {
    let seed = PRESET_SEEDS
        .iter()
        .find(|p| p.preset == preset)
        .unwrap_or(&PRESET_SEEDS[0]);
    ThemeDefinition {
        name: format!("{} Custom", seed.label),
        base_preset: seed.preset.to_string(),
        accent: if accent.is_empty() {
            "#7289da".into()
        } else {
            accent.to_string()
        },
        bg: seed.bg.into(),
        bg_panel: seed.bg_panel.into(),
        text: seed.text.into(),
        text_dim: seed.text_dim.into(),
        ..Default::default()
    }
}

pub fn themes_path() -> PathBuf {
    crate::launcher_root().join("themes.json")
}

pub fn exports_dir() -> PathBuf {
    crate::launcher_root().join("themes").join("exports")
}

/// Saved custom themes. Mutations persist immediately; as in the original,
/// persistence failures are swallowed rather than surfaced.
#[derive(Debug, Default)]
pub struct ThemeService {
    themes: Mutex<Vec<ThemeDefinition>>,
}

impl ThemeService {
    /// Loads `themes.json`, quarantining it if it is present but unparseable
    /// (the C# `JsonException` arm) so a corrupt file is preserved for
    /// inspection rather than silently overwritten on the next save.
    pub fn new() -> Self {
        let path = themes_path();
        let themes = match std::fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Vec<ThemeDefinition>>(&json) {
                Ok(list) => list,
                Err(_) => {
                    gl_persistence::quarantine_corrupt(&path);
                    Vec::new()
                }
            },
            Err(_) => Vec::new(),
        };
        Self {
            themes: Mutex::new(themes),
        }
    }

    fn with<T>(&self, f: impl FnOnce(&mut Vec<ThemeDefinition>) -> T) -> T {
        let mut guard = match self.themes.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        f(&mut guard)
    }

    pub fn themes(&self) -> Vec<ThemeDefinition> {
        self.with(|t| t.clone())
    }

    pub fn get(&self, id: &str) -> Option<ThemeDefinition> {
        if id.is_empty() {
            return None;
        }
        self.with(|t| t.iter().find(|x| x.id == id).cloned())
    }

    pub fn add(&self, theme: ThemeDefinition) -> ThemeDefinition {
        self.with(|t| {
            t.push(theme.clone());
            save(t);
        });
        theme
    }

    /// Updates an existing theme in place. (The C# panel mutated the object
    /// it held a reference to and called `Save()`; there is no such shared
    /// reference across the IPC boundary, so the update is explicit.)
    pub fn update(&self, theme: ThemeDefinition) -> bool {
        self.with(|t| {
            let Some(slot) = t.iter_mut().find(|x| x.id == theme.id) else {
                return false;
            };
            *slot = theme;
            save(t);
            true
        })
    }

    /// Port of `Duplicate`: a copy with a fresh id and " Copy" appended.
    pub fn duplicate(&self, id: &str) -> Option<ThemeDefinition> {
        let src = self.get(id)?;
        let mut copy = src.duplicated();
        copy.name = format!("{} Copy", src.name);
        Some(self.add(copy))
    }

    pub fn delete(&self, id: &str) {
        self.with(|t| {
            t.retain(|x| x.id != id);
            save(t);
        });
    }

    /// Port of `Export`: writes `<slug>.glaciertheme.json` into the exports
    /// folder, where the slug replaces every non-alphanumeric character with
    /// `-` and trims leading/trailing dashes.
    pub fn export(&self, theme: &ThemeDefinition) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(exports_dir())?;
        let slug: String = theme
            .name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        let slug = slug.trim_matches('-').to_string();
        let slug = if slug.is_empty() {
            "theme".to_string()
        } else {
            slug
        };
        let path = exports_dir().join(format!("{slug}.glaciertheme.json"));
        let json = serde_json::to_string_pretty(theme)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        gl_persistence::write_atomic(&path, &json)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(path)
    }

    /// Port of `Import`, including its sanitisation: a fresh id, an unknown
    /// base preset reset to "dark", every numeric field clamped, and the
    /// background image cleared (an imported theme must not point at a file
    /// path from someone else's machine).
    pub fn import(&self, file_path: &Path) -> Option<ThemeDefinition> {
        let json = std::fs::read_to_string(file_path).ok()?;
        let mut theme: ThemeDefinition = serde_json::from_str(&json).ok()?;
        if theme.name.trim().is_empty() {
            return None;
        }

        theme.id = new_id();
        if !PRESET_SEEDS.iter().any(|p| p.preset == theme.base_preset) {
            theme.base_preset = "dark".into();
        }
        theme.blur = theme.blur.clamp(0, 30);
        theme.radius_sm = theme.radius_sm.clamp(0, 24);
        theme.radius_md = theme.radius_md.clamp(0, 32);
        theme.animation_speed = theme.animation_speed.clamp(0.25, 2.0);
        theme.background_image = String::new();

        Some(self.add(theme))
    }

    /// Port of `StashBackgroundAsync`: copies a chosen background next to the
    /// launcher's data so the theme keeps working if the original file moves.
    /// Refuses anything over 20 MB, as the original does.
    pub fn stash_background(&self, source: &Path, theme_id: &str) -> Option<PathBuf> {
        let meta = std::fs::metadata(source).ok()?;
        if !meta.is_file() || meta.len() > 20 * 1024 * 1024 {
            return None;
        }
        let ext = source
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let name = if ext.is_empty() {
            format!("theme-bg-{theme_id}")
        } else {
            format!("theme-bg-{theme_id}.{ext}")
        };
        let dest = crate::launcher_root().join(name);
        std::fs::copy(source, &dest).ok()?;
        Some(dest)
    }
}

fn save(themes: &[ThemeDefinition]) {
    let path = themes_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(themes) {
        let _ = gl_persistence::write_atomic(&path, &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_six_digit_hex() {
        assert_eq!(parse_color("#7289da"), (114, 137, 218, 1.0));
    }

    #[test]
    fn expands_three_digit_hex() {
        assert_eq!(parse_color("#abc"), (0xaa, 0xbb, 0xcc, 1.0));
    }

    #[test]
    fn parses_eight_digit_hex_alpha() {
        let (r, g, b, a) = parse_color("#00000080");
        assert_eq!((r, g, b), (0, 0, 0));
        assert!((a - 128.0 / 255.0).abs() < 1e-9);
    }

    #[test]
    fn parses_rgb_and_rgba() {
        assert_eq!(parse_color("rgb(1, 2, 3)"), (1, 2, 3, 1.0));
        assert_eq!(parse_color("rgba(1,2,3,0.5)"), (1, 2, 3, 0.5));
    }

    /// The C# implementation swallows every parse error and returns the
    /// default accent; a malformed colour must not break theming.
    #[test]
    fn falls_back_on_garbage() {
        assert_eq!(parse_color("not-a-color"), (114, 137, 218, 1.0));
        assert_eq!(parse_color(""), (114, 137, 218, 1.0));
        assert_eq!(parse_color("#12"), (114, 137, 218, 1.0));
    }

    #[test]
    fn lighten_returns_hex_when_opaque_and_rgba_otherwise() {
        assert_eq!(lighten("#000000", 0.5), "#7f7f7f");
        assert_eq!(lighten("rgba(0,0,0,0.5)", 0.5), "rgba(127,127,127,0.5)");
    }

    #[test]
    fn number_formatting_matches_dotnet_0_hash_hash_hash() {
        assert_eq!(fmt3(0.55), "0.55");
        assert_eq!(fmt3(0.2), "0.2");
        assert_eq!(fmt3(1.0), "1");
        assert_eq!(fmt3(0.0), "0");
    }

    #[test]
    fn css_vars_cover_every_variable_the_original_emits() {
        let vars = ThemeDefinition::default().build_css_vars();
        for key in [
            "--accent",
            "--accent-hover",
            "--accent-glow",
            "--accent-bg",
            "--bg",
            "--bg-panel",
            "--bg-item",
            "--bg-item-hover",
            "--text",
            "--text-dim",
            "--red",
            "--green",
            "--orange",
            "--r-sm",
            "--r-md",
            "--overlay-top",
            "--overlay-mid",
            "--overlay-bot",
        ] {
            assert!(vars.contains_key(key), "missing {key}");
        }
        assert_eq!(vars.len(), 18);
    }

    #[test]
    fn light_text_uses_subtler_item_overlays() {
        let dark_ground = ThemeDefinition {
            text: "#ffffff".into(),
            ..Default::default()
        };
        assert_eq!(
            dark_ground.build_css_vars()["--bg-item"],
            "rgba(255,255,255,0.04)"
        );

        let light_ground = ThemeDefinition {
            text: "#000000".into(),
            ..Default::default()
        };
        assert_eq!(
            light_ground.build_css_vars()["--bg-item"],
            "rgba(0,0,0,0.05)"
        );
    }

    #[test]
    fn overlay_strength_scales_the_overlay_alphas() {
        let t = ThemeDefinition {
            bg: "#000000".into(),
            overlay_strength: 0.5,
            ..Default::default()
        };
        let vars = t.build_css_vars();
        assert_eq!(vars["--overlay-top"], "rgba(0,0,0,0.275)");
        assert_eq!(vars["--overlay-mid"], "rgba(0,0,0,0.1)");
        assert_eq!(vars["--overlay-bot"], "rgba(0,0,0,0.425)");
    }

    #[test]
    fn seed_from_unknown_preset_falls_back_to_dark() {
        let t = seed_from("nonsense", "");
        assert_eq!(t.base_preset, "dark");
        assert_eq!(t.name, "Dark Custom");
        assert_eq!(t.accent, "#7289da");
    }

    #[test]
    fn seed_from_light_keeps_its_light_palette() {
        let t = seed_from("light", "#ff0000");
        assert_eq!(t.base_preset, "light");
        assert_eq!(t.bg, "#f5f6f8");
        assert_eq!(t.text, "#0e1116");
        assert_eq!(t.accent, "#ff0000");
    }

    #[test]
    fn duplicated_gets_a_fresh_id() {
        let a = ThemeDefinition::default();
        let b = a.duplicated();
        assert_ne!(a.id, b.id);
        assert_eq!(a.name, b.name);
    }

    #[test]
    fn serializes_with_pascal_case_keys() {
        let json = serde_json::to_string(&ThemeDefinition::default()).unwrap();
        for key in [
            "\"SchemaVersion\"",
            "\"BasePreset\"",
            "\"RadiusSm\"",
            "\"CustomCss\"",
            "\"OverlayStrength\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
    }

    /// C# reads with `PropertyNameCaseInsensitive`, and every field has a
    /// default, so a partial theme file must still load.
    #[test]
    fn deserializes_a_partial_theme_file() {
        let t: ThemeDefinition =
            serde_json::from_str(r##"{"Name":"Mine","Accent":"#123456"}"##).unwrap();
        assert_eq!(t.name, "Mine");
        assert_eq!(t.accent, "#123456");
        assert_eq!(t.base_preset, "dark");
        assert_eq!(t.blur, 14);
    }
}
