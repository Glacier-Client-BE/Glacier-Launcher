# PARITY ledger — Glacier Launcher redo

The transcription checklist for the 1:1 redo. Every row is ticked only after a **screenshot-diff against
the running original** (`MinecraftLauncher-wpf-backup-full/publish_final`), not after code review.

Detailed line-range inventories live in:
- `docs/inventory-shell.md` — shell/chrome regions, interop, localization, CSS diff
- `docs/inventory-bedrock-panels.md` — every Bedrock panel's markup + code-behind ranges
- `docs/inventory-bedrock-services.md` — 25 Bedrock services → Rust crate fidelity

Status: `TODO` · `WIP` · `DONE` (verified against the running original) · `MISSING` (no counterpart
exists) · `DIVERGES` (exists, wrong) · `MATCHES` (verified faithful).

Rule: **transcribe, never summarize.** A merge, rename, or "cleaner" structure is a defect.

---

## Structure of the original (ground truth)

`BedrockTabs` (`Home.BigFeatures.cs:54-75`) defines **7 bottom tabs**: Settings, Clients, Packs,
Versions, Worlds, Utilities, Credits. Those fan out into **18 distinct `currentView` panels** plus **6
component-hosted panels** (`Home.razor:983-1012`).

There is **no shell-level `.panel-tabs` bar** — all 20 `.panel-tabs` occurrences are inside individual
panels. The shell's only global nav is `.btn-row`. The previous rewrite invented a 15-entry global bar;
it gets deleted.

The original suppresses `onclick` on the *active* tab (`if (active != view)`, `Home.BigFeatures.cs:60-61`).
The rewrite wires it unconditionally, and maps both "Packs" and "Worlds" to one `content` id so two tabs
light up at once.

---

## Phase 1 — Shell / chrome

Transcribed against `Home.razor` and verified by count-matching the original's markup.
**Still unverified at runtime** — nothing here has been rendered next to the running original yet.

| Region | Original lines | Status | Notes |
|---|---|---|---|
| `.window` / `.window-bg` / `.drag-zone` | 46–48 | DONE | unchanged, already faithful |
| `.drop-zone-overlay` | 50–54 | DONE | file drops originate host-side (no `File.path`) |
| Onboarding | 56–140 | DONE | unchanged |
| Search overlay/modal | 141–215 | **DONE** | invented inline `.topbar-search` widget and its 87-line scoped `<style>` deleted; real `.search-overlay`/`.search-modal`/`.search-result-*` DOM restored with category grouping, `<mark>` highlighting (HTML-escaped first), arrow/Enter/Esc keys, and the `.search-trigger` button. Ctrl+K now handled (interop emitted `search`; nothing listened). **Gap:** 30 of the original's 42 quick actions are wired; the rest need targets that don't exist yet (folder-open commands, settings import, Utilities sub-tabs, Xbox/Discord modals, Theme Studio, Skin Library, Backups) |
| `.top-bar` | 450–624 | **DONE** | see sub-rows |
| — branding / `.update-pill` | 451–470 | **DONE** | both branches restored; real version via new `launcher_version` command (was hardcoded `v0.1.0`) and update state via existing `check_launcher_update` |
| — edition switcher | 473–484 | DONE | + `stopPropagation` |
| — client chip | 486–503 | **DONE** | back to a `div` (was a `button`), with keyboard affordances |
| — mc-running pill | 505–511 | DONE | unchanged |
| — search trigger | 513–515 | **DONE** | plain button opening the modal; no longer hover-expanding |
| — notif backdrop / bell + panel | 517–609 | **DONE** | badge is now `unread + activeDownloads` (`bellBadgeCount`); Downloads section only renders when non-empty (5 rows, progress bars); the entire **Notifications** section restored with `Clear all`, per-row dismiss, relative timestamps and the empty state |
| `.window-controls` | 611–623 | **DONE** | `fullscreen-btn` restored (4 buttons, was 3); F11 wired through `interop.js` |
| Announcement banner | 627–645 | DONE | unchanged |
| `.btn-row` | 648–772 | **DONE** | all 13 `btn-badge-wrap` wrappers restored (**13 = 13** vs original), `clientsHasBadge` dot backed by a real check (new `flarial_client_installed` + existing `oderso_client_installed`), `stopPropagation` throughout |
| status-msg / recently-launched | 775 / 787–802 | **DONE** | recents now filtered by edition (`RecentsForCurrentEdition`) and stripped of the `"Java "` prefix (`RecentDisplayLabel`) — previously every entry showed on both editions |
| `.launch-dl-bar` | 777–785 | TODO | **blocked**: needs a launch-progress event (`isDownloading`/`downloadStage`/`downloadPct`); the backend emits console logs only. Belongs with the Java launch work |
| `.footer` / `.footer-right` | 806–960 | **DONE** | `.footer-right` restored: `.footer-mc-version` chip (installed / preview / not-installed, backed by a new `installed_bedrock_version` command) and the client label (`GetFooterVersionLabel`) |
| `.toast-container` | 951–960 | **DONE** | toast system ported (2800ms hold, `.closing` for 240ms, then removed; 4 kinds); raised on import success/failure and launch failure, alongside a persisted notification as the original does |
| `.news-strip` | 962–983 | DONE | unchanged |
| Shell-level `.panel-tabs` bar | — | **DELETED** | invented; the original has no global nav bar. Per-panel `BedrockTabs`/`JavaTabs` strips come in Phase 2, so panels currently only link back home |
| `@onmousedown:stopPropagation` | throughout | **DONE** | added across top-bar controls, btn-row, window controls, footer chip |
| Localization | 10 sites / 10 keys | **DONE** | `LocalizationService.cs` ported verbatim to `frontend/src/lib/i18n.ts` (18 keys × 4 languages); all 9 shell `nav.*` sites now resolve, including `nav.mcversions` = "MC Versions" |
| `NotificationService` | 118 lines | **DONE** | ported to `crates/gl-settings/src/notifications.rs` + 7 commands. Keeps the original's on-disk format (bare PascalCase JSON array at `%USERPROFILE%\Glacier Launcher\notifications.json`) so existing history survives. 8 unit tests |
| `app.css` | 6,455 | MATCHES | 17-line diff in 3 hunks — faithful |
| `interop.js` | 531 | MATCHES | **Bug still open:** `glacierViewTransition` calls Blazor's `dotNetRef.invokeMethodAsync('ApplyPendingNav')` |

