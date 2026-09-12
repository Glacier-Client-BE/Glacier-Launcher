<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { onDestroy } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'

  // Ported from Components/LogsPanel.razor + Services/LogService.cs.
  // `instanceId` (Phase 4, optional) enables the "Live" tail toggle below —
  // without it (e.g. the flat Versions-tab log view) the panel is
  // file-browsing only, same as before.
  let {
    instanceDir,
    instanceId,
    onBack,
    embedded = false,
  }: { instanceDir: string; instanceId?: string; onBack?: () => void; embedded?: boolean } = $props()

  type LogFile = { name: string; path: string; size: number; modified_at: string; is_crash: boolean }

  let files = $state<LogFile[]>([])
  let loading = $state(true)
  let error = $state('')

  let openFile = $state<LogFile | null>(null)
  let openText = $state('')
  let openLoading = $state(false)
  let shareUrl = $state('')
  let sharing = $state(false)

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  async function load() {
    loading = true
    error = ''
    try {
      files = await invoke<LogFile[]>('list_logs', { instanceDir })
    } catch (e) {
      error = String(e)
    } finally {
      loading = false
    }
  }

  async function openLog(f: LogFile) {
    openFile = f
    openText = ''
    shareUrl = ''
    openLoading = true
    try {
      openText = await invoke<string>('read_log', { path: f.path })
      // `list_logs` only flags a crash by directory (crash-reports/); a
      // regular .log can still contain an uncaught exception, so re-check
      // the actual content once it's loaded and upgrade the row's icon.
      if (!f.is_crash && (await invoke<boolean>('log_looks_like_crash', { text: openText }))) {
        f.is_crash = true
        files = files
      }
    } catch (e) {
      openText = `Could not read log: ${e}`
    } finally {
      openLoading = false
    }
  }

  async function share() {
    if (!openText) return
    sharing = true
    shareUrl = ''
    try {
      shareUrl = await invoke<string>('share_log', { content: openText })
    } catch (e) {
      error = String(e)
    } finally {
      sharing = false
    }
  }

  load()

  // --- Phase 4: live-tail mode ----------------------------------------------
  // Subscribes to `java-log-line` (emitted by the Rust launch-streaming loop,
  // see `commands::launch::stream_child_output`) filtered to `instanceId`,
  // appending lines to a view instead of a static file read. Independent of
  // the file browser above — toggling it doesn't affect `openFile`.
  type JavaLogLine = { instance_id: string; line: string; level: string }
  let liveTailing = $state(false)
  let liveLines = $state<string[]>([])
  const LIVE_TAIL_MAX_LINES = 5000 // mirrors the Rust ring-buffer cap
  let unlistenLogLine: (() => void) | undefined

  async function toggleLiveTail() {
    if (liveTailing) {
      liveTailing = false
      unlistenLogLine?.()
      unlistenLogLine = undefined
      return
    }
    if (!instanceId) return
    liveLines = []
    try {
      liveLines = await invoke<string[]>('get_instance_log_buffer', { instanceId })
    } catch {
      // best-effort — start tailing from empty if the buffer read fails
    }
    liveTailing = true
    unlistenLogLine = await listen<JavaLogLine>('java-log-line', (evt) => {
      if (evt.payload.instance_id !== instanceId) return
      liveLines = [...liveLines, evt.payload.line]
      if (liveLines.length > LIVE_TAIL_MAX_LINES) {
        liveLines = liveLines.slice(liveLines.length - LIVE_TAIL_MAX_LINES)
      }
    })
  }

  onDestroy(() => {
    unlistenLogLine?.()
  })
</script>

{#if !embedded}
  <PanelHeader title="Logs" {onBack}>
    {#snippet extraActions()}
      {#if instanceId}
        <button
          type="button"
          class="panel-icon-btn {liveTailing ? 'active' : ''}"
          onclick={toggleLiveTail}
          data-tooltip={liveTailing ? 'Stop live tail' : 'Live tail this instance'}
          aria-label="Live tail"
        >
          <i class="fa-solid {liveTailing ? 'fa-satellite-dish' : 'fa-tower-broadcast'}"></i>
        </button>
      {/if}
      <button type="button" class="panel-icon-btn {loading ? 'spinning' : ''}" onclick={load} data-tooltip="Refresh" aria-label="Refresh">
        <i class="fa-solid fa-rotate"></i>
      </button>
    {/snippet}
  </PanelHeader>
{:else}
  <div class="panel-body-inline-actions">
    {#if instanceId}
      <button
        type="button"
        class="panel-icon-btn {liveTailing ? 'active' : ''}"
        onclick={toggleLiveTail}
        data-tooltip={liveTailing ? 'Stop live tail' : 'Live tail this instance'}
        aria-label="Live tail"
      >
        <i class="fa-solid {liveTailing ? 'fa-satellite-dish' : 'fa-tower-broadcast'}"></i>
      </button>
    {/if}
    <button type="button" class="panel-icon-btn {loading ? 'spinning' : ''}" onclick={load} data-tooltip="Refresh" aria-label="Refresh">
      <i class="fa-solid fa-rotate"></i>
    </button>
  </div>
{/if}

<div class="panel-body">
  {#if liveTailing}
    <div class="logs-toolbar">
      <span class="logs-open-name"><i class="fa-solid fa-satellite-dish"></i> Live: {instanceId}</span>
    </div>
    <div class="logs-view">{liveLines.join('\n')}</div>
  {:else if loading}
    <div class="versions-loading"><Spinner size={16} /><span>Scanning logs…</span></div>
  {:else if error}
    <span class="error-text">{error}</span>
  {:else if files.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-file-lines"></i>
      <span>No logs found</span>
      <small>Play a session or trigger a crash to generate log files.</small>
    </div>
  {:else}
    <div class="logs-file-list">
      {#each files as f (f.path)}
        <button type="button" class="logs-file {f.is_crash ? 'crash' : ''} {openFile?.path === f.path ? 'active' : ''}" onclick={() => openLog(f)}>
          <i class={f.is_crash ? 'fa-solid fa-skull-crossbones' : 'fa-solid fa-file-lines'}></i>
          <span class="logs-file-name">{f.name}</span>
          <span class="logs-file-size">{fmtSize(f.size)}</span>
        </button>
      {/each}
    </div>

    {#if openFile}
      <div class="logs-toolbar">
        <span class="logs-open-name">{openFile.name}</span>
        <button type="button" class="btn-sm" disabled={sharing || openLoading} onclick={share}>
          {#if sharing}
            <Spinner size={12} />
          {:else}
            <i class="fa-solid fa-share-nodes"></i>
          {/if}
          Share
        </button>
      </div>
      {#if shareUrl}
        <div class="logs-share-url">
          <i class="fa-solid fa-link"></i>
          <a href={shareUrl} target="_blank" rel="noreferrer">{shareUrl}</a>
        </div>
      {/if}
      {#if openLoading}
        <div class="versions-loading"><Spinner size={16} /><span>Reading…</span></div>
      {:else}
        <div class="logs-view">{openText}</div>
      {/if}
    {/if}
  {/if}
</div>
