<script lang="ts">
  import { t } from './i18nStore.svelte'
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
  import IconBtn from './components/IconBtn.svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import PanelTabs from './components/PanelTabs.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'

  let { onBack, onOpenSettings, onOpenCredits }: {
    onBack?: () => void
    onOpenSettings?: () => void
    onOpenCredits?: () => void
  } = $props()

  type DownloadEntry = {
    id: string
    label: string
    status: 'Queued' | 'Downloading' | 'Completed' | 'Failed' | 'Cancelled' | 'Paused'
    progress: number
    downloaded_bytes: number
    total_bytes: number
    error: string | null
  }

  let entries = $state<DownloadEntry[]>([])
  let loaded = $state(false)
  let unlisten: UnlistenFn | undefined
  let listenerDisposed = false

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

  /// Translated display text for a backend status enum value.
  function statusLabel(status: DownloadEntry['status']): string {
    switch (status) {
      case 'Downloading': return t('status.downloading')
      case 'Completed': return t('status.completed')
      case 'Failed': return t('status.failed')
      case 'Cancelled': return t('status.cancelled')
      case 'Paused': return t('label.paused')
      case 'Queued': return t('status.queued')
    }
    return status
  }

  function statusIcon(status: DownloadEntry['status']) {
    switch (status) {
      case 'Downloading': return 'fa-solid fa-arrow-down'
      case 'Completed': return 'fa-solid fa-circle-check'
      case 'Failed': return 'fa-solid fa-circle-xmark'
      case 'Cancelled': return 'fa-solid fa-ban'
      case 'Paused': return 'fa-solid fa-pause'
      default: return 'fa-solid fa-circle-question'
    }
  }

  async function refresh() {
    try {
      entries = await invoke<DownloadEntry[]>('list_downloads')
    } catch {
      // Non-fatal — keep showing the last known list.
    } finally {
      loaded = true
    }
  }

  // Each action is a fire-and-forget button; failures are logged rather than
  // left as unhandled promise rejections.
  const logFailure = (e: unknown) => console.warn('download action failed:', e)

  async function cancel(entry: DownloadEntry) {
    await invoke('cancel_download', { id: entry.id }).catch(logFailure)
  }

  async function remove(entry: DownloadEntry) {
    await invoke('remove_download', { id: entry.id }).catch(logFailure)
  }

  async function retryOne(entry: DownloadEntry) {
    await invoke('retry_download', { id: entry.id }).catch(logFailure)
  }

  // QOL: pause/resume — pausing keeps the partial file on disk so resume
  // continues via a Range request instead of restarting from zero.
  async function pauseOne(entry: DownloadEntry) {
    await invoke('pause_download', { id: entry.id }).catch(logFailure)
  }
  async function resumeOne(entry: DownloadEntry) {
    await invoke('resume_download', { id: entry.id }).catch(logFailure)
  }

  async function clearFinished() {
    await invoke('clear_finished_downloads').catch(logFailure)
  }

  let retrying = $state(false)
  async function retryAllFailed() {
    retrying = true
    try {
      await invoke('retry_failed_downloads')
    } finally {
      retrying = false
    }
  }

  onMount(() => {
    // One read for whatever was already in flight before the panel opened,
    // then live updates.
    refresh()
    listen<DownloadEntry[]>('downloads-changed', (e) => {
      entries = e.payload
    }).then((fn) => {
      if (listenerDisposed) fn()
      else unlisten = fn
    })
  })
  onDestroy(() => {
    listenerDisposed = true
    unlisten?.()
  })

  // `Paused` counts as neither active nor finished — it has its own section
  // below, and (matching the backend's `clear_finished`, which deliberately
  // keeps Paused entries so `resume` still has something to resume) isn't
  // touched by "Clear finished" either.
  let hasFinished = $derived(entries.some((e) => e.status !== 'Downloading' && e.status !== 'Queued' && e.status !== 'Paused'))
  let hasFailed = $derived(entries.some((e) => e.status === 'Failed'))
  let active = $derived(entries.filter((e) => e.status === 'Downloading' || e.status === 'Queued'))
  let paused = $derived(entries.filter((e) => e.status === 'Paused'))
  let finished = $derived(entries.filter((e) => e.status !== 'Downloading' && e.status !== 'Queued' && e.status !== 'Paused'))
</script>

