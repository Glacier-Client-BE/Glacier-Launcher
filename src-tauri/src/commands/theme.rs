//! Theme Studio commands - thin wrappers over `gl_settings::ThemeService`
//! (port of `Services/ThemeService.cs`).
//!
//! Applying a theme stays on the frontend: `interop.js` already owns
//! `setTheme`/`setThemeVars`/`setFont`/`setCustomCss`/`setBackgroundFit`,
//! which is what the C# `ApplyAsync` drove over JS interop. These commands
//! cover persistence plus the CSS-variable computation the frontend feeds to
//! `setThemeVars`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use gl_settings::ThemeDefinition;
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn list_themes(state: State<'_, AppState>) -> Vec<ThemeDefinition> {
    state.themes.themes()
}

#[tauri::command]
pub fn get_theme(state: State<'_, AppState>, id: String) -> Option<ThemeDefinition> {
    state.themes.get(&id)
}

/// Creates a theme seeded from a base preset. Port of `ThemeService.SeedFrom`
/// followed by `Add`.
#[tauri::command]
pub fn create_theme(
    state: State<'_, AppState>,
    preset: String,
    accent: Option<String>,
) -> ThemeDefinition {
    let seeded = gl_settings::themes::seed_from(&preset, accent.as_deref().unwrap_or_default());
    state.themes.add(seeded)
}

/// Persists edits to an existing theme. Returns false when the id is unknown.
#[tauri::command]
pub fn update_theme(state: State<'_, AppState>, theme: ThemeDefinition) -> bool {
    state.themes.update(theme)
}

#[tauri::command]
pub fn duplicate_theme(state: State<'_, AppState>, id: String) -> Option<ThemeDefinition> {
    state.themes.duplicate(&id)
}

#[tauri::command]
pub fn delete_theme(state: State<'_, AppState>, id: String) {
    state.themes.delete(&id);
}

/// Writes `<slug>.glaciertheme.json` and returns its path.
#[tauri::command]
pub fn export_theme(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let theme = state
        .themes
        .get(&id)
        .ok_or_else(|| format!("No theme with id '{id}'."))?;
    state
        .themes
        .export(&theme)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

/// Imports a `.glaciertheme.json`. Returns `None` when the file is missing,
/// unparseable, or has no name - matching the original's `catch { return null; }`.
#[tauri::command]
pub fn import_theme(state: State<'_, AppState>, file_path: String) -> Option<ThemeDefinition> {
    state.themes.import(&PathBuf::from(file_path))
}

/// Copies a chosen background image next to the launcher's data so the theme
/// survives the source file moving. Rejects files over 20 MB.
#[tauri::command]
pub fn stash_theme_background(
    state: State<'_, AppState>,
    source_path: String,
    theme_id: String,
) -> Option<String> {
    state
        .themes
        .stash_background(&PathBuf::from(source_path), &theme_id)
        .map(|p| p.to_string_lossy().into_owned())
}

/// The CSS variables a theme resolves to, for `setThemeVars`. Computed in
/// Rust so the live preview and a saved theme can never disagree.
#[tauri::command]
pub fn theme_css_vars(theme: ThemeDefinition) -> BTreeMap<String, String> {
    theme.build_css_vars()
}

/// The base presets the Theme Studio offers, as `(preset, label)` pairs.
#[tauri::command]
pub fn theme_presets() -> Vec<(String, String)> {
    gl_settings::themes::PRESET_SEEDS
        .iter()
        .map(|p| (p.preset.to_string(), p.label.to_string()))
        .collect()
}
