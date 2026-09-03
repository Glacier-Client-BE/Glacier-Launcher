# Bedrock Panel Inventory — original WPF/Blazor app

Source of truth: `C:\Users\User\MinecraftLauncher-wpf-backup-full` (READ-ONLY).
All line numbers are 1-based and refer to the file as it exists in that backup.

- `Pages/Home.razor` — 5,646 lines, all markup.
- `Pages/Home.*.cs` — 24 partial-class code-behind files.
- `Components/*.razor` — 17 shared components.

Everything below was verified by reading the files. `HANDOFF_PROMPT.md` and
`injection-plan.md` in the rewrite repo were **not** consulted.

---

## 0. Authoritative navigation model

### 0.1 Bottom tab strip — `BedrockTabs` (`Pages/Home.BigFeatures.cs:54-75`)

Seven tabs, in this exact order. Each renders `<button class="panel-tab">`
(`panel-tab active` when it is the current tab) containing `<i class="…"></i>` +
label text. **Note:** the active tab has *no* `onclick` attribute at all
(`Home.BigFeatures.cs:60-61`) — clicking the active tab is a no-op, not a
re-navigation. `JavaTabs` (lines 31-52) always wires `onclick`; this asymmetry is real.

| # | seq | view key | icon | label | handler |
|---|-----|----------|------|-------|---------|
| 1 | 0   | `settings`   | `fa-solid fa-gear`               | Settings  | `OpenSettings` |
| 2 | 10  | `clients`    | `fa-solid fa-puzzle-piece`       | Clients   | `() => _ = OpenClients()` |
| 3 | 20  | `addons`     | `fa-solid fa-boxes-stacked`      | Packs     | `() => OpenAddons()` |
| 4 | 30  | `mcversions` | `fa-solid fa-box-archive`        | Versions  | `() => _ = OpenMcVersions()` |
| 5 | 40  | `worlds`     | `fa-solid fa-globe`              | Worlds    | `() => _ = OpenBedrockWorlds()` |
| 6 | 50  | `utilities`  | `fa-solid fa-screwdriver-wrench` | Utilities | `() => _ = OpenUtilities()` |
| 7 | 60  | `credits`    | `fa-solid fa-heart`              | Credits   | `OpenCredits` |

The tab *key* is not always the same as `currentView`: e.g. `BedrockTabs("worlds")`
is passed by both the `servers` and `bedrockworlds` views; `BedrockTabs("addons")`
is passed by `addons`, `tweaks`, `bedrockpacks` and `clientlibrary`;
`BedrockTabs("utilities")` by `utilities`, `bedrockbackups` and `bedrockscreenshots`;
`BedrockTabs("mcversions")` by `mcversions` and `bedrockinstances`.

### 0.2 Cross-nav sub-tab strips — `Pages/Home.MergedPanels.cs`

All four are built by the `SubTabs` helper (`Home.MergedPanels.cs:11-29`), which
emits `<div class="versions-client-switcher">` containing
`<button class="vcs-btn">` / `vcs-btn active`, each with `<i class="icon">` and
`" " + Label` (note the **leading space** before the label, line 25).

| fragment | lines | keys → label / icon / handler |
|---|---|---|
| `WorldsServersTabs` | 31-33 | `worlds` Worlds `fa-solid fa-globe` → `OpenBedrockWorlds()`; `servers` Servers `fa-solid fa-server` → `OpenServers` |
| `VersionsInstancesTabs` | 35-37 | `versions` **MC Versions** `fa-solid fa-box-archive` → `OpenMcVersions()`; `instances` Instances `fa-solid fa-layer-group` → `OpenBedrockInstances()` |
| `PacksAddonsTabs` | 39-43 | `addons` **Browse** `fa-solid fa-cloud-arrow-down` → `OpenAddons()`; `clientlib` **Clients** `fa-solid fa-wand-sparkles` → `OpenClientLibrary()`; `tweaks` Tweaks `fa-solid fa-wand-magic-sparkles` → `OpenTweaks()`; `packs` **Manage** `fa-solid fa-boxes-stacked` → `OpenBedrockPacks()` |
| `UtilitiesTabs` | 45-51 | `options` Options `fa-solid fa-sliders`; `shaders` Shaders `fa-solid fa-droplet`; `clients` **Library** `fa-solid fa-file-code`; `system` System `fa-solid fa-microchip` (these four → `SwitchUtilTab(key)`); `backups` Backups `fa-solid fa-clock-rotate-left` → `OpenBedrockBackups()`; `photos` Photos `fa-solid fa-images` → `OpenBedrockScreenshots()` |

Where each strip is rendered in `Home.razor`:
`PacksAddonsTabs` at 2386 (addons), 3158 (tweaks), 3952 (bedrockpacks), 4045 (clientlibrary);
`WorldsServersTabs` at 3736 (servers), 3852 (bedrockworlds);
`UtilitiesTabs(_utilTab)` at 3286 (utilities), `UtilitiesTabs("backups")` at 4209, `UtilitiesTabs("photos")` at 4398;
`VersionsInstancesTabs` at 4319 (instances), 4702 (mcversions).
The `levimods` panel (2988–3101) renders **no** sub-tab strip.

### 0.3 Shared panel chrome (every panel below uses it)

```
<div class="panel-overlay">
  <div class="panel-handle"></div>
  <div class="panel-header">
    <div class="panel-title-wrap">
      <span class="panel-title">…</span>
      <div class="panel-title-underline"></div>
    </div>
    <div class="panel-header-actions"> … <button class="panel-back-btn" @onclick="GoHome"
         data-tooltip="Back"><i class="fa-solid fa-chevron-down"></i></button> </div>
  </div>
  [optional .versions-client-switcher sub-tab strip]
  [optional .panel-search-wrap > i.fa-magnifying-glass + input.panel-search-input + button.panel-search-clear]
  <div class="panel-body"> … </div>
  <div class="panel-tabs">@BedrockTabs("…")</div>
</div>
```

