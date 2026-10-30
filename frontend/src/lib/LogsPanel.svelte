<script lang="ts">
  import { t } from './i18nStore.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { onDestroy } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'

  // Ported from Components/LogsPanel.razor + Services/LogService.cs.
  // `instanceId` (Phase 4, optional) enables the "Live" tail toggle below —
  // without it (e.g. the flat Versions-tab log view) the panel is
  // file-browsing only, same as before.
  let {
    instanceDir,
    instanceId,
    geminiApiKey = '',
    onBack,
    embedded = false,
  }: {
    instanceDir: string
    instanceId?: string
    geminiApiKey?: string
    onBack?: () => void
    embedded?: boolean
  } = $props()

  type LogFile = { name: string; path: string; size: number; modified_at: string; is_crash: boolean }

  let files = $state<LogFile[]>([])
  let loading = $state(true)
  let error = $state('')

  let openFile = $state<LogFile | null>(null)
  let openText = $state('')
  let openLoading = $state(false)
  let shareUrl = $state('')
  let sharing = $state(false)
  let aiExplanation = $state('')
  let aiExplaining = $state(false)
  let aiError = $state('')

  // QOL: "Export logs for support" - one zip with launcher version, redacted
  // settings, and the newest logs, instead of a bug report needing three
  // separate things gathered by hand.
  let exportingSupport = $state(false)
  let exportedSupportPath = $state('')
  async function exportSupportBundle() {
    exportingSupport = true
    exportedSupportPath = ''
    try {
      const path = await invoke<string>('export_support_bundle', { instanceDir, maxLogs: 5 })
      exportedSupportPath = path
      await invoke('open_folder', { path: path.replace(/[\\/][^\\/]*$/, '') })
    } catch (e) {
      error = String(e)
    } finally {
      exportingSupport = false
    }
  }

  /** Maps a log line to a token-coloured level class (color + word, no icon). */
  function lineLevel(line: string): string {
    if (/\b(FATAL|ERROR)\b|Exception|Caused by:/.test(line)) return 'lvl-error'
    if (/\bWARN(ING)?\b/.test(line)) return 'lvl-warn'
    if (/\b(DEBUG|TRACE)\b/.test(line)) return 'lvl-debug'
    return ''
  }

  const openLines = $derived(openText ? openText.split('\n') : [])

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
    aiExplanation = ''
    aiError = ''
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

  async function explainWithAi() {
    if (!openText || aiExplaining) return
    aiExplaining = true
    aiError = ''
    aiExplanation = ''
    try {
      aiExplanation = await invoke<string>('explain_crash_log', {
        apiKey: geminiApiKey,
        logText: openText,
      })
    } catch (e) {
      aiError = String(e)
    } finally {
      aiExplaining = false
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

  // Incoming lines are buffered and flushed at most once per animation frame,
  // so a chatty game process doesn't trigger one array copy + full re-render
  // per line on the weak N5100 target.
  let pendingLines: string[] = []
  let flushHandle: number | undefined
  function flushPendingLines() {
    flushHandle = undefined
    if (pendingLines.length === 0) return
    const batch = pendingLines
    pendingLines = []
    let next = liveLines.concat(batch)
    if (next.length > LIVE_TAIL_MAX_LINES) next = next.slice(next.length - LIVE_TAIL_MAX_LINES)
    liveLines = next
  }
  function cancelPendingFlush() {
    if (flushHandle !== undefined) cancelAnimationFrame(flushHandle)
    flushHandle = undefined
    pendingLines = []
  }

  // Bumped on every toggle and on unmount so an async start that was
  // superseded (second click, or panel closed) releases its listener instead
  // of leaking a handler that keeps pushing lines.
  let tailGen = 0
  let logsDisposed = false

  async function toggleLiveTail() {
    tailGen++
    if (liveTailing) {
      liveTailing = false
      unlistenLogLine?.()
      unlistenLogLine = undefined
      cancelPendingFlush()
      return
    }
    if (!instanceId) return
    const gen = tailGen
    liveLines = []
    try {
      const buffer = await invoke<string[]>('get_instance_log_buffer', { instanceId })
      if (gen !== tailGen) return
      liveLines = buffer
    } catch {
      // best-effort — start tailing from empty if the buffer read fails
    }
    const un = await listen<JavaLogLine>('java-log-line', (evt) => {
      if (evt.payload.instance_id !== instanceId) return
      pendingLines.push(evt.payload.line)
      if (flushHandle === undefined) flushHandle = requestAnimationFrame(flushPendingLines)
    })
    if (gen !== tailGen || logsDisposed) {
      un()
      return
    }
    unlistenLogLine = un
    liveTailing = true
  }

  onDestroy(() => {
    logsDisposed = true
    tailGen++
    unlistenLogLine?.()
    cancelPendingFlush()
  })
</script>

{#if !embedded}
  <style>
  .logs-view {
    font-family: ui-monospace, Consolas, 'Cascadia Mono', monospace;
    font-size: 11.5px;
    line-height: 1.6;
    color: var(--text);
    background: var(--bg-inset);
    border-radius: var(--r-sm);
    padding: 10px 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 60vh;
    overflow: auto;
  }
  .lvl-error { color: var(--red); }
  .lvl-warn { color: var(--orange); }
  .lvl-debug { color: var(--text-dim); }
  .logs-inline-error {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 6px 0;
  }
  .logs-ai-notice {
    align-items: flex-start;
    white-space: pre-wrap;
  }
  .logs-ai-notice > i {
    margin-top: 2px;
    flex-shrink: 0;
  }
</style>

<PanelHeader title={t('panel.logs')} {onBack}>
    {#snippet extraActions()}
      {#if instanceId}
        <button
          type="button"
          class="panel-icon-btn {liveTailing ? 'active' : ''}"
          onclick={toggleLiveTail}
          data-tooltip={liveTailing ? 'Stop live tail' : 'Live tail this instance'}
          aria-label={t('tt.liveTail')}
        >
          <i class="fa-solid {liveTailing ? 'fa-satellite-dish' : 'fa-tower-broadcast'}"></i>
        </button>
      {/if}
      <button type="button" class="panel-icon-btn {loading ? 'spinning' : ''}" onclick={load} data-tooltip={t('tt.refresh')} aria-label={t('tt.refresh')}>
        <i class="fa-solid fa-rotate"></i>
      </button>
      <button
        type="button"
        class="panel-icon-btn {exportingSupport ? 'spinning' : ''}"
        onclick={exportSupportBundle}
        disabled={exportingSupport}
        data-tooltip={t('tt.exportLogsSupport')}
        aria-label={t('tt.exportLogsSupport')}
      >
        <i class="fa-solid fa-file-zipper"></i>
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
        aria-label={t('tt.liveTail')}
      >
        <i class="fa-solid {liveTailing ? 'fa-satellite-dish' : 'fa-tower-broadcast'}"></i>
      </button>
    {/if}
    <button type="button" class="panel-icon-btn {loading ? 'spinning' : ''}" onclick={load} data-tooltip={t('tt.refresh')} aria-label={t('tt.refresh')}>
      <i class="fa-solid fa-rotate"></i>
    </button>
    <button
      type="button"
      class="panel-icon-btn {exportingSupport ? 'spinning' : ''}"
      onclick={exportSupportBundle}
      disabled={exportingSupport}
      data-tooltip={t('tt.exportLogsSupport')}
      aria-label={t('tt.exportLogsSupport')}
    >
      <i class="fa-solid fa-file-zipper"></i>
    </button>
  </div>
{/if}

<div class="panel-body">
  {#if liveTailing}
    <div class="logs-toolbar">
      <span class="logs-open-name"><i class="fa-solid fa-satellite-dish"></i> Live: {instanceId}</span>
    </div>
    <div class="logs-view">{#each liveLines as ln, i (i)}<span class="log-line {lineLevel(ln)}">{ln}</span>{'\n'}{/each}</div>
  {:else if loading}
    <ListSkeleton rows={4} />
  {:else if error}
    <span class="error-text">{error}</span>
  {:else if files.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-file-lines"></i>
      <span>{t('empty.noLogsFound')}</span>
      <small>{t('empty.playSessionOrTriggerCrashGenerate')}</small>
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
        {#if openFile.is_crash}
          <button
            type="button"
            class="btn-sm"
            disabled={aiExplaining || openLoading || !geminiApiKey.trim()}
            onclick={explainWithAi}
            data-tooltip={geminiApiKey.trim()
              ? undefined
              : 'Add your own Gemini API key in Settings → System → AI to use this'}
          >
            {#if aiExplaining}
              <Spinner size={12} />
            {:else}
              <i class="fa-solid fa-wand-magic-sparkles"></i>
            {/if}
            Explain with AI
          </button>
        {/if}
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
      {#if aiError}
        <span class="error-text logs-inline-error"><i class="fa-solid fa-triangle-exclamation"></i> {aiError}</span>
      {/if}
      {#if aiExplanation}
        <div class="cache-notice logs-ai-notice">
          <i class="fa-solid fa-wand-magic-sparkles"></i>
          <span>{aiExplanation}</span>
        </div>
      {/if}
      {#if openLoading}
        <ListSkeleton rows={2} />
      {:else}
        <div class="logs-view">{#each openLines as ln, i (i)}<span class="log-line {lineLevel(ln)}">{ln}</span>{'\n'}{/each}</div>
      {/if}
    {/if}
  {/if}
</div>
