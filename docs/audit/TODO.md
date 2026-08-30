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
      management, so a bad import always has a way back. A "download from a
      direct URL" path was added too (`downloadApk()` in the same class) for
      fetching a build from a mirror/host the user points it at — a plain
      HTTP GET, still no credentials of any kind.

      **Update (2026-08-30): reinstated at explicit project-owner
      direction.** `utils/GPlayAPI.kt` and `utils/Downloader.kt` were
      restored, and a Play Store ownership-check feature
      (`utils/PlayStoreValidator.kt`, `AndroidBridge.checkBedrockOwnership`
      in `MainActivity.kt`) was added on top. The owner was told the risk
      described below in full — a Google account AAS/master token stored
      client-side is full-account-compromise blast radius (email, payment
      methods, everything), not scoped to "download one APK" — and
      overruled it for their own app. This is their call to make; don't
      re-litigate it or revert this again without them asking.

      Mitigation added alongside the reinstatement: the app previously had
      `android:allowBackup="true"` with no backup-content rules referenced
      in `AndroidManifest.xml`, meaning the `accountData` SharedPreferences
      file (`accountEmail`/`accountToken`, read/written by
      `GPlayAPI.getAuthData`) would have been swept into Android's own
      full-data backup (auto cloud backup pre-API 31, `dataExtractionRules`
      cloud-backup/device-transfer on API 31+) — a backup can be restored
      onto any device, which is a strictly worse exposure than the token
      merely living in this app's sandbox. Fixed: `res/xml/backup_rules.xml`
      and the new `res/xml/data_extraction_rules.xml` both now exclude
      `sharedpref` path `accountData.xml`, wired via
      `android:fullBackupContent`/`android:dataExtractionRules` in the
      manifest. This doesn't reduce the core risk (the token is still
      readable by anything with root or a backup exploit of the app's own
      process/storage) but it closes the cheap, real gap of the token
      leaving the device entirely through Android's own backup mechanism.

      Original rejection note, for context (no longer the operative
      decision): this was tried once earlier this session
      (`utils/GPlayAPI.kt`, wrapping Aurora Store's
      `PurchaseHelper`/`AuthHelper` with an AAS token) and reverted
      immediately as the exact mechanism rejected above, just a concrete
      implementation of it. That technical analysis was correct — it's the
      owner's decision to accept the risk anyway that changed, not the
      risk itself.
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
- [x] Diffed `Components/*.razor` output markup against `panels.js`/`app.js`
      generated HTML to close the "Unverified" rows in `07_UI_PARITY.md`.
      Findings: `Components/Spinner.razor` and `Components/Modal.razor` are
      never actually referenced anywhere in `Pages/Home.razor` — every real
      spinner/modal on desktop is a hand-written `<span class="spinner">`/
      `.modal-overlay > .modal-box` (no `role="status"`/`aria-live`/
      `role="dialog"`/`aria-modal` anywhere in the real markup either), which
      Android's `panels.js` already matches byte-for-byte — no gap, those
      two rows are resolved as "already matches, component just unused."
      Tooltips: both platforms already share the exact same CSS-driven
      `[data-tooltip]` system (`app.css`), not a native browser tooltip —
      also resolved, no gap. Error dialogs: real gap found and fixed — the
      Servers panel's header "Add" button was a dead stub with no handler,
      and there was no add/edit-server flow at all (`Pages/Home.razor`'s
      real "ADD/EDIT SERVER MODAL" region with name/address/port fields and
      `Home.Panels.cs`'s `SaveServerModal()` validation). Added the same
      modal (`panels.js`'s `serverModalHtml()`, real `.modal-box`/
      `.modal-input`/`.error-text` markup) with matching validation (address
      required, port 1-65535) and edit support for existing saved servers.
      The other desktop `error-text` sites (Flarial/OderSo/LeviLamina,
      Windows-only third-party clients; per-version-row install errors for a
      feature Android's read-only Versions list doesn't have) have no
      Android surface to add an error to in the first place — not gaps.
- [x] Add a shared `js/apiClient.js` wrapper (Android) to remove duplicated
      fetch/HTTP boilerplate — see `05_REFACTOR_PLAN.md`. `ApiClient.getJson`/
      `getText` (one `fetch` -> `if (!res.ok) throw` -> parse, matching the
      exact pattern every call site already used) now backs
      `CurseForge.search()`, `Modrinth.search()`/`getLatestFile()`,
      `MojangVersions.fetchManifest()`, `BedrockVersions.fetch()`,
      `GlacierClient.fetchManifest()`, and `NewsFeed.fetchPosts()`/
      `fetchReleases()`. Deliberately NOT applied to `xboxauth.js` or
      `skinlibrary.js`: both map specific failure cases to specific user-
      facing messages (Xbox's `XErr` codes, Mojang's 204/404 "no such
      player", a network-exception vs HTTP-error distinction) that a generic
      `getJson()` would flatten into a worse, less actionable error — not
      duplicated boilerplate, real per-endpoint behavior worth keeping.
      `ApiClientBase` (Windows) is a separate change to `Services/*.cs` on
      the desktop side and out of scope for this Android-focused pass.

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
      Skipped this pass: real native-injection code (mismatched-UID cross-
      process memory access, native library loading into another app's
      process) is exactly the class of change this environment can't safely
      ship blind — no device/emulator here to verify it doesn't crash or
      brick the target app, same reasoning already applied to Bedrock
      Backups restore above. Landing this untested is worse than leaving the
      documented root-only path in place.
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
- [x] Wire Java Profile skin-viewer UI to the already-working sign-in flow
      (`js/skinviewer.js`'s `GlacierSkin`, a direct port of `wwwroot/js/
      interop.js`'s `window.glacierSkin` — same vendored-bundle-first/CDN-
      fallback `skinview3d` load, same visibilitychange/IntersectionObserver
      render-pause). `panels.js`'s `skinViewerHtml()` renders the real
      `.skin-viewer`/`.skin-stage`/`.skin-pills` markup from `Components/
      SkinViewer.razor` — 2D static body render (mc-heads, crafatar fallback)
      by default, 3D interactive canvas + Steve/Alex model pills, persisted
      via new `skinViewerMode`/`skinViewerModel` settings the same way
      desktop persists `SkinViewerMode`/`SkinViewerModel`. Cape display is
      NOT ported: Android has no cape/wardrobe data anywhere in
      `AndroidBridge` or settings for the 3D model to show, so desktop's
      cape-cycling pill is left out rather than faked.
- [ ] Split `panels.js` and `app.js` by concern (Bedrock/Java/Settings/state/router).
      Skipped this pass: a pure mechanical split across two ~1900-line files
      with dozens of cross-references is exactly where a silent typo/missing-
      export slips through without a build+run to catch it — no Gradle/
      emulator available here. Leaving both files intact rather than risk an
      unverifiable refactor across the whole UI.
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
      Skipped: explicitly a Windows-side `Services/*.cs` change, not Android.
- [ ] Extract `IThirdPartyClient` interface for Flarial/Latite/OderSo/LeviLamina services (Windows).
      Skipped: explicitly a Windows-side refactor (those four clients have no
      Android equivalent at all — desktop-only third-party injectors).
- [ ] Update `ClientInjectionService.kt` doc comment and in-app Settings →
      Clients copy to reflect the researched non-root path, once the
      injection work itself lands (content change, not a standalone fix).
      Still blocked: the non-root injection item above wasn't landed this
      pass either, so there's nothing new to document yet.

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
      Skipped this pass: genuinely portable but a whole-app undertaking on
      its own (every user-facing string in `panels.js`/`app.js`/`index.html`
      would need extracting into a string table plus a locale-switch
      mechanism) — too large to fold into this gap-closing pass alongside
      everything else without shortcutting it into a half-done stub.
- [ ] Add LevelDat editor to Android (depends on world-listing work above).
      Skipped this pass: an NBT *write* path (vs. `BedrockNbt.kt`'s existing
      read-only parser) risks corrupting a real world file with no
      device/emulator here to verify round-trip correctness — same
      read-vs-write risk asymmetry already called out for Bedrock Backups
      restore above, so left as a real follow-up rather than shipped blind.
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
      Skipped: explicitly a Windows-side (`Home.razor`) profiling pass, not Android.
- [ ] Cache rendered panel HTML strings in `panels.js`, invalidate only on
      underlying data change, to reduce panel-switch jank.
      Skipped this pass: a correct cache needs a real invalidation key per
      panel (settings changes, instance list changes, live network fetches
      landing async, etc.) threaded through every one of ~25 panel bodies —
      getting that wrong produces a worse bug (a panel silently showing
      stale data) than the jank it fixes, and there's no device here to feel
      whether the jank is even still noticeable after it. Left as a real,
      scoped follow-up rather than a half-verified perf change.
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
- [x] Android: per-world Bedrock backups (`BedrockBackupService`'s worlds/<id>
      half), Bedrock storage migration to/from app storage
      (`BedrockStorageMigrationService`, new), and generic level.dat NBT tag
      read/write (`BedrockNbt.getPath/setPath/tagToJson/jsonToTag`,
      `LevelDatService.readTag/writeTag`) plus a recursive world-file
      listing (`BedrockStorageService.listWorldFiles`) — architectural ideas
      from LeviLaunchroid's InstanceBackupManager/StorageMigrationManager,
      reimplemented fresh for this codebase's SAF/DocumentFile model, no
      LevelDB parsing added (out of scope, see BedrockNbt's doc comment).
      **Update (2026-08-30):** the fourth requested feature, Play Store
      license/ownership validation via GPlayAPI/PurchaseHelper, was
      initially refused for the reason below, but the project owner was
      told that reasoning explicitly and overruled it for their own app.
      It's now implemented as `utils/PlayStoreValidator.kt` +
      `AndroidBridge.checkBedrockOwnership`; see the entry above this one
      for the full update and the backup-exclusion mitigation added
      alongside it. Original refusal reasoning, still accurate as
      technical analysis: it's the same rejected mechanism the entry above
      already covers (a Google account AAS/master token stored in this
      app), just framed as "check ownership" instead of "download an APK."
</content>

## Follow-up pass (2026-08-30, separate session)
- [x] Full gap-list survey (Windows `Services/*.cs` vs Android `service/*.kt`
      + wwwroot JS bridge call sites) turned up one remaining real, portable
      gap after everything above: `Services/ServerPingService.cs` (Bedrock
      RakNet unconnected-ping over UDP, Java Server List Ping over TCP) had
      no Android equivalent — the Servers panel's saved/suggested rows never
      showed the `.server-ping`/`.ping-dot` online/offline/player-count
      badge that `app.css` already styles for it (that CSS was dead code on
      Android). Added `ServerPingService.kt` (plain blocking sockets, same
      two wire protocols, same always-resolves-to-offline-on-any-failure
      contract as the Windows service) behind `Bridge.pingServer(host,
      port)`, wired into `panels.js`/`app.js`: pings every saved + suggested
      server when the Servers panel opens, staggered via `setTimeout` since
      this app's native bridge has no async-callback mechanism to avoid each
      ping blocking the JS thread outright.

      Every other Windows `Services/*.cs` file was confirmed already ported
      (by name/functionality, not always 1:1 file naming) or genuinely
      Windows-only with no Android surface to add: `FlarialService`/
      `LeviLaminaService`/`LeviLaminaModsService`/`LunarBadlionService`/
      `OderSoServices` (desktop-only third-party client injectors, no
      Android equivalent exists — `ClientInjectionService.kt`'s root-based
      injection is the closest analogue and is already wired);
      `GameConsoleService`/`GameLauncher`/`JavaGameLauncher` (WebView2
      console window + Windows process launching — Pojav's embedded JVM has
      no spawnable process to attach a console to); `JsonStore`/
      `HttpFactory`/`GitHubApiCache`/`LauncherUtilityService` (internal
      Windows plumbing, not user-facing features to port);
      `StoreInstallService` (Microsoft Store-specific); `NbtIo`/
      `LevelDatEditorService`/`BedrockWorldService`/`BedrockInstanceService`
      (superseded by `BedrockNbt.kt`/`LevelDatService.kt`/
      `BedrockStorageService.kt`'s own SAF-based design, not a 1:1 class
      match but the same functionality); `JavaInstallService`/
      `JavaVersionService`/`JavaRuntimeDownloadService`/`JavaModAnalyzer`/
      `JavaModLoaderService` (Pojav's bundled JVM + `JavaInstanceService.kt`/
      `ModpackInstallService.kt` already cover instance/mod-loader
      management for what's portable — mod-loader *installation* itself is
      the one already-documented gap above, unchanged this pass).
      `DiscordRpcService`, `LocalizationService`, and the `Stats`-panel
      session-length tracking were re-confirmed as the already-documented
      gaps/approximations above (Rich Presence has no Android IPC
      equivalent; localization is a whole-app undertaking; Stats uses the
      `onResume()`-based approximate timer) — no change to those verdicts.