Header action buttons are `<button class="panel-icon-btn">`, sometimes with the
extra class `versions-sort-btn`, `active`, or `spinning` (spin during load).
Nearly every interactive element carries `@onmousedown:stopPropagation`
(the panel is drag-dismissable) and `data-tooltip="…"`.

---

## 1. Panel table — Bedrock views in `Home.razor`

Ranges are `@if (currentView == "…")` block start → the line before the next `@if`.
End lines are therefore upper bounds; the closing `}` of each block is at or just
before the listed end line.

| # | view key (`currentView`) | Title text | `Home.razor` lines | bottom tab passed | sub-tab strip |
|---|---|---|---|---|---|
| 1 | `settings` | `Settings` | 1013–1931 | `BedrockTabs("settings")` @1922 / `JavaTabs("settings")` @1926 | own inline `.versions-client-switcher` (not a MergedPanels fragment) |
| 2 | `clients` | `Clients` | 1932–2188 | `BedrockTabs("clients")` @2184 | none |
| 3 | `versions` | `Versions` | 2189–2361 | none (sub-panel of Clients) | none; inline Latite/OderSo switcher |
| 4 | `addons` | `@(IsJava ? "Mods & Addons" : "Packs")` | 2362–2987 | `BedrockTabs("addons")` @2978 / `JavaTabs("mods")` @2982 | `PacksAddonsTabs("addons")` @2386 |
| 5 | `levimods` | `LeviLamina Mods` | 2988–3101 | none found in range — see uncertainty note | none |
| 6 | `tweaks` | `@TweaksTitle` = `Bedrock Tweaks` | 3102–3266 | `BedrockTabs("addons")` @3257 / `JavaTabs("tweaks")` @3261 | `PacksAddonsTabs("tweaks")` @3158 |
| 7 | `utilities` | `Utilities` | 3267–3714 | `BedrockTabs("utilities")` @3710 | `UtilitiesTabs(_utilTab)` @3286 |
| 8 | `servers` | `Servers` | 3715–3828 | `BedrockTabs("worlds")` @3824 | `WorldsServersTabs("servers")` @3736 |
| 9 | `bedrockworlds` | `Worlds` | 3829–3928 | `BedrockTabs("worlds")` @3924 | `WorldsServersTabs("worlds")` @3852 |
| 10 | `bedrockpacks` | `Packs` | 3929–4021 | `BedrockTabs("addons")` @4017 | `PacksAddonsTabs("packs")` @3952 |
| 11 | `clientlibrary` | `Client Library` | 4022–4185 | `BedrockTabs("addons")` @4181 | `PacksAddonsTabs("clientlib")` @4045 |
| 12 | `bedrockbackups` | `Backups` | 4186–4299 | `BedrockTabs("utilities")` @4295 | `UtilitiesTabs("backups")` @4209 |
| 13 | `bedrockinstances` | `Instances` | 4300–4374 | `BedrockTabs("mcversions")` @4370 | `VersionsInstancesTabs("instances")` @4319 |
| 14 | `bedrockscreenshots` | `Screenshots` | 4375–4477 | `BedrockTabs("utilities")` @4428 | `UtilitiesTabs("photos")` @4398 |
| 15 | `leveldat` (+ `levelDatEditing != null`) | `NBT Editor — @levelDatWorldName` | 4478–4560 | none found in range | none |
| 16 | `downloads` | `Downloads` | 4561–4652 | none found in range | none |
| 17 | `mcversions` | `MC Versions` | 4653–4848 | `BedrockTabs("mcversions")` @4844 | `VersionsInstancesTabs("versions")` @4702 |
| 18 | `credits` | `Credits` | 5495–5646 (EOF) | `BedrockTabs("credits")` @5636 / `JavaTabs("credits")` @5640 | none |

**Uncertainty flag (panel 5, `levimods`):** the grep of tab-fragment call sites
shows `@BedrockTabs("addons")` at 2978 (inside the `addons` block) and the next
one at 3257 (inside `tweaks`). Nothing lands in 2988–3101. Read 3080–3101
directly before transcribing to confirm whether this panel truly has no bottom bar.

Component-hosted panels reachable from Bedrock Settings but not part of
`BedrockTabs` (all at `Home.razor:983–1012`, each a bare component with no
`panel-overlay` written in `Home.razor`):

| view key | line | markup |
|---|---|---|
| `themestudio`  | 983–986   | `<ThemeStudioPanel OnClose="GoHome" OnToast="HandleThemeToast" />` |
| `modpacks`     | 988–991   | `<ModpacksPanel OnClose="GoHome" OnToast="HandleThemeToast" OnInstalled="OnModpackInstalled" />` |
| `stats`        | 993–996   | `<StatsPanel OnClose="GoHome" />` |
| `logs`         | 998–1001  | `<LogsPanel OnClose="GoHome" OnToast="HandleThemeToast" />` |
| `skinlibrary`  | 1003–1006 | `<SkinLibraryPanel OnClose="GoHome" OnToast="HandleThemeToast" />` |
| `backups`      | 1008–1011 | `<BackupsPanel OnClose="GoHome" OnToast="HandleThemeToast" />` — the **Java** backups panel, distinct from `bedrockbackups` |

---

## 2. Per-panel detail

### 2.1 `settings` — Settings (1013–1931)

- Code-behind: `Pages/Home.Settings.cs` (503 lines, whole file) — category/filter
  state, `SetSettingsCategory`, `ClearSettingsFilter`, `VisibleSettingsCount`.
  Reset-confirm / Discord / Xbox modal handlers live in `Home.Settings.cs` and
  `Home.razor.cs`.
- Root classes: `.panel-overlay`, `.panel-header`, own `.versions-client-switcher`
  (inline, **not** `SubTabs`), `.panel-search-wrap` + `.panel-search-input`
  + `.panel-search-clear`, `.panel-body.settings-body`.
