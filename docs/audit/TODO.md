# TODO — Prioritized Task List

## High
- [x] Bedrock **Worlds** panel: pure-Kotlin little-endian NBT reader
      (`BedrockNbt.kt`) + SAF-backed storage service (`BedrockStorageService.kt`)
      reading `levelname.txt`/`level.dat` through a one-time folder grant
      (`MainActivity.kt`'s `requestBedrockStorageAccess`), wired end to end
      in `panels.js`/`app.js`/`js/bedrockstorage.js`. Deliberately does not
      touch the actual LevelDB store in `db/` — nothing needed for listing
      worlds requires parsing it. Packs/Backups/Instances/Screenshots panels
      are still empty-state stubs but need no NBT work, only the same SAF
      pattern applied to different folders (`resource_packs`/`behavior_packs`
      have JSON `manifest.json`, backups/screenshots are plain files) — real
      remaining work, not blocked on anything above.
- [x] Build a Java multi-instance management model on Android
      (`JavaInstanceService.kt`) — built directly on the vendored Pojav
      library's own `LauncherProfiles`/`MinecraftProfile` multi-profile
      system (a real `launcher_profiles.json`) instead of a parallel one,
      since Pojav's `MainActivity` already reads
      `LauncherProfiles.getCurrentProfile().gameDir` at launch — so creating
      a profile per instance and switching
      `LauncherPreferences.PREF_KEY_CURRENT_PROFILE` is a real, working
      instance switch with zero changes to Pojav's own launch code, and the
      existing `Bridge.launchJavaEdition()` call sites automatically launch
      whichever instance is active. Wired into a real Instances card
      (create/switch/rename/delete-with-confirm) in the "javaprofile" panel.
      Still open: porting `ModpackInstallService.cs`'s actual mod-download +
      overrides-zip-extraction step is what would let Modpack "Install"
      itself go live — instance management was the blocker, not the only
      piece.
- [x] Wire a real Bedrock version data source into Android's MC Versions
      panel (`BedrockVersions.fetch()` in `javaedition.js`, same public
      community version-database text file `VanillaVersionService.cs`
      reads). Download/switch/delete stay unavailable — those are AppX
      registration + Windows-Update-SOAP-API operations with no Android
      equivalent — so the panel renders a read-only list
      (`mcVersionInfoRowHtml`) instead of desktop's dead-if-copied
      download/switch/delete buttons.
- [x] Bedrock **Packs** panel: reused the SAF grant from Worlds
      (`BedrockStorageService.listPacks()`, same manifest.json "header.name"
      read as `Services/BedrockPackService.cs`) across all 6 kinds
      (resource/behavior/skin × normal/dev). Backups/Instances/Screenshots
      panels remain stubs — same SAF pattern, smaller scope each (plain
      files, no manifest/NBT parsing at all).
- [ ] Diff `Components/*.razor` output markup against `panels.js`/`app.js`
      generated HTML for spinners, tooltips, error dialogs, and toggles to
      close the "Unverified" rows in `07_UI_PARITY.md`.
- [ ] Add a shared `js/apiClient.js` wrapper (Android) and `ApiClientBase`
      (Windows) to remove duplicated fetch/HTTP boilerplate — see
      `05_REFACTOR_PLAN.md`.

## Medium
- [ ] Implement non-root Bedrock client injection via package-context +
      JNI `dlopen()` (`BedrockGamePackageManager.kt` + `NativeLoader` shim).
- [x] Custom `.so` picker for Bedrock (`pickCustomDllFile()` in
      `MainActivity.kt` + `js/customdll.js`) — SAF `ACTION_OPEN_DOCUMENT`,
      staged into app-private storage since `ClientInjectionService`'s root
      shell command needs a real path, not a `content://` Uri. Wired into a
      new Custom Client card in the "clients" panel with a "Stage for
      injection" action using the already-existing root-based
      `attemptInject`. Still root-only — the non-root package-context+dlopen
      technique remains unbuilt. The picker itself is written narrowly
      (single-file, not a generic bridge method) — reusing it for Skin
      Library "Add PNG"/Theme Studio wallpaper picker is still open and
      would want a shared `pickDocument(mimeTypes)` bridge method instead of
      copy-pasting this one.
- [x] Implement real Discord OAuth on Android reusing the Xbox
      redirect-interception pattern already in `MainActivity.kt`. (Note:
      Discord Rich Presence itself stays a documented gap — it rides local
      IPC to Discord desktop with no Android equivalent; the safe path
      considered, a per-user Gateway self-presence connection, would require
      harvesting each user's raw Discord account token, which risks their
      account and was rejected. This item is the `identify`-scope login
      only, same as desktop's `DiscordToken`/`OpenDiscordOAuth`.)
- [ ] Wire Java Profile skin-viewer UI to the already-working sign-in flow.
- [ ] Split `panels.js` and `app.js` by concern (Bedrock/Java/Settings/state/router).
- [ ] Add a `localStorage`-backed TTL cache for CurseForge/Modrinth/News
      fetches on Android.
- [ ] Add settings schema versioning + migration to `JsonStore`/`LauncherSettings` (Windows).
- [ ] Extract `IThirdPartyClient` interface for Flarial/Latite/OderSo/LeviLamina services (Windows).
- [ ] Update `ClientInjectionService.kt` doc comment and in-app Settings →
      Clients copy to reflect the researched non-root path, once the
      injection work itself lands (content change, not a standalone fix).

## Low
- [ ] Port onboarding wizard to Android.
- [ ] Port announcement banner to Android.
- [ ] Add localization/string-table support to Android (currently hard-coded English).
- [ ] Add LevelDat editor to Android (depends on world-listing work above).
- [ ] Add SAF folder-open shortcuts to Android (replaces desktop's Explorer shortcuts).
- [ ] Add a Logs panel capture pipeline + mclo.gs sharing to Android.
- [ ] Add lightweight session-timer service for Android Stats panel.
- [ ] Audit `Home.razor`'s `StateHasChanged()` call sites for over-broad re-renders (Windows).
- [ ] Cache rendered panel HTML strings in `panels.js`, invalidate only on
      underlying data change, to reduce panel-switch jank.
- [ ] Verify search-input debounce exists on both platforms' CurseForge/Modrinth search.
- [ ] Confirm skin texture caching exists to avoid re-fetching from Mojang CDN each panel open.
</content>
