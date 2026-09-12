<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { save } from '@tauri-apps/plugin-dialog'

  type LineKind = 'stdout' | 'stderr' | 'info' | 'error'
  type Status = 'starting' | 'running' | 'exited' | 'failed'

  interface Line {
    ts: string
    text: string
    kind: LineKind
  }

  let lines = $state<Line[]>([])
  let status = $state<Status>('starting')
  let statusMessage = $state('Starting…')
  let pid = $state<number | null>(null)
  let title = $state('Minecraft')

  let autoScroll = $state(true)
  let wrap = $state(false)
  let autoClose = $state(false)
  let autoCloseSecondsLeft = $state<number | null>(null)
  let footerNote = $state('')
  let sharing = $state(false)

  let logEl: HTMLDivElement

  const win = getCurrentWindow()
  invoke<string>('console_content_title').then((t) => {
    if (t) title = t
  })
  win.listen<string>('game-console://title', (e) => (title = e.payload))

  const STATUS_DOT: Record<Status, string> = {
    starting: '#faa61a',
    running: '#43b581',
    exited: '#6b7280',
    failed: '#f04747',
  }

  let autoCloseTimer: ReturnType<typeof setInterval> | null = null

  function cancelAutoClose() {
    if (autoCloseTimer !== null) {
      clearInterval(autoCloseTimer)
      autoCloseTimer = null
    }
    autoCloseSecondsLeft = null
  }

  function startAutoClose(seconds: number) {
    cancelAutoClose()
    autoCloseSecondsLeft = seconds
    autoCloseTimer = setInterval(() => {
      if (autoCloseSecondsLeft === null) return
      autoCloseSecondsLeft -= 1
      if (autoCloseSecondsLeft <= 0) {
        cancelAutoClose()
        win.close()
      }
    }, 1000)
  }

  function onAnyClick() {
    if (autoCloseTimer !== null) {
      cancelAutoClose()
      footerNote = 'auto-close cancelled'
    }
  }

  function append(text: string, kind: LineKind) {
    lines.push({ ts: new Date().toLocaleTimeString('en-US', { hour12: false }), text, kind })
    if (lines.length > 5000) lines.splice(0, 1000)
    if (autoScroll) queueMicrotask(() => logEl?.scrollTo({ top: logEl.scrollHeight }))
  }

  win.listen<{ stream: LineKind; line: string }>('game-console://log', (e) => {
    append(e.payload.line, e.payload.stream)
  })

  win.listen<{ status: Status; pid: number | null; message: string | null }>(
    'game-console://status',
    (e) => {
      status = e.payload.status
      pid = e.payload.pid
      if (status === 'starting') statusMessage = 'Starting…'
      else if (status === 'running') {
        statusMessage = 'Running'
        cancelAutoClose()
      } else if (status === 'failed') {
        statusMessage = 'Failed: ' + (e.payload.message ?? 'unknown error')
        append(statusMessage, 'error')
        if (autoClose) startAutoClose(8)
      }
    },
  )

  win.listen<{ code: number | null }>('game-console://exit', (e) => {
    status = 'exited'
    statusMessage = e.payload.code === null ? 'Exited' : `Exited · code ${e.payload.code}`
    append(statusMessage, 'info')
    if (autoClose) startAutoClose(e.payload.code === 0 ? 3 : 8)
  })

  function toggleAutoScroll() {
    autoScroll = !autoScroll
    if (autoScroll) logEl?.scrollTo({ top: logEl.scrollHeight })
  }

  function clearLog() {
    lines = []
    footerNote = ''
  }

  async function copyLog() {
    const text = lines.map((l) => `[${l.ts}] ${l.text}`).join('\n')
    await navigator.clipboard.writeText(text)
    footerNote = `Copied ${lines.length} line(s)`
  }

  async function saveLog() {
    const path = await save({
      defaultPath: `minecraft-${new Date().toISOString().replace(/[:.]/g, '-')}.log`,
      filters: [{ name: 'Log files', extensions: ['log', 'txt'] }],
    })
    if (!path) return
    const text = lines.map((l) => `[${l.ts}] ${l.text}`).join('\n')
    try {
      await invoke('save_console_log', { path, content: text })
      footerNote = 'Saved ' + path.split(/[\\/]/).pop()
    } catch (e) {
      append('Failed to save log: ' + String(e), 'error')
    }
  }

  async function shareLog() {
    const text = lines.map((l) => `[${l.ts}] ${l.text}`).join('\n')
    if (!text.trim()) {
      footerNote = 'Nothing to upload — log is empty.'
      return
    }
    sharing = true
    try {
      const url = await invoke<string>('upload_console_log_to_mclogs', { content: text })
      await navigator.clipboard.writeText(url)
      append(`Log uploaded → ${url}  (copied to clipboard)`, 'info')
      footerNote = 'mclo.gs link copied!'
    } catch (e) {
      append('mclo.gs upload failed: ' + String(e), 'error')
    } finally {
      sharing = false
    }
  }

  function toggleAutoClose() {
    autoClose = !autoClose
    if (!autoClose) cancelAutoClose()
  }

  let confirmingStop = $state(false)

  function requestStopGame() {
    confirmingStop = true
  }

  function cancelStopGame() {
    confirmingStop = false
  }

  function stopGame() {
    confirmingStop = false
    append('Stop requested — killing process tree…', 'info')
    invoke('console_stop_game')
  }