- Category buttons (`.vcs-btn`, bound to `settingsCategory`), Home.razor 1029–1059:
  `all` "All" `fa-solid fa-layer-group`; **Bedrock-only** `injection` "Inject"
  `fa-solid fa-syringe` (Java gets `java` "Java" `fa-brands fa-java` instead,
  branch at 1033–1046); `appearance` "Looks" `fa-solid fa-palette`;
  `account` "Account" `fa-solid fa-user`; `system` "System" `fa-solid fa-gear`.
- Search: placeholder `Filter settings`, bound `settingsFilter` on `oninput`,
  clear button `title="Clear filter"`.
- Sections (`<div class="settings-section">` + `<span class="panel-section-label">`):
  Injection @1103, Java Edition @1217 (Java only), Appearance @1428,
  Account @1625, Social @1665, Quality of Life @1708, Updates @1762,
  Folders @1793, CurseForge @1820, Backup @1858, About @1895.
- Components: `Toggle` (CompactMode @1474, AnimationsEnabled @1484 — and more),
  `SettingRow`, `btn-sm` buttons. `ColorPicker` is used inside `ThemeStudioPanel`.
- Modals opened from here: **Reset settings confirm** (`resetConfirmOpen`,
  Home.razor 277–305), **Discord Account** (`discordModalOpen`, 306–347),
  **Xbox Profile** (`xboxModalOpen`, 348–~430). Navigates to `themestudio`
  from the row @1606 and its `btn-sm` @1615.
- Empty state (1076–1098): `.settings-empty` with `<i class="fa-solid fa-folder-open">`,
  `.settings-empty-title` `No matching settings`, `.settings-empty-sub` reading
  `Nothing matches "{filter}".` or `This category is empty.`, plus a `.vcs-btn`
  `Clear filter`. No loading or error state.

### 2.2 `clients` — Clients (1932–2188)

- Code-behind: `Pages/Home.Clients.cs:16-164` — `OnClientChanged` @16,
  `SelectClient` @18, `CycleClient` @26, `ClearCustomDll` @36, `CopyDllPath` @43,
  `OpenClients` @51, `AutoUpdateClientsAsync` @77, `RunClientDownload` @110,
  `HandleFlarialDownload/Delete` @134/@137, `HandleOderSoDownload/Delete`
  @144/@147, `HandleLeviLaminaDownload/Delete` @155/@158.
  Glacier client entry: `Pages/Home.Glacier.cs` (89 lines, whole file) —
  `GlacierLatest` @18, `EnsureGlacierManifestAsync` @22, `InstallGlacierAsync` @31,
  `CancelGlacier` @53, `UninstallGlacier` @55, `LaunchGlacierAsync` @62.
- Header action: `.panel-icon-btn.versions-sort-btn` → `OpenVersions`,
  `data-tooltip="Browse Latite/OderSo versions"`, label `Versions`, plus
  `<span class="notification-badge">` when `versionsHasBadge`.
- Root classes: `.client-card` (+ `.client-active` when
  `SettingsService.Settings.SelectedClient` matches), `.client-card-header`,
  `.client-card-icon`, `.client-card-icon-img`, `.client-card-meta`,
  `.client-card-name`, `.client-card-sub`, `.client-card-note`,
  `.client-card-actions`, `.client-card-desc`, `.tag-uptodate`, `.tag-outdated`,
  `.error-text`, `.icon-btn`, `.icon-btn-ghost`.
- Components: `InlineProgress` — Flarial @1979, OderSo @2048, LeviLamina @2101,
  each with `OnCancel="() => _x.Cancel()"`.
- Status strings: `Up to date` (`fa-circle-check`), `Updating…` (`fa-rotate`),
  `Update available` (`fa-triangle-exclamation`), `Not downloaded`.
  Tooltips: `Select`, `Update` / `Download`, `Delete`.
  Flarial description: `Feature-rich Bedrock client with modules, HUD customization, and active development.`
- No loading spinner and no empty state — the card list is static markup.
- Navigates to `versions` and to `levimods` (from the LeviLamina card).

### 2.3 `versions` — Versions (Latite / OderSo) (2189–2361)

- Code-behind: `Home.Clients.cs:166-526` — `versionsBackTarget` @166,
  `OpenVersions` @168, `BackFromVersions` @180, `RefreshVersionsAsync` @193,
  `SwitchVersionsClient` @199, `latiteFetchError` @208, `LoadVersionsAsync` @210,
  `IsPinned` @218, `TogglePin` @220, `CopyVersionTag` @229,
  `VersionActions` @235-302, `HandleVersionDownload` @304, `HandleVersionDelete`
  @322, `HandleVersionLaunch` @324, `oderSoFetchError` @351,
  `LoadOderSoVersionsAsync` @353, `HandleOderSoEntryDownload` @367,
  `HandleOderSoEntryDelete` @387, `HandleOderSoEntryLaunch` @397,
  `OderSoVersionActions` @426-474, `SortOption` record + `_sortOptions` @476-484,
  `CurrentSortLabel` @485, `CurrentSortIcon` @489, `SortedVersions` @493,
  `SetVersionSort` @505, `CycleVersionSort` @512, `ToggleShowOnlyDownloaded` @521.
- Header actions: `.panel-icon-btn.versions-sort-btn` → `CycleVersionSort`;
  `.panel-icon-btn` + `active` when `SettingsService.Settings.ShowOnlyDownloaded`;
  `.panel-icon-btn` + `spinning` bound to `isLoadingVersions` /
  `isLoadingOderSoVersions` depending on `versionsClient`.
- `<Virtualize Items="unpinnedList" ItemSize="54" Context="v">` @2548;
  pinned versions render above in a non-virtualized list.
- Back navigates to `versionsBackTarget` (default `"home"`), not `GoHome`.

### 2.4 `addons` — Packs (Bedrock) / Mods & Addons (Java) (2362–2987)

- Title `@(IsJava ? "Mods & Addons" : "Packs")` @2372.
- Code-behind: `Home.Clients.cs:528-546` — `OpenAddons()` @528,
  `OpenAddons(string tab)` @530, `OpenAddonsAsync` @532. CurseForge/Modrinth
  search state and `cfDownloadProgress` / `mrDownloadProgress` are also in
  `Home.Clients.cs`.