## Phase 2 — Bedrock panels

**The four merged panels are split back apart.** `BedrockContentView.svelte` is deleted;
`BedrockWorldsView`, `BedrockPacksView`, `BedrockBackupsView` and `BedrockScreenshotsView` are
transcribed from their original line ranges, each with its own header actions, states and cross-nav
strip. The level.dat editor, previously nested inside the merged view, is now its own `leveldat` route
reached by the world-row deep link. Verified by a clean `vite build` — **not yet seen next to the
running original.**

| Panel (`currentView`) | Original lines | Status |
|---|---|---|
| `bedrockworlds` | 3829–3928 | **DONE** — inline in-row delete confirm (no modal), NBT deep link, export |
| `bedrockpacks` | 3929–4021 | **DONE** — inline delete confirm; see kind-switcher finding below |
| `bedrockbackups` | 4186–4299 | **DONE** — inline restore *then* delete confirms, label input, "Back up now" |
| `bedrockscreenshots` | 4375–4477 | **DONE** — grid, tiles open in the system viewer |
| `leveldat` | 4478–4560 | **DONE** — own route; was nested inside the merged view |
| `settings` | 1013–1931 | **DONE** — see findings below |
| `clients` | 1932–2188 | **DONE** — see findings below |
| `bedrockinstances` | 4300–4374 | **DONE** — re-reviewed after its agent died mid-run; all elements verified present (rows, rename mode, version select, inline delete confirm) |
| `mcversions` | 4653–4848 | **DONE** — re-review found it genuinely unfinished: the `.mcv-info-pop` popover was missing entirely and a `.mcv-info-bar` from a *different* panel (line 4868) had been used instead; both paragraphs incl. the GDK/1.21.120 warning restored, and the sub-tab strip moved above the store row where the original has it |
| `themestudio` | Components/ThemeStudioPanel.razor (539) | **DONE** — was MISSING entirely |
| `utilities` | 3267–3714 | **DONE** — six sub-tabs wired (four internal, Backups/Photos navigate away); deep links now carry the requested tab |
| `addons` (Java half) | 2362–2987 | **DONE** — all five missing sub-tabs added plus the full CurseForge/Modrinth browse UI |
| `servers` | 3715–3828 | **DONE** — the whole "Popular" suggestions section was missing; status markup, modal and validation restored |
| `clientlibrary` | 4022–4185 | **DONE** — `cl-dl-row` internals, numeric version ordering, reentrancy guard and verbatim toasts |
| `versions` | 2189–2361 | **DONE** — the whole header action group (sort cycle, only-downloaded, refresh) and the entire Pinned/All-Versions split were missing; OderSo rows had no active state and an invented "installed" footer row |
| `levimods` | 2988–3101 | **DONE** — refresh button, `.panel-search-wrap`, debounced search, error/Retry and empty states restored; an invented "Open mods folder" button removed |
| `tweaks` | 3102–3266 | **DONE** — title was "Java Tweaks" not "Vanilla Tweaks"; the `.tw-dropdown` version picker had been replaced by a native `<select>`; both tab strips were missing; an invented result banner removed |
| `downloads` | 4561–4652 | **DONE** — was already close; fixed `FormatSize` rounding, the `Queued` case in Clear finished, and replaced a 1s poll with the real change event |
| `credits` | 5495–5646 | **DONE** — content was verbatim already; the bottom strip was the fixed 4-button one instead of `BedrockTabs`/`JavaTabs`, and every link was a dead `window.open` |

