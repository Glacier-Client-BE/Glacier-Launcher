<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'

  // Ported from Components/LogsPanel.razor + Services/LogService.cs.
  let { instanceDir, onBack }: { instanceDir: string; onBack?: () => void } = $props()

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
</script>

<PanelHeader title="Logs" {onBack}>
  {#snippet extraActions()}
    <button type="button" class="panel-icon-btn {loading ? 'spinning' : ''}" onclick={load} data-tooltip="Refresh" aria-label="Refresh">
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if loading}
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