- **Java-only block** `@if (IsJava)` spans roughly 2562–2902: `ClientCard`s for
  Fabric @2701, Quilt @2723, Forge @2745, NeoForge @2760 (each with `SubContent`,
  `Actions` containing `IconBtn`, and `DescriptionContent`), plus instance file
  management with `IconBtn` Create backup @2953, Export @2965, Duplicate @2977.
  None of this may appear on the Bedrock side.
- **Bedrock path**: `@if (!IsJava || javaModsTab == "curseforge")` @2902 →
  `.panel-body` with `.panel-search-input` placeholder
  `Search CurseForge addons...` (Java variant:
  `Search CurseForge for Java mods, modpacks, shaders...`), result rows, and
  `InlineProgress Progress="cfDownloadProgress"` @3088.
  The Modrinth block `@if (IsJava && javaModsTab == "modrinth")` @3033 with
  `InlineProgress Progress="mrDownloadProgress"` @3204 is Java-only.
- `.empty-state` sites inside the range: 2602, 2709, 2818, 2826, 2907, 2959,
  2967, 3075, 3083. Bedrock-relevant: 2907 (pre-search), 2959 and 2967
  (no results / error, one of which reads `Try a different search term or category.`).
  Read each before transcribing — several are Java-only.

### 2.5 `levimods` — LeviLamina Mods (2988–3101)

- Code-behind: `Pages/Home.LeviLaminaMods.cs` (86 lines, whole file):
  `levModsQuery` @12, `levModsSearching` @13, `levModsHasSearched` @14,
  `levModsResults` @16, `OpenLeviLaminaMods` @20,
  `CloseLeviLaminaMods` @26 (**returns to `OpenClients()`**, not `GoHome`),
  `OnLevModsSearchInput` @28, `DebouncedLevModsSearch` @36,
  `LevModsSearchAsync` @49, `HandleLevModInstall` @64, `HandleLevModDelete` @80.
- Header: `.panel-icon-btn` + `spinning` bound to `levModsSearching` @2997.
- Empty state @3037; `InlineProgress Progress="levModsInstallProgress"` @3332
  is inside the result rows.
- Reached from the Clients panel's LeviLamina card only.

### 2.6 `tweaks` — Bedrock Tweaks (3102–3266)

- Code-behind: `Pages/Home.Tweaks.cs` (126 lines, whole file): `_tweaksCategories`
  @14, `_tweaksSel` @15, `_javaTweaksVersion` @18, `_tweaksExpanded` @20,
  `IsCategoryExpanded` @22, `ToggleCategory` @24, `CategorySelectedCount` @30,
  `TweakKey` @37, `IsTweakSelected` @39, `TweaksSelectedCount` @40,
  `ToggleTweak` @42, `OpenTweaks` @49, `LoadTweaksAsync` @56,
  `ToggleJavaVerDropdown` @77, `SelectJavaTweaksVersion` @84,
  `InstallTweaksAsync` @92, `TweaksTitle` @121, `TweaksVersionLabel` @123.
- `TweaksTitle` = `IsJava ? "Vanilla Tweaks" : "Bedrock Tweaks"`.
- Header: Java-only version dropdown at `@if (IsJava)` @3113; refresh
  `.panel-icon-btn` + `spinning` on `_tweaksLoading` @3147.
- Loading/empty states at 3169 and 3180; a Java/Bedrock branch on descriptive
  copy at 3189 (`@(IsJava ? … : …)`).
- Sub-tab strip `PacksAddonsTabs("tweaks")` @3158.

### 2.7 `utilities` — Utilities (3267–3714)

Six sub-tabs driven by `_utilTab`. Code-behind `Pages/Home.Utilities.cs`
(428 lines): `_utilTab = "options"` @14, `OpenUtilities` (calls
`LoadUtilTabAsync` @19), `SwitchUtilTab` @22, `LoadUtilTabAsync` @35 with a
`switch (_utilTab)` @37, and the system-polling guard @372
(`if (currentView != "utilities" || _utilTab != "system") { StopSystemPolling(); return; }`).

Two of the six `UtilitiesTabs` entries are *separate views*, not in-panel content:
`backups` → `bedrockbackups`, `photos` → `bedrockscreenshots`. Only `options`,
`shaders`, `clients`, `system` render inside this panel.

| in-panel sub-tab | approx. lines | notable classes |
|---|---|---|
| options | 3371–3462 | `.util-quick-row`, `.util-quick-card`, `.util-quick-head`, `.util-quick-actions`, `.util-filter-row`, `.util-option-input` (+ `.edited` when the key is in `_optionEdits`) |
| shaders | 3464–3510 | `.util-quick-actions`; empty states @3464 and @3503 |
| clients (Library) | 3535–3585 | `.util-quick-actions`, `.util-filter-row`, `.util-option-input` bound to `_clientRenameValue` |
| system | 3667–3778 | `.util-gauge-row`, `.util-gauge`, `.util-gauge-head` (CPU value `@_sysSample.CpuPercent.ToString("F0")%`), `.util-disk-bar` |

Plus `.util-save-bar` at ~3777. `.empty-state` sites at 3399, 3446, 3464, 3503, 3563.
Header action: one `.panel-icon-btn` → `LoadUtilTabAsync`, `data-tooltip="Refresh"`.

**Uncertainty flag:** my sub-tab line boundaries are derived from grep offsets and
the `system` block appears to run past the panel's nominal 3714 end. Re-derive the
exact `@if (_utilTab == …)` boundaries by grepping `_utilTab ==` within 3267–3714
before transcribing.

### 2.8 `servers` — Servers (3715–3828)

