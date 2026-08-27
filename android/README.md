# Glacier Launcher — Android

Two coordinated apps, built from this one directory:

1. **`app/`** — the Glacier shell: native Kotlin/Jetpack Compose UI recreating
   the desktop launcher's panels (Home, Clients, Java, CurseForge, Worlds,
   Packs, Screenshots, Backups, Settings), the Glacier Client manifest/download
   pipeline, CurseForge mod browsing, Xbox Live sign-in, and settings/theming.
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
quirks (see its own `pojavlauncher/build.gradle`), and a byte-level Gradle
merge into one application risks bringing its own long tail of native/JNI
issues without a real device to validate against. Both APKs ship together
in the same release.

Glacier Client jars (and CurseForge mods) installed from the shell app are
copied into Pojav's own shared-storage mods directory
(`.../games/PojavLauncher/.minecraft/mods`, see
`pojavlauncher/app_pojavlauncher/.../Tools.java` `DIR_GAME_HOME`) so they're
picked up the next time Java Edition launches — no cross-app IPC needed
since both write to the same external storage path.

## Why this isn't a literal 1:1 port of the Windows app

The desktop launcher's headline feature is **DLL injection** into the
Windows `Minecraft.Windows.exe` process (Latite, Flarial, OderSo clients).
Android sandboxes every app by UID; there is no supported API to load a
foreign native library into another app's process without root.
`app/.../service/ClientInjectionService.kt` implements the closest
best-effort analogue (root-only staging into Minecraft Bedrock's storage)
and says so explicitly in the UI when root isn't available, rather than
pretending to work.

Also genuinely not portable:
- **Native Discord Rich Presence** — that's an IPC pipe between a local
  Discord *desktop* client and a local game process; there's no equivalent
  channel on Android, and Discord's Game SDK doesn't run on mobile.
  `service/DiscordPresenceService.kt` offers an honest, much smaller
  substitute instead: an opt-in webhook post, clearly not the same feature.
- **System tray** — no desktop shell concept on Android; omitted.

Everything else in the request is real, working code here: Glacier Client
jar install/versioning, CurseForge mod search (`CurseForgeScreen.kt` +
`CurseForgeRepository.kt`, same game/class ids as the desktop
`CurseForgeService.cs`), Java Edition mods (via the Pojav companion app),
screenshots browsing, and Xbox Live sign-in
(`XboxAuthService.kt`, same device-code → XBL → XSTS flow as
`LiveAuthService.cs`/`XboxProfileService.cs` — pure HTTPS, so it ports
1:1 unlike DLL injection).

## Building locally

```
git submodule update --init --recursive   # first time only

cd android
gradle :app:assembleDebug                 # shell app

./scripts/rebrand-pojav.sh
cd pojavlauncher
./gradlew :app_pojavlauncher:assembleDebug  # Java Edition companion app
```

(No wrapper jar is committed for `app/`; CI provisions Gradle directly.
`pojavlauncher/` brings its own `gradlew`.)

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
views sharing one bottom "panel-tabs" bar. This is a running tally of which
ones are matched structurally (same cards, labels, and actions) in Compose
versus still queued — kept honest on purpose rather than claiming "done":

**Matched (real content, same structure/labels as the desktop panel):**
- `home` — launch button, Java Edition handoff, recently-launched chips,
  footer Xbox/Discord row (`HomeScreen.kt`)
- `clients` — all six cards in the same order: Flarial, Latite, OderSo,
  LeviLamina, Vanilla, Custom DLL, with the same select/download/update/
  delete affordances (`ClientsScreen.kt`)
- `servers` — saved servers + "Popular" suggestions, same row actions
  (`ServersScreen.kt`)
- `credits` — same launcher/client credit cards and links (`CreditsScreen.kt`)
- `addons` (Bedrock branch) — same API-key-required empty state, category
  chips (Addons/Maps/Skins/Texture Packs/Scripts), search, results with an
  install action, and "Load more" pagination as the desktop panel
  (`AddonsScreen.kt` + `CurseForgeRepository.kt`, same game/class ids as
  `CurseForgeService.cs`). The Java branch of this panel additionally has
  Loaders/Mods/Assets/Datapacks/Tools sub-tabs and a Modrinth results tab —
  queued along with the other Java-edition views below.
- `settings` — same category filter row (All/Inject/Looks/Account/System)
  and the same sections underneath (Injection, Appearance, Account, Social,
  Quality of Life, Updates, CurseForge, Backup, About), wired to real
  settings storage (`SettingsScreen.kt`). The "Java Edition" section
  deliberately does *not* duplicate RAM/JVM-args/resolution controls — the
  Pojav companion app already has a real settings UI for those, so this
  section links out to it instead of shipping dead sliders. "Folders" and
  "Minimize to tray" are dropped (no filesystem browser / tray on Android).
  Export/Import settings and "Check for updates now" are UI-complete but
  not yet wired to actual file I/O or an update feed.

**Empty-state placeholders (correct panel/labels, listing logic queued):**
`bedrockworlds`, `bedrockpacks`, `bedrockbackups`, `bedrockinstances` — each
needs Storage Access Framework wiring to read real on-device Bedrock/Java
data before the cards can populate.

**Not yet started:**
`mcversions` (Bedrock version manager — currently aliased to the
Clients screen as a placeholder route), `bedrockscreenshots` vs.
`javascreenshots` split, `javaclients`/`javaversions`/`javaprofile` (the
Java-edition variants, reached on desktop via an edition toggle — this
Android app doesn't have that toggle wired yet, see `JavaEditionScreen.kt`
for the standalone Java entry point that exists instead), `downloads`,
`news`, `themestudio`, `modpacks`, `stats`, `logs`, `skinlibrary`,
`levimods`.

Each of those is a real, separate chunk of work (Settings alone is bigger
than everything matched so far combined) — they're being worked through in
priority order across follow-up passes rather than stubbed out in bulk.