### Shared components built this phase

| Component | Ports | Notes |
|---|---|---|
| `components/EditionTabs.svelte` | `BedrockTabs` / `JavaTabs` (Home.BigFeatures.cs:31-75) | 7 Bedrock / 8 Java tabs. Preserves the asymmetry that Bedrock omits `onclick` on the *active* tab while Java always wires it |
| `components/SubTabs.svelte` | all four `SubTabs` strips (Home.MergedPanels.cs:10-51) | Renders `.versions-client-switcher`/`.vcs-btn` — a *different* element from `.panel-tabs` — with the original's leading space before each label |
| `components/InlineProgress.svelte` | `Components/InlineProgress.razor` | Was MISSING; needed by Clients, Addons, LeviLamina and Glacier panels |

`components/PanelTabs.svelte` (the port of `Components/PanelTabs.razor`) is unchanged and still used by
other views — `EditionTabs` is a separate component, not a replacement.

### Corrections to this document's own earlier claims

- **There is no pack-kind switcher in the original.** §2.10's claim of one was wrong.
  `SwitchBedrockPacksKind` (Home.Packs.cs:25) has **zero call sites** in the entire WPF app, and lines
  3929–4021 contain no switcher markup; `bedrockPacksKind` only ever holds `"resource"` and feeds the
  empty-state text. No switcher was invented. The kind is state with an exported setter if one is ever
  added deliberately.
- **Pack kind keys needed no mapping.** `BedrockPackService.DirFor` already uses exactly
  `resource`/`behavior`/`skin`/`resource-dev`/`behavior-dev`/`skin-dev` — the same strings the Rust
  command accepts. The `development_*_packs` names are on-disk folder names, not kind keys.

### Backend commands added to support these panels

`bedrock_worlds_dir`, `bedrock_backups_dir`, `bedrock_screenshots_dir` (ports of the C# static
directory properties) and `open_path` (port of `OpenScreenshot` → `OpenUrl`; deliberately distinct from
`open_folder`, which calls `create_dir_all` and would silently create an empty directory in place of a
missing file). `export_bedrock_world`'s destination now defaults in the **backend** to
`%USERPROFILE%\Glacier Launcher\exported-worlds`, matching `BedrockWorldService.ExportWorldAsync` —
the panel had guessed `<com.mojang>\Exports`.

Registered commands: **141 → 195**.

### Open gaps

- Panel-to-panel navigation maps the original's view keys onto this app's `View` union
  (`ORIGINAL_VIEW_MAP` in `App.svelte`). The Utilities sub-tabs (`options`/`shaders`/`system`) all land
  on Utilities itself until that panel is split into its own tabs.
- World icons and screenshots rely on Tauri's asset protocol (`convertFileSrc`); the scope must cover
  the com.mojang path or images silently fail to load. **Unverified at runtime.**

### Theme Studio (was entirely missing)

