# TODO — Prioritized Task List

## Critical fixes found from real device testing (this pass)
- [x] **`Bridge` wrapper only forwarded 11 of 28 native methods** — every
      Discord login, self-update, Bedrock SAF panel (Worlds/Packs/Backups/
      Screenshots), Java instance management, modpack install, and custom
      DLL picker call site called a `Bridge.xxx()` that didn't exist on the
      wrapper object at all, throwing a hard `TypeError` (not a silent
      no-op — worse than the earlier `window.X` bug). Fixed by adding every
      missing native method to `bridge.js`; cross-checked every `Bridge.`
      call site in the codebase against the wrapper afterward to confirm
      completeness.
- [x] Java Edition crash on real device (`Theme.AppCompat` `IllegalStateException`),
      stray launcher icon investigation, edge-to-edge black bar, and
      panel-tab mobile sizing — see the `force-appcompat-theme` manifest
      patch, `windowLayoutInDisplayCutoutMode`, and `mobile.css` tab tweaks.
- [x] Settings panel had no padding — `settingsPanelHtml()` nested the
      category switcher inside `.panel-body` and gave the body only an
      `id="settings-body"` instead of desktop's real sibling structure and
      `class="panel-body settings-body"`, so the compound-class CSS rule
      that supplies its padding/gap never matched. Rebuilt to mirror
      `Pages/Home.razor`'s actual DOM.
- [x] Home action buttons' `.hover-grow` effect never fired on a
      touchscreen (`:hover` doesn't exist there) — added an `(hover: none)`
      media-query duplicate of the same rules under `:active` in
      `mobile.css` so tapping gives the same expand feedback.
- [x] `.window-bg` used `background-size: cover`, which crops far more
      aggressively on a phone's much wider/shorter forced-landscape aspect
      ratio than on a resizable desktop window — read as "stretched"/zoomed.
      Switched to `contain` + a matching dark `background-color` fallback
      in `mobile.css` so the image is never cropped or distorted.
- [x] Added desktop's `.window-controls` close button (`Bridge.closeApp()`
      → `activity.finishAffinity()`) — minimize/maximize/fullscreen have no
      Android equivalent and stay omitted, same as the top-bar drag-zone.


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
      reads). The list itself stays read-only (`mcVersionInfoRowHtml`) —
      it's metadata, not an installable file per entry.
- [x] Android-only Bedrock version management (`BedrockVersionService.kt` +
      `bedrockBuildManagerHtml` in `panels.js`) — desktop's AppX
      registration + Windows-Update-SOAP-API switching has no Android
      equivalent, and the one alternative considered (an unofficial Google
      Play client library) was rejected: it needs a Google account
      master/AAS token, a full-account-compromise blast radius that isn't
      worth what it buys. Instead: import your own Bedrock APK (SAF picker)
      and install it via the same real `PackageInstaller` flow
      `LauncherUpdateService.kt` already uses for self-updates. A true
      downgrade needs root (`pm install -d`, root-only
      `INSTALL_ALLOW_DOWNGRADE`); without root, Android's own installer
      honestly rejects a downgrade rather than this app pretending to force
      one. Also backs up whatever Bedrock build is currently installed
      before replacing it (`backupCurrentApk()`, plus a manual "Backup now"
      button) — same idea as LiteLDev/LeviLaunchroid's own build
      management, so a bad import always has a way back.
- [x] Bedrock **Packs** panel: reused the SAF grant from Worlds
      (`BedrockStorageService.listPacks()`, same manifest.json "header.name"
      read as `Services/BedrockPackService.cs`) across all 6 kinds
      (resource/behavior/skin × normal/dev). Backups/Instances/Screenshots
      panels remain stubs — same SAF pattern, smaller scope each (plain
      files, no manifest/NBT parsing at all).
- [x] Bedrock **Backups** panel: create/list/delete are real
      (`BedrockBackupService.kt` zips `minecraftWorlds`/`*_packs` — read
      through the same SAF grant as Worlds/Packs — into this app's own
      external-files storage, which needs no SAF *write* access at all).
      Restore is NOT implemented — writing back into com.mojang through SAF
      is a meaningfully bigger, riskier change (a bug there risks wiping
      real worlds) than a read-only listing or an app-private zip write, so
      it's left as an explicit follow-up rather than rushed.
