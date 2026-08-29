# Glacier Launcher — Android

One APK, one process, built from this one directory:

1. **`app/`** — the Glacier shell. Architecturally this is a single
   full-screen `WebView` (`MainActivity.kt`) loading `assets/www/index.html`,
   which reuses the desktop app's **real** `wwwroot/css/app.css` and image
   assets byte-for-byte (see "How the UI is built" below) instead of
   hand-translating each panel into Compose widgets. Kotlin's only job is a
   small native bridge (`AndroidBridge` in `MainActivity.kt`) for the handful
   of things a WebView genuinely can't do: root checks, launching Bedrock/Java
   Edition, and settings persistence.
2. **`pojavlauncher/`** — a **git submodule** pinned to the unmodified
   upstream [PojavLauncherTeam/PojavLauncher](https://github.com/PojavLauncherTeam/PojavLauncher),
   the open-source Android Java Edition runtime. This is what actually runs
   Minecraft Java Edition — its own ARM JVM, LWJGL/GLFW bridge, and
   mod-loader installers (Forge/Fabric) are a huge, mature codebase that
   isn't worth reimplementing; using it (rather than rewriting it) is what
   "make your own version of Pojav" means in practice here.

   Rather than shipping as its own separate installable APK, `app_pojavlauncher`
   is built directly into `:app` as a Gradle **library** dependency —
   `scripts/rebrand-pojav.sh` (via `scripts/patch_pojav_gradle.py`) converts
   it from `com.android.application` to `com.android.library` and applies a
   handful of other build-time-only structural patches (see "Single-APK,
   single-process Java Edition" below for the full mechanics) rather than
   committing any of this inside the submodule — a commit there would only
   exist in whichever clone made it and wouldn't be fetchable from the
   upstream URL `.gitmodules` points at. Run it once before building, locally
   or in CI.

The shell hands off to Java Edition via a direct explicit Intent to
`net.kdt.pojavlaunch.MainActivity` (`JavaEditionBridge.kt`) — Pojav's real
JVM/GLFW game-render surface, now just another Activity in this same
package. There is no separate companion app, no install step, and no
"install unknown apps" prompt for Java Edition; one APK ships in the
release.

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
./scripts/rebrand-pojav.sh                # converts the submodule into a library, see below
gradle :app:assembleDebug                 # builds the whole app, Java Edition runtime included
```

### Single-APK, single-process Java Edition — no separate app to install

The vendored PojavLauncher submodule (`android/pojavlauncher`) is built
directly into this app as a library dependency of `:app`, not as its own
installable APK. There is no companion app, no install step, and no
"install unknown apps" prompt for Java Edition — one APK, one process,
`net.kdt.pojavlaunch.MainActivity` (the real JVM/GLFW game surface — see
below) is just another Activity in this same package, launched by a plain
explicit `Intent(context, MainActivity::class.java)`.

How this works, mechanically:

- **`settings.gradle.kts`** wires `app_pojavlauncher` (plus its three plain
  `java`/`java-library` support modules — `jre_lwjgl3glfw`,
  `arc_dns_injector`, `forge_installer` — which just build small jars
  copied into `app_pojavlauncher`'s own assets) in as subprojects of this
  build, at their existing paths under `android/pojavlauncher/`, bypassing
  that submodule's own root `build.gradle`/`settings.gradle` entirely.
- **`scripts/rebrand-pojav.sh`** (via `scripts/patch_pojav_gradle.py`, a
  brace-depth-aware structural patcher — plain regex is too fragile for
  reliably removing a `{ ... }` block from someone else's Groovy build
  script) converts `app_pojavlauncher` from `com.android.application` to
  `com.android.library` at build time, never committed into the submodule
  itself (same reasoning as always: a submodule commit only this sandbox
  has isn't fetchable by CI or any other clone). Concretely, it: drops the
  library-illegal `applicationId`/`applicationIdSuffix`/`bundle{}`/
  `signingConfigs{}` declarations; retargets the `application_package`/
  `storageProviderAuthorities`/`shareProviderAuthority` resValues from the
  old standalone `net.kdt.pojavlaunch` package to the real merged
  `xyz.glacierclient.launcher` one (these back a real manifest
  `<provider android:authorities>` — leaving them stale would either not
  match the real provider authority, or collide with any other
  Pojav-based app on the same device); and strips the `LAUNCHER`
  intent-filter from `TestStorageActivity` so it doesn't show as a second
  home-screen icon next to Glacier's own.
- **`app/build.gradle.kts`** bumps `com.android.application`/library to
  8.7.2 (matching what `app_pojavlauncher` itself pins — also the AGP
  version that requires Gradle 8.9+, which `gradle/wrapper` already is)
  and adds `implementation(project(":app_pojavlauncher"))`, plus
  `multiDexEnabled = true` (the merged-in dependency graph — constraintlayout,
  viewpager2, preference, bytehook, htmlcleaner, ... — pushes method count
  well past the pre-multidex 64K limit; minSdk 26 has native ART multidex
  support, so no `androidx.multidex` compat library is needed).
- **`AndroidManifest.xml`**'s `<application>` tag adds
  `tools:replace="android:name,android:icon,android:theme,android:process"`
  to resolve real attribute conflicts with the merged-in library's own
  `<application>` declaration: keep Glacier's icon/theme, keep the app in
  its own real default process (`android:process="${applicationId}"`,
  overriding Pojav's own `:launcher` secondary-process default — this is
  one merged app now, not two), and keep `GlacierApp` as the Application
  class — which, critically, isn't a plain `android.app.Application`
  anymore.
- **`GlacierApp.kt`** now extends `net.kdt.pojavlaunch.PojavApplication`
  instead of `android.app.Application`. `PojavApplication.onCreate()` does
  real one-time setup — a crash handler, `LauncherPreferences.loadPreferences()`,
  device-architecture detection, and unpacking the bundled JRE/LWJGL
  runtime out of assets — that `MainActivity` depends on unconditionally.
  Skipping it (e.g. by keeping a plain `Application` and only changing
  `android:name` back) would make the real game activity crash immediately
  on first touch.
- **`JavaEditionBridge.launch()`** now takes a compile-time reference to
  `net.kdt.pojavlaunch.MainActivity` (no more `Intent.setClassName()` by
  string, no more `PackageManager` install checks — the class is just
  linked in) and still passes the selected version through
  `MainActivity.INTENT_MINECRAFT_VERSION` (see the next section) for a
  direct launch into that version's gameplay.

This is a real architectural merge, verified only by a successful Gradle
build in CI, not by running the merged app on a device — there is no
physical device in this environment. A clean CI build proves the manifest
merge, the library conversion, and the dependency graph all resolve
correctly; it does not prove `PojavApplication`'s runtime unpacking or the
game's own render loop behave identically once actually launched. If
something is subtly wrong at that layer, it will surface as a real bug
report against an actual device, not as a CI failure — flagged here
plainly rather than glossed over.

### Native, direct-to-gameplay launching

Reading Pojav's own vendored source found that its manifest's actual
`LAUNCHER` activity (`TestStorageActivity` → `LauncherActivity`, now with
its intent-filter stripped per the previous section) is a setup/
version-picker flow, but `net.kdt.pojavlaunch.MainActivity` — the activity
this app targets directly — **is** the real JVM/GLFW game-render surface
itself: its `onCreate` calls `runCraft()` on the first frame, and it reads
an `intent_version` extra (`MainActivity.INTENT_MINECRAFT_VERSION`) to pick
which installed version to launch, falling back to Pojav's own last-used
profile otherwise. `JavaEditionBridge.launch()` passes the version the user
tapped in *our own* Java Versions panel through that extra
(`AndroidBridge.launchJavaEditionVersion()` /
`Bridge.launchJavaEditionVersion()`), so launching a specific version from
Glacier's own UI is a real, direct, native launch into that version's
gameplay — not just a generic reopen of Pojav's separate home screen.

This only works once Pojav's own one-time setup (JRE download, a saved
launcher profile) has happened at least once; a completely fresh install
may still land in Pojav's own setup screens first; that's an unavoidable
first-run cost of any Pojav-based launcher, this one included.

### Release signing

Release builds are unsigned unless `ANDROID_KEYSTORE_PATH` /
`ANDROID_KEYSTORE_PASSWORD` / `ANDROID_KEY_ALIAS` / `ANDROID_KEY_PASSWORD`
are set (`app/build.gradle.kts`), matching CI's
`ANDROID_KEYSTORE_BASE64` / `ANDROID_KEYSTORE_PASSWORD` / `ANDROID_KEY_ALIAS`
/ `ANDROID_KEY_PASSWORD` repo secrets (the base64-encoded keystore is
decoded to a file at build time). No keystore is checked into this repo —
generate your own once and keep it somewhere safe, since losing it means
future releases can never update past versions in place:

```
keytool -genkeypair -v -keystore glacier-release.keystore -alias glacier \
  -keyalg RSA -keysize 2048 -validity 10000
base64 -w0 glacier-release.keystore > glacier-release.keystore.b64
```

Add `glacier-release.keystore.b64`'s contents as the `ANDROID_KEYSTORE_BASE64`
repo secret, and the alias/store/key passwords as the other three secrets.
Without them, CI still builds successfully — the release APK just comes out
unsigned (installable via `adb install -r` for testing, not distributable
as an in-place update).

To iterate on layout without a device/emulator, open
`android/app/src/main/assets/www/index.html` directly in a desktop browser
— `bridge.js`'s fallback keeps it from erroring on the missing native
bridge (network calls like CurseForge search still work; native-only
actions like launching the Java Edition app no-op).

## CI

`.github/workflows/android-release.yml`:
- checks out submodules recursively,
- runs `scripts/rebrand-pojav.sh` to convert the vendored PojavLauncher
  submodule into a library module (see "Single-APK, single-process Java
  Edition" above),
- builds the one app (`:app:assembleDebug`/`assembleRelease` — this
  transitively builds `:app_pojavlauncher` and its native components too,
  there's nothing separate left to build),
- uploads the APK as a build artifact on every push to `main` touching
  `android/**`,
- and — on commits prefixed `hotfix:`/`update:` — tags and publishes a
  GitHub Release with it attached, mirroring the desktop launcher's
  `release.yml` versioning scheme.

A CurseForge API key is read from the `CURSEFORGE_API_KEY` repo secret at
build time (both this app's own CurseForge search and
`app_pojavlauncher`'s own `getCFApiKey()` read the same env var).

## Mobile-specific adjustments

A few things depart from the desktop app on purpose because "mobile" is a
genuinely different environment, not because of a shortcut:

- **Landscape-only.** `MainActivity` is locked to
  `android:screenOrientation="landscape"` — the desktop layout (a wide
  top-bar, side-by-side footer, horizontal tab bars) was never designed for
  a portrait phone screen, and there's no separate portrait layout to fall
  back to.
- **`css/mobile.css`** loads after the real `css/app.css` (kept byte-for-byte
  for fidelity) and only shrinks chrome — top-bar/footer padding, action
  button height, panel header/tab sizes — for short landscape viewports
  (phones are usually 360-420px tall in landscape vs. app.css's 360px desktop
  *minimum*). It doesn't change any markup or add new UI.
- **Panel tab bar cycler.** `js/tabcycler.js` is a verbatim port of the
  desktop app's own `wwwroot/js/interop.js` `ensureTabCyclers()` (already
  plain, portable browser JS) — footers with more than 4 tabs page 4 at a
  time with sticky prev/next arrows, exactly like desktop, instead of
  showing all 11 Bedrock tabs squeezed into one row.
- **App icon.** The adaptive icon foreground is now inset ~18% on each side
  so the artwork sits inside the mask's real safe zone instead of filling
  the whole 108dp canvas edge-to-edge, which read as oversized once Android
  applied its shape mask on the home screen.
- **Immersive fullscreen.** `MainActivity` hides the status bar and
  nav/gesture bar (`WindowInsetsControllerCompat`, transient-on-swipe), since
  the app already draws its own top-bar/footer chrome and the system bars
  were just dead space at a phone's edges. Re-hides itself whenever the
  window regains focus (e.g. returning from the system package installer).

### Removed: DLL-injected clients (Flarial / Latite / OderSo / LeviLamina)

The Clients panel only offers Vanilla now. All four removed clients work by
loading a native DLL into `Minecraft.Windows.exe`'s own process
(`CreateRemoteThread` + `LoadLibrary`); Android sandboxes every app by UID
with no supported way to load code into another app's process without root,
and Minecraft Bedrock for Android exposes no mod-loader hook the way the
Windows clients target (see `ClientInjectionService.kt`'s doc comment for
the full explanation, now also surfaced in-app under Settings → Clients).
Since there was no honest "select client" action left to offer for any of
the four, the LeviLamina Mods registry browser (`levimods`, a sub-panel of
the now-removed LeviLamina Client card) was removed along with it, and the
old Injection settings category (active-client picker, injection delay,
auto-inject) was replaced with a "Clients" category explaining why. They're
still credited in the Credits panel as real, separate open-source projects
this app doesn't ship — that's unrelated to whether this app can launch them.

### Search, notifications, and sign-in

- **Global search** (`js/panels.js`'s `searchOverlayHtml()`/
  `searchQuickActions()`, tapped via the new top-bar magnifying-glass
  button) is a curated subset of the desktop command palette
  (`Home.Search.cs`'s `BuildDefaultSearchResults()`) — real navigation to
  every panel that exists on Android, minus Windows-only entries (F11
  fullscreen, tray, wallpaper picker, folder shortcuts) and DLL-client
  selection. This also gives the News panel its first real entry point on
  Android, since neither app has one anywhere else in the built UI.
- **Notification bell** (`notifPanelHtml()`) is real, not decorative: its
  badge count and Downloads section are driven by this app's own
  `App.state.downloads`. The Notifications list itself stays honestly empty
  — desktop's bell also surfaces a `NotificationService` event log (crash
  detection, update checks) that doesn't exist on Android yet.
- **Microsoft / Xbox / Minecraft sign-in is real, not stubbed.** Tapping
  "Sign in" (footer Xbox pill, Java Profile panel, or the search palette)
  opens a native `Dialog`+`WebView` on the same legacy Microsoft OAuth
  authorize page desktop's `LiveAuthWindow.xaml.cs` uses (same public
  `client_id`/scope/redirect-URI constants — these identify the
  community-launcher OAuth flow itself, not a secret). Once the dialog's
  WebView reaches the `oauth20_desktop.srf` redirect, the authorization
  `code` is read straight out of the URL (no custom URL scheme needed) and
  handed to `js/xboxauth.js`, which does the rest as real `fetch()` calls
  exactly mirroring `LiveAuthService.cs`/`XboxProfileService.cs`: token
  exchange → Xbox Live user auth → XSTS (both the Xbox and Minecraft
  relying parties) → Xbox profile → `login_with_xbox` → Minecraft profile.
  A successful sign-in populates the same settings fields as desktop
  (`xboxGamertag`, `javaUsername`, `javaUuid`, `javaAccessToken`, …), which
  is what unlocks Skin Library's "Save current" and "Apply" actions —
  applying a skin POSTs the real texture to
  `api.minecraftservices.com/minecraft/profile/skins`
  (`SkinLibrary.applySkin()` in `js/skinlibrary.js`, mirroring
  `SkinService.UploadSkinAsync`), fetching the texture into a `Blob` first
  since this app only ever has a Mojang CDN URL, never local PNG bytes.
  Same caveat as the Skin Library's Mojang lookups: these are browser
  `fetch()` calls from the WebView's `file://` origin rather than a native
  `HttpClient`, so a CORS restriction on any of these endpoints would
  surface as a real, visible sign-in error rather than being silently
  worked around.

## UI parity status

The desktop app (`Pages/Home.razor`, ~4,900 lines) has 26 distinct panel
views sharing one bottom "panel-tabs" bar. Status, kept honest on purpose:

**Matched (real markup ported from the desktop panel, same CSS):**
top bar (branding, edition switcher, client chip), quick-actions dock
(Launch/Settings/Clients/Addons/Servers/MC Versions for Bedrock;
Launchers/Mods/Versions/Profile/Screenshots for Java), footer
(profile/RPC toggle/Xbox/Discord row), news ticker, `clients` (all six
cards), `servers` (saved + "Popular" suggestions), `credits`, `settings`
(category filter + sections — Java Edition section links into the built-in
Java Edition runtime's own settings instead of duplicating them), `clients`
(Vanilla — see "Removed: DLL-injected clients" below for why that's the
only card), `addons` Bedrock branch (CurseForge search with category
chips and pagination),
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
and search, though Install/Launch hand off to the built-in Java Edition
runtime rather than duplicating its own per-version install management.
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

`skinlibrary` is matched for the part that's genuinely portable: "Add by
username" is a real port of `SkinLibraryService.AddFromUsernameAsync` —
`js/skinlibrary.js` calls Mojang's own `api.mojang.com` (username → UUID)
and `sessionserver.mojang.com` (UUID → signed texture URL + slim/classic
model flag) exactly like the desktop app, with no third-party proxy. There
is no filesystem skin library on Android, so the resolved Mojang texture
URL (stable and signed, not a local file) is what's kept, in `localStorage`
under `glacier_skins`, instead of downloaded PNG bytes. The grid, cards,
and empty-state (`.skinlib-grid`/`.skinlib-card`/`.stats-empty`) are the
real markup from `Components/SkinLibraryPanel.razor`. "Save current" and
Apply are honestly disabled/blocked — both need a signed-in Microsoft/Java
account to have a skin to save or somewhere to push one to, and no OAuth
flow exists here (same gate `javaprofile` hit). "Add PNG" is disabled too
(needs a native file picker this app doesn't have a bridge for yet). Since
Mojang's endpoints are called with browser `fetch()` from the WebView
rather than a native `HttpClient`, a CORS restriction there would surface
as a real, visible fetch error rather than being silently worked around.
Reachable via a "Skin Library" row in Settings → Account, since the real
desktop entry points (a header button on the signed-in Java profile view,
and a command-palette search result) are both gated behind sign-in or a
feature that doesn't exist on Android.

This closes out the full 26-panel UI-parity backlog: every panel is either
a byte-for-byte-markup live match, an honest empty-state placeholder for
functionality that needs infrastructure not yet built (Storage Access
Framework file listing, instance management, native file/wallpaper
pickers, real Xbox/Microsoft OAuth), or explicitly documented as not
reachable/not applicable on Android.

## Deep gap analysis vs. the desktop app (2026-08-28)

A systematic pass, not a spot-check: every `@onclick="Method"` handler in
`Pages/Home.razor` was extracted (135 distinct methods) and checked against
what's actually wired on Android. Grouped by theme, with the real technical
approach for each gap rather than just "todo":

### Bedrock client injection — corrected after researching prior art

Earlier in this port, `ClientInjectionService.kt` and the Clients panel
claimed DLL-style client injection (Flarial/Latite/OderSo/LeviLamina) has
**no non-root equivalent on Android**. That was wrong, and researching two
real Android Bedrock launchers proved it: **Kitsuri-Studios/Minimal-Launcher**
and **LiteLDev/LeviLaunchroid** (LeviLamina's own official Android
launcher) both ship real, working, non-root injection using the same
pattern (ideas studied, no code copied):

1. Require the official, licensed `com.mojang.minecraftpe` already
   installed via Google Play (this is a legal requirement, not a technical
   one — the launcher never redistributes Mojang's code).
2. Get a `Context` for that installed package via
   `createPackageContext(pkg, CONTEXT_IGNORE_SECURITY or CONTEXT_INCLUDE_CODE)`
   — this hands you that package's own `ClassLoader`, `AssetManager`, and
   native library directory from *inside your own app's process*, no root
   or cross-process injection needed.
3. Extract/locate `libminecraftpe.so` (and its dependencies —
   `libc++_shared.so`, `libfmod.so`, `libMediaDecoders_Android.so`,
   `libHttpClient.Android.so`) from that package's native lib dir or its
   split APKs, `dlopen()` them from a small JNI shim, and forward
   `ANativeActivity_onCreate`/`android_main` to the real symbols — your
   launcher's activity *becomes* Minecraft's own compiled native code
   instead of trying to reach into someone else's process.
4. A launcher-owned native module (their `libshin.so`) loads alongside and
   hooks the real library the same way a Windows DLL client hooks
   `Minecraft.Windows.exe` — that hook code is the actual "client" (Flarial/
   Latite-equivalent) and is inherently version-specific, hand-written,
   reverse-engineering work no generic launcher can produce.

This is a real, buildable architecture — not something this pass could
finish and verify without a physical device (JNI + `dlopen` against a real
Minecraft binary can't be meaningfully tested in this environment), but
`ClientInjectionService.kt`'s doc comment and the Settings → Clients
explanation should stop asserting "needs root," and the concrete
next-build-step is a `BedrockGamePackageManager.kt` (package-context
resolution + native lib extraction) and a `NativeLoader` JNI shim mirroring
the steps above. What genuinely is in scope without device testing: a
**custom `.so` picker** (Storage Access Framework `ACTION_OPEN_DOCUMENT`)
mirroring desktop's generic `PickDllFile`/`CopyDllPath`/`ClearCustomDll`
"bring your own client" slot — the desktop app doesn't limit clients to
its three built-in ones either.

### World/instance/backup management — genuinely implementable, not just "needs SAF"

LeviLaunchroid's source shows exactly how: `LevelDBReader`/`LevelDBManager`/
`LevelDBKey`/`LevelDBEntry` (Bedrock world saves are LevelDB databases —
there are pure Kotlin/Java LevelDB implementations), `BedrockNbtReader`/
`BedrockNbtWriter` (Bedrock's `level.dat` is NBT, the same format Java
Edition uses, just little-endian), `WorldEditor`, `StructureExtractor`,
`FlatWorldGenerator`, `OptionsEditor`, `ScreenshotManager`, `ServerManager`.
None of that needs root — it's file I/O against
`/storage/emulated/0/games/com.mojang/minecraftWorlds/` (or the SAF-scoped
equivalent on Android 11+) plus format parsers. This reframes
`bedrockworlds`/`bedrockpacks`/`bedrockbackups`/`bedrockinstances`/
`bedrockscreenshots` from "needs infrastructure that doesn't exist" to "needs
a LevelDB/NBT reader + SAF wiring" — a real, scoped follow-up, just not
something this pass had room to implement and verify.

### Desktop-only, correctly not ported (Windows window chrome / self-update)

`MinimizeWindow`/`MaximizeWindow`/`CloseWindow`/`ToggleFullscreen`/`F11`,
`OpenUpdateModal`/`ApplyLauncherUpdate`/`SkipLauncherUpdate`/
`ManualUpdateCheck`/`DismissAnnouncement` (Windows self-update flow — Android
updates ship through the Play Store or a new APK, not an in-app updater),
`PickWallpaper`/`ResetWallpaper` (no file picker bridge yet — tracked
separately), `OpenLauncherFolder`/`OpenMinecraftFolder`/`Open*Folder`
(Windows Explorer shortcuts — Android has no equivalent "reveal in Explorer"
concept; a SAF folder-open intent is the closest analogue once storage
wiring lands).

### Real gaps worth closing next, roughly by effort

1. **Onboarding flow** (`FinishOnboarding`/`OnboardingNext`/`OnboardingBack`/
   `SkipOnboarding`/`OnboardingImportMinecraft`/`ImportOfficialMinecraft`) —
   desktop's first-run wizard, including importing an existing Minecraft
   install. Android's analogue is exactly the Bedrock package-context flow
   above.
2. **LevelDat editor** (`SaveLevelDat`/`ToggleLevelDatCheats`/
   `CloseLevelDatEditor`) — small NBT read/write once a world is reachable.
3. ~~**Real Discord OAuth**~~ — done: `signInDiscord()`/`notifyDiscordSignInResult()`
   in `MainActivity.kt` run the same authorization-code redirect-interception
   flow built for Microsoft sign-in (`signInMicrosoft()`), and `js/discordauth.js`
   mirrors `js/xboxauth.js`'s token-exchange/profile-fetch shape, matching
   desktop's `OpenDiscordOAuth()`. This is only the `identify`-scope login for
   the profile switcher's username/avatar (`EffectiveProfile()`/footer parity
   is wired in `app.js`'s `effectiveProfile()`/`renderFooter()`) — it has
   nothing to do with Discord Rich Presence, which still has no Android
   equivalent (see the Rich Presence note above).
4. **Java multi-instance management** (`NewJavaInstance`/`NewBedrockInstance`/
   `CommitRenameInstance`/instance folders) — this is what actually blocks
   Modpack "Install" and several Java Addons actions from being real instead
   of disabled; needs a instance-directory model on top of the built-in
   Java Edition runtime's shared storage.
5. **Custom DLL/.so picker** for Bedrock (`PickDllFile`/`CopyDllPath`/
   `ClearCustomDll`) — see injection section above, buildable now via SAF
   independent of the fuller package-context work.

### The Java Edition runtime itself

To be direct about scope: a *fully from-scratch* Java Edition runtime —
writing your own portable JRE for Android and your own translation layer
from LWJGL's desktop OpenGL calls to Android's OpenGL ES, rather than using
PojavLauncher's own (real, mature, years-refined) versions of both — is a
multi-year effort by dedicated native-toolchain teams, not something this
pass reimplemented. What this pass did instead: took PojavLauncher's actual
JRE/LWJGL-EGL runtime and merged it directly into this app as a library
(see "Single-APK, single-process Java Edition" above), so the end result is
genuinely "one app, native launching, no separate install" — just built on
Pojav's real engine rather than a second one written from zero, the same
way the Bedrock side of this app launches the real Minecraft Bedrock APK
rather than reimplementing a Bedrock client.