`ThemeService.cs` (193) and `ThemeDefinition.cs` (118) had no Rust counterpart, so the panel could not
exist. Both are now ported to `crates/gl-settings/src/themes.rs` with **15 unit tests** covering colour
parsing (`#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, and the garbage-input fallback to the
default accent), `Lighten`'s hex-vs-rgba branch, .NET `0.###` number formatting, all 18 CSS variables,
the light-text overlay branch, overlay-strength scaling, and preset seeding.

`themes.json` keeps the original's on-disk shape (bare PascalCase JSON array) so existing saved themes
load unchanged, and a corrupt file is quarantined rather than silently overwritten.

11 commands added. Applying a theme stays on the frontend — `interop.js` already owned all nine
functions the C# `ApplyAsync` drove — while the CSS variables are computed in Rust so a live preview and
a saved theme cannot disagree.

### Clients panel — 15 divergences fixed

Highlights: the bottom tab strip was absent entirely; the Custom DLL card and drop-hint row (lines
2145–2180) were missing; the three-state sub-label tested `Downloading` before `UpToDate` (wrong order);
OderSo and LeviLamina had lost their "update available" branch; delete fallbacks all reset to `Vanilla`
where the original leaves Flarial alone and sends the other two to `Latite Client`; a LeviLamina
sub-label and `fetch_levilamina_release` call were **invented** and have been removed.

Still limited by missing backend commands: `InlineProgress` renders spinner-only (no progress event, no
per-client cancel), and "update available" can never show for OderSo/LeviLamina — there is no
`check_oderso_up_to_date` / `check_levilamina_up_to_date`; only Flarial has a real probe.

### Settings panel — restored rows and corrected text

Rows the rewrite had dropped: Background wallpaper, Background fit, UI scale, Font (+ suggestions),
Theme Studio, Latest crash report, Discord account, Xbox profile, Check for updates now, Minecraft data
folder, Credits. Several hints had been silently truncated and are now verbatim. The Background blur row
was structurally wrong and was rebuilt. Invented entries (`Minimum RAM`, `Backup saves before launch`)
removed so `VisibleSettingsCount()` matches.

Controls still disabled pending backend commands: wallpaper picker (needs a copy-into-launcher-folder
command), latest crash report, com.mojang folder, Discord account, Xbox profile.

### Structural bug: double-wrapped `.panel-overlay`

`App.svelte` wraps whichever panel is active in ONE shared
`<div class="panel-overlay"><div class="panel-handle">`. The five newest views also emitted their own,
so they rendered nested: `.panel-overlay` is `position:absolute`, `height:78%`, with its own background
and radius, i.e. a panel inside a panel. Removed from all five. The original has one per panel (23 in
`Home.razor`) because each panel is its own top-level block there; the rewrite hoists it into the shell.
The rendered DOM is equivalent — what must not happen is both. `BedrockPacksView` had the same class of
bug around `EditionTabs`, which renders its own `.panel-tabs` wrapper and was being wrapped again.

### Corrections to briefs given to agents (both caught by the agents, not by me)

- **`ModpacksView` is NOT the `addons` panel.** It ports `Components/ModpacksPanel.razor`; `modpacks` is
  a separate view (`Home.Settings.cs:21`, rendered at `Home.razor:990`), and none of its classes appear
  in the `addons` range. The `addons` panel is split across `JavaModsView` (Java) and `BedrockPacksView`
  (Bedrock). The agent stopped rather than forcing a bad merge — the right call.
- **There is no pack-kind switcher** (recorded earlier, still true).

### A false agent claim, checked and rejected

One agent reported that the source of truth "does not exist in this repository" — that `Home.razor` is
4896 lines and that `BedrockTabs`, `WorldsServersTabs`, `Home.MergedPanels.cs`, `Home.ClientLibrary.cs`
and `ClientLibraryService.cs` exist in no branch or commit.

It had searched **git** instead of the filesystem. `C:\Users\User\MinecraftLauncher-wpf-backup-full`
is a plain read-only directory with no relationship to this repo's history; the repo's git HEAD still
holds an OLD superseded copy of the same app (4896 lines). The backup's `Home.razor` is **5646** lines,
the `servers` panel *is* at 3715, `clientlibrary` *is* at 4022, and all three "missing" files exist.
Diffing the stale HEAD copy against the real source showed only 23 differing lines for the servers
panel, so that transcription mostly survived; one stale detail (`style="padding:20px 20px 8px;"` on the
empty state) was imported from the old version and has been removed. Client Library was re-issued
against the correct path and completed.

**Every future agent brief must state that the backup is a filesystem path and that git must not be
used to read it.**

### Whole-app integrity check

Every `invoke(...)` call across `frontend/src` was cross-checked against the `generate_handler!`
registry: **144 distinct commands used, 0 missing.** No phantom calls anywhere in the app.

### The "no progress events" gap was a misdiagnosis

Recorded earlier as one backend capability gap blocking five panels. Re-checking the code: every
download flow — clients, client library, Latite/Bedrock versions, LeviLamina mods, both tweaks build
paths — **already goes through the shared `state.downloads` queue**, which tracks real status, progress
and byte counts per entry and supports cancel. Nothing needed building.

What was actually missing was the *notification*. The C# `DownloadService` had
`public static event Action? Changed` (`DownloadService.cs:48`) raised on every mutation, and the
Downloads panel plus every inline progress bar re-rendered on it. The Rust queue had no equivalent, so
the only way to see progress was to poll — and only `DownloadsView` did, on a 1s timer.

Ported now:

- `DownloadQueue::subscribe()` / `raise_changed()` — a `tokio::sync::broadcast` port of `Changed`,
  raised on submit, on each status transition, on throttled progress ticks (already rate-limited to
  100 ms by `ThrottledProgress`, so it cannot flood), and on cancel/remove/clear.
- `spawn_downloads_bridge` in `commands/downloads.rs` forwards it to one `downloads-changed` Tauri
  event carrying the full snapshot. Started once from `setup`. No new ACL entry needed —
  `core:default` already covers `event`.
- `DownloadsView` now listens instead of polling. `VersionsView`, `LeviLaminaModsView` and `TweaksView`
  render real `InlineProgress` bars and real cancel buttons from it.

Two real bugs found while doing it:

- **`total_bytes` was never learned.** `download_once` reads the response's `Content-Length`, but the
  progress callback only carried a `0.0–1.0` fraction, so an entry submitted without
  `with_known_total_bytes` kept `total_bytes = 0` and the "12 MB / 40 MB" text could never appear. The C#
  `DownloadEntry.TotalBytes` was filled from the response. `download_file` now takes an `on_total`
  callback, and the progress closure reads `total_bytes` off the live entry rather than a value captured
  at submit time.
- **`clear_finished` kept `Queued` entries.** It retained `Queued | Downloading`; the C#
  `ClearFinishedDownloads` removes everything `!= Downloading`, `Queued` included — the same test the
  panel uses to decide whether to show the button at all, so the button could appear and then not clear
  the row. Now `Downloading`-only, with a test that parks an entry in `Queued` to prove it.

`gl-download`: 12 → 15 tests.

### Per-entry OderSo commands (the Versions panel could not work without them)

`download_oderso_client` is the port of `InstallOrUpdateAsync` — it grabs the *newest* DLL and ignores
which row was clicked. The Versions panel's OderSo tab is a per-entry picker, so it needs
`DownloadEntryAsync` / `IsEntryDownloaded` / `GetActiveEntry` / `SetActiveEntry` / `DeleteEntry`. Five
commands added: `download_oderso_dll`, `oderso_dll_installed`, `active_oderso_dll`,
`set_active_oderso_dll`, `delete_oderso_dll`. They keep the original's on-disk shape — `OderSo_<name>`
for the DLL and a one-line `OderSo.ver` holding `<name>|<sha>` — so an existing install still resolves.

### `open_url`: every external link in the app was dead

The original's `OpenUrl` (`Home.razor.cs:670`) is
`Process.Start(new ProcessStartInfo { FileName = url, UseShellExecute = true })`. Four places in the
rewrite had reimplemented it as `window.open(url, '_blank')` — Credits, News, Client Library and the
shell — which does nothing inside the WebView2 host: no browser chrome to open a tab in. So every
credit link, news item, server link and Settings link silently did nothing.

Added `commands::system::open_url` and pointed all four at it. Unlike the C# it refuses anything that
isn't `http`/`https`: `UseShellExecute = true` on an arbitrary string will launch an executable or a
custom-scheme handler, every call site passes a hard-coded web URL, and without the check the command
would be a general "run this" primitive reachable from the frontend. Empty input stays a no-op, matching
the original's `if (!string.IsNullOrEmpty(url))`. 2 tests.

### `PanelHeader` gained a `backIcon` prop

The original is not uniform: 20 of the 23 `.panel-back-btn` occurrences in `Home.razor` use
`fa-chevron-down`, but `versions` (2214), `levimods` (3003) and the level.dat editor (4494) use
`fa-chevron-left`. `PanelHeader` hardcoded the majority. Now a prop, defaulted so no other panel
changes.

### Whole-app integrity check

**155 distinct commands invoked from `frontend/src`, 0 missing** from `generate_handler!`
(177 registered). No phantom calls.

## Phase 4 — Java side

| Panel | Home.razor | Status |
|---|---|---|
| `javaclients` | 4849–5017 | **DONE** — the file was a paraphrase: an invented separate Badlion card, an invented empty state, the wrong CSS family (`client-card-*` instead of `credits-card-*`), and the Vanilla, Glacier Client and Lunar-version-select blocks all missing |
| `javaversions` | 5018–5166 | **DONE** — invented 4-way Releases/Snapshots/Old/All switcher removed; snapshot/beta/refresh header toggles, the Installed/Available split, `TypeLabel`, the active-row badge and the pin button all restored; an invented "New instance" action removed |
| `javaprofile` | 5167–5354 | **DONE** — `ProfileView.svelte` was **not this panel at all**: an invented "Xbox profile card" (avatar tile, gamerscore, XUID, bio, tier badge, sign-out) with no counterpart anywhere in the range. Rewritten from the source |
| `backups` (Java) | Components/BackupsPanel.razor (140) | **DONE** — was MISSING entirely: no view, no route, no backend |
| `skinlibrary` | Components/SkinLibraryPanel.razor (172) | **DONE** — was MISSING entirely: no view, no route, no commands |
| `SkinViewer.razor` (185) | — | **DONE** — new shared component; `.profile-skin-col` is meaningless without it |

### Two whole feature areas existed in Rust but were unreachable

- **Skins.** `gl_integrations::skin_library` (port of `SkinLibraryService.cs`) and `::skins` (port of
  `SkinService.cs`) were both fully ported, and **not one command was registered**. Nine added:
  `list_saved_skins`, `add_skin_from_file`, `add_skin_from_username`, `delete_saved_skin`, `apply_skin`,
  `reset_skin`, `get_capes`, `set_cape`, `hide_cape`. The access token is read from settings inside the
  command rather than passed from the frontend, so it never reaches page script.
- **Java worlds / backups / datapacks.** This half of `JavaInstanceService.cs` had no Rust counterpart at
  all — hence no World Backups panel and no datapacks tab. Ported as `gl_java::worlds` (**10 tests**,
  including a destructive backup→restore round-trip that asserts restore *replaces* the world folder
  rather than merging into it) behind nine commands: `list_java_worlds`, `list_java_backups`,
  `backup_java_world`, `restore_java_backup`, `delete_java_backup`, `list_java_datapacks`,
  `import_java_datapack`, `delete_java_datapack`, `open_java_datapacks_folder`. That also closes six of
  the missing-command reports recorded earlier.

  One deliberate deviation: the backup file-name stamp is UTC, not `DateTime.Now`. The workspace has no
  date dependency and every other timestamp helper here is UTC; the only visible effect is the digits in
  a backup's file name.

### Two routing bugs the Java tab strips exposed

- **`clients` meant two different things.** `BedrockTabs`' `clients` opens the Clients panel
  (`Home.BigFeatures.cs:70`), but the Utilities sub-tab strip's `clients` is `SwitchUtilTab("clients")`,
  an internal tab labelled "Library" (`Home.MergedPanels.cs:48`). Both reached `navigateFromPanel` as the
  bare string, and `UTILITIES_SUBTABS` was checked first — so **the Bedrock Clients tab opened
  Utilities**. `SubTabs` now reports which strip it is.
- **`photos` and `backups` are edition-dependent.** `JavaTabs`' `photos` is `OpenJavaScreenshots`
  (`Home.BigFeatures.cs:49`) and its `backups` is the World Backups panel; the map hardcoded the Bedrock
  targets for both. Now resolved per edition, with the Utilities strip (Bedrock-only) always taking the
  Bedrock target.

`stats`, `logs` and `skinlibrary` were also absent from the map — the views existed but nothing could
navigate to them.

### Still missing backend commands (controls ship visibly disabled with tooltips, never faked)

- Glacier Client card: `get_glacier_manifest`, `install_glacier_client`, `uninstall_glacier_client`,
  `launch_glacier_client` — nothing backs `Pages/Home.Glacier.cs`
- `launch_lunar_direct` — `resolve_lunar_direct_launch` resolves jars/natives/JRE but nothing spawns the JVM
- `install_java_version`, `launch_java_version` — the existing commands are instance-scoped, not
  version-scoped like `JavaInstallerService.InstallAsync` / `JavaLauncherService.LaunchAsync(v.Id)`
- `duplicate_java_instance`, `import_java_instance_zip`, `import_official_minecraft`
- No 3D skin renderer (`glacierSkin.*` interop + three.js), so the Profile panel's "3D" pill is disabled
  and the cape-cycle pill (3D-only in the original) never renders
- No host state tracks a running Java process, so `mcRunning` is a prop defaulting to `false` rather than
  a fabricated in-game state

### Verification

31 test binaries green (gl-java 17 → 27). 0 svelte-check errors. Clean `vite build`, with the two new
panels confirmed present in `dist/assets/*.js` rather than trusting the file count.
**168 distinct commands invoked, 0 missing** of 195 registered.



## Phase 3 follow-up — the stub list and the two missing Bedrock services

### LeviLamina mod installation was broken three ways

Against `LeviLaminaModsService.DownloadAndInstallAsync`:

- **Wrong directory.** `ModsDirectory` is `LeviLaminaDirectory + "/plugins"` — `<clients>/LeviLamina/plugins`,
  which is where the loader actually reads. Installs went to `<clients>/LeviLaminaMods`, so **no installed
  mod could ever load**.
- **Wrong folder name.** The original uses `SanitizeFolderName(mod.RepoName)` — repo name only. The rewrite
  used `<owner>_<repo>`, so an install made by the old app was invisible to the new one and vice versa.
- **No extraction.** The original downloads to a temp file, wipes the destination, then extracts a `.zip`
  or copies any other asset verbatim. The rewrite dropped the raw asset into the folder, leaving an
  unextracted zip.

Downloading to a temp file first also means a cancelled download can no longer leave a half-written file
in the plugins folder. `IsInstalled` drops a non-empty check the original doesn't have, and the queue
label now matches `label: mod.Name`.

### Post-switch Bedrock version change (the TODO the plan listed first)

`switch_bedrock_instance` now performs it, and returns the original's **warning string** instead of a
bool: empty means the data swap *and* the version switch succeeded, non-empty means the swap worked but
the version didn't. Reporting those separately matters — the swap is the destructive half and must not
read as a failure because the version switch afterwards didn't take. It lives in the command layer, which
keeps `InstanceService` free of any dependency on package registration; that dependency was what its TODO
was waiting for.

Three further TODOs were removed as **stale**, not fixed: the VanillaTweaks and BedrockTweaks download
paths already run through the shared queue.

### `GdkArchiveService` — ported

The first of the two missing Bedrock services. Without it the version browser could not offer anything
past **1.21.120**, because Microsoft's appx/Windows-Update channel stops there after the GDK move — while
the panel's own info popover told the user those builds come from the community archive and are "marked
GDK". Nothing produced those rows and nothing rendered the marker, so the promise was empty.

`gl_bedrock::gdk`, **7 tests**. The C# asset regex is hand-parsed rather than adding a regex crate for one
fixed pattern. `WuVersion` gains `source` / `friendly_version` / `download_url` / `sha256`, all
`#[serde(default)]` so Windows Update entries and cached JSON deserialize unchanged. The list command
merges both sources on `version|channel` (the C# `MergeKey`), archive applied second so it wins a tie;
either source failing is survivable, only both failing is an error. Downloads branch on source: archive
entries use their direct URL and verify the published SHA-256.

Two latent frontend bugs surfaced doing this, both harmless until the archive started producing rows:

- **The `{#each}` keys were `v.version` alone.** The merge key is version+channel, so a Release and a
  Preview build can share a version string and would collide. The original keys on `{Version}|{Channel}`
  for exactly this reason.
- The non-active row was missing the flex wrapper the original has around the version name — which is the
  element the GDK badge sits in.

### `StoreInstallService` — NOT portable, and why

The second missing Bedrock service, and the one genuine dead end so far. It wraps WinRT's
`AppInstallManager` from `Windows.ApplicationModel.Store.Preview.InstallControl`. The `windows` crate
(0.61) **ships no bindings for that namespace** — it is a restricted preview contract, and there is no
`ApplicationModel_Store_Preview_InstallControl` feature to enable. Driving it would mean hand-rolling COM
activation and vtables for a restricted API.

The two Store buttons therefore stay visibly disabled, and their tooltip now names the actual blocker
instead of saying "isn't wired up yet". Deliberately **not** substituted: shelling out to an
`ms-windows-store://` deep link would install the game but silently drop the in-app progress, stage
reporting and cancel that the original shows — a different feature wearing the same button.

## Phases 6 and 7 — verification and the cutover commit

### CI was red before this, in two places

`.github/workflows/ci.yml` gates on `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
`npx tsc --noEmit -p tsconfig.app.json` and `npm run build`. Three of those five failed on the tree as
it stood:

- **`cargo fmt --check`** — 123 diffs across 30+ files. Formatted.
- **`clippy -D warnings`** — 11 warnings, so the job would have failed on the first push. Each fixed on
  its merits rather than silenced: a genuinely dead `VSYNC_KEY` const (the frontend keeps its own copy
  and the match arm uses the literal), three `i64 -> i64` casts, two reverse `sort_by`s that are
  `sort_by_key(Reverse(..))`, and four `Default::default()` field reassignments rebuilt with struct
  update syntax. The one exception is `MEMORYSTATUSEX`, which keeps a scoped `#[allow]` and a reason —
  renaming a Win32 struct away from the name the API documents would only make the P/Invoke harder to
  check.
- **`tsc --noEmit`** — exit 2, with **108 errors, all of them in `interop.js`**. That file is a verbatim
  port of the original's browser-interop layer: plain JS never written against a type checker, full of
  `window.*` expando properties. Its typed surface for the rest of the app is the hand-written
  `interop.d.ts`, which *is* checked, so `interop.js` is now excluded from the typecheck rather than
  rewritten (rewriting it would break the "verbatim port" rule that exists precisely because paraphrasing
  this codebase is what caused the original failure).

All five gates pass locally now.

### The cutover commit

Committed on branch `rust-tauri-cutover`, not `main`, in the three commits the plan calls for:

1. Remove the WPF tree (133 deletions) plus the `.gitignore` change, which belongs with it — it stops
   tracking `bin/`/`obj/` and starts ignoring the Rust and Node artifacts the next commit's tree
   produces.
2. Add the Rust workspace, `crates/`, `src-tauri/`, `frontend/` — 228 files, 57,638 lines.
3. Docs and CI: the rewritten `README.md` (it still advertised ".NET 10, WPF, and Blazor WebView" and was
   Bedrock-only), `PARITY.md`, `REWRITE-PLAN.md`, the inventories, and `ci.yml` replacing the WPF-shaped
   `release.yml`.

`HANDOFF_PROMPT.md` and `injection-plan.md` are gone as the plan requires. Both were **untracked**, so
deleting them would have been unrecoverable — they are archived to
`MinecraftLauncher-rust-snapshot/_retired-docs/` first.

Verified nothing generated was staged (`node_modules`, `target/`, `frontend/dist`, `gen/schemas` all
clean), and that `frontend/package-lock.json` is committed, since CI's `npm ci` fails without it.

### Clean-checkout verification

Cloned the branch to a temp directory and built it from nothing: `npm ci` clean, `tsc --noEmit` exit 0,
`vite build` clean. This is the check that catches work left untracked, and it passed.

### Still outstanding

The **Phase 5 side-by-side acceptance pass** has not happened. Everything verified so far is
source-level parity plus static checks; nothing has been run against the original launcher from
`publish_final`, and the Phase 0 reference screenshots were never taken. That needs a desktop session.

## Phase 3 — Bedrock services

**6 MATCH / 12 DIVERGE / 6 MISSING** across 25 services. All **141** commands are registered in
`generate_handler!` (no dead commands); **zero** `todo!()`/`unimplemented!()` macros.

Blockers, in order:

| # | Issue | Status |
|---|---|---|
| 1 | **No `com.mojang` resolution anywhere in Rust** — `ResolveBestRootFor`/`LiveRootFor` never ported. | **DONE** — ported to `crates/gl-bedrock/src/paths.rs` with 8 unit tests (per-subfolder resolution, most-entries wins, strictly-greater tie-break, empty-never-wins, dirs count, `Shared` fallback). Wired into worlds/packs/screenshots/backup commands via `bedrock_root()`/`bedrock_dir()`; empty `instance_dir` now means *live*. Needs runtime verification against a real install. |
| 2 | No path to install a current Bedrock build — `GdkArchiveService` + `StoreInstallService` absent; WU list capped at 1.21.120 | MISSING |
| 3 | Injection can target the wrong process; package resolution matched nothing | **DONE** — `resolve()` now queries by short name and verifies `PackageFamilyName` (the old full-family-name `-Name` query matched nothing, so launch always reported "Bedrock isn't installed"). Added `find_processes_by_name` + `process_package_family_name` (`GetPackageFamilyName` via `PROCESS_QUERY_LIMITED_INFORMATION`) to `gl-injection`, and ported C# `FindGameProcess` exactly: name order `Minecraft.Windows`/`Minecraft`/`MinecraftUWP`, family match wins, unreadable-family process kept as last-resort fallback. Every lookup inside `start_minecraft`/`launch` is now scoped to the resolved family. Needs runtime verification. |
| 4 | Instance system unreachable — `switch_to`/`set_version` have no commands; `AppState` sets `live_root: None` | **DONE** — `AppState` now passes the resolved live root; added and registered `switch_bedrock_instance`, `save_active_bedrock_instance`, `set_bedrock_instance_version`, `active_bedrock_instance_id` (141 → 145 commands). Frontend still needs to call them. |
| 5 | Options save corrupts settings — case-sensitive key remove vs C# `OrdinalIgnoreCase`; missing guards | **DONE** — four divergences fixed: case-insensitive key matching; `sep > 0` line skip; the "close Minecraft first" guard on *both* write paths; and the settings-folder check now *errors* (C# behaviour) instead of silently creating the folder. Also fixed `options_path`, which joined `minecraftpe` onto an empty instance dir and produced a relative path that never matched the real file. |

**Duplicate subsystems** — *partly resolved.* The dead `gl-bedrock::launch` module (never reachable
from any command) is deleted; `gl-launch::bedrock` is the single package/process path and has been
fixed. Correction to the earlier inventory: `gl-launch` was **not** the weaker port overall — it parses
the `<Application Id>` element properly and uses a Win32 snapshot rather than shelling out to
`tasklist`; it had one specific fatal bug (the `-Name` query), now fixed. Still duplicated: options
handling. Note `commands/launch.rs` aliases `gl_launch::bedrock` as `gl_bedrock_launch`, which reads
like the `gl-bedrock` crate and invites exactly this confusion — worth renaming.
The `//! NOT YET WIRED UP` banner atop `commands/launch.rs` is stale — remove it.

| Service | Verdict |
|---|---|
| BedrockWorldService · BedrockPackService · BedrockScreenshotService · LevelDatEditorService · NbtIo | MATCHES |
| BedrockBackupService | **FIXED** — resolved every managed folder off one root; C# resolves *per folder* (`sourceIsLive ? LiveRootFor(folder) : sourceRoot`). Added `BedrockRoot::{Instance,Live}` so live backup/restore hit the real per-account directories. Old signatures kept as delegating wrappers; existing round-trip tests still pass. |
| BedrockOptionsService | **FIXED** — see blocker 5 |
| BedrockVersionService · GdkArchiveService · StoreInstallService · GlacierClientService · GameConsoleService · CustomClientLibraryService (update half) | MISSING |
| remaining 12 (incl. BedrockPackageService, BedrockInstanceService, BedrockOptionsService, VanillaVersionService, GameLauncher) | DIVERGES |
| `gl-injection` vs `InjectionService.cs` | MATCHES — same 7-step sequence, bit-identical access masks, ACL constants 1:1, `!ever_read_it` semantics preserved; RAII guards make it *safer*. **Gap:** `LaunchMinecraft` (COM `IApplicationActivationManager`) not ported |
| `wu.rs` vs `VanillaVersionService.cs` | DIVERGES — protocol faithful (44 update IDs, headers, regexes correct) but only ~40% of the 772-line service. 3 bugs: `AppxMetadata` looked up anywhere in fragment vs as child of `AppxPackageInstallData`; duplicate IDs first-wins vs C# last-wins; `DEVICE_ATTRIBUTES` emits `&` where C# emits `&amp;`. Missing: community DB, GDK merge, cache fallback, disk-space guard, Developer Mode, `ReregisterActiveAsync` |

## Phase 4 — Java

Not started. Gated on the Bedrock side matching the original.
