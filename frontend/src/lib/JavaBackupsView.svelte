<script lang="ts">
  /**
   * Port of `Components/BackupsPanel.razor` (140 lines) — the "World Backups"
   * panel, reached from `currentView == "backups"` (Home.razor:1008).
   *
   * This panel had **no counterpart at all** in the rewrite: no view, no route,
   * and no backend. `JavaInstanceService`'s worlds/backups half was never
   * ported either, so it is now `gl_java::worlds` behind the
   * `list_java_worlds` / `list_java_backups` / `backup_java_world` /
   * `restore_java_backup` / `delete_java_backup` commands.
   */
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'
  import ViewModeToggle from './components/ViewModeToggle.svelte'
  import { loadViewMode, saveViewMode, type ViewMode } from './viewMode'

  let {
    instanceName,
    onBack,
    onToast,
    embedded = false,
  }: {
    /** `Instances.ActiveInstance.Name`, used only in the section label. */
    instanceName: string
    onBack?: () => void
    onToast?: (message: string, kind?: string) => void
    /** Phase 8: hosted inside `InstanceDetailView`'s Worlds tab — suppress
     * the standalone PanelHeader/back-button. */
    embedded?: boolean
  } = $props()

  type JavaWorld = { name: string; path: string; size: number; modified_at: string }
  type BackupFile = { name: string; path: string; size_bytes: number; modified_at: string }

  let worlds = $state<JavaWorld[]>([])
  let backups = $state<BackupFile[]>([])
  /** Port of `_confirmRestore` — the path awaiting a second click, or null. */
  let confirmRestore = $state<string | null>(null)
  let busy = $state(false)
  let isLoading = $state(true)
  let backupsViewMode = $state<ViewMode>(loadViewMode('java-backups'))
  function setBackupsViewMode(m: ViewMode) {
    backupsViewMode = m
    saveViewMode('java-backups', m)
  }

  /** Port of `FormatSize` — note this one is `0.#` (ONE decimal), not the
   *  `0.##` used elsewhere in the app. Kept as the original has it. */
  function formatSize(bytes: number): string {
    if (!bytes || bytes <= 0) return '0 B'
    const u = ['B', 'KB', 'MB', 'GB']
    let v = bytes
    let i = 0
    while (v >= 1024 && i < u.length - 1) {
      v /= 1024
      i++
    }
    return `${Math.round(v * 10) / 10} ${u[i]}`
  }

  /** Port of `Reload` — reloads both lists and drops any pending confirm. */
  async function reload() {
    try {
      worlds = await invoke<JavaWorld[]>('list_java_worlds')
      backups = await invoke<BackupFile[]>('list_java_backups')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      confirmRestore = null
      isLoading = false
    }
  }

  /** Port of `BackupWorld`. */
  async function backupWorld(worldDir: string) {
    busy = true
    try {
      await invoke('backup_java_world', { worldDir })
      await reload()
      onToast?.('World backed up.', 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }

  /** Port of `RestoreBackup` — clears the confirm before starting, as the
   *  original does, so a second click can't re-enter. */
  async function restoreBackup(zipPath: string) {
    busy = true
    confirmRestore = null
    try {
      await invoke('restore_java_backup', { zipPath })
      await reload()
      onToast?.('Backup restored.', 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }

  /** Port of `Delete` — the C# never surfaces a failure here. */
  async function deleteBackup(zipPath: string) {
    await invoke('delete_java_backup', { zipPath })
    await reload()
    onToast?.('Backup deleted.', 'info')
  }

  reload()
</script>

{#if !embedded}
  <PanelHeader title="World Backups" {onBack}>
    {#snippet extraActions()}
      <button
        type="button"
        class="panel-icon-btn {isLoading ? 'spinning' : ''}"
        disabled={isLoading}
        onclick={reload}
        onmousedown={(e) => e.stopPropagation()}
        data-tooltip="Refresh"
        aria-label="Refresh"
      >
        <i class="fa-solid fa-rotate"></i>
      </button>
      <ViewModeToggle mode={backupsViewMode} onChange={setBackupsViewMode} />
    {/snippet}
  </PanelHeader>
{:else}
  <div class="panel-body-inline-actions">
    <button
      type="button"
      class="panel-icon-btn {isLoading ? 'spinning' : ''}"
      disabled={isLoading}
      onclick={reload}
      data-tooltip="Refresh"
      aria-label="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <ViewModeToggle mode={backupsViewMode} onChange={setBackupsViewMode} />
  </div>
{/if}

<div class="panel-body">
  {#if isLoading}
    <div class="versions-loading"><Spinner size={18} /><span>Loading worlds and backups...</span></div>
  {:else}
  <span class="panel-section-label">Worlds ({instanceName})</span>
  {#if worlds.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-globe"></i>
      <span>No worlds in this instance's saves folder yet.</span>
    </div>
  {:else}
    <div class="version-list {backupsViewMode === 'grid' ? 'grid-view' : ''}">
    {#each worlds as w (w.path)}
      <div class="version-row">
        <div class="version-meta">
          <span class="version-name">{w.name}</span>
          <span class="version-sub">{formatSize(w.size)}</span>
        </div>
        <button
          class="vcs-btn"
          onclick={() => backupWorld(w.path)}
          disabled={busy}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-box-archive"></i>&nbsp;Back up
        </button>
      </div>
    {/each}
    </div>
  {/if}

  <span class="panel-section-label">Backups</span>
  {#if backups.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-clock-rotate-left"></i>
      <span>No backups yet. Backing up a world zips it into the instance's backups folder.</span>
    </div>
  {:else}
    <div class="version-list {backupsViewMode === 'grid' ? 'grid-view' : ''}">
    {#each backups as b (b.path)}
      <div class="version-row">
        <div class="version-meta">
          <span class="version-name">{b.name}</span>
          <span class="version-sub">{formatSize(b.size_bytes)}</span>
        </div>
        {#if confirmRestore === b.path}
          <button
            class="vcs-btn"
            onclick={() => restoreBackup(b.path)}
            disabled={busy}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-check"></i>&nbsp;Confirm restore
          </button>
          <button
            class="vcs-btn vcs-btn-ghost"
            onclick={() => (confirmRestore = null)}
            onmousedown={(e) => e.stopPropagation()}>Cancel</button
          >
        {:else}
          <button
            class="vcs-btn vcs-btn-ghost"
            onclick={() => (confirmRestore = b.path)}
            onmousedown={(e) => e.stopPropagation()}
            data-tooltip="Replaces the current world with this backup"
          >
            <i class="fa-solid fa-clock-rotate-left"></i>&nbsp;Restore
          </button>
          <button
            class="vcs-btn vcs-btn-ghost"
            onclick={() => deleteBackup(b.path)}
            onmousedown={(e) => e.stopPropagation()}
            aria-label="Delete backup"
          >
            <i class="fa-solid fa-trash"></i>
          </button>
        {/if}
      </div>
    {/each}
    </div>
  {/if}
  {/if}
</div>
