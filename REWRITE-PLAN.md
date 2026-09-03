# Glacier Launcher — full redo of the Rust/Tauri port

## Context

Glacier Launcher was a .NET 10 WPF + Blazor WebView app, preserved read-only at
`C:\Users\User\MinecraftLauncher-wpf-backup-full`. A Rust + Tauri v2 + Svelte 5 rewrite exists at
`C:\Users\User\MinecraftLauncher` but **does not look or behave like the original**, so it is being
redone. Nothing in the current rewrite is trusted until it is checked against the original.

### Why the first attempt failed

The evidence is specific, and it is not a styling problem:

- `frontend/src/app.css` **is** a faithful verbatim copy of the original — the diff against
  `wwwroot/css/app.css` is 17 lines (two font paths, one `#root` rule). The styling primitives are correct.
- The **markup those styles are applied to is not**. The original `Pages/Home.razor` is 5,646 lines;
  the rewrite condenses it into a 955-line `App.svelte` plus 26 view components.
- Panels were **merged** rather than ported: the original's separate Worlds / Packs / Backups /
  Screenshots panels became one `BedrockContentView.svelte`.
- Even the closest-matching region diverges. In the launch button row the rewrite drops the
  `btn-badge-wrap` wrapper element and replaces every `L.T("nav.…")` localization call with a
  hardcoded English string.

Root cause: the previous port **paraphrased and condensed** the original instead of transcribing it.
Correct CSS over reorganized DOM produces exactly the reported symptom — it doesn't look like it, and
the behavior hangs off structure that no longer exists.

**The governing rule for this redo: transcribe, never summarize.** Structure comes from the original
source, read line by line. A merge, a rename, or a "cleaner" structure is a defect, not an improvement.

### Scale of the original (the actual work to be ported)

| Area | Location | Lines |
|---|---|---|
| Main page markup | `Pages/Home.razor` | 5,646 |
| Page code-behind (24 files) | `Pages/Home.*.cs` | 6,106 |
| Components (17 files) | `Components/*.razor` | 2,116 |
| Services (58 files) | `Services/*.cs` | 12,426 |
| Stylesheet | `wwwroot/css/app.css` | 6,455 |
| Interop | `wwwroot/js/interop.js` | 531 |

Largest code-behinds: `Home.Panels.cs` (1,049), `Home.Clients.cs` (1,022), `Home.razor.cs` (893),
`Home.Settings.cs` (503), `Home.Utilities.cs` (428), `Home.QuickActions.cs` (364),
`Home.Launch.cs` (345), `Home.BigFeatures.cs` (253).

### What "redo everything" means here

Scope is the whole port, frontend and backend. One deliberate qualification, stated up front rather
than applied silently: **existing Rust crates are re-verified against their C# service, not deleted
unread.** Some are real, substantial, working ports — `gl-injection` (553 lines of Win32),
`gl-bedrock/src/wu.rs` (632 lines of Windows Update SOAP). Deleting those unread would discard correct
work and re-introduce solved bugs. Every crate must be *proven* faithful line-by-line against its
original service; anything unproven is rewritten. Nothing is trusted by default — but nothing correct
is thrown away either. Say so if you'd rather delete outright.

### Order of work

**Bedrock first, completely 1:1, then Java.** Bedrock is the product's primary edition (DLL clients,
injection, packs/worlds, WU version management) and it is where the divergence is worst. Java work does
not begin until the Bedrock side matches the original panel for panel.

---

## Phase 0 — Safety net and ground truth

1. **Snapshot before anything.** The entire rewrite is untracked in git (`git ls-files` returns zero
   files under `crates/`, `src-tauri/`, `frontend/`) and the WPF tree shows as deleted-but-unstaged.
   Copy the working tree to `C:\Users\User\MinecraftLauncher-rust-snapshot` (excluding `target/` and
   `node_modules/`) so no later git operation can destroy it.
2. **Run the original.** Launch `MinecraftLauncher-wpf-backup-full/publish_final` and keep it running
   for the whole project. It is the acceptance bar — every phase is checked against the real app on
   screen, not against a reading of its source.
3. **Capture reference screenshots** of every Bedrock panel first (then Java), plus every modal, empty
   state, and error state. These are the visual diff targets.
4. **Build the transcription ledger** at `PARITY.md`: one row per panel and per service, recording
   original file + line range → target Svelte view / Rust crate → status. Every later phase ticks rows.
   This replaces `HANDOFF_PROMPT.md`, which is stale and misleading.
