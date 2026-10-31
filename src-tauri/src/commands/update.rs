use gl_update::{LauncherRelease, LauncherUpdateInfo};
use tauri::{AppHandle, State};

use crate::state::AppState;

const REPO_ORG: &str = "Glacier-Client-BE";
const REPO_NAME: &str = "Glacier-Launcher";

#[tauri::command]
pub async fn check_launcher_update(
    state: State<'_, AppState>,
) -> Result<Option<LauncherUpdateInfo>, String> {
    let checker = gl_update::UpdateChecker::with_client(REPO_ORG, REPO_NAME, state.http.clone());
    checker
        .check_launcher_update(env!("CARGO_PKG_VERSION"))
        .await
        .map_err(|e| e.to_string())
}

/// Lists recent (non-draft) launcher releases for the News panel's changelog
/// section, mirroring `AutoUpdateService.GetRecentReleasesAsync`.
#[tauri::command]
pub async fn list_launcher_releases(
    state: State<'_, AppState>,
    max: Option<usize>,
) -> Result<Vec<LauncherRelease>, String> {
    let checker = gl_update::UpdateChecker::with_client(REPO_ORG, REPO_NAME, state.http.clone());
    checker
        .recent_releases(max.unwrap_or(12))
        .await
        .map_err(|e| e.to_string())
}

// Latite is the only injected Bedrock client with a real versioned-release
// GitHub changelog (see gl-clients::latite); Flarial/OderSo/LeviLamina don't
// publish per-version release notes the same way, so the News panel's
// "Latite" changelog tab tracks Latite specifically.
const CLIENT_REPO_ORG: &str = "LatiteClient";
const CLIENT_REPO_NAME: &str = "Latite";

/// Lists recent Latite Client releases for the News panel's "Latite"
/// changelog tab — same shape and caching as `list_launcher_releases`, just
/// pointed at a different repo.
#[tauri::command]
pub async fn list_client_releases(
    state: State<'_, AppState>,
    max: Option<usize>,
) -> Result<Vec<LauncherRelease>, String> {
    let checker = gl_update::UpdateChecker::with_client(
        CLIENT_REPO_ORG,
        CLIENT_REPO_NAME,
        state.http.clone(),
    );
    checker
        .recent_releases(max.unwrap_or(12))
        .await
        .map_err(|e| e.to_string())
}

