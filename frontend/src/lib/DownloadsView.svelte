<script lang="ts">
  // Ported from the "downloads" panel in Home.razor (lines 4561-4652) /
  // Home.Downloads.cs. Backed by `gl_download::DownloadQueue`'s
  // `active_entries()` registry (see src-tauri/src/commands/downloads.rs) —
  // every download submitted anywhere in the app through `state.downloads`
  // shows up here.
  //
  // The original re-rendered on `DownloadService.Changed`
  // (`OnDownloadsChanged` in Home.Downloads.cs); that event is now bridged to
  // the `downloads-changed` Tauri event, which carries the whole snapshot. So
  // this listens rather than polling on a timer — the poll was both a
  // divergence and needless wake-ups on the N5100 target.
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import PanelTabs from './components/PanelTabs.svelte'

  let { onBack, onOpenSettings, onOpenCredits }: {
    onBack?: () => void
    onOpenSettings?: () => void
    onOpenCredits?: () => void
  } = $props()

  type DownloadEntry = {
    id: string
    label: string
    status: 'Queued' | 'Downloading' | 'Completed' | 'Failed' | 'Cancelled'
    progress: number
    downloaded_bytes: number
    total_bytes: number
    error: string | null
  }

  let entries = $state<DownloadEntry[]>([])
  let unlisten: UnlistenFn | undefined

  /// Port of `FormatSize` (Home.razor.cs:692). The C# format string is
  /// `0.##`: up to two decimals with trailing zeros trimmed, applied at every
  /// unit including bytes.
  function formatSize(bytes: number): string {
    if (!bytes || bytes <= 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB']
    let v = bytes
    let i = 0
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024
      i++
    }
    const rounded = Math.round(v * 100) / 100
    return `${rounded} ${units[i]}`
  }

  function statusIcon(status: DownloadEntry['status']) {
    switch (status) {
      case 'Downloading': return 'fa-solid fa-arrow-down'
      case 'Completed': return 'fa-solid fa-circle-check'
      case 'Failed': return 'fa-solid fa-circle-xmark'
      case 'Cancelled': return 'fa-solid fa-ban'
      default: return 'fa-solid fa-circle-question'
    }
  }

  async function refresh() {
    try {
      entries = await invoke<DownloadEntry[]>('list_downloads')
    } catch {
      // Non-fatal — keep showing the last known list.
    }
  }

  async function cancel(entry: DownloadEntry) {
    await invoke('cancel_download', { id: entry.id })
  }

  async function remove(entry: DownloadEntry) {
    await invoke('remove_download', { id: entry.id })
  }

  async function clearFinished() {
    await invoke('clear_finished_downloads')
  }

  onMount(() => {
    // One read for whatever was already in flight before the panel opened,
    // then live updates.
    refresh()
    listen<DownloadEntry[]>('downloads-changed', (e) => {
      entries = e.payload
    }).then((fn) => {
      unlisten = fn
    })
  })
  onDestroy(() => {
    unlisten?.()
  })

  let hasFinished = $derived(entries.some((e) => e.status !== 'Downloading'))
</script>

<PanelHeader title="Downloads" {onBack}>
  {#snippet extraActions()}
    {#if hasFinished}
      <button type="button" class="panel-icon-btn" data-tooltip="Clear finished" aria-label="Clear finished" onclick={clearFinished}>
        <i class="fa-solid fa-broom"></i>
      </button>
    {/if}
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if entries.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-download"></i>
      <span>No downloads yet</span>
      <small>Versions, mods, and packs you download this session show up here.</small>
    </div>
  {:else}
    {#each entries as entry (entry.id)}
      <div class="version-row">
        <div class="version-meta" style="flex:1; min-width:0;">
          <span class="version-name" style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">{entry.label}</span>
          <span class="version-sub">
            <i class={statusIcon(entry.status)}></i>
            {entry.status}
            {#if entry.status === 'Downloading'}
              · {(entry.progress * 100).toFixed(0)}%
              {#if entry.total_bytes > 0}
                · {formatSize(entry.downloaded_bytes)} / {formatSize(entry.total_bytes)}
              {/if}
            {:else if entry.status === 'Failed' && entry.error}
              · {entry.error}
            {/if}
          </span>
          {#if entry.status === 'Downloading'}
            <div class="progress-bar-wrap" style="margin-top:4px;">
              <div class="progress-bar-fill" style="width:{(entry.progress * 100).toFixed(0)}%"></div>
            </div>
          {/if}
        </div>
        <div class="version-actions">
          {#if entry.status === 'Downloading'}
            <button class="icon-btn icon-btn-ghost" data-tooltip="Cancel" aria-label="Cancel" onclick={() => cancel(entry)}>
              <i class="fa-solid fa-xmark"></i>
            </button>
          {:else}
            <button class="icon-btn icon-btn-ghost" data-tooltip="Remove" aria-label="Remove" onclick={() => remove(entry)}>
              <i class="fa-solid fa-trash"></i>
            </button>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>

<PanelTabs
  tabs={[
    { id: 'settings', label: 'Settings', icon: 'fa-solid fa-gear' },
    { id: 'home', label: 'Home', icon: 'fa-solid fa-house' },
    { id: 'downloads', label: 'Downloads', icon: 'fa-solid fa-download' },
    { id: 'credits', label: 'Credits', icon: 'fa-solid fa-heart' },
  ]}
  active="downloads"
  onSelect={(id) => {
    if (id === 'settings') onOpenSettings?.()
    else if (id === 'home') onBack?.()
    else if (id === 'credits') onOpenCredits?.()
  }}
/>