5. **Delete `injection-plan.md`.** It proposes a C# `BedrockInjection.cs` helper invoked from Rust — an
   architecture that does not exist here, contradicting the shipped `gl-injection` crate.

## Phase 1 — Shell and chrome, transcribed literally

Rebuild the app shell from `Home.razor` top to bottom, using the verified line ranges in
`docs/inventory-shell.md`. Build it in its **Bedrock** state first; Java-specific chrome follows in Phase 4.

Shell regions and their exact locations in `Pages/Home.razor`:

| Region | Lines |
|---|---|
| `.window` / `.window-bg` / `.drag-zone` | 46–48 |
| `.drop-zone-overlay` | 50–54 |
| Onboarding | 56–140 |
| Search overlay/modal | 141–215 |
| `.top-bar` (branding 451–469, spacer 471, edition switcher 473–484, client chip 486–503, mc-running pill 505–511, search trigger 513–515, notif backdrop 517–520, bell + panel 525–609) | 450–624 |
| `.window-controls` | 611–623 |
| Announcement banner | 627–645 |
| `.main-content` / `.btn-row` | 647 / 648–772 |
| status-msg, `.launch-dl-bar`, recently-launched | 775, 777–785, 787–802 |
| `.footer` (incl. `.toast-container` 951–960) | 806–960 |
| `.news-strip` | 962–983 |

**Correction to an earlier assumption — the original has no shell-level `.panel-tabs` bottom nav bar.**
All 20 `.panel-tabs` occurrences live *inside individual panels*, generated by `JavaTabs`/`BedrockTabs`
(`Home.BigFeatures.cs:31–75`). The shell's only global nav is `.btn-row`. The previous rewrite invented
a shell-level bar with 15 entries against the original's 7 (Bedrock) / 8 (Java); **delete it**, do not port it.

Confirmed shell regressions to fix (each verified directly, not taken on trust):

- `btn-badge-wrap` — 13 occurrences in the original wrapping every non-launch button, **0** in the
  rewrite; the `.notification-badge` gated on `clientsHasBadge` disappeared with it.
- `.wnd-btn.fullscreen-btn` (original line 615, `ToggleFullscreen`) — deleted; window controls are 3
  buttons instead of 4, and F11 fires with nothing handling it.
- `.toast-container` and `.launch-dl-bar` — styled in `app.css` (lines 3268 and 887) but rendered by no
  markup anywhere in the rewrite.
- `.footer-right` missing from `Footer.svelte` — no `.footer-mc-version`, no version/clock span;
  `.footer-profile-area` lost its onclick.
- The whole `.search-modal` / `.search-result*` DOM is unrendered — search was rebuilt as an inline
  `.topbar-search` with brand-new CSS in a scoped `<style>` block. Remove that and restore the original.
- `.update-pill` update-available branch dropped; client chip changed from `div` to `button`.
- `@onmousedown:stopPropagation` is absent from every top-bar control, so clicks bubble into the drag zone.
- **Localization: all 9 shell `L.T()` calls are gone** (`nav.launch`, `nav.settings`, `nav.clients`,
  `nav.versions`, `nav.mcversions`, `nav.mods`, `nav.photos`, `nav.profile`, `nav.launchers`) and no
  localization module exists in `frontend/src` — `LocalizationService.cs` was never ported, so es/de/fr
  don't work. **Scope is small and bounded:** the original has only **10 `L.T()` sites and 10 keys in
  total** across `Pages/` + `Components/` (those 9 `nav.*` plus `settings.language`). Everything else in
  the original — including the `BedrockTabs`/`JavaTabs` labels — is a hardcoded literal to be copied
  verbatim. One string is simply wrong: `nav.mcversions` defaults to "MC Versions", the rewrite
  hardcodes "Versions".

`frontend/src/interop.js` is otherwise a faithful port; only the WebView2→Tauri bridge, `openUrl` and the
skinview3d path differ. One carried-over bug to fix: `glacierViewTransition` still calls
`dotNetRef.invokeMethodAsync('ApplyPendingNav')`, a Blazor call that cannot resolve under Tauri.

- Work from the original markup **element by element**, preserving DOM order, nesting, class names,
  wrapper elements (`btn-badge-wrap` and friends), `data-tooltip` text, and `aria-label`s exactly.
- Restore localization: every `L.T("key")` becomes a real lookup, ported from
  `Services/LocalizationService.cs`. No hardcoded English where the original translated.
- Port `wwwroot/js/interop.js` (531 lines) 1:1, adapting only WebView2 window/IPC calls to Tauri's.
- Re-copy `app.css` verbatim; keep the diff to the two font paths and the `#root` rule.

