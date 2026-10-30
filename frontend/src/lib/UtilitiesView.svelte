<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * 1:1 port of the "utilities" panel — Home.razor lines 3267-3714 plus
   * Home.Utilities.cs (428 lines).
   *
   * Structure mirrors the original exactly: panel header (refresh + back),
   * the six-entry `UtilitiesTabs` strip (options / shaders / clients /
   * system switch an internal tab; backups / photos navigate away to the
   * separate panels), the `.panel-body` with the active sub-tab's content,
   * the options save bar, and `BedrockTabs("utilities")` at the bottom.
   *
   * Gaps against the original, all left visibly disabled rather than faked
   * (no Tauri command exists for them):
   *   - "Enable Developer Mode"      → needs `enable_developer_mode`
   *   - per-client "Check for updates now" → needs `update_custom_client`
   *   - "Open folder" with no instance selected → needs a command that
   *     returns/opens the live per-account `minecraftpe` folder.
   * Data the backend does not expose (documented in
   * src-tauri/src/commands/utilities.rs): live CPU%/uptime sampling, GPU /
   * motherboard / BIOS / local-IP (all WMI in C#), shader apply/restore
   * progress, and `AppliedShader.HadStock`.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { untrack } from 'svelte'
  import { open } from '@tauri-apps/plugin-dialog'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'
  import Toggle from './components/Toggle.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import { UTILITIES } from './components/SubTabs.svelte'
  import { loadFilterPref, saveFilterPref } from './filterMemory'
  import type { LauncherSettings } from './settings'

  let {
    instanceDir,
    initialTab = 'options',
    onBack,
    onNavigate,
    onToast,
  }: {
    instanceDir: string
    initialTab?: string
    onBack?: () => void
    onNavigate?: (view: string, strip?: string) => void
    onToast?: (message: string, kind?: 'info' | 'success' | 'error' | 'warning') => void
  } = $props()

  type Tab = 'options' | 'shaders' | 'clients' | 'system'
  const INTERNAL: Tab[] = ['options', 'shaders', 'clients', 'system']

  // `UTILITIES` (imported from SubTabs.svelte) is this same 6-item list —
  // the vertical `.settings-nav-rail` here and the Bedrock Backups/Screenshots
  // panels' rails all render it, so it only needs to exist in one place.

  // QOL: remembers the last internal sub-tab across restarts. `initialTab`
  // is `'options'` both as its literal default *and* whenever App.svelte's
  // own default fires (i.e. "just open Utilities", not a deep-link from the
  // Backups/Photos rail) - in that ambiguous case, prefer whatever tab was
  // last open instead of always resetting to Options. An explicit non-
  // default `initialTab` (a real deep-link) always wins.
  const requestedTab = untrack(() => initialTab)
  const effectiveInitialTab =
    requestedTab !== 'options' && (INTERNAL as string[]).includes(requestedTab)
      ? requestedTab
      : loadFilterPref('utilities.tab', 'options')
  let utilTab = $state<Tab>(
    (INTERNAL as string[]).includes(effectiveInitialTab) ? (effectiveInitialTab as Tab) : 'options',
  )

  /// Labels for the breadcrumb's trailing segment — the sub-tab is internal
  /// state here (not a top-level `View` in App.svelte), so it can't come
  /// from the shared navHistory and is spelled out locally instead.
  const UTIL_TAB_LABELS: Record<Tab, string> = {
    options: 'Options',
    shaders: 'Shaders',
    clients: 'Clients',
    system: 'System Info',
  }

  function toast(message: string, kind: 'info' | 'success' | 'error' | 'warning' = 'info') {
    if (onToast) onToast(message, kind)
    else console[kind === 'error' ? 'error' : 'log'](`[utilities] ${message}`)
  }

  // ---- shared formatting (Home.razor.cs:677-700, LauncherUtilityService:114) ----
  function formatSize(bytes: number): string {
    if (!bytes || bytes <= 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB']
    let v = bytes
    let i = 0
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024
      i++
    }
    return `${Math.round(v * 100) / 100} ${units[i]}`
  }

  function formatBytes(bytes: number): string {
    if (!bytes || bytes <= 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    let v = bytes
    let i = 0
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024
      i++
    }
    return `${Math.round(v * 100) / 100} ${units[i]}`
  }

  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
  function formatRelativeTime(iso: string): string {
    if (!iso) return ''
    const dt = new Date(iso)
    if (isNaN(dt.getTime())) return ''
    const diffMs = Date.now() - dt.getTime()
    const minutes = diffMs / 60000
    if (minutes < 2) return 'just now'
    const hours = minutes / 60
    if (hours < 1) return `${Math.floor(minutes)}m ago`
    const days = hours / 24
    if (days < 1) return `${Math.floor(hours)}h ago`
    if (days < 7) return `${Math.floor(days)}d ago`
    return `${MONTHS[dt.getMonth()]} ${dt.getDate()}`
  }

  // =====================================================================
  // options  (Home.Utilities.cs:47-177)
  // =====================================================================
  type OptionKind = 'toggle' | 'integer' | 'decimal' | 'text'
  type OptionEntry = {
    key: string
    value: string
    kind: OptionKind
    label: string
    group: string
    hint: string
    known: boolean
  }

  const VSYNC_KEY = 'gfx_vsync'

  let options = $state<OptionEntry[]>([])
  let optionEdits = $state<Map<string, string>>(new Map())
  let optionsFilter = $state('')
  let optionsShowAll = $state(false)
  let optionsError = $state('')
  let optionsSaving = $state(false)
  let hasBackup = $state(false)

  // BedrockOptionsService.GroupRank order.
  const GROUP_ORDER = ['Video', 'Interface', 'Controls', 'Audio', 'Gameplay', 'Advanced', 'Other']
  function groupRank(g: string) {
    const i = GROUP_ORDER.indexOf(g)
    return i < 0 ? GROUP_ORDER.length : i
  }

  async function loadGameOptions() {
    optionsError = ''
    optionEdits = new Map()
    try {
      // An empty instanceDir means "the live install"; the backend resolves
      // the real per-account com.mojang path itself.
      options = await invoke<OptionEntry[]>('get_bedrock_options', { instanceDir })
      hasBackup = await invoke<boolean>('bedrock_options_has_backup', { instanceDir })
      if (options.length === 0 && !(await invoke<boolean>('bedrock_options_exist', { instanceDir })))
        optionsError = "Minecraft hasn't written its settings yet. Launch the game once, then come back."
    } catch (e) {
      options = []
      optionsError = String(e)
    }
  }

  function optionValue(o: OptionEntry) {
    return optionEdits.get(o.key) ?? o.value
  }
  function optionIsOn(o: OptionEntry) {
    const v = optionValue(o)
    return v === '1' || v === 'true' || v === 'True'
  }
  function setOptionValue(o: OptionEntry, value: string) {
    const next = new Map(optionEdits)
    if (value === o.value) next.delete(o.key)
    else next.set(o.key, value)
    optionEdits = next
  }
  function toggleOption(o: OptionEntry) {
    setOptionValue(o, optionIsOn(o) ? '0' : '1')
  }

  const visibleOptionGroups = $derived.by(() => {
    let list = optionsShowAll ? options : options.filter((o) => o.known)
    const f = optionsFilter.trim().toLowerCase()
    if (f) list = list.filter((o) => o.label.toLowerCase().includes(f) || o.key.toLowerCase().includes(f))
    const groups = new Map<string, OptionEntry[]>()
    for (const o of list) {
      if (!groups.has(o.group)) groups.set(o.group, [])
      groups.get(o.group)!.push(o)
    }
    return Array.from(groups.entries()).sort(
      (a, b) => groupRank(a[0]) - groupRank(b[0]) || a[0].localeCompare(b[0], undefined, { sensitivity: 'accent' }),
    )
  })

  const optionsPendingCount = $derived(optionEdits.size)

  function discardOptionEdits() {
    optionEdits = new Map()
  }

  async function saveGameOptions() {
    if (optionEdits.size === 0 || optionsSaving) return
    optionsSaving = true
    try {
      const count = optionEdits.size
      const changes: Record<string, string> = {}
      for (const [k, v] of optionEdits) changes[k] = v
      await invoke('save_bedrock_options', { instanceDir, changes })
      await loadGameOptions()
      toast(`Saved ${count} setting${count === 1 ? '' : 's'} to options.txt.`, 'success')
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      optionsSaving = false
    }
  }

  const vsyncEntry = $derived(options.find((o) => o.key.toLowerCase() === VSYNC_KEY))
  const vsyncKeyPresent = $derived(!!vsyncEntry)
  const vsyncCurrentlyOn = $derived(!!vsyncEntry && optionIsOn(vsyncEntry))

  async function quickToggleVSync() {
    const next = !vsyncCurrentlyOn
    try {
      await invoke('save_bedrock_options', { instanceDir, changes: { [VSYNC_KEY]: next ? '1' : '0' } })
      await loadGameOptions()
      toast(next ? 'V-Sync enabled.' : 'V-Sync disabled.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function restoreOptionsBackup() {
    try {
      const restored = await invoke<boolean>('restore_bedrock_options_backup', { instanceDir })
      toast(restored ? 'Restored the options.txt Glacier backed up.' : 'No backup to restore.', 'info')
      await loadGameOptions()
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  // No command exposes the live per-account `minecraftpe` folder, so this is
  // only wired when an explicit instance directory is available.
  const canOpenOptionsFolder = $derived(!!instanceDir)
  function openOptionsFolder() {
    if (!instanceDir) return
    invoke('open_folder', { path: `${instanceDir}\\minecraftpe` }).catch((e) => toast(String(e), 'error'))
  }

  // =====================================================================
  // shaders  (Home.Utilities.cs:179-235, 411-427)
  // =====================================================================
  type AppliedShader = { file_name: string; source_name: string; applied_iso: string; size_bytes: number }

  let shaderBlockReason = $state<string | null>(null)
  let appliedShaders = $state<AppliedShader[]>([])
  let shaderBusy = $state(false)
  let stockMaterialCount = $state(0)

  async function loadShaderState() {
    try {
      shaderBlockReason = await invoke<string | null>('shaders_unavailable_reason')
      appliedShaders = await invoke<AppliedShader[]>('list_applied_shaders')
    } catch (e) {
      toast(String(e), 'error')
      return
    }
    if (shaderBlockReason) {
      stockMaterialCount = 0
      return
    }
    try {
      stockMaterialCount = (await invoke<string[]>('list_stock_shader_materials')).length
    } catch {
      stockMaterialCount = 0
    }
  }

  async function applyShaders() {
    if (shaderBusy) return
    const picked = await open({
      multiple: true,
      title: 'Select shader files',
      filters: [
        { name: 'Shader files', extensions: ['bin', 'zip', 'mcpack'] },
        { name: 'All files', extensions: ['*'] },
      ],
    })
    if (!picked) return
    const list = Array.isArray(picked) ? picked : [picked]
    if (list.length === 0) return

    shaderBusy = true
    try {
      const applied = await invoke<string[]>('apply_shaders', { sourcePaths: list })
      await loadShaderState()
      toast(`Applied ${applied.length} material${applied.length === 1 ? '' : 's'}.`, 'success')
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      shaderBusy = false
    }
  }

  async function restoreShaders() {
    if (shaderBusy) return
    shaderBusy = true
    try {
      const n = await invoke<number>('restore_shaders')
      await loadShaderState()
      toast(
        n > 0 ? `Restored ${n} stock material${n === 1 ? '' : 's'}.` : 'Nothing to restore.',
        n > 0 ? 'success' : 'info',
      )
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      shaderBusy = false
    }
  }

  function openMaterialsFolder() {
    invoke('open_shaders_materials_folder').catch((e) => toast(String(e), 'error'))
  }

  // =====================================================================
  // clients — the user's own DLL/EXE library  (Home.Utilities.cs:237-354)
  // =====================================================================
  type CustomClientEntry = {
    id: string
    name: string
    file_name: string
    kind: string
    source_url: string
    sha256: string
    size_bytes: number
    added_iso: string
    updated_iso: string
    auto_update: boolean
  }

  let clientLib = $state<CustomClientEntry[]>([])
  let clientUrlInput = $state('')
  let clientBusy = $state(false)
  let clientRenamingId = $state<string | null>(null)
  let clientRenameValue = $state('')
  let clientConfirmDeleteId = $state<string | null>(null)
  let defaultClientId = $state('')

  /** "More" menu per card — check-updates/auto-update are secondary to
   *  default/rename/delete, same overflow-avoidance pattern used on the
   *  Instances cards. */
  let openClientMenuId = $state<string | null>(null)
  function toggleClientMenu(id: string) {
    openClientMenuId = openClientMenuId === id ? null : id
  }
  function closeClientMenu() {
    openClientMenuId = null
  }

  let clientError = $state('')
  async function loadClientLibrary() {
    try {
      clientLib = await invoke<CustomClientEntry[]>('list_custom_clients')
      defaultClientId = (await invoke<LauncherSettings>('get_settings')).DefaultCustomClientId ?? ''
      clientError = ''
    } catch (e) {
      clientError = String(e)
      toast(String(e), 'error')
    }
  }

  function isDefaultClient(c: CustomClientEntry) {
    return !!defaultClientId && defaultClientId === c.id
  }

  async function setDefaultClient(c: CustomClientEntry) {
    const makeDefault = !isDefaultClient(c)
    try {
      const settings = await invoke<LauncherSettings>('get_settings')
      settings.DefaultCustomClientId = makeDefault ? c.id : ''
      if (makeDefault && settings.SelectedClient !== 'Custom DLL') settings.SelectedClient = 'Custom DLL'
      await invoke('set_settings', { settings })
      defaultClientId = settings.DefaultCustomClientId
      toast(makeDefault ? `${c.name} is now the default client.` : 'Default client cleared.', 'info')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function addClientFromFile() {
    if (clientBusy) return
    const path = await open({
      multiple: false,
      title: 'Select a client DLL or EXE',
      filters: [
        { name: 'Clients', extensions: ['dll', 'exe'] },
        { name: 'All files', extensions: ['*'] },
      ],
    })
    if (!path || typeof path !== 'string') return
    try {
      const entry = await invoke<CustomClientEntry>('add_custom_client_from_file', { path })
      await loadClientLibrary()
      toast(`Added ${entry.name} to your library.`, 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function addClientFromUrl() {
    if (clientBusy || !clientUrlInput.trim()) return
    clientBusy = true
    try {
      const entry = await invoke<CustomClientEntry>('add_custom_client_from_url', { url: clientUrlInput.trim() })
      clientUrlInput = ''
      await loadClientLibrary()
      toast(`Downloaded ${entry.name}. It will auto-update before each launch.`, 'success')
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      clientBusy = false
    }
  }

  async function toggleClientAutoUpdate(c: CustomClientEntry) {
    try {
      await invoke('toggle_custom_client_auto_update', { id: c.id })
      await loadClientLibrary()
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  function startRenameClient(c: CustomClientEntry) {
    clientRenamingId = c.id
    clientRenameValue = c.name
  }
  async function commitRenameClient(c: CustomClientEntry) {
    try {
      await invoke('rename_custom_client', { id: c.id, newName: clientRenameValue })
    } catch (e) {
      toast(String(e), 'error')
    }
    clientRenamingId = null
    await loadClientLibrary()
  }
  function cancelRenameClient() {
    clientRenamingId = null
  }
  function onClientRenameKey(c: CustomClientEntry, e: KeyboardEvent) {
    if (e.key === 'Enter') commitRenameClient(c)
    else if (e.key === 'Escape') cancelRenameClient()
  }

  function confirmDeleteClient(id: string) {
    clientConfirmDeleteId = id
  }
  function cancelDeleteClient() {
    clientConfirmDeleteId = null
  }
  async function deleteClient(c: CustomClientEntry) {
    try {
      await invoke('delete_custom_client', { id: c.id })
      clientConfirmDeleteId = null
      await loadClientLibrary()
      toast(`Removed ${c.name}.`, 'info')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  function openClientLibraryFolder() {
    invoke('open_custom_client_library_folder').catch((e) => toast(String(e), 'error'))
  }

  // =====================================================================
  // system  (Home.Utilities.cs:356-409)
  // =====================================================================
  type SystemSnapshot = {
    cpu_name: string
    cpu_cores: number
    total_ram_bytes: number
    used_ram_bytes: number
    os: string
    arch: string
    disks: { name: string; total_bytes: number; free_bytes: number }[]
  }
  let sysInfo = $state<SystemSnapshot | null>(null)

  const ramPercent = $derived(
    sysInfo && sysInfo.total_ram_bytes ? (sysInfo.used_ram_bytes / sysInfo.total_ram_bytes) * 100 : 0,
  )

  async function loadSystemInfo() {
    try {
      sysInfo = await invoke<SystemSnapshot>('get_system_snapshot')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function copySystemReport() {
    let s: SystemSnapshot
    try {
      s = sysInfo ?? (await invoke<SystemSnapshot>('get_system_snapshot'))
    } catch (e) {
      toast(String(e), 'error')
      return
    }
    const lines = [
      `CPU:        ${s.cpu_name} (${s.cpu_cores} logical cores)`,
      `Memory:     ${formatBytes(s.total_ram_bytes)}`,
      `OS:         ${s.os} (${s.arch})`,
    ]
    for (const d of s.disks)
      lines.push(`Disk ${d.name.padEnd(3)}    ${formatBytes(d.free_bytes)} free of ${formatBytes(d.total_bytes)}`)
    try {
      await navigator.clipboard.writeText(lines.join('\n'))
      toast('System report copied.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  // =====================================================================
  // tab switching  (Home.Utilities.cs:22-45)
  // =====================================================================
  // Drives the ListSkeleton for whichever sub-tab is loading.
  let tabLoading = $state(false)
  async function loadUtilTab() {
    tabLoading = true
    try {
      if (utilTab === 'options') await loadGameOptions()
      else if (utilTab === 'shaders') await loadShaderState()
      else if (utilTab === 'clients') await loadClientLibrary()
      else if (utilTab === 'system') await loadSystemInfo()
    } finally {
      tabLoading = false
    }
  }

  function handleSubTab(key: string) {
    if (key === 'backups' || key === 'photos') {
      // Navigating away to the separate panels — the parent (App.svelte's
      // navigateFromPanel) resolves `backups`/`photos` + the `utilities`
      // strip to the Bedrock backups/screenshots panels via
      // EDITION_VIEW_MAP. Passing the already-resolved view name here
      // instead of the original key meant navigateFromPanel's lookup always
      // missed and silently did nothing — these buttons never worked.
      onNavigate?.(key, 'utilities')
      return
    }
    if (!(INTERNAL as string[]).includes(key)) return
    if (utilTab === key) return
    utilTab = key as Tab
    saveFilterPref('utilities.tab', utilTab)
    loadUtilTab()
  }

  loadUtilTab()
</script>

<PanelHeader title={t('panel.utilities')} {onBack} breadcrumb={['Home', 'Utilities', UTIL_TAB_LABELS[utilTab]]}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn"
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
      onclick={loadUtilTab}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<!--
  IA change: was a horizontal 6-tab strip (Options/Shaders/Library/System/
  Backups/Photos) — the exact "more than ~3 sections" case the rest of this
  overhaul moved to a vertical rail for. Backups/Photos still just navigate
  away to their own panels (handleSubTab is unchanged), so they never show
  as active here — only the four internal tabs can be.
-->
<div class="settings-layout">
  <nav class="settings-nav-rail" aria-label={t('tt.utilitiesSections')}>
    {#each UTILITIES as nav (nav.key)}
      <button
        type="button"
        class="settings-nav-btn {utilTab === nav.key ? 'active' : ''}"
        onclick={() => handleSubTab(nav.key)}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class={nav.icon}></i> {nav.label}
      </button>
    {/each}
  </nav>

  <div class="settings-content">
<div class="panel-body">
  {#if utilTab === 'options'}
    <div class="util-quick-row">
      <div class="util-quick-card">
        <div class="util-quick-head">
          <i class="fa-solid fa-display"></i>
          <span>{t('x.vSync')}</span>
        </div>
        <span class="setting-hint">
          {vsyncKeyPresent
            ? vsyncCurrentlyOn
              ? 'On — framerate capped to your refresh rate.'
              : 'Off — uncapped, lower input lag.'
            : 'Not set in options.txt yet — toggling writes the key for you.'}
        </span>
        <Toggle on={vsyncCurrentlyOn} onChanged={quickToggleVSync} />
      </div>
      <div class="util-quick-actions">
        <button
          class="btn-sm"
          disabled={!canOpenOptionsFolder}
          data-tooltip={canOpenOptionsFolder
            ? undefined
            : "No instance selected — opening the live install's settings folder needs a backend command that doesn't exist yet."}
          onclick={openOptionsFolder}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-folder-open"></i> Open folder
        </button>
        {#if hasBackup}
          <button
            class="btn-sm"
            data-tooltip={t('tt.restoreOptionsTxtGlacierSavedBefore')}
            onclick={restoreOptionsBackup}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-clock-rotate-left"></i> Restore backup
          </button>
        {/if}
      </div>
    </div>

    {#if tabLoading && options.length === 0 && !optionsError}
      <ListSkeleton rows={5} />
    {:else if optionsError}
      <div class="empty-state">
        <i class="fa-solid fa-triangle-exclamation"></i>
        <span>{t('empty.optionsTxtIsnTAvailable')}</span>
        <small>{optionsError}</small>
        <button type="button" class="btn-sm gap-top" onclick={loadGameOptions} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-rotate"></i> {t('common.retry')}
        </button>
      </div>
    {:else}
      <div class="util-filter-row">
        <input
          class="panel-search-input"
          aria-label={t('ph.filterSettingsEllipsis')}
          placeholder={t('ph.filterSettingsEllipsis')}
          bind:value={optionsFilter}
          onmousedown={(e) => e.stopPropagation()}
        />
        <button
          class="ts-chip {optionsShowAll ? 'selected' : ''}"
          data-tooltip={t('tt.showEveryKeyOptionsTxt')}
          onclick={() => (optionsShowAll = !optionsShowAll)}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-list"></i> All keys ({options.length})
        </button>
      </div>

      {#each visibleOptionGroups as [group, entries] (group)}
        <span class="panel-section-label">{group}</span>
        {#each entries as o (o.key)}
          <div class="setting-row">
            <div class="setting-meta">
              <span class="setting-label">{o.label}</span>
              <span class="setting-hint">{o.hint ? o.hint : o.key}</span>
            </div>
            {#if o.kind === 'toggle'}
              <Toggle on={optionIsOn(o)} onChanged={() => toggleOption(o)} />
            {:else}
              <input
                class="util-option-input {optionEdits.has(o.key) ? 'edited' : ''}"
                type={o.kind === 'text' ? 'text' : 'number'}
                step={o.kind === 'decimal' ? '0.01' : '1'}
                value={optionValue(o)}
                oninput={(e) => setOptionValue(o, (e.currentTarget as HTMLInputElement).value)}
                onmousedown={(e) => e.stopPropagation()}
              />
            {/if}
          </div>
        {/each}
      {/each}

      {#if visibleOptionGroups.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-magnifying-glass"></i>
          <span>{t('empty.nothingMatches')}</span>
          <small>{t('empty.tryAllKeysSeeEverythingMinecraft')}</small>
        </div>
      {/if}
    {/if}
  {:else if utilTab === 'shaders'}
    <p class="setting-hint intro">
      RenderDragon shaders replace compiled <code>.material.bin</code> files inside Minecraft. Glacier backs up every
      stock file it overwrites, so "Restore stock" always puts the game back exactly as it was.
    </p>

    {#if shaderBlockReason}
      <div class="empty-state">
        <i class="fa-solid fa-lock"></i>
        <span>{t('empty.shadersUnavailable')}</span>
        <small>{shaderBlockReason}</small>
        <!-- The original offers "Enable Developer Mode" here; there is no
             `enable_developer_mode` command, so the button stays disabled
             rather than pretending to work. -->
        <button
          class="btn-sm gap-top"
          disabled
          data-tooltip={t('tt.notWiredYetThisNeedsEnable')}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-shield-halved"></i><span>{t('empty.enableDeveloperMode')}</span>
        </button>
      </div>
    {:else}
      <div class="util-quick-actions toolbar-mb">
        <button class="btn-sm" disabled={shaderBusy} onclick={applyShaders} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-file-import"></i> Apply shader…
        </button>
        <button
          class="btn-sm"
          disabled={shaderBusy || appliedShaders.length === 0}
          onclick={restoreShaders}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-rotate-left"></i> Restore stock
        </button>
        <button class="btn-sm" onclick={openMaterialsFolder} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-folder-open"></i> Materials folder
        </button>
        <span class="setting-hint self-center">{stockMaterialCount} materials in this build</span>
      </div>

      <!-- The original shows a progress bar here; `apply_shaders` reports no
           progress events, so there is nothing to drive it. -->

      {#if tabLoading && appliedShaders.length === 0}
        <ListSkeleton rows={3} />
      {:else if appliedShaders.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-droplet"></i>
          <span>{t('empty.noShadersApplied')}</span>
          <small>{t('empty.pickShaderSMaterialBinFiles')}</small>
        </div>
      {:else}
        <div class="client-card-cols">
        {#each appliedShaders as sh (sh.file_name)}
          <div class="client-card">
            <div class="client-card-header">
              <div class="client-card-icon client-icon-accent">
                <i class="fa-solid fa-droplet"></i>
              </div>
              <div class="client-card-meta">
                <span class="client-card-name truncate" title={sh.file_name}>{sh.file_name}</span>
                <!-- The original appends " · added (no stock file)" from
                     AppliedShader.HadStock, which the Rust view doesn't carry. -->
                <span class="client-card-sub">from {sh.source_name} · {formatSize(sh.size_bytes)}</span>
              </div>
            </div>
            <p class="client-card-desc">Applied {formatRelativeTime(sh.applied_iso)}</p>
          </div>
        {/each}
        </div>
      {/if}
    {/if}
  {:else if utilTab === 'clients'}
    <p class="setting-hint intro">
      Your own DLL and EXE clients. Set one as default and pick <b>{t('x.customDll')}</b> as the active client to launch it.
      Entries added from a URL re-check their hash before every launch and refresh themselves when the build changes.
    </p>

    <div class="util-quick-actions toolbar-mb-8">
      <button class="btn-sm" disabled={clientBusy} onclick={addClientFromFile} onmousedown={(e) => e.stopPropagation()}>
        <i class="fa-solid fa-folder-open"></i> Add from file
      </button>
      <button class="btn-sm" onclick={openClientLibraryFolder} onmousedown={(e) => e.stopPropagation()}>
        <i class="fa-solid fa-box-open"></i> Library folder
      </button>
    </div>

    <div class="util-filter-row toolbar-mb">
      <input
        class="panel-search-input"
        placeholder="https://…/client.dll"
        bind:value={clientUrlInput}
        onmousedown={(e) => e.stopPropagation()}
      />
      <button
        class="btn-sm"
        disabled={clientBusy || !clientUrlInput.trim()}
        onclick={addClientFromUrl}
        onmousedown={(e) => e.stopPropagation()}
      >
        {#if clientBusy}<Spinner />{:else}<i class="fa-solid fa-cloud-arrow-down"></i>{/if}
        <span>{t('x.addFromUrl')}</span>
      </button>
    </div>

    {#if clientError && clientLib.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-triangle-exclamation"></i>
        <span>Could not load your library</span>
        <small>{clientError}</small>
        <button type="button" class="btn-sm gap-top" onclick={loadClientLibrary} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-rotate"></i> {t('common.retry')}
        </button>
      </div>
    {:else if tabLoading && clientLib.length === 0}
      <ListSkeleton rows={3} />
    {:else if clientLib.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-file-code"></i>
        <span>{t('empty.yourLibraryIsEmpty')}</span>
        <small>{t('empty.addClientFromDiskOrPaste')}</small>
      </div>
    {:else}
      <div class="client-card-cols">
      {#each clientLib as c (c.id)}
        <div class="client-card {isDefaultClient(c) ? 'client-active' : ''}">
          <div class="client-card-header">
          <div class="client-card-icon client-icon-accent">
            <i class="fa-solid {c.kind === 'exe' ? 'fa-square-arrow-up-right' : 'fa-file-code'}"></i>
          </div>
          <div class="client-card-meta">
            {#if clientRenamingId === c.id}
              <input
                class="util-option-input util-option-full"
                aria-label={t('tt.rename')}
                bind:value={clientRenameValue}
                onkeydown={(e) => onClientRenameKey(c, e)}
                onmousedown={(e) => e.stopPropagation()}
              />
            {:else}
              <span class="client-card-name truncate" title={c.name}>
                {c.name}
                {#if isDefaultClient(c)}<span class="tweaks-version-pill default-pill">{t('label.default')}</span>{/if}
              </span>
            {/if}
            <span class="client-card-sub">
              {c.kind.toUpperCase()} · {formatSize(c.size_bytes)}
              {#if c.source_url}
                · web{#if c.auto_update} · auto-update on{/if}
              {:else}
                · local
              {/if}
              {#if c.updated_iso}
                · {formatRelativeTime(c.updated_iso)}
              {/if}
            </span>
          </div>
          <div class="client-card-actions">
            {#if clientRenamingId === c.id}
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={t('tt.saveName')}
                aria-label={t('tt.saveName')}
                onclick={() => commitRenameClient(c)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-check"></i>
              </button>
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={t('tt.cancel')}
                aria-label={t('tt.cancel')}
                onclick={cancelRenameClient}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-xmark"></i>
              </button>
            {:else if clientConfirmDeleteId === c.id}
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={t('tt.confirmDelete')}
                aria-label={t('tt.confirmDelete')}
                onclick={() => deleteClient(c)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-check"></i>
              </button>
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={t('tt.cancel')}
                aria-label={t('tt.cancel')}
                onclick={cancelDeleteClient}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-xmark"></i>
              </button>
            {:else}
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={isDefaultClient(c) ? 'Clear default' : 'Set as default'}
                aria-label={isDefaultClient(c) ? 'Clear default' : 'Set as default'}
                onclick={() => setDefaultClient(c)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class={isDefaultClient(c) ? 'fa-solid fa-star' : 'fa-regular fa-star'}></i>
              </button>
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={t('tt.rename')}
                aria-label={t('tt.rename')}
                onclick={() => startRenameClient(c)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-pen"></i>
              </button>
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={t('tt.remove')}
                aria-label={t('tt.remove')}
                onclick={() => confirmDeleteClient(c.id)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-trash"></i>
              </button>
              {#if c.source_url}
                <div class="menu-anchor" onmousedown={(e) => e.stopPropagation()}>
                  <button
                    class="icon-btn icon-btn-ghost"
                    data-tooltip={t('tt.more')}
                    aria-label={t('tt.more')}
                    onclick={() => toggleClientMenu(c.id)}
                  >
                    <i class="fa-solid fa-ellipsis-vertical"></i>
                  </button>
                  {#if openClientMenuId === c.id}
                    <div class="tw-dropdown-menu">
                      <!-- "Check for updates now" needs an `update_custom_client`
                           command (C# ClientLib.UpdateIfChangedAsync); disabled. -->
                      <button
                        type="button"
                        class="tw-dropdown-item"
                        disabled
                        data-tooltip={t('tt.notWiredYetNeedsUpdateCustom')}
                        onclick={closeClientMenu}
                      >
                        <span><i class="fa-solid fa-cloud-arrow-down"></i> Check for updates now</span>
                      </button>
                      <button
                        type="button"
                        class="tw-dropdown-item"
                        onclick={() => { closeClientMenu(); toggleClientAutoUpdate(c) }}
                      >
                        <span
                          ><i class="fa-solid {c.auto_update ? 'fa-rotate' : 'fa-rotate-left'}"></i> Auto-update {c.auto_update
                            ? 'on'
                            : 'off'}</span
                        >
                      </button>
                    </div>
                  {/if}
                </div>
              {/if}
            {/if}
          </div>
          </div>
        </div>
      {/each}
      </div>
    {/if}
  {:else if utilTab === 'system'}
    {#if !sysInfo}
      <ListSkeleton rows={4} />
    {:else}
      <div class="util-quick-actions toolbar-mb">
        <button class="btn-sm" onclick={copySystemReport} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-clipboard"></i> Copy report
        </button>
        <!-- The original also shows "Up {uptime}" from HardwareMonitorService's
             live sample; there is no live-sample command here. -->
      </div>

      <!-- The original's CPU% gauge comes from the same live sample; only the
           memory gauge can be driven from `get_system_snapshot`. -->
      <div class="util-gauge-row">
        <div class="util-gauge">
          <div class="util-gauge-head">
            <span>{t('label.memory')}</span>
            <span>{formatBytes(sysInfo.used_ram_bytes)} / {formatBytes(sysInfo.total_ram_bytes)}</span>
          </div>
          <div class="progress-bar-wrap">
            <div class="progress-bar-fill" style="width:{ramPercent.toFixed(0)}%"></div>
          </div>
        </div>
      </div>

      <span class="panel-section-label">{t('x.processor')}</span>
      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">{sysInfo.cpu_name}</span>
          <span class="setting-hint">{sysInfo.cpu_cores} logical cores</span>
        </div>
      </div>

      <span class="panel-section-label">{t('x.graphics')}</span>
      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">{t('x.gpuDetailsUnavailable')}</span>
          <span class="setting-hint">
            Name, VRAM and driver came from WMI in the original launcher; no backend command exposes them here.
          </span>
        </div>
      </div>

      <span class="panel-section-label">{t('x.system')}</span>
      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">{sysInfo.os}</span>
          <span class="setting-hint">{sysInfo.arch}</span>
        </div>
      </div>
      <!-- Motherboard / BIOS / local IP rows need the same missing WMI data. -->

      <span class="panel-section-label">{t('x.storage')}</span>
      {#each sysInfo.disks as disk (disk.name)}
        {@const usedPercent = disk.total_bytes ? ((disk.total_bytes - disk.free_bytes) / disk.total_bytes) * 100 : 0}
        <div class="setting-row">
          <div class="setting-meta">
            <span class="setting-label">Drive {disk.name}</span>
            <span class="setting-hint">
              {formatBytes(disk.free_bytes)} free of {formatBytes(disk.total_bytes)}
            </span>
          </div>
          <div class="util-disk-bar">
            <div class="progress-bar-wrap"><div class="progress-bar-fill" style="width:{usedPercent.toFixed(0)}%"></div></div>
          </div>
        </div>
      {/each}

      <p class="setting-hint intro-top">
        Temperatures and fan speeds need a kernel-level driver and administrator rights, which Glacier deliberately
        doesn't ship — everything above is read without either.
      </p>
    {/if}
  {/if}
</div>
  </div>
</div>

{#if utilTab === 'options' && optionsPendingCount > 0}
  <div class="util-save-bar">
    <span>{optionsPendingCount} unsaved change{optionsPendingCount === 1 ? '' : 's'}</span>
    <button class="btn-sm" onclick={discardOptionEdits} onmousedown={(e) => e.stopPropagation()}>{t('x.discard')}</button>
    <button
      class="btn-sm btn-primary"
      disabled={optionsSaving}
      onclick={saveGameOptions}
      onmousedown={(e) => e.stopPropagation()}
    >
      {#if optionsSaving}<Spinner /><span>Saving…</span>{:else}<i class="fa-solid fa-floppy-disk"></i><span
          >{t('x.saveToOptionsTxt')}</span
        >{/if}
    </button>
  </div>
{/if}

<style>
  /* Utility cards and rows reuse the shared client-card / setting-row surfaces
     from app.css. This block only sets the spacing and small text roles the
     markup needs; colours come from tokens. */
  .setting-row {
    min-height: 48px;
    box-sizing: border-box;
  }
  .setting-row + .setting-row {
    border-top: 1px solid color-mix(in srgb, var(--text) 7%, transparent);
  }
  .intro {
    margin: 0 0 10px;
  }
  .intro-top {
    margin-top: 12px;
  }
  .self-center {
    align-self: center;
  }
  .gap-top {
    margin-top: 10px;
  }
  .toolbar-mb {
    margin-bottom: 10px;
  }
  .toolbar-mb-8 {
    margin-bottom: 8px;
  }
  .client-icon-accent {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .default-pill {
    margin-left: 6px;
  }
  .menu-anchor {
    position: relative;
  }
  .util-option-full {
    width: 100%;
  }
  .truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
</style>
