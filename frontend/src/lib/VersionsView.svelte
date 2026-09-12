<script lang="ts">
  /**
   * Port of the "versions" panel — Home.razor:2189-2361, with its code-behind
   * in Home.Clients.cs (BackFromVersions:180, RefreshVersionsAsync:193,
   * SwitchVersionsClient:199, IsPinned:218, TogglePin:220, CopyVersionTag:229,
   * VersionActions:235, LoadOderSoVersionsAsync:353, the OderSo entry handlers
   * :367-424, OderSoVersionActions:426, the sort options :477-518 and
   * ToggleShowOnlyDownloaded:521), plus FilteredVersions in Home.razor.cs:34
   * and OderSoDllState in Home.razor.cs:58.
   *
   * Two real clients live here: Latite (per-release picker, with pinning, sort
   * and an only-downloaded filter) and OderSo (per-DLL picker off the repo
   * listing, with one entry marked active). The panel has no `.panel-tabs`
   * strip — the original ends at `.panel-body`.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import IconBtn from './components/IconBtn.svelte'
  import ViewModeToggle from './components/ViewModeToggle.svelte'
  import { loadViewMode, saveViewMode, type ViewMode } from './viewMode'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import type { LauncherSettings } from './settings'

  let {
    settings,
    onBack,
    onSaveSettings,
    onSelectClient,
    onLaunchDll,
    onToast,
    breadcrumb = [],
  }: {
    settings: LauncherSettings | null
    onBack?: () => void
    onSaveSettings?: (next: LauncherSettings) => void
    onSelectClient?: (name: string) => void
    /** Home's `HandleVersionLaunch` / `HandleOderSoEntryLaunch` tail: go home, inject, report. */
    onLaunchDll?: (dllPath: string, label: string) => void
    onToast?: (message: string, kind?: string) => void
    breadcrumb?: string[]
  } = $props()

  type LatiteVersion = {
    tag: string
    display_name: string
    download_url: string
    asset_size: number
    published_at: string | null
    variant: string
  }

  type LatiteRow = LatiteVersion & {
    downloaded: boolean
    downloading: boolean
    error: string
  }

  /** Port of `OderSoService.DllEntry`. */
  type OderSoEntry = { name: string; sha: string; size: number }

  /** Port of the `OderSoDllState` wrapper (Home.razor.cs:58). */
  type OderSoRow = OderSoEntry & {
    downloaded: boolean
    downloading: boolean
    active: boolean
    error: string
  }

  let versionsClient = $state<'Latite' | 'OderSo'>('Latite')
  let versionsFilter = $state('')
  let versionsViewMode = $state<ViewMode>(loadViewMode('client-versions'))
  function setVersionsViewMode(m: ViewMode) {
    versionsViewMode = m
    saveViewMode('client-versions', m)
  }

  let isLoadingVersions = $state(false)
  let latiteFetchError = $state('')
  let versions = $state<LatiteRow[]>([])

  let isLoadingOderSoVersions = $state(false)
  let oderSoFetchError = $state('')
  let oderSoVersions = $state<OderSoRow[]>([])

  /** Port of `_sortOptions` (Home.Clients.cs:478) — order matters, it's a cycle. */
  const SORT_OPTIONS = [
    { key: 'newest', label: 'Newest first', icon: 'fa-arrow-down-9-1' },
    { key: 'oldest', label: 'Oldest first', icon: 'fa-arrow-up-9-1' },
    { key: 'name', label: 'Name A–Z', icon: 'fa-arrow-down-a-z' },
    { key: 'downloaded', label: 'Downloaded first', icon: 'fa-circle-down' },
  ]

  const sortMode = $derived(settings?.VersionSortMode || 'newest')
  const showOnlyDownloaded = $derived(settings?.ShowOnlyDownloaded ?? false)
  const pinnedVersions = $derived(settings?.PinnedVersions ?? [])

  /** Port of `CurrentSortLabel` — falls back to "Newest first" on an unknown mode. */
  const currentSortLabel = $derived(
    SORT_OPTIONS.find((o) => o.key === sortMode)?.label ?? 'Newest first',
  )
  /** Port of `CurrentSortIcon` — note the fallback icon differs from the label's. */
  const currentSortIcon = $derived(
    SORT_OPTIONS.find((o) => o.key === sortMode)?.icon ?? 'fa-arrow-down-wide-short',
  )

  function isPinned(tag: string) {
    return pinnedVersions.includes(tag)
  }

  /** Port of `TogglePin` — an unpin removes, a pin inserts at the FRONT. */
  function togglePin(row: LatiteRow) {
    if (!settings) return
    const pins = [...pinnedVersions]
    const at = pins.indexOf(row.tag)
    if (at >= 0) pins.splice(at, 1)
    else pins.unshift(row.tag)
    onSaveSettings?.({ ...settings, PinnedVersions: pins })
  }

  /** Port of `CycleVersionSort` — advances one step, wrapping. */
  function cycleVersionSort() {
    if (!settings) return
    const idx = SORT_OPTIONS.findIndex((o) => o.key === sortMode)
    const next = SORT_OPTIONS[(idx + 1) % SORT_OPTIONS.length]
    onSaveSettings?.({ ...settings, VersionSortMode: next.key })
  }

  function toggleShowOnlyDownloaded() {
    if (!settings) return
    onSaveSettings?.({ ...settings, ShowOnlyDownloaded: !showOnlyDownloaded })
  }

  /** Port of `CopyVersionTag` — same toast text, quotes included. */
  async function copyVersionTag(row: LatiteRow) {
    await window.copyToClipboard?.(row.tag)
    onToast?.(`Copied "${row.tag}"`, 'info')
  }

  /** Port of `SortedVersions` (Home.Clients.cs:494). */
  function sortedVersions(src: LatiteRow[]): LatiteRow[] {
    switch (sortMode) {
      case 'oldest':
        return [...src].reverse()
      case 'name':
        return [...src].sort((a, b) =>
          a.display_name.localeCompare(b.display_name, undefined, { sensitivity: 'accent' }),
        )
      case 'downloaded':
        // C#: OrderByDescending(IsDownloaded).ThenBy(0) — a stable partition,
        // downloaded first, original order preserved inside each group.
        return [...src.filter((v) => v.downloaded), ...src.filter((v) => !v.downloaded)]
      default:
        return src
    }
  }

  /** Port of `FilteredVersions` (Home.razor.cs:34) — filter matches name OR tag. */
  const filteredVersions = $derived.by(() => {
    let q = versions
    const filter = versionsFilter.trim()
    if (filter) {
      const needle = filter.toLowerCase()
      q = q.filter(
        (v) =>
          v.display_name.toLowerCase().includes(needle) || v.tag.toLowerCase().includes(needle),
      )
    }
    if (showOnlyDownloaded) q = q.filter((v) => v.downloaded)
    return sortedVersions(q)
  })

  const pinnedList = $derived(filteredVersions.filter((v) => isPinned(v.tag)))
  const unpinnedList = $derived(filteredVersions.filter((v) => !isPinned(v.tag)))

  /** Port of `SizeLabel` on `OderSoDllState` — F1 in MB, F0 in KB, empty at 0. */
  function sizeLabel(bytes: number): string {
    if (!bytes || bytes <= 0) return ''
    return bytes >= 1_048_576
      ? `${(bytes / 1_048_576).toFixed(1)} MB`
      : `${Math.round(bytes / 1024)} KB`
  }

  /** OderSo's list is already newest-first from the backend; only filtering here. */
  const filteredOderSo = $derived(
    versionsFilter.trim()
      ? oderSoVersions.filter((s) =>
          displayName(s).toLowerCase().includes(versionsFilter.trim().toLowerCase()),
        )
      : oderSoVersions,
  )

  /** Port of `OderSoDllState.DisplayName` — the file name without `.dll`. */
  function displayName(entry: OderSoEntry): string {
    return entry.name.replace(/\.dll$/i, '')
  }

  async function loadVersions() {
    isLoadingVersions = true
    latiteFetchError = ''
    try {
      const list = await invoke<LatiteVersion[]>('list_latite_versions')
      const next: LatiteRow[] = []
      for (const v of list) {
        const downloaded = await invoke<boolean>('latite_version_installed', { tag: v.tag }).catch(
          () => false,
        )
        next.push({ ...v, downloaded, downloading: false, error: '' })
      }
      versions = next
    } catch (e) {
      latiteFetchError = String(e)
    } finally {
      isLoadingVersions = false
    }
  }

  /** Port of `LoadOderSoVersionsAsync` — list, then mark downloaded/active. */
  async function loadOderSoVersions() {
    isLoadingOderSoVersions = true
    oderSoFetchError = ''
    try {
      const entries = await invoke<OderSoEntry[]>('list_oderso_dlls')
      const active = await invoke<string | null>('active_oderso_dll').catch(() => null)
      const next: OderSoRow[] = []
      for (const e of entries) {
        const downloaded = await invoke<boolean>('oderso_dll_installed', { name: e.name }).catch(
          () => false,
        )
        next.push({ ...e, downloaded, downloading: false, active: active === e.name, error: '' })
      }
      oderSoVersions = next
    } catch (e) {
      oderSoFetchError = String(e)
    } finally {
      isLoadingOderSoVersions = false
    }
  }

  /** Port of `RefreshVersionsAsync`. */
  function refreshVersions() {
    if (versionsClient === 'Latite') loadVersions()
    else loadOderSoVersions()
  }

  /**
   * Port of `SwitchVersionsClient` — clears the filter, and only loads the
   * newly selected client's list if it hasn't been loaded yet.
   */
  function switchVersionsClient(client: 'Latite' | 'OderSo') {
    versionsClient = client
    versionsFilter = ''
    if (client === 'Latite' && versions.length === 0) loadVersions()
    else if (client === 'OderSo' && oderSoVersions.length === 0) loadOderSoVersions()
  }

  /** Port of `HandleVersionDownload` — same toast text. */
  async function handleVersionDownload(row: LatiteRow) {
    if (row.downloading) return
    row.downloading = true
    row.error = ''
    try {
      await invoke('download_latite_version', { tag: row.tag, downloadUrl: row.download_url })
      row.downloaded = true
      onToast?.(`${row.display_name} downloaded!`, 'success')
    } catch (e) {
      row.error = String(e)
    } finally {
      row.downloading = false
    }
  }

  function handleVersionDelete(row: LatiteRow) {
    invoke('delete_latite_version', { tag: row.tag })
      .then(() => {
        row.downloaded = false
      })
      .catch((e) => {
        row.error = String(e)
      })
  }

  /** Port of `HandleVersionLaunch`'s head; the shell owns the launch itself. */
  async function handleVersionLaunch(row: LatiteRow) {
    try {
      const dllPath = await invoke<string>('latite_version_dll_path', { tag: row.tag })
      onSelectClient?.('Latite Client')
      onLaunchDll?.(dllPath, row.display_name)
    } catch (e) {
      row.error = String(e)
    }
  }

  /** Port of `HandleOderSoEntryDownload`. */
  async function handleOderSoDownload(row: OderSoRow) {
    if (row.downloading) return
    row.downloading = true
    row.error = ''
    try {
      await invoke('download_oderso_dll', { name: row.name })
      row.downloaded = true
      onToast?.(`${displayName(row)} downloaded!`, 'success')
    } catch (e) {
      row.error = String(e)
    } finally {
      row.downloading = false
    }
  }

  /**
   * Port of `HandleOderSoEntryDelete`. The original also falls back to Latite
   * when the deleted DLL was the selected client's only install; the backend's
   * `delete_oderso_dll` clears the active marker, and the fallback is left to
   * the shell's client selection, same as `ClientsView` does.
   */
  async function handleOderSoDelete(row: OderSoRow) {
    try {
      await invoke('delete_oderso_dll', { name: row.name })
      row.downloaded = false
      row.active = false
    } catch (e) {
      row.error = String(e)
    }
  }

  /** Port of `HandleOderSoEntryLaunch` — set active, then launch. */
  async function handleOderSoLaunch(row: OderSoRow) {
    try {
      const dllPath = await invoke<string>('set_active_oderso_dll', { name: row.name })
      for (const s of oderSoVersions) s.active = s.name === row.name
      onSelectClient?.('OderSo Client')
      onLaunchDll?.(dllPath, `OderSo ${row.name}`)
    } catch (e) {
      row.error = String(e)
    }
  }

  const refreshing = $derived(
    versionsClient === 'Latite' ? isLoadingVersions : isLoadingOderSoVersions,
  )

  /**
   * Live progress for the rows currently downloading, keyed by the queue label
   * the backend attaches (`Latite <tag>` / `OderSo <file>.dll`). This replaces
   * the original's per-`MinecraftVersion` `Progress` field + `ThrottledProgress`:
   * the queue is the single source of truth now, and `downloads-changed`
   * (the port of `DownloadService.Changed`) pushes it.
   */
  type QueueEntry = { id: string; label: string; status: string; progress: number }
  let queue = $state<QueueEntry[]>([])
  let unlisten: UnlistenFn | undefined

  function queueEntry(label: string): QueueEntry | undefined {
    return queue.find((e) => e.label === label && e.status === 'Downloading')
  }

  function progressFor(label: string): number {
    return queueEntry(label)?.progress ?? 0
  }

  function cancelFor(label: string) {
    const entry = queueEntry(label)
    if (entry) invoke('cancel_download', { id: entry.id }).catch(() => {})
  }

  onMount(() => {
    listen<QueueEntry[]>('downloads-changed', (e) => {
      queue = e.payload
    }).then((fn) => {
      unlisten = fn
    })
  })
  onDestroy(() => {
    unlisten?.()
  })

  loadVersions()