**Acceptance:** the Bedrock home screen is visually indistinguishable from the running original at the
same window size.

## Phase 2 — Bedrock panels, one at a time, unmerged

One phase-unit per original Bedrock panel, in the original's own `BedrockTabs` order
(`Home.BigFeatures.cs`). **Undo the merges** — Worlds, Packs, Backups and Screenshots are four separate
panels, as in `Home.MergedPanels.cs`, including the `WorldsServersTabs` / `PacksAddonsTabs` cross-nav strips.

Bedrock-side sources, largest first: `Home.Clients.cs` (1,022), the Bedrock half of `Home.Panels.cs`
(1,049), `Home.BedrockInstances.cs`, `Home.Packs.cs`, `Home.Worlds.cs`, `Home.Backups.cs`,
`Home.BedrockScreenshots.cs`, `Home.LevelDat.cs`, `Home.LeviLaminaMods.cs`, `Home.Tweaks.cs`,
`Home.Glacier.cs`, `Home.ClientLibrary.cs`.

Per panel: read the original markup and its code-behind in full → transcribe markup → port each event
handler's real logic → wire to backend commands → screenshot-diff against the original panel → tick
`PARITY.md`. Do not start the next panel until the current one matches.

Port the 17 `Components/*.razor` as Svelte components with their original names and classes —
including `ThemeStudioPanel.razor` (539 lines), which has no counterpart in the current rewrite at all.

**Rules that caused real bugs before:** no scoped `<style>` for anything with a real class in
`app.css` (specificity silently overrides the global rule); register every `#[tauri::command]` in
`generate_handler!` in `src-tauri/src/lib.rs`; add every permission to
`src-tauri/capabilities/default.json` — a missing ACL entry silently no-ops the JS call instead of erroring.

## Phase 3 — Bedrock backend verification, service by service

Inventory complete — see `docs/inventory-bedrock-services.md`. Verdict across the 25 Bedrock services:
**6 MATCH / 12 DIVERGE / 6 MISSING**. Two pieces of good news that narrow the work: all **141**
registered `#[tauri::command]`s are present in `generate_handler!` (no silently-dead commands), and
there are **zero** `todo!()`/`unimplemented!()` macros — every stub is a prose note.

**Blockers, in priority order (each verified directly):**

1. **No `com.mojang` resolution exists anywhere in Rust.** Grep across the whole workspace returns 13
   hits, *all of them doc comments* — zero real code. The C# `ResolveBestRootFor`/`LiveRootFor` logic
   (the subject of the last two hotfix commits on the WPF side) was never ported. Every Bedrock command
   instead takes an `instance_dir` from the frontend and joins a literal subfolder, so **the rewrite
   cannot read or write the player's actual game data at all.** Port this first; most of the Bedrock
   surface is meaningless without it.
2. **No path to install a current Bedrock build** — `GdkArchiveService` and `StoreInstallService` are
   both absent, and the WU sideload list is capped at 1.21.120 by the GDK move.
3. **Injection can target the wrong process.** `gl-launch::bedrock::find_game_process` matches on name
   only; C# `FindGameProcess` matched the package family name via `GetPackageFamilyName`. Worse,
   `resolve()` passes the *full* family name to `Get-AppxPackage -Name`, which expects the short name —
   it likely resolves nothing.
4. **The instance system is unreachable** — `switch_to` and `set_version` have no commands, and
   `AppState` constructs the service with `live_root: None`.
5. **Options save corrupts settings.** `src-tauri/src/commands/utilities.rs:234` removes keys with a
   case-sensitive exact match; C# `BedrockOptionsService.cs:176` uses `OrdinalIgnoreCase`. A case
   mismatch appends a duplicate line instead of updating. The "close Minecraft first" guard is also
   missing from both write paths.

**Three subsystems exist in duplicate** — package resolution, options, and process lookup — and in each
case *the reachable copy is the weaker port*. Delete the weak copy and wire the good one. Also: the
`//! NOT YET WIRED UP` banner atop `commands/launch.rs` is stale and should go.

**Deep-dive results on the two crates believed genuine:**

- **`gl-injection` — genuinely faithful.** Same 7-step sequence, bit-identical access masks, ACL
  constants matching one for one, and the subtle `!ever_read_it` semantics of `WaitForModule` preserved.
  RAII guards and path canonicalization make it *safer* than the original. Keep it. **One gap:**
  `InjectionService.LaunchMinecraft` (COM `IApplicationActivationManager`) — the primary fallback
  activation strategy — isn't ported anywhere.