- Code-behind: `Pages/Home.Panels.cs:17-207` — `serverModalOpen` @17,
  `editingServer` @18, `serverInputName` @19, `serverInputAddress` @20,
  `serverInputPort = 19132` @21, `serverInputError` @22, `_serverSuggestions` @24,
  `_serverStatus` @33, `OpenServers` @36, `ServerKey` @45, `RenderServerStatus` @47,
  `PingAllServersAsync` @71, `OpenAddServerModal` @98, `OpenEditServerModal` @109,
  `CloseServerModal` @120, `SaveServerModal` @127, `DeleteServer` @162,
  `SaveSuggestedServer` @169, `IsSuggestionUnsaved` @190, `GetServerIconClass` @194,
  `GetServerIconStyle` @197, `CopyServerAddress` @203.
- Header action: `.panel-icon-btn.versions-sort-btn` → `OpenAddServerModal`.
- Empty state @3739 when `SettingsService.Settings.SavedServers.Count == 0`.
- **Modal:** Add/Edit server, flag `serverModalOpen`, `Home.razor:4433–~4477`;
  title `@(editingServer == null ? "Add server" : "Edit server")`, icon
  `fa-solid fa-server` styled `color:var(--accent);margin-right:8px;`.
- Sub-tab strip `WorldsServersTabs("servers")` @3736.

### 2.9 `bedrockworlds` — Worlds (3829–3928)

- Code-behind: `Pages/Home.Worlds.cs` (69 lines, whole file): `bedrockWorlds` @10,
  `isLoadingBedrockWorlds` @11, `_confirmDeleteWorldPath` @12,
  `OpenBedrockWorlds` @14, `RefreshBedrockWorldsAsync` @21,
  `OpenBedrockWorldFolder` @30, `ConfirmDeleteWorld` @36, `CancelDeleteWorld` @37,
  `DeleteBedrockWorld` @39, `ExportBedrockWorld` @51, `OpenBedrockWorldsFolder` @63.
- Header actions: `.panel-icon-btn` → `OpenBedrockWorldsFolder`
  `data-tooltip="Open folder"` @3953-ish (rel 125); `.panel-icon-btn` +
  `spinning` + `disabled` bound to `isLoadingBedrockWorlds` (rel 128).
- States: loading @3969, empty `.empty-state` @3975. No error state.
- Row action: NBT-editor button @4010 → `OpenLevelDatEditor(world)`.
- **Inline confirm, not a modal:** `@if (_confirmDeleteWorldPath == world.FolderPath)`
  @4013 swaps the row's actions for confirm/cancel.
- Sub-tab strip `WorldsServersTabs("worlds")` @3852.

### 2.10 `bedrockpacks` — Packs (Manage) (3929–4021)

- Code-behind: `Pages/Home.Packs.cs` (69 lines, whole file): `bedrockPacks` @10,
  `isLoadingBedrockPacks` @11, `bedrockPacksKind = "resource"` @12,
  `_confirmDeletePackPath` @13, `OpenBedrockPacks()` @15,
  `OpenBedrockPacks(string kind)` @17, `SwitchBedrockPacksKind` @25,
  `RefreshBedrockPacksAsync` @33, `OpenBedrockPacksFolder` @43,
  `OpenBedrockPackFolder` @49, `ConfirmDeletePack` @55, `CancelDeletePack` @56,
  `DeleteBedrockPack` @58.
- Header actions: open-folder `.panel-icon-btn` @4053-rel-225
  (`data-tooltip="Open folder"`); refresh with `spinning` on `isLoadingBedrockPacks`.
- A resource/behavior **kind switcher** sits just above the list (~4028).
- States: loading @4030, empty `.empty-state` @4036, inline delete confirm @4066
  (`_confirmDeletePackPath == pack.FolderPath`).
- Sub-tab strip `PacksAddonsTabs("packs")` @3952.

### 2.11 `clientlibrary` — Client Library (4022–4185)

- Code-behind: `Pages/Home.ClientLibrary.cs` (108 lines, whole file): `_clPacks` @12,
  `_clFilter` @15, `_clVersion = "all"` @16, `_clExpanded` @17,
  `OpenClientLibrary` @21, `LoadClientLibraryAsync(bool refresh = false)` @27,
  `ClientLibVersions` @46, `FilteredClientLibrary` @48, `IsClientPackExpanded` @61,
  `ToggleClientPack` @63, `SetClientLibVersion` @69,
  `IsClientDownloadInstalled` @75, `InstallClientPackAsync` @78,
  `UninstallClientPack` @95. Markup also references `_clLoading` and `_clError`.
- Header actions: `.panel-icon-btn` → `OpenUrl(ClientLibraryService.SiteUrl)`
  `data-tooltip="Indigo-Tools/ClientLibrary on GitHub"`; refresh `.panel-icon-btn`
  + `spinning` on `_clLoading`.
- **Three distinct states** — the only Bedrock panel with a real error state:
  loading @4373 (`_clLoading && _clPacks.Count == 0`, renders a spinner);
  error @4377 (`!string.IsNullOrEmpty(_clError) && _clPacks.Count == 0`,
  `.empty-state`); no-match @4397 (`clFiltered.Count == 0`, `.empty-state`).
- Sub-tab strip `PacksAddonsTabs("clientlib")` @4045.

### 2.12 `bedrockbackups` — Backups (4186–4299)

- Code-behind: `Pages/Home.Backups.cs` (90 lines, whole file): `bedrockBackups` @10,
  `isLoadingBackups` @11, `isCreatingBackup` @12, `_confirmDeleteBackupPath` @13,
  `_confirmRestoreBackupPath` @14, `backupLabelInput` @15, `OpenBedrockBackups` @17,
  `RefreshBedrockBackupsAsync` @24, `CreateBedrockBackup` @33,
  `ConfirmDeleteBackup` @50, `CancelDeleteBackup` @51, `DeleteBedrockBackup` @53,
  `ConfirmRestoreBackup` @65, `CancelRestoreBackup` @66, `RestoreBedrockBackup` @68,
  `OnBackupLabelInput` @82, `OpenBedrockBackupsFolder` @85.
