# Glacier Launcher — Android

Two coordinated apps, built from this one directory:

1. **`app/`** — the Glacier shell. Architecturally this is now a single
   full-screen `WebView` (`MainActivity.kt`) loading `assets/www/index.html`,
   which reuses the desktop app's **real** `wwwroot/css/app.css` and image
   assets byte-for-byte (see "How the UI is built" below) instead of
   hand-translating each panel into Compose widgets. Kotlin's only job is a
   small native bridge (`AndroidBridge` in `MainActivity.kt`) for the handful
   of things a WebView genuinely can't do: root checks, launching the Java
   Edition companion app, and settings persistence.
2. **`pojavlauncher/`** — a **git submodule** pinned to the unmodified
   upstream [PojavLauncherTeam/PojavLauncher](https://github.com/PojavLauncherTeam/PojavLauncher),
   the open-source Android Java Edition runtime. This is what actually runs
   Minecraft Java Edition — its own ARM JVM, LWJGL/GLFW bridge, and
   mod-loader installers (Forge/Fabric) are a huge, mature codebase that
   isn't worth reimplementing; using it (rather than rewriting it) is what
   "make your own version of Pojav" means in practice here.

   `scripts/rebrand-pojav.sh` applies the "Glacier Launcher (Java Edition)"
   rebrand (`applicationId xyz.glacierclient.launcher.java`, app name) as a
   build-time patch rather than a commit inside the submodule — a commit
   there would only exist in whichever clone made it and wouldn't be
   fetchable from the upstream URL `.gitmodules` points at. Run it once
   before building the companion APK, locally or in CI.

The shell app hands off to the Pojav companion app via an explicit intent
(`JavaEditionBridge.kt`) rather than merging both into a single APK — Pojav
is itself a multi-module Android app with its own native libraries and build
quirks, and a byte-level Gradle merge risks bringing its own long tail of
native/JNI issues without a real device to validate against. Both APKs ship
together in the same release.

## How the UI is built

`android/app/src/main/assets/www/` mirrors the relevant parts of the
desktop app's `wwwroot/`:

```
www/
  index.html          real markup from Pages/Home.razor's top-bar/main-content/
                       footer (Razor directives stripped, structure kept)
  css/app.css          <- copied verbatim from wwwroot/css/app.css
  vendor/fontawesome/   <- copied verbatim (same icon set, works offline)
  vendor/fonts/         <- copied verbatim (Plus Jakarta Sans, Roboto)
  images/               <- copied verbatim (icon.png, bg.jpg, client logos)
  js/
    app.js       app controller: state, quick-actions/home/footer rendering,
                 panel routing, event delegation (JS translation of
                 Home.razor.cs's currentView + OpenXxx() methods)
    panels.js    panel body markup, ported line-for-line from the real
                 Pages/Home.razor blocks for Clients/Servers/Credits/
                 Settings/Addons (same CSS classes, same copy text)
    curseforge.js  same base URL/game/class ids as Services/CurseForgeService.cs;
                   calls the CurseForge API directly via fetch()
    bridge.js    thin wrapper over the native AndroidBridge, with a
                 browser-safe fallback so index.html can be opened directly
                 in a desktop browser while iterating on layout
```

Because it's the same stylesheet and the same image files, this gets
pixel-identical results for anything the CSS controls (colors, spacing,
glass/blur panel-overlay treatment, card layout, icons) — not an
approximation of them. What's genuinely different is native-only mechanics
Kotlin's bridge exists for, not styling.

`flarial.svg`, `latite.png`, `oderso.png` and `icon.png` are the actual
files from `wwwroot/images/` — not redrawn.

## Why this isn't a literal 1:1 port of the Windows app's *behavior*

The desktop launcher's headline feature is **DLL injection** into the
Windows `Minecraft.Windows.exe` process (Latite, Flarial, OderSo clients).
Android sandboxes every app by UID; there is no supported API to load a
foreign native library into another app's process without root.
`app/.../service/ClientInjectionService.kt` implements the closest
best-effort analogue (root-only staging into Minecraft Bedrock's storage)
rather than pretending to work; the client cards in the UI still render and
select correctly, but the actual injection step is honestly gated on root.

Also genuinely not portable:
- **Native Discord Rich Presence** — that's an IPC pipe between a local
  Discord *desktop* client and a local game process; there's no equivalent
  channel on Android, and Discord's Game SDK doesn't run on mobile. The
  footer's Discord RPC toggle persists a preference but has no native
  presence to drive yet.
- **System tray** — no desktop shell concept on Android; omitted.

Real, working code here: the Glacier Client card states, CurseForge mod
search (`js/curseforge.js`, same game/class ids as `CurseForgeService.cs`),
Java Edition hand-off (`JavaEditionBridge.kt`), and settings persistence
through the native bridge (`AndroidBridge` → `SharedPreferences`).

## Building locally

```
git submodule update --init --recursive   # first time only

cd android
gradle :app:assembleDebug                 # shell app

./scripts/rebrand-pojav.sh
cd pojavlauncher
./gradlew :app_pojavlauncher:assembleDebug  # Java Edition companion app
```

To iterate on layout without a device/emulator, open
`android/app/src/main/assets/www/index.html` directly in a desktop browser
— `bridge.js`'s fallback keeps it from erroring on the missing native
bridge (network calls like CurseForge search still work; native-only
actions like launching the Java Edition app no-op).

## CI

`.github/workflows/android-release.yml`:
- checks out submodules recursively,
- builds the shell app (`:app:assembleDebug`/`assembleRelease`),
- runs `scripts/rebrand-pojav.sh` then builds the rebranded Pojav companion
  app (`:app_pojavlauncher:assembleRelease`),
- uploads both APKs as build artifacts on every push to `main` touching
  `android/**`,
- and — on commits prefixed `hotfix:`/`update:` — tags and publishes a
  GitHub Release with both APKs attached, mirroring the desktop launcher's
  `release.yml` versioning scheme.

A CurseForge API key is read from the `CURSEFORGE_API_KEY` repo secret at
build time for both apps, same as the desktop build.

## UI parity status

The desktop app (`Pages/Home.razor`, ~4,900 lines) has 26 distinct panel
views sharing one bottom "panel-tabs" bar. Status, kept honest on purpose:

**Matched (real markup ported from the desktop panel, same CSS):**
top bar (branding, edition switcher, client chip), quick-actions dock
(Launch/Settings/Clients/Addons/Servers/MC Versions for Bedrock;
Launchers/Mods/Versions/Profile/Screenshots for Java), footer
(profile/RPC toggle/Xbox/Discord row), news ticker, `clients` (all six
cards), `servers` (saved + "Popular" suggestions), `credits`, `settings`
(category filter + sections — Java Edition section links out to the Pojav
companion app instead of duplicating its own settings UI), `addons`
Bedrock branch (CurseForge search with category chips and pagination),
`mcversions` (channel tabs, filter, version rows with download/switch/
delete actions — the desktop panel's "Install from Microsoft Store" row
is Windows-only sideloading with no Android equivalent, since Android
Bedrock is a single always-current Play Store app; replaced with an
honest note instead of a non-functional button, and the list itself is
empty pending a real version data source rather than seeded with fake
version numbers).

**Empty-state placeholders (correct panel/labels, listing logic queued):**
`bedrockworlds`, `bedrockpacks`, `bedrockbackups`, `bedrockinstances`,
`bedrockscreenshots` — each needs real on-device data (world files,
packs, backups) wired in; the empty-state markup itself is the real
`.empty-state` class from app.css.

`addons` Java branch is now matched too: the same javaModsTab sub-tab bar
(Loaders/Mods/Assets/Datapacks/Tools/CurseForge/Modrinth) from the desktop
panel. Loaders honestly shows the real "no version selected" empty-state
(this app doesn't track an active Java version, so that's the true current
state, not a shortcut); Assets/Tools render the real card sets
(Resource/Shader Packs, Saves, Screenshots, Schematics / Backup, Export,
Duplicate) with their actions disabled rather than faked, since there's no
backing service for them yet; Datapacks is queued (needs a world picker);
Modrinth search is real (`js/modrinth.js`, same base URL/facets as
`ModrinthService.cs`, no API key needed).

`javaclients` ("Launchers") and `javaversions` are matched now too: Vanilla
built-in + Glacier Client (real manifest fetch, same CDN as
`GlacierClientService.cs`) + Lunar Client/Badlion (honestly marked "not
available on Android" — neither ships an Android build to detect or
launch, unlike the desktop panel's local .exe detection); Java Versions
pulls the real Mojang `version_manifest_v2.json` (same as
`JavaVersionService.cs`) with working release/snapshot/historical filters
and search, though Install/Launch hand off to the Java Edition companion
app rather than duplicating its own per-version install management.
The Java-edition panel-tabs bar (`JAVA_PANEL_TABS` in `panels.js`, mirrors
`JavaTabs()` in `Home.BigFeatures.cs`) is also now used for every Java
panel instead of the Bedrock tab set.

`javaprofile` honestly shows the desktop panel's own "not signed in"
branch (`.skin-empty`) — Microsoft/Xbox sign-in isn't wired on Android
yet, so that's this app's true current state, not a shortcut around the
full skin-viewer/cape-wardrobe view. `javascreenshots` has the real
empty-state copy, listing queued (needs shared-storage wiring). `news`
pulls real data — the Glacier news feed and this repo's public GitHub
releases (`js/javaedition.js`'s `NewsFeed`, same endpoints as
`NewsService.cs`/`AutoUpdateService.cs`, no auth needed) — with its own
distinct 4-tab bar (Settings/Home/News/Credits) matching the desktop
panel exactly. Note: like the desktop app, nothing in the built UI
currently opens this panel (it's reachable there via the command-palette
search this app hasn't built) — it exists and renders correctly, just
needs an entry point once a command palette or similar exists.

`downloads` is matched too: a session-scoped list (`App.state.downloads`)
fed by the same client-download action that already drives the Clients
panel's progress bars, with the real empty-state, clear-finished action,
and its own distinct 4-tab bar (Settings/Home/Downloads/Credits) matching
the desktop panel.

`stats` and `logs` are matched too (`Components/StatsPanel.razor` and
`LogsPanel.razor` — note neither has a `.panel-tabs` footer on desktop
either, so these route through a bare overlay shell instead of
`panelShell()`). Stats shows the honest zero/empty state for every
figure (no session tracking exists yet, same as a fresh desktop profile);
Logs shows the real empty-state (listing needs shared-storage wiring;
mclo.gs sharing is a real public paste API with nothing to share until
then).

`levimods` is matched too: real search against the same public registry
LeviLaminaModsService.cs reads (LiteLDev/lipr's `index.json` — the actual
"lip" package manager index, same package-key parsing, same
levilamina+mod tag filter, same latest-version resolution), rendered as
the same client-card list with avatar/version/star-count. Install/remove
are disabled — LeviLamina itself is a native Bedrock injection mod loader
with no Android build to install these plugins into, the same limitation
as `ClientInjectionService`. Matches the desktop panel's own quirks too:
no `.panel-tabs` footer, and a chevron-left back button (it's a sub-panel
of Clients) instead of the chevron-down every top-level panel uses.

`modpacks` is matched too: real CurseForge/Modrinth modpack search (reusing
the same clients the Addons panel already uses) with source tabs, the same
result-row layout (icon/author/downloads/summary), and the real "API key
required" notice. Install is disabled — `ModpackInstallService.cs` unpacks
a modpack into a brand-new Java instance, and this app has no
instance-management model yet, so there's nothing to wire the button to
truthfully. Reachable the same way as desktop: the "Modpacks" button in
the Java Addons panel's header. No `.panel-tabs` footer, matching
`ModpacksPanel.razor`.

`themestudio` is matched and genuinely live, not a mockup: `js/theme.js`
(`ThemeEngine`) is a port of the desktop app's own `wwwroot/js/interop.js`
theme-application functions (already plain browser JS, portable as-is) and
`Models/ThemeDefinition.cs`'s `BuildCssVars()` — same derived accent-glow/
hover/background-overlay math. Theme create/select/duplicate/delete,
per-color editing (native `<input type="color">` swatches instead of
porting `ColorPicker.razor`'s custom hue/saturation picker), radius/blur/
overlay/animation-speed sliders, and custom CSS all apply live by setting
real CSS custom properties on the document — picking a color or dragging a
slider actually re-skins the running app. Themes persist to `localStorage`
(the closest analogue to the desktop's own `themes.json` file) rather than
the settings blob, since it's a list of documents, not a single settings
object. Wallpaper picking is disabled (needs a file picker this app
doesn't have a bridge for yet). Reachable the same way as desktop: the
"Theme Studio" row in Settings → Appearance.

**Not yet started:** `skinlibrary` — needs a 3D skin canvas via
`skinview3d.js`, real Microsoft/Xbox sign-in to have a skin to show in the
first place (same gate `javaprofile` hit), and skin upload/library
management on top.