- **`wu.rs` — faithful protocol, incomplete service.** All 44 magic update IDs, the security header,
  device attributes, fragment filters and both version regexes are correct. Three real bugs:
  `AppxMetadata` is looked up anywhere in the fragment rather than as a child of
  `AppxPackageInstallData`; duplicate update IDs resolve first-wins vs C#'s last-wins; and
  `DEVICE_ATTRIBUTES` emits raw `&amp;` where C# emits `&amp;amp;`. It covers only ~40% of the 772-line
  service — community DB, GDK merge, cache fallback, disk-space guard, Developer Mode and
  `ReregisterActiveAsync` are all absent.

Then diff the remaining services against their `crates/gl-*` counterparts: same inputs, same
paths, same error cases, same user-facing message text. Covers `BedrockPackageService`,
`BedrockInstanceService`, `BedrockWorldService`, `BedrockPackService`, `BedrockBackupService`,
`BedrockScreenshotService`, `BedrockOptionsService`, `BedrockVersionService`, `VanillaVersionService`,
`GdkArchiveService`, `StoreInstallService`, `LevelDatEditorService`, `NbtIo`, `ShaderService`,
`FlarialService`, `OderSoServices`, `LeviLaminaService`, `LeviLaminaModsService`, `GlacierClientService`,
`ClientLibraryService`, `CustomClientLibraryService`, `TweaksService`, `GameLauncher`, `GameConsoleService`.
Mark each row verified or rewrite it. Known open stubs: `gl-bedrock/src/instance.rs:186` (post-switch
version change), the `gl-download` wiring markers, `gl-update/src/resume.rs`,
`src-tauri/src/commands/utilities.rs:12`.

Highest-risk item, handled last and carefully: **injection**. Read `crates/gl-injection/src/lib.rs`
fully before editing and diff against `Services/InjectionService.cs` (337 lines) — `SeDebugPrivilege`,
the UWP ACL grant (`SetNamedSecurityInfoW` for `ALL APPLICATION PACKAGES`), alloc/write/`CreateRemoteThread`,
`WaitForSingleObject` timeout, `GetExitCodeThread`, module probe, and `VirtualFreeEx` + handle close on
every exit path. **Test against a disposable host process with a throwaway DLL first — never the real
game on the first run.** Confirm process detection matches `Services/GameLauncher.cs`, including
disambiguation when several `Minecraft.Windows.exe` candidates exist.

**Gate: the Bedrock side must fully match the original before Phase 4 begins.**

## Phase 4 — Java side, same method

Repeat Phases 1–3 for Java: the Java chrome state, the `JavaTabs` panels (`Home.Datapacks.cs`,
`Home.Downloads.cs`, Java halves of `Home.Panels.cs`/`Home.Utilities.cs`, mods, screenshots, instances,
versions, modpacks), then the Java services (`JavaGameLauncher`, `JavaInstallService`,
`JavaInstanceService`, `JavaVersionService`, `JavaRuntimeDownloadService`, `JavaModAnalyzer`,
`JavaModLoaderService`, `ModpackInstallService`, `ModrinthService`, `CurseForgeService`). Same rule:
transcribe, screenshot-diff, tick `PARITY.md`.

Shared services (`SettingsService`, `LogService`, `NotificationService`, `ThemeService`,
`LocalizationService`, `AutoUpdateService`, `DiscordRpcService`, `NewsService`, `AnnouncementService`,
`SkinService`, `SkinLibraryService`, `XboxProfileService`, `LiveAuthService`, `StatsService`,
`HardwareMonitorService`, `ServerPingService`, `DownloadService`, `JsonStore`, `HttpFactory`,
`GitHubApiCache`, `LaunchDiagnosticsService`, `LauncherUtilityService`) are verified in whichever phase
first depends on them.

## Phase 5 — Side-by-side acceptance pass

Run both apps side by side and walk every panel, modal, empty state, error state, tooltip and keyboard
binding, Bedrock then Java. Compare against the Phase 0 screenshots. Verify the strings too — user-facing
copy is part of parity. Anything that genuinely cannot reach parity ships as a visibly disabled control
with an explanatory tooltip (the established pattern) and gets a `PARITY.md` row — never as fake UI.

## Phase 6 — Verification