</script>

<svelte:window onmousedown={onAnyClick} />

<div class="console">
  <header class="console-header">
    <div class="header-left">
      <span class="status-dot" style="background:{STATUS_DOT[status]}"></span>
      <div class="header-labels">
        <div class="title">{title}</div>
        <div class="status">{statusMessage}</div>
      </div>
    </div>
    <div class="pid">{pid !== null ? `PID ${pid}` : ''}</div>
    <div class="header-right">
      <button class="pill" class:active={autoClose} onclick={toggleAutoClose} title="Close this window automatically when the game exits">Auto-close</button>
      <button class="pill" class:active={autoScroll} onclick={toggleAutoScroll}>Auto-scroll</button>
      <button class="pill" class:active={wrap} onclick={() => (wrap = !wrap)}>Wrap</button>
      <button class="pill" onclick={clearLog}>Clear</button>
      <button class="pill" onclick={copyLog}>Copy</button>
      <button class="pill" onclick={saveLog}>Save</button>
      <button class="pill" onclick={shareLog} disabled={sharing} title="Upload log to mclo.gs and copy the link">
        {sharing ? 'Uploading…' : 'mclo.gs'}
      </button>
      {#if confirmingStop}
        <span class="stop-confirm-label">Stop the running game?</span>
        <button class="pill danger" onclick={stopGame}>Confirm</button>
        <button class="pill" onclick={cancelStopGame}>Cancel</button>
      {:else}
        <button class="pill danger" onclick={requestStopGame}>Stop</button>
      {/if}
    </div>
  </header>

  <div class="log" class:wrap bind:this={logEl}>
    {#each lines as line}
      <div class="line line-{line.kind}"><span class="ts">[{line.ts}]</span> {line.text}</div>
    {/each}
  </div>

  <footer class="console-footer">
    <span>
      {#if autoCloseSecondsLeft !== null}
        Closing in {autoCloseSecondsLeft}s · click anywhere to keep open
      {:else if footerNote}
        {footerNote}
      {:else}
        {lines.length} {lines.length === 1 ? 'line' : 'lines'}
      {/if}
    </span>
    <span class="brand">GLACIER LAUNCHER · GAME CONSOLE</span>
  </footer>
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    background: #23272a;
  }

  .console {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #23272a;
    color: #fff;
    font-family:
      'Segoe UI',
      -apple-system,
      sans-serif;
    user-select: none;
  }

  .console-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    background: #2c2f33;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .status-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
  }

  .status {
    font-size: 11px;
    color: #99aab5;
    margin-top: 2px;
  }

  .pid {
    flex: 1;
    text-align: right;
    font-family: Consolas, 'Cascadia Mono', monospace;
    font-size: 11px;
    color: #6b7280;
    margin-right: 4px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .pill {
    background: rgba(255, 255, 255, 0.04);
    color: #99aab5;
    border: none;
    border-radius: 8px;
    padding: 5px 11px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    font-family: inherit;
    transition: background-color 0.1s ease;
  }

  .pill:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.075);
    color: #fff;
  }

  .pill.active {
    background: #7289da;
    color: #fff;
  }

  .pill.danger {
    background: rgba(240, 71, 71, 0.13);
    color: #fecaca;
  }

  .pill.danger:hover:not(:disabled) {
    background: rgba(240, 71, 71, 0.22);
  }

  .pill:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .stop-confirm-label {
    font-size: 11px;
    color: #fecaca;
    margin-right: 2px;
    white-space: nowrap;
  }

  .log {
    flex: 1;
    overflow: auto;
    padding: 14px 14px;
    font-family: Consolas, 'Cascadia Mono', 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre;
  }

  .log.wrap {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .line {
    color: #cbd5e1;
  }

  .line-stderr {
    color: #fba1a1;
  }

  .line-info {
    color: #86efac;
  }

  .line-error {
    color: #ff6b6b;
  }

  .ts {
    color: #6b7280;
  }

  .console-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 7px 14px;
    background: #2c2f33;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 11px;
    color: #6b7280;
    flex-shrink: 0;
  }

  .brand {
    font-size: 9px;
    opacity: 0.55;
  }
</style>