- Header actions: open-folder `.panel-icon-btn` `data-tooltip="Open folder"`;
  refresh `.panel-icon-btn` + `spinning` + `disabled` on `isLoadingBackups`.
- A create-backup row with the `backupLabelInput` field sits above the list
  (~4197–4215, between the sub-tab strip @4209 and the list).
- States: loading ≈4229, empty `.empty-state` ≈4235.
- **Two inline confirms:** restore ≈4252
  (`_confirmRestoreBackupPath == backup.FilePath`) and delete ≈4270
  (`_confirmDeleteBackupPath == backup.FilePath`).
- Sub-tab strip `UtilitiesTabs("backups")` @4209.

### 2.13 `bedrockinstances` — Instances (4300–4374)

- Code-behind: `Pages/Home.BedrockInstances.cs` (92 lines, whole file):
  `isSwitchingBedrockInstance` @9, `_renamingBedrockInstanceId` @10,
  `_renameBedrockInstanceValue` @11, `_confirmDeleteBedrockInstanceId` @12,
  `OpenBedrockInstances` @14, `SwitchBedrockInstance` @21, `NewBedrockInstance` @40,
  `OnBedrockInstanceVersionChanged` @47, `BeginRenameBedrockInstance` @53,
  `OnRenameBedrockInstanceInput` @59, `CommitRenameBedrockInstance` @62,
  `ConfirmDeleteBedrockInstance` @70, `CancelDeleteBedrockInstance` @71,
  `DeleteBedrockInstance` @73, `BackupBedrockInstance` @81.
- Header action: `.panel-icon-btn` → `NewBedrockInstance`,
  `data-tooltip="New instance"`.
- **Uncertainty:** a `.empty-state` div appears at ≈4307, seemingly at the top of
  `.panel-body` and possibly *unguarded* (an explanatory banner rather than a real
  empty state). Verify the surrounding `@if` before transcribing.
- Inline delete confirm at ≈4340
  (`_confirmDeleteBedrockInstanceId == inst.Id`). Per-instance version `<select>`
  bound to `OnBedrockInstanceVersionChanged`; inline rename input bound to
  `_renameBedrockInstanceValue`.
- No loading spinner — the instance list is local state.
- Sub-tab strip `VersionsInstancesTabs("instances")` @4319.

### 2.14 `bedrockscreenshots` — Screenshots (4375–4477)

- Code-behind: `Pages/Home.BedrockScreenshots.cs` (37 lines, whole file):
  `bedrockScreenshots` (typed `List<JavaInstanceFile>`) @10,
  `isLoadingBedrockScreenshots` @11, `OpenBedrockScreenshots` @13,
  `RefreshBedrockScreenshotsAsync` @23, `OpenBedrockScreenshotsFolder` @32.
- Header actions: open-folder `.panel-icon-btn` `data-tooltip="Open folder"`;
  refresh with `spinning` + `disabled` on `isLoadingBedrockScreenshots`.
- States: loading ≈4401, empty `.empty-state` ≈4407. No error state.
- Sub-tab strip `UtilitiesTabs("photos")` @4398.

### 2.15 `leveldat` — NBT Editor (4478–4560)

- Guarded by `currentView == "leveldat" && levelDatEditing != null`.
- Title: `NBT Editor — @levelDatWorldName`.
- Code-behind: `Pages/Home.LevelDat.cs` (76 lines, whole file): `levelDatSaving` @9,
  `levelDatEditing` (`LevelDatSummary?`) @10, `levelDatWorldName` @11,
  `OpenLevelDatEditor(BedrockWorld w)` @13, `CloseLevelDatEditor` @24,
  `SaveLevelDat` @30, `OnLevelDatGameTypeChanged` @45,
  `OnLevelDatDifficultyChanged` @50, `OnLevelDatGeneratorChanged` @55,
  `ToggleLevelDatCheats` @60, `ToggleLevelDatExperiment(string name)` @62,
  `GameTypeOptions` @68, `DifficultyOptions` @71, `GeneratorOptions` @74
  (each a `(int Value, string Label)[]`).
- Header action: `.panel-icon-btn.versions-sort-btn` with
  `disabled="@levelDatSaving"` → save.
- Entered only from a Worlds row; no tab strips, no loading/empty/error states.

### 2.16 `downloads` — Downloads (4561–4652)

- Code-behind: `Pages/Home.Downloads.cs` (44 lines, whole file).
- Header action: `.panel-icon-btn` → `ClearFinishedDownloads`,
  `data-tooltip="Clear finished"`.
- Empty `.empty-state` @4586. Edition-agnostic; not in `BedrockTabs`.

### 2.17 `mcversions` — MC Versions (4653–4848)

- Code-behind: `Pages/Home.Panels.cs:209-402` — `OpenMcVersions` @209,
  `RefreshMcVersionsAsync` @219, `LoadMcVersionsAsync` @221,
  `HandleMcVersionDownload` @229, `HandleMcVersionSwitch` @247,
  `HandleMcVersionDelete` @269, `McVersionActions` @277-341,
  `InstallFromStoreAsync(bool preview)` @343, `CancelStoreInstall` @399.
- Header actions: info toggle `.panel-icon-btn` + `active` on `mcvInfoOpen`;
  refresh `.panel-icon-btn` + `spinning` on `isLoadingMcVersions`.
- **Microsoft Store install row** @4704: `.mcv-store-row` > `.mcv-store-meta` >
  `.mcv-store-title` / `.mcv-store-sub`. Idle copy: title
  `<i class="fa-brands fa-microsoft" style="margin-right:6px;"></i>Install latest from Microsoft Store`,
  sub `No Developer Mode required.`; busy copy is `@storeInstallStage` /
  `@storeInstallDetail`.
- `<Virtualize Items="restList" ItemSize="54" Context="v">` @5080 — the sideload
  version list is virtualized.
- Sub-tab strip `VersionsInstancesTabs("versions")` @4702.

### 2.18 `credits` — Credits (5495–5646, EOF)

- Code-behind: `Home.Panels.cs:15` —
  `OpenCredits() => _ = NavigateAsync(() => currentView = "credits")`.
