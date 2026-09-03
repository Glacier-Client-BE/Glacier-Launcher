# Bedrock backend inventory — WPF original vs Rust rewrite

Read-only audit. Original: `C:\Users\User\MinecraftLauncher-wpf-backup-full` (WPF+Blazor,
`Services/*.cs`, 58 files / 12,426 lines). Rewrite: `C:\Users\User\MinecraftLauncher`
(Rust workspace, `crates/gl-*` + `src-tauri`).

Every claim below was verified against actual file contents. `HANDOFF_PROMPT.md` and
`injection-plan.md` were **not** used and remain untrusted — in particular there is no C#
helper invoked from Rust anywhere; injection is a native Rust Win32 port (`crates/gl-injection`).

## Verdict summary

| Verdict | Count | Services |
|---|---|---|
| MATCHES | 6 | BedrockWorldService, BedrockPackService, BedrockBackupService (crate-level), BedrockScreenshotService, LevelDatEditorService, NbtIo |
| DIVERGES | 12 | BedrockPackageService, BedrockInstanceService, BedrockOptionsService, VanillaVersionService, ShaderService, FlarialService, OderSoServices, LeviLaminaService, LeviLaminaModsService, ClientLibraryService, TweaksService, GameLauncher |
| MISSING | 6 | BedrockVersionService, GdkArchiveService, StoreInstallService, GlacierClientService, GameConsoleService (partial substitute), CustomClientLibraryService (download half) |
| Verified faithful (special review) | 2 | `gl-injection` (mostly faithful, 1 gap), `gl-bedrock/wu.rs` (faithful for the WU half only) |

All **141** `#[tauri::command]` functions defined under `src-tauri/src/commands/` are registered in
`tauri::generate_handler!` in `src-tauri/src/lib.rs` — the defined-vs-registered diff is empty.
No silently-dead commands. The gaps are commands that were **never written**, not unregistered ones.

---

## The single highest-risk finding: no `com.mojang` resolution exists in the Rust build

The C# app resolves the live Bedrock data root dynamically per-account:

`Services/CurseForgeService.cs:32-91`
```
public static readonly string ComMojangRoot = ResolveBestRootFor("minecraftWorlds");
public static string ResolveBestRootFor(string subFolderName)   // walks <pkg>/LocalState/games/com.mojang
                                                                 // and each users/<id>/games/com.mojang
public static string LiveRootFor(string folder) => ...           // per-folder root, independently resolved
```
Every Bedrock service (`BedrockWorldService.WorldsDir`, `BedrockPackService.DirFor`,
`BedrockOptionsService.OptionsPath`, `BedrockBackupService.ComMojangRoot`,
`BedrockScreenshotService.ComMojangScreenshotsDir`, `BedrockInstanceService.SyncFolderSet`)
routes through it. This was the subject of the two most recent hotfix commits
(`a341123`, `2a52a2e`).

A full-workspace grep for `com.mojang` / `LocalState` / `ResolveBestRootFor` / `LiveRootFor`
across `crates` and `src-tauri/src` returns **only doc comments**. There is no implementation.
Instead every Bedrock command takes an `instance_dir: String` from the frontend and joins a
literal subfolder:

- `src-tauri/src/commands/bedrock.rs:41` — `Path::new(&instance_dir).join("minecraftWorlds")`
- `src-tauri/src/commands/bedrock.rs:66-73` — `instance_dir.join("resource_packs")` etc.
- `src-tauri/src/commands/utilities.rs:145` — `instance_dir.join("minecraftpe").join("options.txt")`
- `src-tauri/src/commands/bedrock.rs:110` — `instance_dir.join("Screenshots")`
- `crates/gl-bedrock/src/lib.rs:19-22` explicitly declares this out of scope.

**Consequence:** the rewrite can only ever see worlds/packs/options inside a Glacier-managed
instance folder. It cannot read or write the player's actual installed game data. This is not a
cosmetic divergence — it is the core data path of the Bedrock half of the app, and it is absent.

---

## Second-order structural finding: two divergent copies of the same logic

Three subsystems exist twice, with only one copy reachable from a command:

| Logic | Copy A (reachable) | Copy B (dead) |
|---|---|---|
| Bedrock package resolve + activation | `crates/gl-launch/src/bedrock.rs` (used by `commands::launch`, `commands::shaders`) | `crates/gl-bedrock/src/launch.rs` (exported from `gl_bedrock`, never called by any command) |
| `options.txt` read/write | `src-tauri/src/commands/utilities.rs:145-270` (inline reimplementation) | `crates/gl-bedrock/src/options.rs` (`BedrockOptionsService`, never called by any command) |
| Process lookup for Bedrock | `gl_injection::find_process_by_name` (toolhelp snapshot) | `gl_bedrock::launch::find_game_process` (`tasklist.exe` CSV parse) |

The two copies behave differently (see per-service rows). The dead copies are the *better* ports;
the live ones are the weaker ones. `commands/leveldat.rs` also carries `#![allow(dead_code)]`
and `commands/tweaks.rs` likewise.

---

## Per-service rows

### BedrockPackageService — DIVERGES
- **Original:** `Services/BedrockPackageService.cs`, 234 lines.
  - `Resolve()` — WinRT `PackageManager.FindPackagesForUser` for both family names, release preferred; reads `InstalledLocation.Path`; `ApplicationId` via `ReadApplicationId`.
  - `ReadApplicationId(path)` — XML-parses `AppxManifest.xml`, takes the first `<Application Id>` with a non-blank value; falls back to `"Game"` if `gamelaunchhelper.exe` exists, else `"App"`.
  - `FindGameProcess(familyName?)` — scans process names `Minecraft.Windows`, `Minecraft`, `MinecraftUWP`; for each candidate calls `GetPackageFamilyName` via `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` and **matches the package family name**, so a Preview process isn't mistaken for release. Access-denied candidates are kept as a last-resort fallback.
  - `TryInvokeInDesktopPackageAsync(info, console)` — checks the exe exists, locates `System32\WindowsPowerShell\v1.0\powershell.exe`, runs `Invoke-CommandInDesktopPackage -PackageFamilyName … -AppId … -Command '<full exe path>'`, pipes stderr to the console, returns `ExitCode == 0`.
- **Rust:** `crates/gl-launch/src/bedrock.rs` — `resolve()`, `read_application_id()`, `extract_application_id()`, `find_game_process()`, `invoke_in_desktop_package()`. Reached via `commands::launch::launch_bedrock_instance` / `is_bedrock_game_running` / `commands::shaders::resolve_materials_dir`; both registered.
- **Verdict: DIVERGES.**
  - `find_game_process()` (`gl-launch/src/bedrock.rs:130`) does **no package-family-name matching** — it is `gl_injection::find_process_by_name("Minecraft.Windows.exe")`, name only. The C# guard against targeting the wrong channel's process is gone. This is an *injection-targeting* correctness issue: with both Release and Preview installed, the launcher can inject into the wrong game.
  - It also drops the `"Minecraft"` / `"MinecraftUWP"` alternate process names.
  - `resolve()` uses PowerShell `Get-AppxPackage` instead of WinRT; acceptable substitute, but `-Name '{family}'` is passed the **full family name** (`Microsoft.MinecraftUWP_8wekyb3d8bbwe`) whereas `Get-AppxPackage -Name` expects the short name. The dead copy in `gl-bedrock/src/launch.rs:74` gets this right (`family.split('_').next()`). The live copy likely resolves nothing.
  - `invoke_in_desktop_package` omits `-PreventBreakaway` (present in the dead `gl-bedrock` copy and consistent with the GDK path).
  - The COM `IApplicationActivationManager` activation path (`InjectionService.LaunchMinecraft`) is not ported anywhere — see gl-injection below.