/// Hardcoded Glacier Client changelogs sourced from the in-pack changelog JSON
/// and en_US.lang files shipped with Glacier Client v4–v7.
/// Formatted as markdown so `formatChangelog` in NewsView renders section
/// headers bold and each entry as a bullet item.
#[tauri::command]
pub async fn list_glacier_client_releases(
    _state: State<'_, AppState>,
    max: Option<usize>,
) -> Result<Vec<LauncherRelease>, String> {
    let releases = vec![
        LauncherRelease {
            tag: "Glacier Client v7".into(),
            published_at: "2025-01-01".into(),
            changelog: "\
**Module Settings — Back Button:**
- Added a Back button to every module settings panel.
- Tap it to instantly return to the Modules list without closing the Mod Menu.
- The header also displays the current module name so you always know where you are.

**New Modules:**
- Ore Visualizer — highlights ore blocks with configurable color overlays.
- Death Coords — saves and displays your last death location on the HUD.
- Mob Indicator — shows a sword icon when aiming at a mob.

**Animation Overhaul:**
- All 50+ animations renamed with glacier_ prefix for consistent naming across the entire pack.
- Replaced linear easings with polished curves: out_expo for entrances, in_cubic for exits, out_back for popups.
- Tightened animation durations across chat, actionbar, jukebox, item name, tip text, and notification popups.
- Start screen logo animation refined with smoother bounce and slide transitions.

**HUD Module Rewrites:**
- Server Display rewritten with property_bag dynamic lookup — cleaner code, same functionality.
- Slot Counter rewritten with collection_panel and property_bag text lookup.
- Mainhand HUD durability rewritten with unified durability indicator pattern.

**UI & Fixes:**
- Fixed scroll wheel not working on PC — images no longer block mouse wheel events.
- Start screen UI now stays hidden during the logo intro animation.

**Code Optimizations:**
- Created a reusable color picker template for crosshair and experience bar color settings.
- Refactored settings panel rendering into a shared glacier_settings_scrolling_template.
- Back button visual logic consolidated into reusable toggle templates.
- Pack-wide naming consistency enforced with glacier_ prefix on all custom elements.

**Entity Module Expansion:**
- All 6 digit texture sets (1–6) are now fully utilized across render controllers.
- Sets distributed: 1–3 for primary digits, 4–6 for secondary/extended digits in each module.
- Affected modules: FPS Counter, Walk Distance, Force Coords, Death Coords, Speedometer.

**Waypoint System Rework:**
- Waypoint activation now requires looking down (head pitch > 70) in addition to sneaking for 3 seconds.
- Player glow effect re-added: a Glacier-blue overlay appears when the waypoint is armed and ready to place.
- Deathpoint and Greenscreen activation/deactivation unified to use sneak + look-down pattern.

**Coordinate System Unification:**
- All coordinate displays now read from a single glacier_chunk_player_position source.
- Nether and Overworld conversion coords computed centrally, eliminating duplicate parsing.
- Chunk and ore visualizer coordinate bindings unified through shared position data.

**UI & Performance:**
- Module card info icon removed; description text now rendered directly in settings header.
- Settings header redesigned with module description and accent separator line.
- Settings icon on module cards resized and repositioned for cleaner alignment.
- Opacity toggle panel optimized: removed 3 redundant visibility bindings.
- Low Durability module cleaned up: removed all empty binding arrays from nested controls.

**Bug Fixes:**
- Fixed player list appearing on TAB press when disabled in mod menu.
- All settings panels now include module description text for clarity.
- Tutorial instructions updated to reflect new waypoint/greenscreen/deathpoint activation methods.

**Module Background Fixes:**
- glacier_module_bg_template now accepts $bg_size and $bg_alpha parameters to support modules without inline custom_content.
- InventoryHUD slot backgrounds now render — bg_images fills its 17x17 slot panel via $bg_size: 100%, 100%.
- MainhandHUD item bg and durability bg now render correctly using $bg_size and $custom_content patterns.
- Low Durability warning bg now renders by passing $bg_size: 100%, 100% to fill its parent panel.
- Removed broken offset [0, 2] from basebg custom_content — modules no longer jump 2px when toggling underline mode.

**HUD Factories Rewrite:**
- Replaced property_bag texture-swap with the dual-image visibility pattern — actionbar, item name, jukebox, and tip text BGs now render reliably.
- Extracted shared glacier_factories_label template; both BG variants reuse the same animated label.

**Item Counter Overhaul:**
- Item counter BG now renders at 0.7 alpha as intended.
- Each counter (totem, potion, arrow) now has its own opacity slider in settings.
- $totem_counter_opacity, $potion_counter_opacity, $arrow_counter_opacity added to config.json (default 1.0).
- Increased bottom padding so item counters no longer touch the experience bar.

**Server Display & Exp Calculator:**
- Server Display: icon and label wrapped in panels so anchor centering works inside the horizontal stack.
- Exp Calculator: wrapper now stacks horizontally; green [X/Y] level display wrapped in a centering panel and aligned to its sibling labels.

**Timer & Chat:**
- Timer HUD: replaced trailing +4px size hack with symmetric padd_left and padd_right so the digits sit centered within the BG.
- Chat BG converted to dual-image pattern — chat now switches between underlined and base BG via the global hide-underline toggle.

**Render Controllers:**
- DirectionHUD rotation digits now render — texture array indices wrapped in math.floor + math.mod so values above 9 no longer go out of bounds.".into(),
        },
        LauncherRelease {
            tag: "Glacier Client v6".into(),
            published_at: "2024-01-01".into(),
            changelog: "\
**Modules:**

*Waypoints:*
- Implemented a waypoint activation sound that resembles a beacon for improved user feedback.
- The waypoint activation method has been reworked following results from a recent poll. Please consult the Mod Menu's Information Tab for the new procedure.

*Mobile Shortcut Buttons:*
- Added the F1 shortcut to hide the HUD.
- Added the F8 shortcut to toggle the Paperdoll display.
- Introduced Easy Hotbar (Hotbar Swapper) for significantly quicker hotbar navigation.
- All new mobile shortcut buttons are now movable and resizable by editing the config.json file.

*Other Module Changes:*
- Shiny Potions have been re-added, leveraging an improved and more stable implementation method.
- Added a Glacier-themed Hotbar Style.
- The DebugHUD and its associated settings have been added.
- The ArmorHUD has been changed to Paperdoll.
- Added F1, F8, and Easy Hotbar.

**Mod Menu Overhaul:**
- All module icons have been overhauled for a fresh, consistent look.
- A new Modules Information hover toggle has been added.

**Module Settings:**
- Added an opacity slider for the Player Tab List.
- Added an option to hide the player icon within the Player Tab List.
- Introduced a unified Editor section that combines the Hotbar, Crosshair, and Display editors into one place.
- Added a crosshair outline toggle.
- Added a crosshair size slider.
- Expanded the available crosshair color options.
- Editor preview images have been overhauled for clearer visualization of changes.

**EXP Bar Customizer:**
- Made a tab in Editor Section for the EXP Bar.
- 8 new colors are now available for the EXP Bar.
- An RGB color option has been added for custom coloring.

**Music Player:**
- All music discs have been added to the built-in player for on-demand listening.

**Container Updates:**
- The Container Guides have been overhauled.
- A Trash button has been added to the player inventory (available in Creative mode only).
- The Paperdoll showcase is now displayed in multiple container screens.".into(),
        },
        LauncherRelease {
            tag: "Glacier Client v5".into(),
            published_at: "2023-06-01".into(),
            changelog: "\
**General:**
- Various bugs were fixed, and overall performance was improved.
- The entire interface was fully redesigned with a cleaner aesthetic and a new color theme.
- The client retains backwards compatibility with old custom play screens.
- Added more animations for a smoother and more polished user experience.

**Start Screen:**
- Added a skin preview to the start screen. Tap your player name to toggle the preview on/off.
- The layout was updated for smoother and more intuitive navigation.
- A built-in changelog section has been introduced directly on the start screen.

**Modules:**
- Added Chunk Map for a better overview of your world.
- Introduced Bow Indicator to assist with aiming.
- Shiny Potions were temporarily removed due to performance issues.
- Added Server Display to show the Server IP or World Name.
- Creeper Distance was removed due to persistent bugs.
- New Hotbar Modules are available, offering multiple layout options.
- Added new Module settings, such as the option to choose orientation and show slot numbers for the InventoryHUD.
- TimerHUD was rebuilt using JSON UI for enhanced performance and flexibility.

**Pause Screen:**
- The pause screen was redesigned for a more organized layout.
- A player list was added to the pause menu.

**Chat Screen:**
- The chat screen layout was updated for easier reading.
- Added a subtle animation to make the chat screen feel more dynamic.".into(),
        },
        LauncherRelease {
            tag: "Glacier Client v4".into(),
            published_at: "2023-01-01".into(),
            changelog: "\
- Project Light was officially rebranded to Glacier Client.
- Added the Creeper Range module.
- Added the Slot Counter module.
- Added the XP Calculator module.
- General code optimization was performed.".into(),
        },
    ];

    let max = max.unwrap_or(releases.len());
    Ok(releases.into_iter().take(max).collect())
}

