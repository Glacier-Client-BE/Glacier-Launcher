# 03 — Missing/Partial Feature Roadmap (Android)

Prioritized by (impact on core loop) × (implementability without a physical
device, per the constraints already documented in `android/README.md`).

## P0 — Blocks core gameplay loop or is user-visible as broken
1. **Bedrock world/pack/backup/instance listing** — implement a pure-Kotlin
   LevelDB reader + little-endian NBT reader (prior art: LeviLaunchroid's
   `LevelDBReader`/`BedrockNbtReader`), read from
   `/storage/emulated/0/games/com.mojang/...` via SAF on Android 11+. Unblocks
   5 panels (`bedrockworlds`, `bedrockpacks`, `bedrockbackups`,
   `bedrockinstances`, `bedrockscreenshots`).
2. **Java multi-instance management model** — a `JavaInstance`-equivalent
   Kotlin/JS model + instance directories on top of Pojav's shared storage.
   Directly unblocks Modpack "Install" and several disabled Java Addons
   actions (currently the single largest disabled-button surface in the app).
3. **MC Versions (Bedrock) real data source** — the panel renders but the
   list is empty; wire to whatever the desktop `Home.Panels.cs` version
   source is (likely a static/CDN manifest, not the MS Store row which is
   correctly omitted).

## P1 — Real, scoped, non-root-dependent
4. **Non-root Bedrock client injection** — package-context (`createPackageContext`
   + `CONTEXT_INCLUDE_CODE`) + JNI `dlopen()` shim forwarding
   `ANativeActivity_onCreate`, per the architecture already researched and
   written up in `android/README.md`. Concrete next file:
   `BedrockGamePackageManager.kt` + a `NativeLoader` JNI shim. Note:
   hook code itself (the Flarial/Latite-equivalent mod) is inherently
   version-specific reverse-engineering work, out of scope for a generic
   launcher — only the *loading* infrastructure is a fair scope target.
5. **Custom `.so` picker for Bedrock** — SAF `ACTION_OPEN_DOCUMENT`, mirrors
   desktop's `PickDllFile`/`CopyDllPath`/`ClearCustomDll`. Independent of #4.
6. **Real Discord OAuth** — reuse the exact redirect-interception technique
   already built for Microsoft sign-in (`MainActivity.kt`'s Dialog+WebView)
   against Discord's public-client OAuth2 flow. Closes `OpenDiscordOAuth`/
   `SaveDiscordManual`/`DisconnectDiscord`.
7. **Skin Library "Save current"/Apply/"Add PNG"** — Apply needs the Java
   profile's access token (already obtainable post-sign-in); "Add PNG" needs
   a generic SAF file-picker bridge in `AndroidBridge`, which #5 also needs —
   build the shared bridge method once, use in both places.
8. **Java Profile skin viewer wiring** — sign-in already populates the needed
   fields; the `.skin-empty` branch just needs a signed-in check + port of
   `Components/SkinViewer.razor`'s render logic to `panels.js`.

## P2 — Infra/quality-of-life
9. **Onboarding wizard** — port `Home.Onboarding.cs` flow; Android's
   "import Minecraft" step maps to the package-context Bedrock detection (#4).
10. **Announcement banner** — port `Services/AnnouncementService.cs` +
    `Home.Announcement.cs` (small; same GitHub/news-style feed pattern
    `js/javaedition.js`'s `NewsFeed` already uses).
11. **Localization** — `Services/LocalizationService.cs` has no Android
    counterpart at all; UI strings are hard-coded in `panels.js`/`app.js`.
    Needs a string-table approach before any non-English release.
12. **LevelDat editor** — small NBT read/write, depends on #1.
13. **Folder-open shortcuts** — SAF folder-open `Intent`, replaces
    `OpenLauncherFolder`/`OpenMinecraftFolder`.
14. **Logs panel + log capture pipeline** — no `LogService`/`GameConsoleService`
    equivalent exists; needs a Kotlin log sink + shared-storage export
    (mclo.gs sharing endpoint already public, no auth needed).
15. **Stats panel session tracking** — needs a lightweight session-timer
    service (start/stop hooks around Java/Bedrock launch calls already in
    `AndroidBridge`).

## Explicitly out of scope (documented, correct to leave alone)
- In-app auto-update (Play Store/APK model is platform-correct).
- System tray, window chrome, F11 fullscreen (no desktop shell on Android).
- Native Discord Rich Presence via IPC (no such channel exists on Android;
  #6 above is the closest honest substitute, not a full equivalent).
- Lunar Client / Badlion Client launch detection (neither ships an Android build).
</content>