- Classes: `.credits-card`, `.credits-card-glacier`, `.credits-card-icon`,
  `.credits-card-meta`, `.credits-card-name`, `.credits-card-sub`,
  `.credits-card-actions`, `.credits-link-btn`; section labels use
  `.panel-section-label` — `Launcher` @5512, `Clients` @5536.
- Strings: `Glacier Launcher` / `Built by Pepe · Glacier Productions`;
  `Latite Client` / `by Imrglop & contributors`; link tooltips `GitHub`,
  `Website`, `Discord`.
- **Note:** the Glacier card icon at Home.razor:5515 is a hard-coded Discord CDN
  URL carrying an expiring `ex=/is=/hm=` signature. It will 404 eventually —
  flag before transcribing verbatim.
- No empty/loading/error states.

---

## 3. Modals

`Components/Modal.razor` exists (39 lines) but `Home.razor` **never instantiates it**.
Every modal is hand-rolled as `div.modal-overlay`:

```
<div class="modal-overlay" @onclick="Close…">
  <div class="modal-box [extra]" @onclick:stopPropagation>
    <div class="modal-header">
      <span class="modal-title"> <i class="…"></i> Title </span>
      <button class="modal-close-btn" @onclick="Close…"><i class="fa-solid fa-xmark"></i></button>
    </div>
    <div class="modal-body">
      <p class="modal-desc">…</p>
      [<div class="modal-divider">or enter manually</div>]
      [<label class="modal-input-label">…</label><input class="modal-input" …>]
      <div class="modal-btn-row">
        <button class="modal-btn modal-btn-cancel">…</button>
        <button class="modal-btn modal-btn-confirm|modal-btn-disconnect">…</button>
      </div>
    </div>
  </div>
</div>
```

| modal | flag | `Home.razor` lines | reachable from |
|---|---|---|---|
| Launcher update | `launcherUpdateModalOpen && launcherUpdateInfo != null` | 218–~277, `.modal-box.update-modal` | global / Settings → Updates |
| Reset settings confirm | `resetConfirmOpen` | 277–305 | Settings |
| Discord Account | `discordModalOpen` | 306–347 | Settings → Social |
| Xbox Profile | `xboxModalOpen` | 348–~430 (end bounded by the next `@if`; verify) | Settings → Social |
| Add / Edit server | `serverModalOpen` | 4433–~4477 | Servers panel |

Launcher-update strings: buttons `Skip`, `Later`, apply; progress copy
`Downloading update... launcher will restart automatically.` in a `.modal-desc`.
Discord strings: title `Discord Account` (`fa-brands fa-discord`, `color:#7289da`);
signed-out desc `Connect your Discord account to sync your username and avatar
with Rich Presence.`; divider `or enter manually`; label `Discord Username`;
placeholder `username`; buttons `Cancel` / `Disconnect` / `Close`; signed-in desc
`Signed in as <strong>@DiscordUsername</strong>. Username and avatar will sync to
Discord Rich Presence.` Xbox modal title `Xbox Profile` (`fa-solid fa-gamepad`,
`color:#107c10`), shows `GamerPictureUrl`, `AccountTier`, `Bio`, and a
`JavaAccounts` list when non-empty.

Everything else that reads like a confirmation is an **inline row confirm**, not a
dialog: worlds delete, packs delete, backups delete + restore, Bedrock instance
delete (and, Java-side, `_confirmDeleteInstanceId`, `Home.Clients.cs:641`).
There is no generic `ConfirmAsync` helper anywhere in `Home.razor.cs`.

---

## 4. User-facing strings and localization

Localization coverage in the original is **very thin** — exactly 10 `L.T(...)`
call sites exist in `Pages/Home.razor`, and the complete set of keys used anywhere
across `Pages/` + `Components/` is:

```
nav.clients   nav.launch   nav.launchers   nav.mcversions   nav.mods
nav.photos    nav.profile  nav.settings    nav.versions     settings.language
```

Every other user-facing string in the Bedrock panels is a **hard-coded literal**
in markup — panel titles, `data-tooltip` values, empty-state copy, button labels
— or a literal in a `ShowToast(...)` call in the code-behind. A 1:1 transcription
must copy those literals exactly; do not invent localization keys the original
does not have, and do not "improve" wording.

Toast strings from `Home.BigFeatures.cs` alone (each panel's own code-behind has
more — grep `ShowToast(` per file when transcribing):
`This instance: {v} MB`, `Using global memory`, `UUID copied`,
`Sign in with Microsoft first.`, `That skin file no longer exists.`,
`Applying skin…`, `Skin updated! Saved to your library.`,
`Skin change failed: {err}`, `Skin reset to default.`, `Reset failed: {err}`,
`Cape hidden.`, `Now wearing: {alias}`, `Cape change failed: {err}`.

---

## 5. Cross-check against the rewrite (`C:\Users\User\MinecraftLauncher\frontend\src\lib`)

Rewrite views present: BedrockContentView, BedrockVanillaVersionsView,
ClientLibraryView, ClientsView, CreditsView, DownloadsView, InstancesView,
JavaModsView, JavaScreenshotsView, LauncherClientsView, LevelDatEditorView,
LeviLaminaModsView, LogsPanel, MCVersionsView, ModpacksView, NewsView,
OnboardingModal, ProfileView, ServersView, SettingsView, StatsPanel, TweaksView,
UtilitiesView, VersionsView. Components: Card, IconBtn, InfoCard, Modal,
PanelHeader, PanelTabs, SettingRow, Spinner, Toggle.

