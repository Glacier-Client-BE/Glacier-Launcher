<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { open } from '@tauri-apps/plugin-dialog'
  import PanelHeader from './components/PanelHeader.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import type { LauncherSettings } from './settings'

  /**
   * 1:1 port of the `clients` panel - Home.razor lines 1932-2188, with
   * behaviour from Pages/Home.Clients.cs.
   *
   * Client downloads go through the same `state.downloads` queue as every
   * other panel, so they already carry real progress + cancel support at
   * the backend - this panel just wasn't listening to `downloads-changed`
   * yet (see LeviLaminaModsView/TweaksView for the same pattern), so
   * InlineProgress rendered its spinner-only form.
   */
  let {
    settings,
    onChange,
    onBack,
    onOpenVersions,
    onNavigate,
    versionsHasBadge = false,
    breadcrumb = [],
  }: {
    settings: LauncherSettings
    onChange: (s: LauncherSettings) => void
    onBack?: () => void
    onOpenVersions?: () => void
    onNavigate?: (view: string) => void
    versionsHasBadge?: boolean
    breadcrumb?: string[]
  } = $props()

  type ClientState = {
    downloaded: boolean
    upToDate: boolean
    downloading: boolean
    error: string
  }

  function freshState(): ClientState {
    return { downloaded: false, upToDate: false, downloading: false, error: '' }
  }

  let flarial = $state(freshState())
  let oderso = $state(freshState())
  let levilamina = $state(freshState())

  /**
   * Feature 26 — client compatibility matrix.
   *
   * Static, hand-maintained (client version, MC version) pairs known to
   * work together. This is informational only (a tooltip/badge), never a
   * launch gate — clients are notoriously fast-moving and a stale entry
   * here must never block a play session.
   *
   * NOTE: the specific version numbers below are PLACEHOLDERS, not
   * verified compatibility data. Replace with real (client version, MC
   * version) pairs sourced from each client's own changelog/Discord before
   * relying on this for anything user-facing.
   */
  type CompatEntry = { clientVersion: string; mcVersion: string; note?: string }
  const COMPAT_MATRIX: Record<string, CompatEntry[]> = {
    'Flarial Client': [
      { clientVersion: 'example-0.0.0', mcVersion: 'example-1.0.0', note: 'PLACEHOLDER — fill in real data' },
    ],
    'OderSo Client': [
      { clientVersion: 'example-0.0.0', mcVersion: 'example-1.0.0', note: 'PLACEHOLDER — fill in real data' },
    ],
    LeviLamina: [
      { clientVersion: 'example-0.0.0', mcVersion: 'example-1.0.0', note: 'PLACEHOLDER — fill in real data' },
    ],
  }

  let installedMcVersion = $state('')

  /** Known-good if the matrix has no entries for this client (nothing to warn about yet), or an entry names the currently-installed MC version. */
  function isKnownCompatible(clientName: string): boolean {
    const entries = COMPAT_MATRIX[clientName]
    if (!entries || entries.length === 0) return true
    if (!installedMcVersion) return true
    return entries.some((e) => e.mcVersion === installedMcVersion)
  }

  function compatTooltip(clientName: string): string {
    const entries = COMPAT_MATRIX[clientName]
    if (!entries || entries.length === 0) return ''
    if (isKnownCompatible(clientName)) return ''
    const known = entries.map((e) => `${e.clientVersion} ↔ MC ${e.mcVersion}`).join(', ')
    return `Not in the known-compatible list for MC ${installedMcVersion || '?'}. Known-good pairs: ${known}. Informational only — launching is not blocked.`
  }

  async function refreshInstalledMcVersion() {
    try {
      const v = await invoke<{ installed: boolean; display_version: string; is_preview: boolean }>(
        'installed_bedrock_version',
      )
      installedMcVersion = v.installed ? v.display_version : ''
    } catch {
      installedMcVersion = ''
    }
  }

  const customDllPath = $derived(settings.CustomDllPath ?? '')
  let copiedDllPath = $state(false)

  function fileName(p: string): string {
    return p.split(/[\\/]/).pop() ?? p
  }

  function openVersions() {
    if (onOpenVersions) onOpenVersions()
    else onNavigate?.('versions')
  }

  function openLeviLaminaMods() {
    onNavigate?.('levilaminamods')
  }

  async function refreshFlarial() {
    try {
      const path = await invoke<string>('flarial_client_path')
      const downloaded = await invoke<boolean>('path_exists', { path }).catch(() => false)
      flarial.downloaded = downloaded
      flarial.upToDate = downloaded
        ? await invoke<boolean>('check_flarial_dll_up_to_date', { dllPath: path })
        : false
    } catch (e) {
      flarial.error = String(e)
    }
  }

  async function refreshOderso() {
    try {
      oderso.downloaded = await invoke<boolean>('oderso_client_installed')
      oderso.upToDate = oderso.downloaded
        ? await invoke<boolean>('check_oderso_up_to_date')
        : false
    } catch (e) {
      oderso.error = String(e)
    }
  }

  async function refreshLevilamina() {
    try {
      levilamina.downloaded = await invoke<boolean>('levilamina_client_installed')
      levilamina.upToDate = levilamina.downloaded
        ? await invoke<boolean>('check_levilamina_up_to_date')
        : false
    } catch (e) {
      levilamina.error = String(e)
    }
  }

  function selectClient(name: string) {
    onChange({ ...settings, SelectedClient: name })
  }

  /** Live download queue, for real per-download progress and cancel. */
  type QueueEntry = { id: string; label: string; status: string; progress: number }
  let queue = $state<QueueEntry[]>([])
  let unlistenQueue: UnlistenFn | undefined

  function activeEntry(label: string): QueueEntry | undefined {
    return queue.find((e) => e.label === label && e.status === 'Downloading')
  }

  async function cancelDownload(label: string) {
    const entry = activeEntry(label)
    if (entry) await invoke('cancel_download', { id: entry.id }).catch(() => {})
  }

  onMount(() => {
    listen<QueueEntry[]>('downloads-changed', (e) => {
      queue = e.payload
    }).then((fn) => {
      unlistenQueue = fn
    })
  })
  onDestroy(() => unlistenQueue?.())

  async function handleFlarialDownload() {
    if (flarial.downloading) return
    flarial.downloading = true
    flarial.error = ''
    try {
      await invoke('download_flarial_client')
      await refreshFlarial()
    } catch (e) {
      flarial.error = String(e)
    } finally {
      flarial.downloading = false
    }
  }

  async function handleFlarialDelete() {
    try {
      await invoke('delete_flarial_client')
      flarial = freshState()
    } catch (e) {
      flarial.error = String(e)
    }
  }

  async function handleOderSoDownload() {
    if (oderso.downloading) return
    oderso.downloading = true
    oderso.error = ''
    try {
      await invoke('download_oderso_client')
      oderso.downloaded = true
      oderso.upToDate = true
    } catch (e) {
      oderso.error = String(e)
    } finally {
      oderso.downloading = false
    }
  }

  async function handleOderSoDelete() {
    try {
      await invoke('delete_oderso_client')
      oderso = freshState()
      if (settings.SelectedClient === 'OderSo Client') selectClient('Latite Client')
    } catch (e) {
      oderso.error = String(e)
    }
  }

  async function handleLeviLaminaDownload() {
    if (levilamina.downloading) return
    levilamina.downloading = true
    levilamina.error = ''
    try {
      await invoke('download_levilamina_client')
      levilamina.downloaded = true
      levilamina.upToDate = true
    } catch (e) {
      levilamina.error = String(e)
    } finally {
      levilamina.downloading = false
    }
  }

  async function handleLeviLaminaDelete() {
    try {
      await invoke('delete_levilamina_client')
      levilamina = freshState()
      if (settings.SelectedClient === 'LeviLamina') selectClient('Latite Client')
    } catch (e) {
      levilamina.error = String(e)
    }
  }

  function clearCustomDll() {
    const next = { ...settings, CustomDllPath: '' }
    if (settings.SelectedClient === 'Custom DLL') next.SelectedClient = 'Latite Client'
    onChange(next)
  }

  async function copyDllPath() {
    await navigator.clipboard?.writeText(customDllPath)
    copiedDllPath = true
    setTimeout(() => (copiedDllPath = false), 1500)
  }

  async function pickDllFile() {
    const path = await open({ multiple: false, filters: [{ name: 'DLL files', extensions: ['dll'] }] })
    if (typeof path === 'string' && path) {
      onChange({ ...settings, SelectedClient: 'Custom DLL', CustomDllPath: path })
    }
  }

  refreshFlarial()
  refreshOderso()
  refreshLevilamina()
  refreshInstalledMcVersion()