- `cargo check --workspace`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`
- `cd frontend && npx svelte-check --tsconfig ./tsconfig.json`
- `npm run dev`, then exercise for real: every panel opens; launch without Bedrock installed still
  reports "Minecraft Bedrock isn't installed for this user…"; the `game-console` window actually
  receives its streamed log events (previously failed with `event.listen not allowed on window
  "game-console"`); cancel-launch and Escape abort a live launch; injection succeeds against the real
  game; Theme Studio round-trips a custom theme across a restart.
- Perf on the target Celeron N5100: no dropped frames opening panels; profile over CDP on port 9223.
  Never `await` a rAF inside `startViewTransition` — it deadlocks for ~4s.

## Phase 7 — Commit the cutover

Only once Phases 1–6 are green.

1. Confirm `.gitignore` covers `target/`, `frontend/node_modules/`, `frontend/dist/`,
   `src-tauri/gen/schemas/`, and that `Cargo.lock` **is** committed (binary workspace).
2. Confirm `frontend/index.html` has no `window.__TAURI_INTERNALS__` mock — the browser-only test shim
   must never ship.
3. Rewrite `README.md` (it still advertises ".NET 10, WPF, and Blazor WebView"); delete `HANDOFF_PROMPT.md`.
4. Commit on a branch, not `main`: (1) remove the WPF tree, (2) add the Rust workspace + frontend,
   (3) docs and `.github/workflows/ci.yml` (untracked today; the old `release.yml` is deleted and needs
   a Tauri-shaped replacement later).
5. Prove completeness: clone to a temp dir, `npm i` in `frontend`, `cargo check --workspace`.

---

## Tools, commands, and references

**Build / run** — from the repo root:

```bash
npm run dev
```

- Backend: `cargo check -p glacier-launcher` (or `--workspace`)
- Frontend types: `cd frontend && npx svelte-check --tsconfig ./tsconfig.json`
- Tests: `cargo test --workspace`
- Release bundle: `npm run build`
- Frontend-only preview: `cd frontend && npm run dev`, with a temporary `window.__TAURI_INTERNALS__`
  mock in `index.html` to intercept `invoke()` — **remove it afterward, always**

**Toolchain present:** rustc/cargo 1.96.0, node v24.13.0, npm 11.6.2, `frontend/node_modules` installed.
Also required: MSVC build tools, WebView2 runtime, and an elevated shell for injection testing.

**Diagnostics**

- The running original (`publish_final`) — the acceptance bar, side by side.
- Browser devtools / CDP on port 9223 for DOM comparison and profiling.
- Process Explorer — confirm an injected module really loaded.
- `taskkill //F //IM node.exe` and `//IM glacier-launcher.exe` before each `npm run dev`; Vite's port
  1420 collides otherwise and aborts the whole tauri dev process.

**Reference material (read-only — never edit)**

- Original C#: `C:\Users\User\MinecraftLauncher-wpf-backup-full` — `Pages/`, `Components/`, `Services/`,
  `Models/`, `wwwroot/`, and a runnable build in `publish_final`.
- Website source: `C:\Users\User\Glacier-Website` — `assets/data/downloads.json` (real client versions
  and changelogs), live at `https://glacierclient.xyz/assets/data/downloads.json`.

**Conventions**

- Backend shape: thin `#[tauri::command]` wrappers in `src-tauri/src/commands/*.rs` calling into `crates/gl-*`.
- Atomic writes for persisted state via `gl-persistence`; defensive I/O throughout.
- Win32 strings: `*W` APIs with `HSTRING`/`PCWSTR`. Don't round-trip UTF-8 source through PowerShell
  text cmdlets — it corrupts encoding.
- Animation budget (N5100): transform/opacity only. No animated `box-shadow` or `filter`; hover states in CSS.
- Build once at the end of a multi-phase chunk, not after every step.

**Parallel agents.** Phase 2 parallelizes per panel: give each agent the exact original file and line
range to transcribe, explicit file ownership, and reserve `lib.rs` / `state.rs` / `Cargo.toml` /
`App.svelte` for the orchestrating session. Re-run all checks after merging any agent's work.

## Risks

| Risk | Mitigation |
|---|---|
| Rewrite is untracked — one `git checkout .` destroys it | Phase 0 snapshot outside the repo, first, before anything |
| Condensing the original again reproduces the same failure | Transcribe line by line; screenshot-diff every panel before moving on |
| Injection crashes the real game | Throwaway host process + test DLL first; full read before edits; elevated shell |
| Silent Tauri ACL no-ops | Runtime-test every phase; static checks already missed one |
| Redo scope balloons past parity | Parity is the finish line; packaging/updater/release is a later plan |