### BedrockInstanceService — DIVERGES (data-loss-adjacent)
- **Original:** `Services/BedrockInstanceService.cs`, 282 lines. `Create`, `SetVersion`, `Rename`, `Delete`, `SwitchToAsync`, `SaveActiveAsync`, `BackupInstanceAsync`, `RestoreIntoInstanceAsync`, `ActiveInstance`, `Instances`. `SwitchToAsync` archives live com.mojang into the current instance, copies the target instance into live, persists `BedrockActiveInstanceId` to settings, then optionally switches the installed game build to `VersionTag` via `VanillaVersionService.SwitchVersionAsync`, returning a user-facing warning string.
- **Rust:** `crates/gl-bedrock/src/instance.rs` — `create`, `set_version`, `rename`, `delete`, `switch_to`, `save_active`, `backup_instance`, `restore_into_instance`, `active_instance`. Index quarantine on corrupt JSON matches `JsonStore.QuarantineCorrupt`. Slug/dedup/guard-rails match (verified by the crate's own tests).
- **Commands:** `list_bedrock_instances`, `create_bedrock_instance`, `rename_bedrock_instance`, `delete_bedrock_instance` — all registered.
- **Verdict: DIVERGES.**
  - **`switch_to` has no command.** There is no `switch_bedrock_instance` in `commands/bedrock.rs` and none in the handler list. The whole point of the instance system — swapping data sets in and out of the live game folder — is unreachable from the UI.
  - **`set_version` has no command** either, so `version_tag` can only be set at creation.
  - `save_active` has no command.
  - `src-tauri/src/state.rs:33` constructs `BedrockInstanceService::new(root.join("bedrock"), None)` — `live_root: None`. The "Default" instance is therefore never seeded from the player's real com.mojang folder, unlike `EnsureDefaultInstance` in C# which calls `SyncFolderSet(ComMojangRoot, …)`.
  - Active-instance id is persisted in the crate's own `instances.json` rather than `Settings.BedrockActiveInstanceId` (documented deviation; benign in isolation but means settings export/import loses it).
  - The post-switch game-version switch is an explicit `TODO` at `instance.rs:186`.

### BedrockWorldService — MATCHES
- **Original:** `Services/BedrockWorldService.cs`, 126 lines. `ListWorlds` (dir counts as a world if `db/` or `level.dat`; name from `levelname.txt`; recursive size; `LastWriteTimeUtc` ISO "o"; icon from `world_icon.jpeg/jpg/png`; ordered by `ModifiedAt` descending), `OpenFolder`, `DeleteWorld`, `ExportWorldAsync` (zip to `~/Glacier Launcher/exported-worlds/<safeName>.mcworld`, `(n)` disambiguation).
- **Rust:** `crates/gl-bedrock/src/world.rs` — `list_worlds`, `delete_world`, `export_world`. Same predicate, same icon list, same sort, same `(n)` collision loop.
- **Commands:** `list_bedrock_worlds`, `delete_bedrock_world`, `export_bedrock_world` — registered.
- **Verdict: MATCHES**, with two notes: (a) the export directory is now a caller-supplied `export_dir` rather than the fixed `~/Glacier Launcher/exported-worlds`; (b) `OpenFolder` has no dedicated command (the generic `commands::system::open_folder` covers it). Root path resolution is the cross-cutting issue above, not a fault of this module.

### BedrockPackService — MATCHES
- **Original:** `Services/BedrockPackService.cs`, 118 lines. `DirFor(kind)`, `ListPacks(kind)` (name from `manifest.json` → `header.name`, **rejected if it starts with `pack.`** — the untranslated localization key — falling back to the folder name; sorted by name, `OrdinalIgnoreCase`), `OpenFolder`, `DeletePack`.
- **Rust:** `crates/gl-bedrock/src/pack.rs` — `list_packs`, `delete_pack`, plus `PackDirs::dir_for`. The `pack.`-prefix rejection is reproduced at `pack.rs:83`. Sort is `sort_by_key(name.to_lowercase())`, equivalent for this data.
- **Commands:** `list_bedrock_packs`, `delete_bedrock_pack` — registered; the six kind strings map 1:1 to the C# `DirFor` arms and to the six real folder names.
- **Verdict: MATCHES.**

### BedrockBackupService — DIVERGES (only because of live-root handling)
- **Original:** `Services/BedrockBackupService.cs`, 141 lines. `ManagedFolders` (8 entries), `ListBackups` (`*.zip`, `CreationTimeUtc` desc), `CreateBackupAsync` / `CreateBackupFromFolderAsync(sourceRoot, label, sourceIsLive)`, `RestoreBackupAsync` / `RestoreBackupIntoFolderAsync(path, targetRoot, targetIsLive)`, `DeleteBackup`, `OpenBackupsFolder`. Error text when empty: **"Nothing to back up yet — no worlds or packs found."**
- **Rust:** `crates/gl-bedrock/src/backup.rs` — `MANAGED_FOLDERS` (identical 8, same order), `list_backups`, `create_backup_from_folder`, `restore_backup_into_folder`, `delete_backup`. Restore wipes each top-level folder present in the archive before extracting, matching C#. Writes are atomic (`write_atomic`), an improvement.
- **Commands:** `list_bedrock_backups`, `create_bedrock_backup`, `restore_bedrock_backup`, `delete_bedrock_backup` — registered.
- **Verdict: DIVERGES.** The crate logic itself matches. The divergences are:
  - The `sourceIsLive` / `targetIsLive` variants — which route each managed folder through `CurseForgeService.LiveRootFor(folder)` *independently*, the whole point of hotfix `a341123` — have no equivalent. Backing up or restoring the **real** game data is impossible.
  - Error message text differs: `BedrockError::NothingToBackUp` vs the C# string above. Check `crates/gl-bedrock/src/error.rs` if exact text parity matters.
  - Backups dir is `<launcher_root>/Backups` (`commands/bedrock.rs:37`) vs C# `~/Glacier Launcher/bedrock-backups`. Existing users' backups will not be found.
  - `OpenBackupsFolder` has no command.
  - Backup timestamp is UTC-derived (`now_iso`) vs C# `DateTime.Now` (local). Cosmetic filename difference only.

### BedrockScreenshotService — MATCHES
- **Original:** `Services/BedrockScreenshotService.cs`, 82 lines. `ListScreenshots` — `*.jpeg` recursive under `<com.mojang>/Screenshots`, plus `*.png` **top-directory-only** under `MyVideos\Captures` filtered to names containing "minecraft"; sorted by `ModifiedAt` desc. `OpenFolder`, `DeleteFile`.
- **Rust:** `crates/gl-bedrock/src/screenshot.rs` — `list_screenshots` (recursive jpeg from the com.mojang dir; non-recursive png from captures, "minecraft" substring filter), `delete_file`. Both filters and the sort match exactly.
- **Commands:** `list_bedrock_screenshots`, `delete_bedrock_screenshot` — registered.
- **Verdict: MATCHES**, with one path caveat: `commands/bedrock.rs:135` derives the captures dir as `%USERPROFILE%\Videos\Captures` rather than the `MyVideos` known folder, so a user with a redirected/OneDrive Videos folder gets an empty list. And the com.mojang side is fed `instance_dir/Screenshots`, per the root-resolution issue above.

### BedrockOptionsService — DIVERGES
- **Original:** `Services/BedrockOptionsService.cs`, 225 lines. `Load`, `Get`, `IsVSyncEnabled`, `Save(changes)`, `Save(key,value)`, `SetVSync`, `RestoreBackup`, `HasBackup`, `Exists`, `GroupRank`, plus the 31-entry `KnownKeys` metadata table and `InferKind`. `Save` **throws** `"Close Minecraft first — it rewrites options.txt on exit and would undo these changes."` when the game is running, and `"Minecraft's settings folder doesn't exist yet. Launch the game once, then try again."` when `minecraftpe` is missing. Backup is one-shot (`.glacier.bak`, first write wins). Key matching in `Save` uses an `OrdinalIgnoreCase` dictionary. `RestoreBackup` also refuses while the game runs (`"Close Minecraft first."`).
- **Rust (live path):** `src-tauri/src/commands/utilities.rs` — `get_bedrock_options`, `bedrock_options_exist`, `bedrock_options_has_backup`, `save_bedrock_options`, `restore_bedrock_options_backup`. All registered. The 31-key table, `prettify`, `infer_kind` and the `Integer`-override needle list are faithfully reproduced.
- **Rust (dead path):** `crates/gl-bedrock/src/options.rs` `BedrockOptionsService` — a fuller port including `group_rank`, `save_one`, `set_vsync`, `is_vsync_enabled`, and case-insensitive key merge. Not called by any command.
- **Verdict: DIVERGES.**
  - **Case sensitivity bug in the live path.** `utilities.rs:234` does `pending.remove(&key)` against the raw key from the file. C# uses `OrdinalIgnoreCase`. A change submitted with different casing than the file's will **append a duplicate line** instead of updating, and Minecraft reads the first occurrence — the setting silently won't apply. The dead `gl-bedrock` copy gets this right (`options.rs:249`, lowercased keys).
  - **The "close Minecraft first" guard is gone** in both write paths (explicitly documented as omitted at `utilities.rs:203-206` and `options.rs:5-9`). The exact user-facing error strings are therefore absent, and options edits made while the game runs are silently discarded when it exits — precisely the failure the C# guard existed to prevent. Note `commands::shaders` already has `is_bedrock_running()` available; the check is one line away.
  - The `"Minecraft's settings folder doesn't exist yet…"` error is replaced by `create_dir_all` — the command now creates a `minecraftpe` folder that the game didn't ask for instead of telling the user to launch once.
  - `group_rank` is not exposed to the frontend (grouping order is a UI concern; flagged for the UI rewrite).
  - Options path is `instance_dir/minecraftpe/options.txt`, not the resolved live `MinecraftPeDir`.

### BedrockVersionService — MISSING
- **Original:** `Services/BedrockVersionService.cs`, 48 lines. `Refresh()` — WinRT `PackageManager` lookup for both family names, sets `IsInstalled`/`IsPreview`/`DisplayVersion`, with `FormatGameVersion` decoding the packed build number (`build >= 100 → {Major}.{Minor}.{build/100}.{build%100}`).
- **Rust:** none. No crate function, no command. `gl_launch::bedrock::resolve()` returns install location and app id but never the package **version**.
- **Verdict: MISSING.** The "installed Bedrock version" readout the UI shows has no data source. The `build/100` packing rule in particular exists nowhere in the rewrite.

### VanillaVersionService — DIVERGES (see wu.rs deep-dive below)
- **Original:** `Services/VanillaVersionService.cs`, 772 lines. Public surface: `GetVersionsAsync`, `DownloadVersionAsync`, `SwitchVersionAsync`, `DeleteVersion`, `ReregisterActiveAsync`, `EnableDeveloperModeAsync`, `DeveloperModeEnabled`, `VersionsDirectory`, `CacheFile`, `LastError`.
- **Rust:** `crates/gl-bedrock/src/wu.rs` (`list_versions`, `get_download_url`, `extract_appx`, `unregister_current`, `register_version`) + `src-tauri/src/commands/versions.rs` (`list_vanilla_bedrock_versions`, `vanilla_bedrock_version_installed`, `download_vanilla_bedrock_version`, `delete_vanilla_bedrock_version`, `switch_vanilla_bedrock_version`) — all registered.
- **Verdict: DIVERGES.** The WU protocol half is faithful (below). What is missing from the *service*:
  - **Community DB source gone.** `FetchFromCommunityDbAsync` (`versions.txt` from `ddf8196/mc-w10-versiondb-auto-update`) has no Rust equivalent. This was one of three merged sources.
  - **GDK archive source gone** (see GdkArchiveService).
  - **The three-source merge + `LastError` aggregation gone.** C# merges by `"{Version}|{Channel}"`, joins per-source errors with `" | "`, and on total failure falls back to the on-disk cache appending `" (showing cached data)"`. None of this exists; a WU failure now returns a bare error and an empty list.
  - **Version cache gone.** No `SaveVersionCache`/`LoadVersionCache`; `mc-versions.json` is never written or read.
  - **`EnsureDiskSpace` gone.** C# refuses to download when free space `< 4 × downloadBytes`, with the message `"Not enough free space on {root} — about {X} GB is needed and {Y} GB is available."` No equivalent; downloads now fill the disk and fail mid-extract.
  - **Developer Mode gone.** `IsDeveloperModeEnabled` (registry `AppModelUnlock\AllowDevelopmentWithoutDevLicense`) and `EnableDeveloperModeAsync` (elevated `reg add`, with three distinct user-facing failure strings) are absent. `switch_vanilla_bedrock_version` documents the omission at `commands/versions.rs:159` and lets `Add-AppxPackage` fail with its own raw PowerShell error instead of `"Developer Mode must be enabled in Windows Settings > For Developers."`
  - **`ReregisterActiveAsync` gone** — the active sideloaded build is never re-registered on startup, so it silently reverts to the Store build after Windows drops the dev registration.
  - **`ActiveVanillaVersion` is not persisted by the command**; `commands/versions.rs:157` pushes that onto the caller. `DeleteVersion`'s "clear active if it was the deleted one" logic is likewise gone.
  - **Versions directory changed:** `<launcher_root>/mc-bedrock-versions` vs C# `~/Glacier Launcher/mc-versions`. Existing downloads won't be found.
  - `SizeBytes`-only downloads: `DownloadVersionAsync`'s `expectedSha256` path (used for GDK entries) is unreachable since GDK entries don't exist.

### GdkArchiveService — MISSING
- **Original:** `Services/GdkArchiveService.cs`, 78 lines. `GetVersionsAsync()` — GitHub releases of `bubbles-wow/mcbe-gdk-unpack-archive` via `GitHubApiCache`, assets matched by regex `^Microsoft\.(MinecraftUWP|MinecraftWindowsBeta)_(\d+\.\d+\.\d+\.\d+)_x64__8wekyb3d8bbwe\.zip$`, extracting version, channel, `browser_download_url`, `size`, and the `sha256:` digest.
- **Rust:** none — no crate module, no command, no reference to the repo name anywhere.
- **Verdict: MISSING.** Per this project's own memory note (`bedrock-version-download-gdk-cap`), the WU sideload list is **capped at 1.21.120** because of the GDK move; the GDK archive is the *only* source for 1.26.x sideloads. Losing it means the version browser cannot offer any current build.

### StoreInstallService — MISSING
- **Original:** `Services/StoreInstallService.cs`, 157 lines. `InstallAsync(product, progress)` — WinRT `AppInstallManager`; installs Gaming Services first for non-GamingServices products; resolves an existing in-flight `AppInstallItem`, else `UpdateAppByPackageFamilyNameAsync` when already installed, else `StartAppInstallAsync`; `MoveToFrontOfDownloadQueue`; streams `InstallProgress` (state, percent, bytes) via `StatusChanged`. `Cancel()` pauses if already installed, else cancels. `IsBusy`. Three known `Product` records (Minecraft Release `9NBLGGH2JHXJ`, Preview `9P5X4QVLC2XR`, Gaming Services `9MWPM2CQNLHN`).
- **Rust:** none.
- **Verdict: MISSING.** Per the same memory note, Store install is the documented workaround for 1.26.x. Together with the GdkArchiveService gap, the rewrite currently has **no working path to install a current Bedrock build**.

### LevelDatEditorService — MATCHES (improved)
- **Original:** `Services/LevelDatEditorService.cs`, 117 lines. `Load(worldPath)` — 8-byte header (`version`, `length`), length sanity check against file size (`"level.dat header doesn't match the file size — it may be corrupt."`), `NbtIo.ReadRootTag`, then `GameType` (default 0), `Difficulty` (default 2), `commandsEnabled`, `Generator` (default 1), `RandomSeed` (Long or Int), and byte-valued children of `experiments`. `Save(summary)` — re-reads, `SetOrAdd`s the four scalars, creates `experiments` if absent, writes header + payload to `.tmp`, `File.Move(overwrite)`.
- **Rust:** `crates/gl-nbt/src/lib.rs` (`read_level_dat`, `write_level_dat`, `summary_from_root`, `apply_summary_to_root`, `LevelDatSummary`) + `src-tauri/src/commands/leveldat.rs` (`read_level_dat_summary`, `write_level_dat_summary`) — both registered.
- **Verdict: MATCHES.** The write path is strictly *safer* than the original: it copies to `<file>.bak-<unix_ts>` before mutating, writes to `.writing.tmp`, **re-reads and re-parses the temp file and asserts the derived summary equals the intended one**, and only then renames — leaving the original untouched on any failure. Divergences are additive: the backup file is new, and the file-missing error is `"couldn't read <path>: <io error>"` rather than `"This world has no level.dat."` Flag the message text if exact parity matters.

### NbtIo — MATCHES
- **Original:** `Services/NbtIo.cs`, 193 lines. `ReadRootTag`, `WriteRootTag`, `Find`, `SetOrAdd`, `ReadNamedTag`, `ReadPayload`. Uses `BinaryReader`/`BinaryWriter` → **little-endian** (Bedrock NBT), `u16` length-prefixed strings, all 12 tag types including `ByteArray`/`IntArray`/`LongArray`/`List`/`Compound`.
- **Rust:** `crates/gl-nbt/src/lib.rs`, 826 lines. Explicit `from_le_bytes` for every numeric read (`lib.rs:211-251`), `u16` LE string length, `read_root_tag`/`write_root_tag`/`find`/`set_or_add`, gzip helpers, and a `numbers_are_little_endian` test.
- **Verdict: MATCHES.** The endianness distinction — the classic way a Bedrock NBT port goes wrong — is explicitly handled and tested.

### ShaderService — DIVERGES
- **Original:** `Services/ShaderService.cs`, 276 lines. `ShaderRoot`, `BackupDir` (`stock-backup`), `ManifestPath` (`applied.json`), `GetInstall()` (returns `InstalledPath`, **`IsSideloaded`**, `IsPreview`), `MaterialsDir`, `UnavailableReason()`, `LoadApplied`, `HasAppliedShaders`, `ApplyAsync`, `RestoreAsync`, `ListStockMaterials`, `OpenMaterialsFolder`.
- **Rust:** `crates/gl-bedrock/src/shader.rs` (`backup_dir`, `manifest_path`, `load_applied`, `has_applied_shaders`, `apply`, `restore`, `list_stock_materials` — same `stock-backup` / `applied.json` names) + `src-tauri/src/commands/shaders.rs` (`shaders_unavailable_reason`, `list_applied_shaders`, `apply_shaders`, `restore_shaders`, `list_stock_shader_materials`, `open_shaders_materials_folder`) — all registered. Apply/restore round-trip is unit-tested in `gl-bedrock/src/lib.rs`.
- **Verdict: DIVERGES.** The file-copy engine is a faithful port. The precondition check is not:
  - `IsSideloaded` (WinRT) is replaced by a **write-probe** — `check_writable` writes `.glacier-write-test` into the live game's `data/renderer/materials` and deletes it (`commands/shaders.rs:44`). Documented as deliberate. It is a reasonable proxy, but it writes into the installed package directory on every one of the six commands, and any transient failure (AV lock, in-flight update) produces the sideload error message rather than the true cause.
  - Error message text is rewritten wholesale; compare against `ShaderService.UnavailableReason()` if parity matters.
  - `GetInstall().IsPreview` is not surfaced.
  - Shader root is `%USERPROFILE%\Glacier Launcher\shaders`, which does match the C# location.

### FlarialService — DIVERGES
- **Original:** `Services/FlarialService.cs`, 79 lines. `IsDownloaded`, `IsUpToDateAsync` (SHA-256 of local vs the `"Release"` key of `cdn.flarial.xyz/dll_hashes.json`, `OrdinalIgnoreCase`), `DownloadAsync` (**early-returns if already up to date**), `Delete`. Path: `<Downloads>/Flarial/Flarial.Client.Release.dll`.
- **Rust:** `crates/gl-clients/src/flarial.rs` (`fetch_remote_hash`, `parse_remote_hash`, `local_hash`, `is_up_to_date`, correct `DOWNLOAD_URL`/`HASHES_URL`/`HASH_KEY`/`FILE_NAME` constants, well unit-tested) + `commands/clients.rs` (`check_flarial_dll_up_to_date`, `download_flarial_client`, `flarial_client_path`, `delete_flarial_client`) — registered.
- **Verdict: DIVERGES (minor).** `download_flarial_client` does **not** short-circuit when already up to date — it re-downloads unconditionally. It does verify against the published hash via `with_sha256`, which is an improvement over the C# download (which verified only on the *next* `IsUpToDateAsync`). Install dir moved from `<Downloads>/Flarial` to `<launcher_root>/clients/Flarial`.

### OderSoServices — DIVERGES
- **Original:** `Services/OderSoServices.cs`, 198 lines. `ListDllsAsync` (GitHub contents API via `GitHubApiCache`, `.dll` files only, sorted by `CompareVersionNames` — a **numeric-segment-aware** comparer, descending), `GetLatestDllAsync`, `IsEntryDownloaded`, `GetEntryFilePath`, `GetActiveEntry`/`SetActiveEntry` (persisted to `OderSo.ver` as `name|sha`), `DownloadEntryAsync`, `DeleteEntry`, `IsUpToDateAsync` (stored sha vs latest sha), `DownloadAsync`, `Delete`, `IsDownloaded` (`OderSo.ver` exists **and** the referenced DLL exists), `FilePath`, `LastError`.
- **Rust:** `crates/gl-clients/src/oderso.rs` (`list_dlls`, `parse_dll_list`, `raw_url`) + `commands/clients.rs` (`list_oderso_dlls`, `download_oderso_client`, `oderso_client_installed`, `oderso_client_path`, `delete_oderso_client`) — registered.
- **Verdict: DIVERGES.**
  - **Sort is wrong.** `oderso.rs:59` uses `b.name.cmp(&a.name)` — a lexicographic descending string sort, self-described as an "approximation". The C# `CompareVersionNames` extracts all `\d+` runs and compares numerically. So `v10` sorts *below* `v9`, and `download_oderso_client` (which takes `entries[0]`) will fetch a stale build once a two-digit version appears.
  - **No `OderSo.ver` tracking.** `GetActiveEntry`/`SetActiveEntry`/`IsUpToDateAsync`/per-entry download and delete are all absent. `oderso_client_installed` merely checks the folder is non-empty; `oderso_client_path` returns the *first* `.dll` found by `read_dir` (arbitrary order), not the active one.
  - `urlencoding_lite` escapes only spaces, whereas `Uri.EscapeDataString` escapes the full unsafe set.
  - `LastError` / GitHub rate-limit message surfacing is gone (no `GitHubApiCache` equivalent in this path).

### LeviLaminaService — DIVERGES
- **Original:** `Services/LeviLaminaService.cs`, 126 lines. `IsUpToDateAsync` (`.version` file vs latest tag), `DownloadAsync` — downloads to a temp zip, **deletes and recreates the install directory**, extracts, **verifies a loader DLL exists and throws `"LeviLamina was downloaded but no loader DLL could be found inside the release archive."` if not**, and only then writes `.version`. `Delete` (recursive delete + recreate). `FindLoaderDll` prefers a name containing "Lamina", then "Loader", then the sole DLL if exactly one.
- **Rust:** `crates/gl-clients/src/levilamina.rs` (`fetch_latest_release`, `find_loader_dll`) + `commands/clients.rs` (`download_levilamina_client`, `levilamina_client_installed`, `levilamina_client_path`, `delete_levilamina_client`, `fetch_levilamina_release`) — registered.
- **Verdict: DIVERGES.**
  - **Ordering bug:** `commands/clients.rs` writes `.version` *before* checking `find_loader_dll`. A release archive without a loader DLL leaves `.version` on disk, so `levilamina_client_installed` reports `true` forever while `levilamina_client_path` errors. C# writes `.version` last, after the check.
  - **No pre-extract wipe.** The Rust path extracts on top of whatever is already there; C# deletes the directory first. Stale DLLs from a previous version survive and can be picked by `find_loader_dll`.
  - `IsUpToDateAsync` has no command.
  - Install dir moved to `<launcher_root>/clients/LeviLamina`.

### LeviLaminaModsService — DIVERGES
- **Original:** `Services/LeviLaminaModsService.cs`, 196 lines. `SearchAsync(query)`, `IsInstalled(mod)`, `DownloadAndInstallAsync(mod, progress, cancel)` — resolves the release asset then **extracts the archive into the mods folder**; `Delete(mod)`; `ModsDirectory`; `LastError`.
- **Rust:** `crates/gl-clients/src/levilamina_mods.rs` (`parse_registry`, `resolve_latest_asset`, `sanitize_folder_name`, `REGISTRY_URL`) + `commands/levilamina_mods.rs` (`fetch_levilamina_mods`, `levilamina_mod_installed`, `download_levilamina_mod`, `delete_levilamina_mod`, `open_levilamina_mods_folder`) — registered.
- **Verdict: DIVERGES.** The **extraction step is explicitly not ported** (`commands/levilamina_mods.rs:8-11` and `levilamina_mods.rs:160`): the release asset is downloaded as a raw file into the install folder and left as-is. A downloaded mod is therefore never actually installed in a form LeviLamina can load. `levilamina_mod_installed` returns `true` for that inert archive. Client-side `SearchAsync` filtering is also absent (the whole registry is returned).

### GlacierClientService — MISSING
- **Original:** `Services/GlacierClientService.cs`, 71 lines. `GetManifestAsync(forceRefresh)` (cached fetch of `cdn.glacierclient.xyz/versions.json`, `LastError` on failure), `InstallAsync` (SHA-256-verified download to `JarPath`), `Uninstall`, `LaunchAsync` (Java launch with `-javaagent:<jar>`, throws `"Glacier {id} is not installed."`), `GetGlacierDir` (`~/.glacier`), `GetInstalledSizeBytes`.
- **Rust:** `crates/gl-clients/src/glacier_client.rs` exists (186 lines), but **no `#[tauri::command]` references it** — grep across `src-tauri/src` finds only a doc-comment mention in `commands/launchers.rs:14`.
- **Verdict: MISSING** (crate code present but entirely unreachable). Java-side, so lower priority for a Bedrock-first pass, but it is dead code today.

### ClientLibraryService — DIVERGES
- **Original:** `Services/ClientLibraryService.cs`, 249 lines. `GetPacksAsync(refresh)` (with an in-process cache honoring `refresh`), `Versions()`, `InstallAsync`, `IsInstalled(download)`, `Uninstall(download)`, `SiteUrl`, `LastError`.
- **Rust:** `crates/gl-clients/src/client_library.rs` (`site_url`, `index_url`, `parse_index`, `versions`, `media_url`) + `commands/client_library.rs` (`client_library_site_url`, `get_client_library_packs`, `client_library_versions`, `is_client_library_download_installed`, `install_client_library_download`, `uninstall_client_library_download`) — registered.
- **Verdict: DIVERGES.** The `refresh` parameter is accepted and **ignored** (`_refresh`, `commands/client_library.rs:22-25`) — there is no cache, so every panel render re-fetches the catalog over the network. `LastError` surfacing is gone. Install target is `instance_dir/resource_packs/<stem>` rather than the resolved live `ResourcePacksDir`.

### CustomClientLibraryService — DIVERGES
- **Original:** `Services/CustomClientLibraryService.cs`, 250 lines. `All`, `PathOf`, `Default`/`SetDefault`, `AddFromFile`, `AddFromUrlAsync`, `Rename`, `SetSourceUrl`, `ToggleAutoUpdate`, `Delete`, `UpdateIfChangedAsync`, `Rehash`, `OpenFolder`, `LastUpdateNote`.
- **Rust:** `crates/gl-clients/src/custom_client_library.rs` (pure helpers) + `commands/utilities.rs` (`list_custom_clients`, `add_custom_client_from_file`, `add_custom_client_from_url`, `rename_custom_client`, `toggle_custom_client_auto_update`, `delete_custom_client`, `open_custom_client_library_folder`) — registered.
- **Verdict: DIVERGES.** `SetDefault`/`Default`, `SetSourceUrl`, `UpdateIfChangedAsync` (the actual auto-update execution — the toggle exists but nothing acts on it), `Rehash`, and `LastUpdateNote` have no commands. `custom_client_library.rs:5-6` documents the download half as out of scope.

### TweaksService — DIVERGES
- **Original:** `Services/TweaksService.cs`, 274 lines. `GetVanillaResourcePacksAsync(version)`, `BuildVanillaResourcePackAsync`, `GetBedrockResourcePacksAsync` (parses `packs.json`, derives `_btEngineVersion` as `[maj, min(min,21), 0]`, sets `BedrockVersionLabel`, strips HTML from descriptions, builds jsDelivr icon URLs), `BuildBedrockResourcePackAsync` (git-tree blob resolution, 6-way concurrent raw downloads, `WriteBedrockManifest` with fresh UUIDs + `min_engine_version`, zip to a timestamped `.mcpack`, then **`_curseforge.ImportBedrockPackFileAsync(mcpack)`** to install it), `JavaVersions`.
- **Rust:** `crates/gl-integrations/src/tweaks.rs` + `commands/tweaks.rs` (`tweaks_java_versions`, `list_vanilla_tweaks`, `build_vanilla_tweaks_pack`, `list_bedrock_tweaks`, `build_bedrock_tweaks_pack`, `fetch_image_bytes`) — all registered. Matches this project's memory note on the two distinct API shapes.
- **Verdict: DIVERGES.** `build_bedrock_tweaks_pack` writes the assembled `.mcpack` to a caller-supplied `dest_dir` and returns the path; the **final import step** (`ImportBedrockPackFileAsync`, which detects the pack type and extracts it into the right com.mojang folder) is explicitly out of scope (`gl-integrations/src/tweaks.rs:11-14`, `:144`). The user gets a file, not an installed pack. `fetch_image_bytes` is a sensible addition (works around the vanillatweaks.net referer block).

### GameLauncher — DIVERGES
- **Original:** `Services/GameLauncher.cs`, 471 lines. `GetVersionsAsync` (Latite releases), `DownloadVersionAsync`, `DeleteVersion`, `GetDllPath(tag)` (`Latite_{tag}.dll`), `LaunchWithDllAsync`, `LaunchVanillaAsync`, `LaunchServerAsync(dll, name, host, port)`, `LaunchAsync(versionTag, useFlarial)`, `IsMinecraftRunning`, plus the private `LaunchWithPathCoreAsync` orchestration, `StartMinecraftAsync` (5 activation strategies), `PreventSuspend`, `AccelerateLoadingAsync`.
- **Rust:** `crates/gl-launch/src/bedrock.rs::launch` + `commands/launch.rs::launch_bedrock_instance`, `is_bedrock_game_running`, `cancel_launch`; Latite side in `commands/versions.rs`. All registered. (The `//! NOT YET WIRED UP` banner at `commands/launch.rs:8-12` is **stale** — `gl-launch` is a dependency in `src-tauri/Cargo.toml:45` and every command in the file is registered.)
- **Verdict: DIVERGES.**
  - **Activation strategy list is shorter and reordered.** C#: `Invoke-CommandInDesktopPackage (GDK)` → COM `ApplicationActivationManager` → `minecraft://` URI → `cmd /c start minecraft:` → `Invoke-CommandInDesktopPackage`. `gl-launch`: GDK invoke → `cmd /c start minecraft:` → invoke. The COM activation path and the shell `minecraft:` `UseShellExecute` path are gone; the dead `gl-bedrock` copy at least keeps an `explorer.exe shell:AppsFolder\<aumid>` strategy.
  - **Wait budget halved.** `gl-launch/src/bedrock.rs:198` polls 20 × 500 ms (10 s) per strategy; C# does 20 × 500 ms per strategy *plus* a separate 60 × 500 ms (30 s) main-window wait after activation. On a cold GDK start this will time out where the original succeeded. (The dead `gl-bedrock` copy uses 40 iterations.)
  - **Main-window wait gone** — the C# `"Waiting for Minecraft main window…"` loop, which is what makes the injection delay meaningful, is absent.
  - **PID re-target logging gone.** C# re-resolves after the injection delay and, if the PID changed, logs `"Minecraft restarted during the delay — retargeting PID …"`. Rust re-resolves but reports `LaunchError::BedrockLaunchTimedOut` on absence rather than `"Minecraft exited before injection could complete."`
  - **`PreventSuspend`** (AppLifecycleOptOut.exe) and **`AccelerateLoadingAsync`** (delayed RuntimeBroker kill) are not ported — documented at `gl-launch/src/bedrock.rs:11-15`. Both are settings-driven (`PreventGameSuspend`, `AccelerateLoadingMs`), so those settings are now inert.
  - **`LaunchServerAsync`** (the `minecraft://addExternalServer` direct-connect deep link) is not ported.
  - **`LaunchAsync`'s `LastUsedVersion` persistence** is not ported.
  - Latite DLL path/dir moved to `<launcher_root>/clients/Latite`.

### GameConsoleService — MISSING (substituted)
- **Original:** `Services/GameConsoleService.cs`, 109 lines. `Open(title)` returns a `GameConsoleHandle` wrapping a WPF `GameConsoleWindow`, with `Info`/`Stdout`/`Stderr`/`Error`, `MarkRunning`/`MarkExited(code)`/`MarkFailed(why)`, `SetPid`, `Attach(Process)` (redirected stdout/stderr pumping), `Dispose`. Gated on `Settings.ShowLaunchConsole`.
- **Rust:** `commands/launch.rs` opens a second `WebviewWindow` labelled `game-console` and emits `game-console://log` / `game-console://exit` events; `emit_log`, `open_console_window`, `stream_child_output`.
- **Verdict: MISSING as a service; partially substituted.** The transport exists but the semantics don't: there is no `MarkRunning`/`MarkExited`/`MarkFailed`/`SetPid` state model, and the `ShowLaunchConsole` setting is not consulted — the console window opens unconditionally on every launch. Crucially, the C# console is where the **entire Bedrock launch diagnostic trail** goes (`"Launching via {name}…"`, per-strategy stderr, `"Main window appeared after Xs"`, `"Injecting … into PID …"`, `"Injection confirmed"`); `gl-launch::bedrock::launch` takes no progress callback at all, so only three summary lines ever reach the user. (The dead `gl_bedrock::launch::start_and_wait` *does* take an `on_progress` closure and emits exactly those strings — another case where the unreachable copy is the better port.)

### InjectionService — see deep-dive

---

## Deep-dive 1: `crates/gl-injection/src/lib.rs` (553 lines) vs `Services/InjectionService.cs` (337 lines)

**Claim verified: this is a genuine, faithful, and in places safer port — with one real gap.**

Read in full. Correspondence:

| C# | Rust | Notes |
|---|---|---|
| `EnableDebugPrivilege` | `enable_debug_privilege` / `try_enable_debug_privilege` | Same `OpenProcessToken(TOKEN_ADJUST_PRIVILEGES\|TOKEN_QUERY)` → `LookupPrivilegeValueW("SeDebugPrivilege")` → `AdjustTokenPrivileges` sequence. C#'s `catch {}` best-effort is mirrored by discarding the `Result`, and the `ERROR_NOT_ALL_ASSIGNED` non-error case is explicitly documented. |
| `InjectDll` | `inject_dll` | Same 7-step sequence in the same order: grant UWP ACL on the DLL and its parent → enable debug privilege → `OpenProcess(PROCESS_ALL_ACCESS)` with fallback to the narrow mask → `VirtualAllocEx` + `WriteProcessMemory` of the UTF-16 NUL-terminated path → `GetProcAddress(GetModuleHandleW("kernel32.dll"), "LoadLibraryW")` → `CreateRemoteThread` → 30,000 ms `WaitForSingleObject` → `GetExitCodeThread == 0` rejection check → optional 5 s module verify. |
| `PROCESS_INJECT_ACCESS` constant | `PROCESS_INJECT_ACCESS` | C# `0x0002\|0x0008\|0x0010\|0x0020\|0x0400` = VM_OPERATION\|VM_READ\|VM_WRITE\|QUERY_INFORMATION\|CREATE_THREAD. Rust composes the same five named flags. **Bit-identical.** |
| `ProbeModule` (via `Process.Modules`) | `probe_module` (via `CreateToolhelp32Snapshot(TH32CS_SNAPMODULE\|TH32CS_SNAPMODULE32)`) | Different mechanism, same tri-state `Confirmed`/`Absent`/`Unavailable` result. |
| `IsModuleLoaded` | `is_module_loaded` | Identical. |
| `WaitForModule` | `wait_for_module` | Same 150 ms poll, same deadline, and — importantly — the same subtle `!ever_read_it` semantics: never having *successfully read* the module list counts as "can't tell, assume okay", while a readable list that never contained the DLL is a genuine failure. This is the kind of detail that usually gets lost; it is preserved and explicitly commented. |
| `GrantUwpAccess` | `grant_uwp_access` / `try_grant_uwp_access` | `BuildTrusteeWithNameW("ALL APPLICATION PACKAGES")` with `TRUSTEE_IS_NAME` + `TRUSTEE_IS_WELL_KNOWN_GROUP`, `EXPLICIT_ACCESS_W { GENERIC_ALL, SET_ACCESS, SUB_CONTAINERS_AND_OBJECTS_INHERIT }`, `SetEntriesInAclW`, `SetNamedSecurityInfoW(SE_FILE_OBJECT, DACL_SECURITY_INFORMATION)`, `LocalFree`. Matches constant for constant. |
| `LaunchMinecraft(aumid)` | **absent** | See gap below. |
| — | `find_process_by_name` | New; `TH32CS_SNAPPROCESS` scan. Replaces `BedrockPackageService.FindGameProcess`, but without its family-name matching. |

Improvements over the original (all safe and documented):
- RAII `OwnedHandle` / `RemoteAlloc` guards close every handle and free the remote allocation on **every** exit path, including the early `return Err` paths. The C# version's `finally` covers this too, but the Rust version cannot regress.
- `dll_path.canonicalize()` + `strip_verbatim_prefix` before injecting, so a relative path can't resolve differently inside the target's own working directory. The C# version writes whatever string it was given.
- Error messages are `thiserror` variants carrying the Win32 code, and the two most important user-facing strings are preserved in substance: `OpenProcessFailed` contains "try running the launcher as administrator" (C#: "Try running the launcher as administrator.") and `LoadLibraryRejected` contains "architecture mismatch, missing dependency, or the DLL crashed in DllMain" (C#: "x86 DLL injected into x64 Minecraft, missing dependency, or the DLL crashed in DllMain"). Wording differs; meaning is intact.
- Six unit tests covering the parts testable without a live target (verbatim-prefix stripping, missing-DLL early return, NUL-terminated buffer handling, error-message content).

**The one real gap:** `InjectionService.LaunchMinecraft(aumid)` — the COM
`IApplicationActivationManager::ActivateApplication` activation, including its
`"Minecraft Bedrock isn't installed for this user. Install it from the Microsoft Store first."`
and `"ActivateApplication({aumid}) failed with HRESULT 0x{hr:X8}."` errors — is not ported here or
anywhere. It was the **second** activation strategy in `GameLauncher.StartMinecraftAsync`, i.e. the
primary fallback when the GDK `Invoke-CommandInDesktopPackage` path fails. Its absence is the main
reason the Rust activation chain is weaker (see GameLauncher row).

Also note `inject_dll`'s `verify` parameter is required in Rust, whereas C# defaults it to `true`;
`commands/injection.rs` takes it from the frontend, and `gl-launch` takes it from
`settings.verify_injection` — both correct, but a frontend that omits it will silently skip
verification.

**Verdict: FAITHFUL** for everything it covers. The gap is `LaunchMinecraft`, not the injection core.

## Deep-dive 2: `crates/gl-bedrock/src/wu.rs` (632 lines) vs `Services/VanillaVersionService.cs`

**Claim verified with a caveat: the Windows Update SOAP protocol is faithfully ported. The
*service* around it is not (see the VanillaVersionService row above for the six missing pieces).**

Read in full. Protocol-level correspondence:

| Element | Fidelity |
|---|---|
| `WU_ENDPOINT` / `WU_ENDPOINT_SECURED` / `WU_CATEGORY_ID` (`d25480ca-…`) / `WUWS_NS` | Identical strings. |
| `INSTALLED_NON_LEAF_UPDATE_IDS` | All 44 IDs, same values, same order. Verified element by element. |
| `DEVICE_ATTRIBUTES` | Byte-identical to the C# inline string (modulo the C#'s XML `&amp;` escaping — Rust emits raw `&`, which is technically invalid XML inside an element; WU has historically tolerated it, but this is worth a live test). |
| `security_header()` | Same WS-Security envelope, same `Created`/`Expires` (+5 min), same `WindowsUpdateTicketsToken` / `MBI_SSL` ticket. Uses `iso7_from_systemtime` for a genuine +5 min rather than a string hack — the code comment explicitly calls out and avoids that trap. |
| `get_auth_cookie` (`GetCookie`) | Same body, same `lastChange` literal `2015-10-21T17:01:07.1472913Z`, same `protocolVersion 1.40`, extracts `EncryptedData`. Error text `"failed to get auth cookie from Microsoft Store"` matches C#'s `"Failed to get auth cookie from Microsoft Store."` |
| `fetch_updates` (`SyncUpdates`) | Every element of the `<parameters>` block reproduced in the same order with the same values. |
| Fragment filtering | Same four filters: must contain `Microsoft.MinecraftUWP` or `Microsoft.MinecraftWindowsBeta`; skip `.EAppx`; skip if contains "arm" without "x64"; `PackageMoniker` must contain `x64`. |
| `get_download_url` (`GetExtendedUpdateInfo2`) | Same body, `FileUrl` + `FileDecryption` info types, same secured endpoint, same URL selection predicate (`.appx` or contains "microsoft"). |
| `parse_version_from_package` | Hand-written equivalents of the two C# regexes: `[_\-]v(\d{1,3}\.\d{1,3}(?:\.\d{1,3})?)(?=[_\-]\|$)` → `named_scheme_version` (2–3 parts, ≤3 digits each, boundary check) and `(\d+\.\d+\.\d+(?:\.\d+)?)` → `generic_version` (3–4 parts, unbounded digits). Careful and correct; unit-tested. |
| `extract_appx` | Mirrors `ExtractPackage`: finds `AppxManifest.xml`, derives and strips its directory prefix case-insensitively, skips `AppxSignature*` and `[Content_Types]*`, and keeps the zip-slip guard. **Error text is verbatim identical**: "This archive has no AppxManifest.xml — it isn't a registrable Minecraft package." |
| `unregister_current` / `register_version` | Same PowerShell commands, same `''` quote escaping, same "not found"/"no package" → success special-case, same `"Failed to unregister current version: …"` / `"Failed to register version: …"` prefixes. |
| `version_key` + sort | Equivalent to `TryParseVersion` + `OrderByDescending`. |

Three genuine protocol-level divergences found:

1. **`PackageMoniker` lookup is looser.** C# (`VanillaVersionService.cs`, `FetchUpdatesAsync`) reads
   `fragDoc.Descendants("AppxPackageInstallData").Select(e => e.Element("AppxMetadata")…)` — i.e.
   `AppxMetadata` must be a *direct child of* `AppxPackageInstallData`. Rust (`wu.rs:248`) takes the
   first `AppxMetadata` **anywhere** in the fragment. On a fragment carrying multiple appx entries
   (e.g. a bundle with dependencies) this can pick the wrong package moniker, yielding a wrong
   version string or a dependency package instead of the game.
2. **Duplicate-ID resolution is inverted.** C# builds a `Dictionary<string,string>` keyed by update
   ID, so a later `<UpdateInfo>` with the same ID **overwrites** the earlier one. Rust (`wu.rs:213`)
   does `if !id_map.iter().any(…)` — first wins. Rarely observable, but it is a behavior difference.
3. **Errors are not swallowed per-fragment.** C# wraps each fragment in `try/catch {}`; Rust uses
   `let Ok(…) = … else { continue }`, which is equivalent in effect. No issue — noted for completeness.

**Verdict: FAITHFUL for the WU protocol.** `wu.rs` is a real port, not a stub, and the hard parts
(the 44 magic IDs, the security header, the fragment filters, the two version regexes, the appx
prefix-stripping) are all correct. It is *not* a port of `VanillaVersionService` as a whole — it
covers roughly the WU-facing 40% of that 772-line file, and the surrounding service logic
(community DB, GDK archive, merge, cache, disk-space guard, developer mode, re-register) is absent.

---

## Remaining stubs

`grep -rn "TODO\|todo!\|unimplemented!" --include=*.rs crates src-tauri/src` — there are **zero**
`todo!()` or `unimplemented!()` macros in the workspace. Nothing will panic at runtime. Every hit is
a `// TODO` comment or a prose "not ported / out of scope" note. Bedrock-relevant ones:

| Location | What it says | User-visible? |
|---|---|---|
| `crates/gl-bedrock/src/instance.rs:186` | Post-switch game-version change not wired | **Yes** — but moot: `switch_to` has no command at all. |
| `crates/gl-bedrock/src/instance.rs:10-12` | Installed-version switch out of scope | Yes (same). |
| `crates/gl-bedrock/src/lib.rs:6-9` | `BedrockPackageService` out of scope for this crate | No — `gl-launch` covers it. |
| `crates/gl-bedrock/src/lib.rs:19-22` | com.mojang root resolution is a "cross-cutting concern outside this crate" | **Yes — the highest-risk item in this document.** Nothing else implements it. |
| `crates/gl-bedrock/src/shader.rs:7` | Materials dir resolution deferred to caller | No — `commands/shaders.rs` resolves it. |
| `crates/gl-bedrock/src/launch.rs:15` | Injection out of scope here | No — module is dead code anyway. |
| `crates/gl-integrations/src/tweaks.rs:11,13,144` | Blob downloads deferred; `ImportBedrockPackFileAsync` out of scope | **Yes** — built Bedrock tweak packs are never installed into com.mojang. |
| `crates/gl-clients/src/levilamina_mods.rs:160` | Download/extract out of scope | **Yes** — mods download but never extract, so they never load. |
| `crates/gl-clients/src/levilamina.rs:4-5,30` | Download/extract out of scope; GitHub API caching not ported | Partly — download is wired in `commands/clients.rs`; caching loss means rate-limit exposure. |
| `crates/gl-clients/src/client_library.rs:6-7` | Download out of scope | No — wired in `commands/client_library.rs`. |
| `crates/gl-clients/src/custom_client_library.rs:5-6` | URL download out of scope | Partly — `add_custom_client_from_url` exists; `UpdateIfChangedAsync` does not. |
| `crates/gl-clients/src/glacier_client.rs:3,66` | Download + launch out of scope | **Yes** — no commands at all; whole module unreachable. |
| `crates/gl-launch/src/bedrock.rs:8-16,244` | `PreventSuspend`, `AccelerateLoadingAsync`, `addExternalServer` deep link not ported | **Yes** — three settings are now inert. |
| `src-tauri/src/commands/versions.rs:159` | Developer Mode precondition not ported | **Yes** — raw PowerShell error instead of the actionable message. |
| `src-tauri/src/commands/utilities.rs:12,203-206,317` | Live CPU%, GPU/mobo/BIOS (WMI), and the "close Minecraft first" options guard not implemented | **Yes** — the options guard especially. |
| `src-tauri/src/commands/launch.rs:8-12` | "NOT YET WIRED UP" banner | **No — this comment is stale/wrong.** `gl-launch` is a dependency and all five commands are registered. Delete it. |
| `crates/gl-update/src/resume.rs:17-18`, `crates/gl-java/src/*` | Java/updater-side download TODOs | Out of Bedrock scope. |

`#![allow(dead_code)]` appears in `commands/leveldat.rs` and `commands/tweaks.rs`; `gl-bedrock`'s
`options.rs` and `launch.rs` are dead in practice without the attribute.

---

## Recommended order of work (Bedrock-first)

1. **Implement com.mojang root resolution** (`ResolveBestRootFor` / `LiveRootFor`, per-folder, walking every per-account `games/com.mojang`) and route worlds/packs/options/backups/screenshots/instances through it. Nothing else in the Bedrock half is correct until this exists.
2. **Restore version-install capability** — GdkArchiveService and/or StoreInstallService. Without one of them, no current (1.26.x) Bedrock build can be installed.
3. **Fix `find_game_process` to match the package family name** before injecting (`gl-launch/src/bedrock.rs:130`), and fix `resolve()`'s `Get-AppxPackage -Name` to use the short package name.
4. **Add `switch_bedrock_instance` / `set_bedrock_instance_version` / `save_active_bedrock_instance` commands**, and seed `AppState`'s instance service with a real `live_root`.
5. **Restore the activation chain**: port `LaunchMinecraft` (COM `IApplicationActivationManager`), re-add the `explorer.exe shell:AppsFolder` and shell-`minecraft:` strategies, restore the 30 s main-window wait, and thread an `on_progress` callback into the console window.
6. **Fix the options case-sensitivity bug** and re-add the "close Minecraft first" guard (`is_bedrock_running()` already exists in `commands/shaders.rs`).
7. **Fix the LeviLamina `.version` ordering bug and pre-extract wipe**; fix the OderSo numeric version sort; add mod archive extraction.
8. **Reconcile the duplicate implementations** — delete either `gl-bedrock::launch` or `gl-launch::bedrock`, and either `gl-bedrock::options` or the inline copy in `utilities.rs`, keeping the better port in each pair (`gl-bedrock` in both cases).
9. Restore `VanillaVersionService`'s community-DB source, three-source merge, cache fallback, disk-space guard, Developer Mode check, and `ReregisterActiveAsync`.
10. Delete the stale `NOT YET WIRED UP` banner in `commands/launch.rs`, and treat `HANDOFF_PROMPT.md` / `injection-plan.md` as deleted.