/// The running launcher version, port of `AutoUpdateService.CurrentVersion`.
/// The frontend previously hardcoded this string.
#[tauri::command]
pub fn launcher_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Downloads the update installer named by `check_launcher_update`'s
/// `download_url`, through the same `DownloadQueue` every other download in
/// the app uses — so it shows real progress in the Downloads panel instead
/// of the previous "here's a changelog, go get it yourself" dead end.
/// Returns the local path of the downloaded installer.
#[tauri::command]
pub async fn download_launcher_update(
    state: State<'_, AppState>,
    download_url: String,
    tag: String,
) -> Result<String, String> {
    let file_name = download_url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("glacier-launcher-update.exe");
    let dest = gl_settings::cache_dir().join(file_name);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let req = gl_download::DownloadRequest::new(&download_url, &dest)
        .with_label(format!("Glacier Launcher {tag}"));
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().into_owned())
}

/// Launches the downloaded installer and closes the running launcher so it
/// releases its file lock and the installer can overwrite it — the same
/// "close so the installer can proceed" step the user otherwise has to know
/// to do manually. The installer runs with its normal UI (NSIS `/CURRENTUSER`
/// mode from `tauri.conf.json`'s `nsis.installMode`) rather than silently,
/// since a silent self-update that fails with no dialog is worse than one
/// that at least shows the user what happened.
///
/// Mirrors the resumability half of `AutoUpdateService.TryResumePendingUpdate`
/// (see the doc comment on `gl_update::resume` for why "swap" here means
/// "launch the installer" rather than "copy over the exe"): if spawning the
/// installer fails, the downloaded installer is *not* lost — its path is
/// recorded in the pending-update marker so `resume_pending_launcher_update`
/// can retry on the next launch before any UI is shown, instead of silently
/// discarding a good download because of a transient spawn failure (e.g. AV
/// briefly locking the just-written file).
#[tauri::command(async)]
pub fn install_launcher_update(app: AppHandle, installer_path: String) -> Result<(), String> {
    match std::process::Command::new(&installer_path).spawn() {
        Ok(_) => {
            let _ = gl_update::clear_pending_marker();
            app.exit(0);
            Ok(())
        }
        Err(e) => {
            let _ = gl_update::write_pending_marker(std::path::Path::new(&installer_path));
            Err(format!("couldn't launch the installer: {e}"))
        }
    }
}

/// Checks for and retries a pending update installer left over from a prior
/// launch that failed to spawn, mirroring `TryResumePendingUpdate`. Intended
/// to be called once during app startup, before any update-check UI is
/// shown. Returns `true` if a pending installer was found and successfully
/// (re)launched — in which case the caller should exit the app, matching the
/// C# original relaunching the (old) exe only after the swap script starts.
/// Returns `false` for "nothing to resume", including a marker whose target
/// file has since disappeared (matches the C# swallowing that case to
/// `false`) or whose installer still fails to spawn (marker is left in place
/// for the next attempt rather than lost).
#[tauri::command(async)]
pub fn resume_pending_launcher_update(app: AppHandle) -> Result<bool, String> {
    let Some(installer_path) = gl_update::take_pending_update().map_err(|e| e.to_string())? else {
        return Ok(false);
    };

    match std::process::Command::new(&installer_path).spawn() {
        Ok(_) => {
            app.exit(0);
            Ok(true)
        }
        Err(e) => {
            // Re-arm the marker so this isn't a one-shot: `take_pending_update`
            // already consumed it, so without this a transient failure here
            // (rather than at the original `install_launcher_update` call)
            // would silently drop the pending update forever.
            let _ = gl_update::write_pending_marker(&installer_path);
            log::warn!("failed to resume pending launcher update: {e}");
            Ok(false)
        }
    }
}
