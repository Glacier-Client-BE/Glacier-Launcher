<script lang="ts">
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
  import { open } from '@tauri-apps/plugin-dialog'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import EditionTabs from './components/EditionTabs.svelte'
  import Spinner from './components/Spinner.svelte'
  import Toggle from './components/Toggle.svelte'
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
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: 'info' | 'success' | 'error' | 'warning') => void
  } = $props()

  type Tab = 'options' | 'shaders' | 'clients' | 'system'
  const INTERNAL: Tab[] = ['options', 'shaders', 'clients', 'system']

  let utilTab = $state<Tab>(
    (INTERNAL as string[]).includes(initialTab) ? (initialTab as Tab) : 'options',
  )

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
    shaderBlockReason = await invoke<string | null>('shaders_unavailable_reason')
    appliedShaders = await invoke<AppliedShader[]>('list_applied_shaders')
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

  async function loadClientLibrary() {
    try {
      clientLib = await invoke<CustomClientEntry[]>('list_custom_clients')
      defaultClientId = (await invoke<LauncherSettings>('get_settings')).DefaultCustomClientId ?? ''
    } catch (e) {
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
    const s = sysInfo ?? (await invoke<SystemSnapshot>('get_system_snapshot'))
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
  async function loadUtilTab() {
    if (utilTab === 'options') await loadGameOptions()
    else if (utilTab === 'shaders') await loadShaderState()
    else if (utilTab === 'clients') await loadClientLibrary()
    else if (utilTab === 'system') await loadSystemInfo()
  }

  function handleSubTab(key: string) {
    if (key === 'backups' || key === 'photos') {
      // Navigating away to the separate panels — the parent routes.
      onNavigate?.(key === 'backups' ? 'bedrockbackups' : 'bedrockscreenshots')
      return
    }
    if (!(INTERNAL as string[]).includes(key)) return
    if (utilTab === key) return
    utilTab = key as Tab
    loadUtilTab()
  }

  loadUtilTab()
</script>

<PanelHeader title="Utilities" {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn"
      data-tooltip="Refresh"
      aria-label="Refresh"
      onclick={loadUtilTab}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<SubTabs strip="utilities" active={utilTab} onNavigate={handleSubTab} />

<div class="panel-body">
  {#if utilTab === 'options'}
    <div class="util-quick-row">
      <div class="util-quick-card">
        <div class="util-quick-head">
          <i class="fa-solid fa-display"></i>
          <span>V-Sync</span>
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
            data-tooltip="Restore the options.txt Glacier saved before its first edit"
            onclick={restoreOptionsBackup}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-clock-rotate-left"></i> Restore backup
          </button>
        {/if}
      </div>
    </div>

    {#if optionsError}
      <div class="empty-state">
        <i class="fa-solid fa-triangle-exclamation"></i>
        <span>options.txt isn't available</span>
        <small>{optionsError}</small>
      </div>
    {:else}
      <div class="util-filter-row">
        <input
          class="panel-search-input"
          placeholder="Filter settings…"
          bind:value={optionsFilter}
          onmousedown={(e) => e.stopPropagation()}
        />
        <button
          class="ts-chip {optionsShowAll ? 'selected' : ''}"
          data-tooltip="Show every key in options.txt"
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
          <span>Nothing matches</span>
          <small>Try "All keys" to see everything Minecraft wrote.</small>
        </div>
      {/if}
    {/if}
  {:else if utilTab === 'shaders'}
    <p class="setting-hint" style="margin:0 0 10px;">
      RenderDragon shaders replace compiled <code>.material.bin</code> files inside Minecraft. Glacier backs up every
      stock file it overwrites, so "Restore stock" always puts the game back exactly as it was.
    </p>

    {#if shaderBlockReason}
      <div class="empty-state">
        <i class="fa-solid fa-lock"></i>
        <span>Shaders unavailable</span>
        <small>{shaderBlockReason}</small>
        <!-- The original offers "Enable Developer Mode" here; there is no
             `enable_developer_mode` command, so the button stays disabled
             rather than pretending to work. -->
        <button
          class="btn-sm"
          style="margin-top:10px;"
          disabled
          data-tooltip="Not wired yet — this needs an `enable_developer_mode` backend command. Turn Developer Mode on in Windows Settings for now."
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-shield-halved"></i><span>Enable Developer Mode</span>
        </button>
      </div>
    {:else}
      <div class="util-quick-actions" style="margin-bottom:10px;">
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
        <span class="setting-hint" style="align-self:center;">{stockMaterialCount} materials in this build</span>
      </div>

      <!-- The original shows a progress bar here; `apply_shaders` reports no
           progress events, so there is nothing to drive it. -->

      {#if appliedShaders.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-droplet"></i>
          <span>No shaders applied</span>
          <small>Pick the shader's .material.bin files, or the .zip / .mcpack they shipped in.</small>
        </div>
      {:else}
        {#each appliedShaders as sh (sh.file_name)}
          <div class="version-row">
            <div class="version-icon version-icon-placeholder"><i class="fa-solid fa-droplet"></i></div>
            <div class="version-meta">
              <span class="version-name">{sh.file_name}</span>
              <span class="version-sub">
                <!-- The original appends " · added (no stock file)" from
                     AppliedShader.HadStock, which the Rust view doesn't carry. -->
                from {sh.source_name} · {formatSize(sh.size_bytes)} · {formatRelativeTime(sh.applied_iso)}
              </span>
            </div>
          </div>
        {/each}
      {/if}
    {/if}
  {:else if utilTab === 'clients'}
    <p class="setting-hint" style="margin:0 0 10px;">
      Your own DLL and EXE clients. Set one as default and pick <b>Custom DLL</b> as the active client to launch it.
      Entries added from a URL re-check their hash before every launch and refresh themselves when the build changes.
    </p>

    <div class="util-quick-actions" style="margin-bottom:8px;">
      <button class="btn-sm" disabled={clientBusy} onclick={addClientFromFile} onmousedown={(e) => e.stopPropagation()}>
        <i class="fa-solid fa-folder-open"></i> Add from file
      </button>
      <button class="btn-sm" onclick={openClientLibraryFolder} onmousedown={(e) => e.stopPropagation()}>
        <i class="fa-solid fa-box-open"></i> Library folder
      </button>
    </div>

    <div class="util-filter-row" style="margin-bottom:10px;">
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
        <span>Add from URL</span>
      </button>
    </div>

    {#if clientLib.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-file-code"></i>
        <span>Your library is empty</span>
        <small>Add a client from disk, or paste a direct link to a .dll / .exe.</small>
      </div>
    {:else}
      {#each clientLib as c (c.id)}
        <div class="version-row {isDefaultClient(c) ? 'version-row-active' : ''}">
          <div class="version-icon version-icon-placeholder">
            <i class="fa-solid {c.kind === 'exe' ? 'fa-square-arrow-up-right' : 'fa-file-code'}"></i>
          </div>
          <div class="version-meta">
            {#if clientRenamingId === c.id}
              <input
                class="util-option-input"
                style="width:100%;"
                bind:value={clientRenameValue}
                onkeydown={(e) => onClientRenameKey(c, e)}
                onmousedown={(e) => e.stopPropagation()}
              />
            {:else}
              <span class="version-name">
                {c.name}
                {#if isDefaultClient(c)}<span class="tweaks-version-pill" style="margin-left:6px;">Default</span>{/if}
              </span>
            {/if}
            <span class="version-sub">
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
          <div class="version-actions">
            {#if clientRenamingId === c.id}
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip="Save name"
                aria-label="Save name"
                onclick={() => commitRenameClient(c)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-check"></i>
              </button>
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip="Cancel"
                aria-label="Cancel"
                onclick={cancelRenameClient}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-xmark"></i>
              </button>
            {:else if clientConfirmDeleteId === c.id}
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip="Confirm delete"
                aria-label="Confirm delete"
                onclick={() => deleteClient(c)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-check"></i>
              </button>
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip="Cancel"
                aria-label="Cancel"
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
              {#if c.source_url}
                <!-- "Check for updates now" needs an `update_custom_client`
                     command (C# ClientLib.UpdateIfChangedAsync); disabled. -->
                <button
                  class="icon-btn icon-btn-ghost"
                  data-tooltip="Check for updates now — not wired yet, needs an `update_custom_client` backend command"
                  aria-label="Check for updates now"
                  disabled
                  onmousedown={(e) => e.stopPropagation()}
                >
                  <i class="fa-solid fa-cloud-arrow-down"></i>
                </button>
                <button
                  class="icon-btn icon-btn-ghost"
                  data-tooltip={c.auto_update ? 'Auto-update on' : 'Auto-update off'}
                  aria-label={c.auto_update ? 'Auto-update on' : 'Auto-update off'}
                  onclick={() => toggleClientAutoUpdate(c)}
                  onmousedown={(e) => e.stopPropagation()}
                >
                  <i class="fa-solid {c.auto_update ? 'fa-rotate' : 'fa-rotate-left'}"></i>
                </button>
              {/if}
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip="Rename"
                aria-label="Rename"
                onclick={() => startRenameClient(c)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-pen"></i>
              </button>
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip="Remove"
                aria-label="Remove"
                onclick={() => confirmDeleteClient(c.id)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-trash"></i>
              </button>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  {:else if utilTab === 'system'}
    {#if !sysInfo}
      <div class="versions-loading"><Spinner /><span>Reading hardware…</span></div>
    {:else}
      <div class="util-quick-actions" style="margin-bottom:10px;">
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
            <span>Memory</span>
            <span>{formatBytes(sysInfo.used_ram_bytes)} / {formatBytes(sysInfo.total_ram_bytes)}</span>
          </div>
          <div class="progress-bar-wrap">
            <div class="progress-bar-fill" style="width:{ramPercent.toFixed(0)}%"></div>
          </div>
        </div>
      </div>

      <span class="panel-section-label">Processor</span>
      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">{sysInfo.cpu_name}</span>
          <span class="setting-hint">{sysInfo.cpu_cores} logical cores</span>
        </div>
      </div>

      <span class="panel-section-label">Graphics</span>
      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">GPU details unavailable</span>
          <span class="setting-hint">
            Name, VRAM and driver came from WMI in the original launcher; no backend command exposes them here.
          </span>
        </div>
      </div>

      <span class="panel-section-label">System</span>
      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">{sysInfo.os}</span>
          <span class="setting-hint">{sysInfo.arch}</span>
        </div>
      </div>
      <!-- Motherboard / BIOS / local IP rows need the same missing WMI data. -->

      <span class="panel-section-label">Storage</span>
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

      <p class="setting-hint" style="margin-top:12px;">
        Temperatures and fan speeds need a kernel-level driver and administrator rights, which Glacier deliberately
        doesn't ship — everything above is read without either.
      </p>
    {/if}
  {/if}
</div>

{#if utilTab === 'options' && optionsPendingCount > 0}
  <div class="util-save-bar">
    <span>{optionsPendingCount} unsaved change{optionsPendingCount === 1 ? '' : 's'}</span>
    <button class="btn-sm" onclick={discardOptionEdits} onmousedown={(e) => e.stopPropagation()}>Discard</button>
    <button
      class="btn-sm btn-primary"
      disabled={optionsSaving}
      onclick={saveGameOptions}
      onmousedown={(e) => e.stopPropagation()}
    >
      {#if optionsSaving}<Spinner /><span>Saving…</span>{:else}<i class="fa-solid fa-floppy-disk"></i><span
          >Save to options.txt</span
        >{/if}
    </button>
  </div>
{/if}

<EditionTabs edition="bedrock" active="utilities" onNavigate={(view) => onNavigate?.(view)} />