- [x] Bedrock **Screenshots** panel: `BedrockStorageService.listScreenshots()`
      recursively lists `.jpeg` files under `com.mojang/Screenshots/`
      through the shared SAF grant, into the real `.screenshot-grid`
      markup. Xbox Game Bar's Captures-folder merge (Windows-only) is
      skipped rather than faked, matching `BedrockScreenshotService.cs`'s
      own per-source availability. Bedrock **Instances** panel remains a
      stub — it's a materially different feature (isolated Bedrock
      account-folder copies, `Home.BedrockInstances.cs`) rather than a
      plain SAF list, and is lower priority than what's already real.
- [ ] Diff `Components/*.razor` output markup against `panels.js`/`app.js`
      generated HTML for spinners, tooltips, error dialogs, and toggles to
      close the "Unverified" rows in `07_UI_PARITY.md`.
- [ ] Add a shared `js/apiClient.js` wrapper (Android) and `ApiClientBase`
      (Windows) to remove duplicated fetch/HTTP boilerplate — see
      `05_REFACTOR_PLAN.md`.

## Medium
- [x] Modrinth-only modpack install (`ModpackInstallService.kt` +
      `js/modpackinstall.js`) — downloads a project's latest `.mrpack`,
      extracts `overrides/` and every listed file into a brand-new Java
      instance (`JavaInstanceService.directoryFor`/`setVersion`, both added
      alongside this). CurseForge packs stay disabled (would need
      CurseForge's own API-keyed project/file-ID resolution ported
      natively, not worth duplicating the existing JS client for a first
      pass). Mod-loader installation (Fabric/Quilt/Forge/NeoForge) is NOT
      done — Forge/NeoForge's installer-jar-as-subprocess step has no
      Android equivalent at all (Pojav embeds its JVM via JNI, there's no
      spawnable `java` binary), and shipping Fabric/Quilt-only support
      would make one loader silently work and the others silently not,
      which is worse than surfacing the gap — the UI tells the user which
      loader the pack needs and that they must add it manually via Pojav's
      own version-install flow before launching.
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
- [x] Add a `localStorage`-backed TTL cache for CurseForge/Modrinth/News
      fetches on Android (`js/httpcache.js`'s `HttpCache.fetch(key, ttlMs,
      fetcher)`, wired into `CurseForge.search()`/`Modrinth.search()` at a
      5-minute TTL and `NewsFeed.fetchPosts()`/`fetchReleases()` at 10
      minutes). Same intent as `NewsService.cs`'s on-disk cache: avoids
      re-hitting CurseForge's/Modrinth's rate-limited search APIs on every
      panel re-open or unchanged load-more click, and falls back to the
      last-good response (any age) when a fresh fetch throws. Deliberately
      NOT applied to `AnnouncementFeed` — already documented above as a
      silent-fail-only, no-cache contract on purpose.
- [ ] Add settings schema versioning + migration to `JsonStore`/`LauncherSettings` (Windows).
- [ ] Extract `IThirdPartyClient` interface for Flarial/Latite/OderSo/LeviLamina services (Windows).
- [ ] Update `ClientInjectionService.kt` doc comment and in-app Settings →
      Clients copy to reflect the researched non-root path, once the
      injection work itself lands (content change, not a standalone fix).

## Low
- [x] Port onboarding wizard to Android (`onboardingModalHtml` in
      `panels.js`, real `.modal-box`/`.btn-accent` markup from
      `Pages/Home.razor`) — edition pick + display name, first run only,
      gated on the new `onboardingCompleted` setting. Drops desktop's
      "Import existing .minecraft" step: that assumes a prior install of
      Mojang's official Java launcher on the same machine, which has no
      Android equivalent at all (nothing to import from).
- [x] Port announcement banner to Android (`AnnouncementFeed.fetch()` in
      `javaedition.js`, same remote `announcement.json`/silent-fail contract
      as `Services/AnnouncementService.cs` — no cached fallback, since a
      stale "maintenance in progress" banner would be actively misleading).
      Real `.announcement-banner` markup, dismiss persisted per-id in
      settings the same way desktop does.
- [ ] Add localization/string-table support to Android (currently hard-coded English).
- [ ] Add LevelDat editor to Android (depends on world-listing work above).
- [x] Add SAF folder-open shortcuts to Android (`BedrockStorageService.folderUri()`
      + `MainActivity.kt`'s `openBedrockFolder()`) — Worlds/Packs/Screenshots
      panel headers now have a real "Open folder" button (best-effort: not
      every device has a file manager that handles
      `vnd.android.document/directory`, same as desktop's own shortcut
      failing quietly with no Explorer-equivalent registered).
- [x] Add a Logs panel capture pipeline + mclo.gs sharing to Android
      (`service/LogService.kt` lists/reads the active instance's `logs/` +
      `crash-reports/` — same dirs `JavaInstanceService.kt` already resolves
      for everything else — `js/logs.js` ports `LogService.cs`'s exact
      redaction patterns and mclo.gs upload, real markup/CSS from
      `Components/LogsPanel.razor`, reachable from the Java Profile panel
      and global search same as desktop).
- [x] Add lightweight session-timer service for Android Stats panel —
      `MainActivity.kt`'s `onResume()` is the closest signal Android gives
      for "the launched game Activity closed" (there's no real callback),
      so `App.recordLaunchStart()`/`onResumeFromGame()` treat a return to
      this Activity after 10+ seconds as one play session. Approximate by
      nature (matches any launcher without an OS play-time API), documented
      as such in both files' comments.

### Critical fix found while adding the above
While wiring `onResume()`'s `window.App.onResumeFromGame()` callback,
discovered that **every existing native→JS callback was silently broken**:
`MicrosoftAuth`/`DiscordAuth`/`LauncherUpdate`/`BedrockStorage`/
`CustomDllPicker`/`App` are all declared as top-level `const`, which does
NOT attach to `window` in a classic (non-module) script — so every
`window.X.method(...)` call from `MainActivity.kt` (sign-in completion,
update progress, storage-access results, custom DLL pick results, and now
the session timer) was a silent no-op this entire session, since
`window.X && ...` just short-circuits to false with no error. Fixed by
adding an explicit `window.X = X;` assignment at the end of each affected
file (`xboxauth.js`, `discordauth.js`, `updater.js`, `bedrockstorage.js`,
`customdll.js`, `app.js`). This was never caught by CI (build-only, no
runtime JS execution) or by this session's own device-less verification —
worth specifically re-testing Microsoft/Discord sign-in and the update
flow once a device is available, since those are the highest-value
features this bug affected.
- [ ] Audit `Home.razor`'s `StateHasChanged()` call sites for over-broad re-renders (Windows).
- [ ] Cache rendered panel HTML strings in `panels.js`, invalidate only on
      underlying data change, to reduce panel-switch jank.
- [x] Verify search-input debounce exists on both platforms' CurseForge/Modrinth search.
      Confirmed: `Pages/Home.razor.cs` cancels/re-arms `_cfDebounceCts`/
      `_mrDebounceCts` per keystroke; Android's `app.js` does the same with
      plain `setTimeout`/`clearTimeout` (`cfDebounce`/`mrDebounce`, both
      350ms) in its input-change handler. No change needed.
- [x] Confirm skin texture caching exists to avoid re-fetching from Mojang CDN each panel open.
      Neither platform has an explicit skin-texture cache — `Services/
      SkinService.cs`/`SkinLibraryService.cs` and `js/skinlibrary.js` both
      just point an `<img>`/element at Mojang's `textures.minecraft.net`
      URL directly and rely on the platform's own HTTP image cache
      (WebView2 on Windows, Android's WebView here) honoring Mojang's cache
      headers. Android already matches desktop's actual behavior, so
      nothing to add.
</content>