</script>

<PanelHeader title="Versions" {onBack} {breadcrumb}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn versions-sort-btn"
      onclick={cycleVersionSort}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={`Click to cycle sort (${currentSortLabel})`}
    >
      <i class="fa-solid {currentSortIcon}"></i>
      <span>{currentSortLabel}</span>
    </button>
    <button
      type="button"
      class="panel-icon-btn {showOnlyDownloaded ? 'active' : ''}"
      onclick={toggleShowOnlyDownloaded}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={showOnlyDownloaded ? 'Show all' : 'Only downloaded'}
      aria-label={showOnlyDownloaded ? 'Show all' : 'Only downloaded'}
    >
      <i class="fa-solid fa-circle-down"></i>
    </button>
    <button
      type="button"
      class="panel-icon-btn {refreshing ? 'spinning' : ''}"
      disabled={refreshing}
      onclick={refreshVersions}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
      aria-label="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <ViewModeToggle mode={versionsViewMode} onChange={setVersionsViewMode} />
  {/snippet}
</PanelHeader>

<div class="versions-client-switcher">
  <button
    class="vcs-btn {versionsClient === 'Latite' ? 'active' : ''}"
    onclick={() => switchVersionsClient('Latite')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-bolt"></i> Latite
  </button>
  <button
    class="vcs-btn {versionsClient === 'OderSo' ? 'active' : ''}"
    onclick={() => switchVersionsClient('OderSo')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-wand-magic-sparkles"></i> OderSo
  </button>
</div>

<div class="panel-search-wrap">
  <i class="fa-solid fa-magnifying-glass"></i>
  <input
    class="panel-search-input"
    placeholder={versionsClient === 'Latite'
      ? 'Filter Latite versions...'
      : 'Filter OderSo versions...'}
    bind:value={versionsFilter}
    onkeydown={(e) => e.stopPropagation()}
  />
</div>

<div class="panel-body">
  {#if versionsClient === 'Latite'}
    {#if latiteFetchError && versions.length > 0}
      <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>{latiteFetchError}</span></div>
    {/if}
    {#if isLoadingVersions}
      <ListSkeleton rows={6} />
    {:else if versions.length === 0 && latiteFetchError}
      <div class="versions-loading versions-error">
        <i class="fa-solid fa-triangle-exclamation"></i>
        <span>{latiteFetchError}</span>
        <button class="vcs-btn" style="margin-top:8px;" onclick={loadVersions}>
          <i class="fa-solid fa-rotate"></i> Retry
        </button>
      </div>
    {:else if versions.length === 0}
      <div class="versions-loading"><span style="opacity:0.5;">No versions found.</span></div>
    {:else}
      {#if pinnedList.length > 0}
        <span class="panel-section-label">Pinned</span>
        {#each pinnedList as v (v.tag)}
          <div class="version-row {v.error ? 'error-row' : ''} pinned">
            <div class="version-meta">
              <span class="version-name">{v.display_name}</span>
              {#if v.error}
                <span class="error-text">{v.error}</span>
              {:else if v.downloaded && !v.downloading}
                <span class="version-sub">Ready</span>
              {/if}
            </div>
            {#if !v.error}
              {#if v.downloading}
                <InlineProgress
                  progress={progressFor(`Latite ${v.tag}`)}
                  onCancel={() => cancelFor(`Latite ${v.tag}`)}
                />
              {:else}
                <div class="version-actions">
                  <button
                    class={isPinned(v.tag) ? 'pin-btn pinned' : 'pin-btn'}
                    title={isPinned(v.tag) ? 'Unpin' : 'Pin'}
                    onclick={() => togglePin(v)}
                  >
                    <i class="fa-solid fa-thumbtack" style={isPinned(v.tag) ? '' : 'opacity:0.4;'}></i>
                  </button>
                  <button class="copy-btn" title="Copy tag" onclick={() => copyVersionTag(v)}>
                    <i class="fa-solid fa-copy"></i>
                  </button>
                  {#if v.downloaded}
                    <IconBtn ghost icon="fa-solid fa-trash" title="Delete" onClick={() => handleVersionDelete(v)} />
                    <IconBtn icon="fa-solid fa-play" title="Launch" onClick={() => handleVersionLaunch(v)} />
                  {:else}
                    <IconBtn icon="fa-solid fa-download" title="Download" onClick={() => handleVersionDownload(v)} />
                  {/if}
                </div>
              {/if}
            {/if}
          </div>
        {/each}
        <span class="panel-section-label">All Versions</span>
      {/if}

      {#if unpinnedList.length === 0 && pinnedList.length === 0}
        <div class="versions-loading"><span style="opacity:0.5;">No results for "{versionsFilter}"</span></div>
      {/if}

      <div class="version-list {versionsViewMode === 'grid' ? 'grid-view' : ''}">
      {#each unpinnedList as v (v.tag)}
        <div class="version-row {v.error ? 'error-row' : ''}">
          <div class="version-meta">
            <span class="version-name">{v.display_name}</span>
            {#if v.error}
              <span class="error-text">{v.error}</span>
            {:else if v.downloaded && !v.downloading}
              <span class="version-sub">Ready</span>
            {/if}
          </div>
          {#if !v.error}
            {#if v.downloading}
              <InlineProgress
                progress={progressFor(`Latite ${v.tag}`)}
                onCancel={() => cancelFor(`Latite ${v.tag}`)}
              />
            {:else}
              <div class="version-actions">
                <button
                  class={isPinned(v.tag) ? 'pin-btn pinned' : 'pin-btn'}
                  title={isPinned(v.tag) ? 'Unpin' : 'Pin'}
                  onclick={() => togglePin(v)}
                >
                  <i class="fa-solid fa-thumbtack" style={isPinned(v.tag) ? '' : 'opacity:0.4;'}></i>
                </button>
                <button class="copy-btn" title="Copy tag" onclick={() => copyVersionTag(v)}>
                  <i class="fa-solid fa-copy"></i>
                </button>
                {#if v.downloaded}
                  <IconBtn ghost icon="fa-solid fa-trash" title="Delete" onClick={() => handleVersionDelete(v)} />
                  <IconBtn icon="fa-solid fa-play" title="Launch" onClick={() => handleVersionLaunch(v)} />
                {:else}
                  <IconBtn icon="fa-solid fa-download" title="Download" onClick={() => handleVersionDownload(v)} />
                {/if}
              </div>
            {/if}
          {/if}
        </div>
      {/each}
      </div>
    {/if}
  {/if}

  {#if versionsClient === 'OderSo'}
    {#if oderSoFetchError && oderSoVersions.length > 0}
      <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>{oderSoFetchError}</span></div>
    {/if}
    {#if isLoadingOderSoVersions}
      <ListSkeleton rows={5} />
    {:else if oderSoFetchError && oderSoVersions.length === 0}
      <div class="versions-loading versions-error">
        <i class="fa-solid fa-triangle-exclamation"></i>
        <span>{oderSoFetchError}</span>
        <button class="vcs-btn" style="margin-top:8px;" onclick={loadOderSoVersions}>
          <i class="fa-solid fa-rotate"></i> Retry
        </button>
      </div>
    {:else if oderSoVersions.length === 0}
      <div class="versions-loading"><span style="opacity:0.5;">No DLLs found in repo.</span></div>
    {:else}
      {#if filteredOderSo.length === 0}
        <div class="versions-loading"><span style="opacity:0.5;">No results for "{versionsFilter}"</span></div>
      {/if}

      <div class="version-list {versionsViewMode === 'grid' ? 'grid-view' : ''}">
      {#each filteredOderSo as state (state.name)}
        <div class="version-row {state.active ? 'oderso-active-row' : ''}">
          <div class="version-meta">
            <div style="display:flex; align-items:center; gap:6px;">
              <span class="version-name">{displayName(state)}</span>
              {#if state.active}<span class="tag-active">Active</span>{/if}
            </div>
            {#if state.error}
              <span class="error-text">{state.error}</span>
            {:else if state.downloaded && !state.downloading}
              <span class="version-sub">Ready · {sizeLabel(state.size)}</span>
            {:else if !state.downloaded}
              <span class="version-sub">{sizeLabel(state.size)}</span>
            {/if}
          </div>
          {#if state.downloading}
            <InlineProgress
              progress={progressFor(`OderSo ${state.name}`)}
              onCancel={() => cancelFor(`OderSo ${state.name}`)}
            />
          {:else}
            <div class="version-actions">
              {#if state.downloaded}
                <IconBtn ghost icon="fa-solid fa-trash" title="Delete" onClick={() => handleOderSoDelete(state)} />
                <IconBtn icon="fa-solid fa-play" title="Set active & Launch" onClick={() => handleOderSoLaunch(state)} />
              {:else}
                <IconBtn icon="fa-solid fa-download" title="Download" onClick={() => handleOderSoDownload(state)} />
              {/if}
            </div>
          {/if}
        </div>
      {/each}
      </div>
    {/if}
  {/if}
</div>
