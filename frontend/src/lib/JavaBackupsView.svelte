<script lang="ts">
  import { t } from './i18nStore.svelte'
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
  import ViewModeToggle from './components/ViewModeToggle.svelte'
  import { loadViewMode, saveViewMode, type ViewMode } from './viewMode'
  import ListSkeleton from './components/ListSkeleton.svelte'

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
      onToast?.(t('toast.worldBackedUp'), 'success')
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
      onToast?.(t('toast.backupRestored'), 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }

  /** Port of `Delete`; failures are surfaced as a toast. */
  async function deleteBackup(zipPath: string) {
    try {
      await invoke('delete_java_backup', { zipPath })
      await reload()
      onToast?.(t('toast.backupDeleted'), 'info')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  reload()
</script>

{#if !embedded}
  <PanelHeader title={t('panel.worldBackups')} {onBack}>
    {#snippet extraActions()}
      <button
        type="button"
        class="panel-icon-btn {isLoading ? 'spinning' : ''}"
        disabled={isLoading}
        onclick={reload}
        onmousedown={(e) => e.stopPropagation()}
        data-tooltip={t('tt.refresh')}
        aria-label={t('tt.refresh')}
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
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <ViewModeToggle mode={backupsViewMode} onChange={setBackupsViewMode} />
  </div>
{/if}

<div class="panel-body">
  {#if isLoading}
    <ListSkeleton rows={4} />
  {:else}
  <span class="panel-section-label">Worlds ({instanceName})</span>
  {#if worlds.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-globe"></i>
      <span>{t('empty.noWorldsThisInstanceSSaves')}</span>
    </div>
  {:else}
    <div class="version-list {backupsViewMode === 'grid' ? 'grid-view' : ''}">
    {#each worlds as w (w.path)}
      <div class="version-row">
        <div class="version-meta">
          <span class="version-name" title={w.name}>{w.name}</span>
          <span class="version-sub">{formatSize(w.size)}</span>
        </div>
        <button
          class="btn-sm btn-primary"
          onclick={() => backupWorld(w.path)}
          disabled={busy}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class={busy ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-box-archive'}></i>&nbsp;{busy ? 'Backing up…' : 'Back up'}
        </button>
      </div>
    {/each}
    </div>
  {/if}

  <span class="panel-section-label">{t('label.backups')}</span>
  {#if backups.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-clock-rotate-left"></i>
      <span>{t('empty.noBackupsYetBackingUpWorld')}</span>
    </div>
  {:else}
    <div class="version-list {backupsViewMode === 'grid' ? 'grid-view' : ''}">
    {#each backups as b (b.path)}
      <div class="version-row">
        <div class="version-meta">
          <span class="version-name" title={b.name}>{b.name}</span>
          <span class="version-sub">{formatSize(b.size_bytes)}</span>
        </div>
        {#if confirmRestore === b.path}
          <button
            class="btn-sm btn-primary"
            onclick={() => restoreBackup(b.path)}
            disabled={busy}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class={busy ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-check'}></i>&nbsp;{busy ? 'Restoring…' : 'Restore backup'}
          </button>
          <button
            class="btn-sm"
            onclick={() => (confirmRestore = null)}
            onmousedown={(e) => e.stopPropagation()}>{t('common.cancel')}</button
          >
        {:else}
          <button
            class="btn-sm"
            onclick={() => (confirmRestore = b.path)}
            onmousedown={(e) => e.stopPropagation()}
            data-tooltip={t('tt.replacesCurrentWorldWithThisBackup')}
          >
            <i class="fa-solid fa-clock-rotate-left"></i>&nbsp;Restore
          </button>
          <button
            class="btn-sm"
            onclick={() => deleteBackup(b.path)}
            onmousedown={(e) => e.stopPropagation()}
            aria-label={t('tt.deleteBackup')}
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

<style>
  /* Backups rhythm: section gutter and band gaps between stacked rows,
     row gap inside each row, hover on row buttons, dimmed disabled state. */
  .panel-body {
    gap: var(--panel-gutter);
  }
  .panel-body > .panel-section-label {
    margin-top: 0;
  }
  .panel-body :global(.version-list) {
    gap: var(--panel-band);
  }
  .version-row {
    gap: var(--row-pad-x);
  }
  .version-row :global(button:not(.btn-primary):hover:not(:disabled)) {
    background: var(--bg-item-hover);
  }
  .version-row :global(button:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