| original Bedrock panel | rewrite counterpart | verdict |
|---|---|---|
| `settings` | `SettingsView.svelte` (852 lines) | exists; section-by-section audit still needed |
| `clients` | `ClientsView.svelte` | exists |
| `versions` (Latite / OderSo) | `VersionsView.svelte` | exists |
| `addons` (Packs / CurseForge browse) | split across `BedrockContentView` and `JavaModsView` | **restructured** |
| `levimods` | `LeviLaminaModsView.svelte` | exists |
| `tweaks` | `TweaksView.svelte` | exists |
| `utilities` | `UtilitiesView.svelte` (608 lines) | exists, but its backups/photos sub-tabs were moved out |
| `bedrockworlds` | `BedrockContentView` tab `worlds` | **merged** |
| `bedrockpacks` | `BedrockContentView` tab `packs` | **merged** |
| `bedrockbackups` | `BedrockContentView` tab `backups` | **merged** |
| `bedrockscreenshots` | `BedrockContentView` tab `screenshots` | **merged** |
| `leveldat` | `LevelDatEditorView.svelte` | exists, but nested inside `BedrockContentView` rather than a top-level view |
| `servers` | `ServersView.svelte` | exists |
| `downloads` | `DownloadsView.svelte` | exists |
| `mcversions` | `MCVersionsView` + `BedrockVanillaVersionsView` | split into two; needs audit |
| `bedrockinstances` | `InstancesView.svelte` | exists, but promoted to a top-level bottom tab |
| `credits` | `CreditsView.svelte` | exists |
| `themestudio` (`ThemeStudioPanel.razor`, 539 lines) | **none** | **MISSING ENTIRELY** |
| `skinlibrary` (`SkinLibraryPanel.razor`, 172 lines) | **none** | **MISSING ENTIRELY** |
| `backups` — Java (`BackupsPanel.razor`, 140 lines) | **none** | **MISSING ENTIRELY** |
| `Components/ColorPicker.razor` (198 lines) | **none** | **MISSING** (component) |
| `Components/SkinViewer.razor` (185 lines) | **none** | **MISSING** (component) |
| `Components/InlineProgress.razor` (27 lines) | **none** | **MISSING** (component; used by Clients, Addons, LeviLamina, Glacier) |
| `Components/ClientCard.razor` (96 lines) | `Card.svelte` / `InfoCard.svelte` | different API — not a transcription |

### Confirmed structural divergences

1. **Four panels collapsed into one.** `BedrockContentView.svelte:11-13` states in
   its own comment that Worlds, Packs, Backups and Screenshots are "one tabbed
   component instead of four near-duplicate ones". The original has four
   independent `currentView` values, each with its own header actions, its own
   loading/empty states, and — critically — *different* sub-tab strips
   (`WorldsServersTabs` for worlds, `PacksAddonsTabs` for packs, `UtilitiesTabs`
   for backups and photos). The merged component's tab set
   `worlds / packs / backups / screenshots` exists nowhere in the original.

2. **All four cross-nav sub-tab strips are gone.** No counterpart exists for
   `WorldsServersTabs`, `VersionsInstancesTabs`, `PacksAddonsTabs`, or
   `UtilitiesTabs`. The rewrite has a generic `components/PanelTabs.svelte` fed
   ad-hoc per-view arrays, so the original's navigation graph (Worlds ↔ Servers;
   Browse ↔ Clients ↔ Tweaks ↔ Manage; Options/Shaders/Library/System ↔ Backups ↔
   Photos; MC Versions ↔ Instances) is not reproduced.

3. **Bottom tab bar padded with 8 extra entries.** `App.svelte:64-79` —
   `BEDROCK_BOTTOM_TABS` keeps the original 7 in order, then appends Instances,
   LeviLamina, Servers, Logs, Stats, Client Library, Downloads and News. The
   original has exactly 7.

4. **Two bottom tabs point at the same view.** In `BEDROCK_BOTTOM_TABS`, both
   `Packs` and `Worlds` have `id: 'content'`, so both light up as active
   simultaneously — impossible in the original, where they are distinct views.

5. **Active-tab click behavior lost.** The original omits `onclick` entirely on
   the active Bedrock tab (`Home.BigFeatures.cs:60-61`); `App.svelte` wires
   `onclick` unconditionally on every tab.

6. **Icon drift in the merged view.** `BedrockContentView` uses
   `fa-solid fa-earth-americas` (worlds) and `fa-solid fa-box-open` (packs);
   the original uses `fa-solid fa-globe` and `fa-solid fa-boxes-stacked`.
   Backups matches (`fa-clock-rotate-left`); screenshots differs
   (`fa-solid fa-image` vs the original's `fa-solid fa-images`), and the label
   is `Screenshots` where `UtilitiesTabs` says `Photos`.

7. **Modal vocabulary replaced.** The original hand-rolls every modal with a
   specific class set (`modal-overlay`, `modal-box`, `modal-header`, `modal-title`,
   `modal-close-btn`, `modal-body`, `modal-desc`, `modal-divider`,
   `modal-input-label`, `modal-input`, `modal-btn-row`, `modal-btn`,
   `modal-btn-cancel`, `modal-btn-confirm`, `modal-btn-disconnect`) and never
   uses `Components/Modal.razor`. The rewrite routes everything through
   `components/Modal.svelte`. Separately, the original's delete/restore
   confirmations are *inline in-row*, never dialogs.

8. **Theme Studio, Skin Library and the Java Backups panel do not exist at all** —
   851 lines of original component markup with zero counterpart. Settings'
   Theme Studio entry points (`Home.razor` 1606 and 1615) therefore have nowhere
   to navigate in the rewrite.

---

## 6. Open items to verify before transcription

1. Exact `@if (_utilTab == …)` boundaries inside `utilities` (3267–3714) — my
   sub-tab ranges are offset-derived and the `system` block appears to overrun.
2. Whether `levimods` (2988–3101) renders a bottom tab strip at all.
3. Whether the `.empty-state` at ≈4307 in `bedrockinstances` is conditional or an
   unguarded explanatory banner.
4. Exact end line of the Xbox modal (bounded at ~430 by the next `@if`).
5. The `addons` Bedrock-branch empty/loading/error copy — the block has nine
   `.empty-state` sites and most are Java-only.
6. Panel end lines throughout are "line before the next `@if`" upper bounds; the
   real closing brace sits at or slightly before each listed end.