<PanelHeader title={t('panel.downloads')} {onBack}>
  {#snippet extraActions()}
    {#if hasFailed}
      <button
        type="button"
        class="panel-icon-btn {retrying ? 'spinning' : ''}"
        data-tooltip={t('tt.retryAllFailed')}
        aria-label={t('tt.retryAllFailed')}
        disabled={retrying}
        onclick={retryAllFailed}
      >
        <i class="fa-solid fa-rotate-right"></i>
      </button>
    {/if}
    {#if hasFinished}
      <button type="button" class="panel-icon-btn" data-tooltip={t('tt.clearFinished')} aria-label={t('tt.clearFinished')} onclick={clearFinished}>
        <i class="fa-solid fa-broom"></i>
      </button>
    {/if}
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if !loaded}
    <ListSkeleton rows={4} />
  {:else if entries.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-download"></i>
      <span>{t('empty.noDownloadsYet')}</span>
      <small>{t('empty.versionsModsPacksYouDownloadThis')}</small>
    </div>
  {:else}
    {#if active.length > 0}
      <span class="panel-section-label"><i class="fa-solid fa-arrow-down"></i> {t('label.active')} ({active.length})</span>
      <div class="client-card-cols">
      {#each active as entry (entry.id)}
        <div class="client-card">
          <div class="client-card-header">
            <div class="client-card-icon dl-icon-accent">
              <i class={statusIcon(entry.status)}></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name dl-name" title={entry.label}>{entry.label}</span>
              <span class="client-card-sub">
                {statusLabel(entry.status)}
                {#if entry.status === 'Downloading'}
                  · {(entry.progress * 100).toFixed(0)}%
                  {#if entry.total_bytes > 0}
                    · {formatSize(entry.downloaded_bytes)} / {formatSize(entry.total_bytes)}
                  {/if}
                {/if}
              </span>
            </div>
            <div class="client-card-actions">
              {#if entry.status === 'Downloading'}
                <IconBtn ghost icon="fa-solid fa-pause" title={t('tt.pause')} onClick={() => pauseOne(entry)} />
              {/if}
              <IconBtn ghost icon="fa-solid fa-xmark" title={t('common.cancel')} onClick={() => cancel(entry)} />
            </div>
          </div>
          {#if entry.status === 'Downloading'}
            <div class="progress-bar-wrap">
              <div class="progress-bar-fill" style="width:{(entry.progress * 100).toFixed(0)}%"></div>
            </div>
          {/if}
        </div>
      {/each}
      </div>
    {/if}

    {#if paused.length > 0}
      <span class="panel-section-label"><i class="fa-solid fa-pause"></i> {t('label.paused')} ({paused.length})</span>
      <div class="client-card-cols">
      {#each paused as entry (entry.id)}
        <div class="client-card">
          <div class="client-card-header">
            <div class="client-card-icon dl-icon-accent">
              <i class={statusIcon(entry.status)}></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name dl-name" title={entry.label}>{entry.label}</span>
              <span class="client-card-sub">
                {t('label.paused')}
                {#if entry.total_bytes > 0}
                  · {formatSize(entry.downloaded_bytes)} / {formatSize(entry.total_bytes)}
                {/if}
              </span>
            </div>
            <div class="client-card-actions">
              <IconBtn ghost icon="fa-solid fa-play" title={t('tt.resume')} onClick={() => resumeOne(entry)} />
              <IconBtn ghost icon="fa-solid fa-xmark" title={t('common.cancel')} onClick={() => cancel(entry)} />
            </div>
          </div>
          {#if entry.total_bytes > 0}
            <div class="progress-bar-wrap">
              <div class="progress-bar-fill" style="width:{(entry.progress * 100).toFixed(0)}%"></div>
            </div>
          {/if}
        </div>
      {/each}
      </div>
    {/if}

    {#if finished.length > 0}
      <span class="panel-section-label"><i class="fa-solid fa-check"></i> {t('label.finished')} ({finished.length})</span>
      <div class="client-card-cols">
      {#each finished as entry (entry.id)}
        <div class="client-card {entry.status === 'Failed' ? 'error-row' : ''}">
          <div class="client-card-header">
            <div class="client-card-icon {entry.status === 'Failed' ? 'dl-icon-fail' : 'dl-icon-accent'}">
              <i class={statusIcon(entry.status)}></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name dl-name" title={entry.label}>{entry.label}</span>
              <span class="client-card-sub">{statusLabel(entry.status)}</span>
            </div>
            <div class="client-card-actions">
              {#if entry.status === 'Failed'}
                <IconBtn ghost icon="fa-solid fa-rotate-right" title={t('common.retry')} onClick={() => retryOne(entry)} />
              {/if}
              <IconBtn ghost icon="fa-solid fa-trash" title={t('tt.remove')} onClick={() => remove(entry)} />
            </div>
          </div>
          {#if entry.status === 'Failed' && entry.error}
            <p class="client-card-desc error-text">{entry.error}</p>
          {/if}
        </div>
      {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .dl-icon-accent {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .dl-icon-fail {
    background: color-mix(in srgb, var(--red) 12%, transparent);
    color: var(--red);
  }
  .dl-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