</script>

<PanelHeader title="Clients" {onBack} {breadcrumb}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn versions-sort-btn"
      onclick={openVersions}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Browse Latite/OderSo versions"
      aria-label="Browse Latite/OderSo versions"
    >
      <i class="fa-solid fa-clock-rotate-left"></i>
      <span>Versions</span>
      {#if versionsHasBadge}<span class="notification-badge"></span>{/if}
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">

  <div class="client-card" class:client-active={settings.SelectedClient === 'Flarial Client'}>
    <div class="client-card-header">
      <div class="client-card-icon client-card-icon-img"><img src="/images/clients/flarial.svg" alt="Flarial Client" /></div>
      <div class="client-card-meta">
        <span class="client-card-name">
          Flarial Client
          {#if flarial.downloaded && !isKnownCompatible('Flarial Client')}
            <i class="fa-solid fa-circle-info compat-warn-icon" data-tooltip={compatTooltip('Flarial Client')} aria-label="Compatibility notice"></i>
          {/if}
        </span>
        <span class="client-card-sub">
          {#if flarial.downloaded}
            {#if flarial.upToDate}
              <span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Up to date</span>
            {:else if flarial.downloading}
              <span class="tag-uptodate"><i class="fa-solid fa-rotate"></i> Updating…</span>
            {:else}
              <span class="tag-outdated"><i class="fa-solid fa-triangle-exclamation"></i> Update available</span>
            {/if}
          {:else}
            <span class="client-card-note">Not downloaded</span>
          {/if}
        </span>
      </div>
      <div class="client-card-actions">
        {#if settings.SelectedClient !== 'Flarial Client' && flarial.downloaded}
          <button
            class="icon-btn"
            data-tooltip="Select"
            aria-label="Select"
            onclick={() => selectClient('Flarial Client')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-check"></i>
          </button>
        {/if}
        {#if flarial.downloading}
          <InlineProgress progress={activeEntry('Flarial Client')?.progress ?? 0} showSpinner onCancel={() => cancelDownload('Flarial Client')} />
        {:else}
          {#if !flarial.downloaded || !flarial.upToDate}
            <button
              class="icon-btn"
              data-tooltip={flarial.downloaded ? 'Update' : 'Download'}
              aria-label={flarial.downloaded ? 'Update' : 'Download'}
              onclick={handleFlarialDownload}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid {flarial.downloaded ? 'fa-arrow-up' : 'fa-download'}"></i>
            </button>
          {/if}
          {#if flarial.downloaded}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Delete"
              aria-label="Delete"
              onclick={handleFlarialDelete}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          {/if}
        {/if}
      </div>
    </div>
    {#if flarial.error}<span class="error-text">{flarial.error}</span>{/if}
    <p class="client-card-desc">Feature-rich Bedrock client with modules, HUD customization, and active development.</p>
  </div>

  <div class="client-card" class:client-active={settings.SelectedClient === 'Latite Client'}>
    <div class="client-card-header">
      <div class="client-card-icon client-card-icon-img"><img src="/images/clients/latite.png" alt="Latite Client" /></div>
      <div class="client-card-meta">
        <span class="client-card-name">Latite Client</span>
        <span class="client-card-sub client-card-note">Versioned GitHub releases</span>
      </div>
      <div class="client-card-actions">
        {#if settings.SelectedClient !== 'Latite Client'}
          <button
            class="icon-btn"
            data-tooltip="Select"
            aria-label="Select"
            onclick={() => selectClient('Latite Client')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-check"></i>
          </button>
        {/if}
        <button
          class="icon-btn icon-btn-ghost"
          style="background:var(--bg-item);"
          data-tooltip="View Versions"
          aria-label="View Versions"
          onclick={openVersions}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-clock-rotate-left"></i>
        </button>
      </div>
    </div>
    <p class="client-card-desc">Classic Minecraft Bedrock client. Choose a specific release from the Versions panel.</p>
  </div>

  <div class="client-card" class:client-active={settings.SelectedClient === 'OderSo Client'}>
    <div class="client-card-header">
      <div class="client-card-icon client-card-icon-img"><img src="/images/clients/oderso.png" alt="OderSo Client" /></div>
      <div class="client-card-meta">
        <span class="client-card-name">
          OderSo Client
          {#if oderso.downloaded && !isKnownCompatible('OderSo Client')}
            <i class="fa-solid fa-circle-info compat-warn-icon" data-tooltip={compatTooltip('OderSo Client')} aria-label="Compatibility notice"></i>
          {/if}
        </span>
        <span class="client-card-sub">
          {#if oderso.downloaded}
            {#if oderso.upToDate}
              <span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Up to date</span>
            {:else if oderso.downloading}
              <span class="tag-uptodate"><i class="fa-solid fa-rotate"></i> Updating…</span>
            {:else}
              <span class="tag-outdated"><i class="fa-solid fa-triangle-exclamation"></i> Update available</span>
            {/if}
          {:else}
            <span class="client-card-note">Not downloaded</span>
          {/if}
        </span>
      </div>
      <div class="client-card-actions">
        {#if settings.SelectedClient !== 'OderSo Client' && oderso.downloaded}
          <button
            class="icon-btn"
            data-tooltip="Select"
            aria-label="Select"
            onclick={() => selectClient('OderSo Client')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-check"></i>
          </button>
        {/if}
        {#if oderso.downloading}
          <InlineProgress progress={activeEntry('OderSo Client')?.progress ?? 0} showSpinner onCancel={() => cancelDownload('OderSo Client')} />
        {:else}
          {#if !oderso.downloaded || !oderso.upToDate}
            <button
              class="icon-btn"
              data-tooltip={oderso.downloaded ? 'Update' : 'Download'}
              aria-label={oderso.downloaded ? 'Update' : 'Download'}
              onclick={handleOderSoDownload}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid {oderso.downloaded ? 'fa-arrow-up' : 'fa-download'}"></i>
            </button>
          {/if}
          {#if oderso.downloaded}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Delete"
              aria-label="Delete"
              onclick={handleOderSoDelete}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          {/if}
        {/if}
      </div>
    </div>
    {#if oderso.error}<span class="error-text">{oderso.error}</span>{/if}
    <p class="client-card-desc">OderSo Client — curated Minecraft Bedrock experience by MasonOderSo.</p>
  </div>

  <div class="client-card" class:client-active={settings.SelectedClient === 'LeviLamina'}>
    <div class="client-card-header">
    <div class="client-card-icon client-card-icon-img"><img src="/images/clients/levi-lamina.png" alt="Vanilla" /></div>
      <div class="client-card-meta">
        <span class="client-card-name">
          LeviLamina
          {#if levilamina.downloaded && !isKnownCompatible('LeviLamina')}
            <i class="fa-solid fa-circle-info compat-warn-icon" data-tooltip={compatTooltip('LeviLamina')} aria-label="Compatibility notice"></i>
          {/if}
        </span>
        <span class="client-card-sub">
          {#if levilamina.downloaded}
            {#if levilamina.upToDate}
              <span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Up to date</span>
            {:else if levilamina.downloading}
              <span class="tag-uptodate"><i class="fa-solid fa-rotate"></i> Updating…</span>
            {:else}
              <span class="tag-outdated"><i class="fa-solid fa-triangle-exclamation"></i> Update available</span>
            {/if}
          {:else}
            <span class="client-card-note">Not downloaded</span>
          {/if}
        </span>
      </div>
      <div class="client-card-actions">
        {#if levilamina.downloaded}
          <button
            class="icon-btn"
            data-tooltip="Browse LeviLamina mods"
            aria-label="Browse LeviLamina mods"
            onclick={openLeviLaminaMods}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-puzzle-piece"></i>
          </button>
        {/if}
        {#if settings.SelectedClient !== 'LeviLamina' && levilamina.downloaded}
          <button
            class="icon-btn"
            data-tooltip="Select"
            aria-label="Select"
            onclick={() => selectClient('LeviLamina')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-check"></i>
          </button>
        {/if}
        {#if levilamina.downloading}
          <InlineProgress progress={activeEntry('LeviLamina')?.progress ?? 0} showSpinner onCancel={() => cancelDownload('LeviLamina')} />
        {:else}
          {#if !levilamina.downloaded || !levilamina.upToDate}
            <button
              class="icon-btn"
              data-tooltip={levilamina.downloaded ? 'Update' : 'Download'}
              aria-label={levilamina.downloaded ? 'Update' : 'Download'}
              onclick={handleLeviLaminaDownload}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid {levilamina.downloaded ? 'fa-arrow-up' : 'fa-download'}"></i>
            </button>
          {/if}
          {#if levilamina.downloaded}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Delete"
              aria-label="Delete"
              onclick={handleLeviLaminaDelete}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          {/if}
        {/if}
      </div>
    </div>
    {#if levilamina.error}<span class="error-text">{levilamina.error}</span>{/if}
    <p class="client-card-desc">LeviLamina — open-source native Bedrock mod loader by LiteLDev, injected the same way as the other clients here.</p>
  </div>

  <div class="client-card" class:client-active={settings.SelectedClient === 'Vanilla'}>
    <div class="client-card-header">
    <div class="client-card-icon client-card-icon-img"><img src="/images/mc-logo.png" alt="Vanilla" /></div>
      <div class="client-card-meta">
        <span class="client-card-name">Vanilla</span>
        <span class="client-card-sub client-card-note">Launches Minecraft with no DLL injection</span>
      </div>
      <div class="client-card-actions">
        {#if settings.SelectedClient !== 'Vanilla'}
          <button
            class="icon-btn"
            data-tooltip="Select"
            aria-label="Select"
            onclick={() => selectClient('Vanilla')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-check"></i>
          </button>
        {/if}
      </div>
    </div>
    <p class="client-card-desc">Pure stock Minecraft Bedrock — useful for diagnostics, multiplayer realms, or just playing un-modified.</p>
  </div>

  {#if customDllPath}
    <div
      class="client-card"
      class:client-active={settings.SelectedClient === 'Custom DLL'}
      style="border-left-color:var(--accent);"
    >
      <div class="client-card-header">
        <div class="client-card-icon" style="background:var(--accent-bg); color:var(--accent);">
          <i class="fa-solid fa-file-code"></i>
        </div>
        <div class="client-card-meta">
          <span class="client-card-name">{fileName(customDllPath)}</span>
          <span class="client-card-sub client-card-path">{customDllPath}</span>
        </div>
        <div class="client-card-actions">
          <button
            class="copy-btn"
            class:copied={copiedDllPath}
            data-tooltip="Copy path"
            aria-label="Copy path"
            onclick={copyDllPath}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid {copiedDllPath ? 'fa-check' : 'fa-copy'}"></i>
          </button>
          <button
            class="icon-btn"
            data-tooltip="Select"
            aria-label="Select"
            onclick={() => selectClient('Custom DLL')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-check"></i>
          </button>
          <button
            class="icon-btn icon-btn-ghost"
            data-tooltip="Remove"
            aria-label="Remove"
            onclick={clearCustomDll}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-xmark"></i>
          </button>
        </div>
      </div>
      <p class="client-card-desc">Custom DLL loaded via drag-and-drop or file picker.</p>
    </div>
  {/if}

  <div class="drop-hint-row">
    <i class="fa-solid fa-file-arrow-down"></i>
    <span>Drag &amp; drop a DLL onto the window</span>
    <button
      class="btn-sm"
      onclick={pickDllFile}
      onmousedown={(e) => e.stopPropagation()}
      style="margin-left:auto;"
    >
      <i class="fa-solid fa-folder-open"></i> Browse...
    </button>
  </div>

</div>

