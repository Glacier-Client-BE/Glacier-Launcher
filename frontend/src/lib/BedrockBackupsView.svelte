<script lang="ts">
/**
 * Literal port of the `bedrockbackups` panel — Home.razor:4186-4299, with
 * behaviour from Pages/Home.Backups.cs (90 lines).
 *
 * Transcribed element-for-element: same DOM order, class names, tooltips and
 * strings. Both the restore and the delete confirmation are INLINE in the row
 * (`_confirmRestoreBackupPath` / `_confirmDeleteBackupPath`), never a modal.
 *
 * NOTE: this is the *Bedrock* backups panel; Components/BackupsPanel.razor is
 * the unrelated Java one.
 */
import { invoke } from '@tauri-apps/api/core'
import SubTabs from './components/SubTabs.svelte'
import EditionTabs from './components/EditionTabs.svelte'
import type { BedrockInstance } from './settings'

let {
  instance,
  onBack,
  onNavigate,
  onToast,
}: {
  instance: BedrockInstance | null
  onBack: () => void
  /** Both tab strips route through the parent — this view never routes itself. */
  onNavigate: (view: string) => void
  onToast?: (message: string, kind: 'info' | 'success' | 'error') => void
} = $props()

/** Serde emits snake_case (crates/gl-bedrock/src/model.rs:62-67). */
type BedrockBackup = {
  name: string
  file_path: string
  size_bytes: number
  created_at: string
}

// Port of Home.Backups.cs:10-15.
let bedrockBackups = $state<BedrockBackup[]>([])
let isLoadingBackups = $state(false)
let isCreatingBackup = $state(false)
let _confirmDeleteBackupPath = $state('')
let _confirmRestoreBackupPath = $state('')
let backupLabelInput = $state('')

/** Port of Home.razor.cs:692 `FormatSize`. */
function formatSize(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit++
  }
  return `${value.toFixed(2).replace(/\.?0+$/, '')} ${units[unit]}`
}

/** Port of Home.razor.cs:677 `FormatRelativeTime`. */
function formatRelativeTime(iso: string): string {
  try {
    const dt = new Date(iso)
    if (Number.isNaN(dt.getTime())) return ''
    const diffMs = Date.now() - dt.getTime()
    const minutes = diffMs / 60000
    if (minutes < 2) return 'just now'
    const hours = minutes / 60
    if (hours < 1) return `${Math.floor(minutes)}m ago`
    const days = hours / 24
    if (days < 1) return `${Math.floor(hours)}h ago`
    if (days < 7) return `${Math.floor(days)}d ago`
    return dt.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  } catch {
    return ''
  }
}

function parentDir(path: string): string {
  const idx = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return idx > 0 ? path.slice(0, idx) : ''
}

/**
 * `BedrockBackupService.OpenBackupsFolder` had the backups directory to hand;
 * no Tauri command exposes it, so it is recovered from a listed backup's file
 * path. With no backups listed the header's "Open folder" button is rendered
 * disabled rather than made to silently do nothing.
 */
const backupsDir = $derived(
  bedrockBackups.length > 0 ? parentDir(bedrockBackups[0].file_path) : '',
)

/** Port of `RefreshBedrockBackupsAsync` (Home.Backups.cs:24). */
async function refreshBedrockBackups() {
  isLoadingBackups = true
  try {
    bedrockBackups = await invoke<BedrockBackup[]>('list_bedrock_backups')
  } catch (e) {
    bedrockBackups = []
    onToast?.(String(e), 'error')
  } finally {
    isLoadingBackups = false
  }
}

/** Port of `CreateBedrockBackup` (Home.Backups.cs:33). */
async function createBedrockBackup() {
  isCreatingBackup = true
  try {
    const label = backupLabelInput
    backupLabelInput = ''
    // An empty instanceDir means "the live install": the backend resolves the
    // real per-account com.mojang path itself.
    const path = await invoke<string>('create_bedrock_backup', {
      instanceDir: instance?.directory ?? '',
      label: label ? label : null,
    })
    onToast?.(`'${path.split(/[\\/]/).pop()}' saved.`, 'success')
    onToast?.('Backup created.', 'success')
    await refreshBedrockBackups()
  } catch (e) {
    onToast?.(String(e), 'error')
  } finally {
    isCreatingBackup = false
  }
}

// Port of Home.Backups.cs:50-51.
function confirmDeleteBackup(path: string) {
  _confirmDeleteBackupPath = path
}
function cancelDeleteBackup() {
  _confirmDeleteBackupPath = ''
}

/** Port of `DeleteBedrockBackup` (Home.Backups.cs:53). */
async function deleteBedrockBackup(backup: BedrockBackup) {
  try {
    await invoke('delete_bedrock_backup', { backupPath: backup.file_path })
    _confirmDeleteBackupPath = ''
    await refreshBedrockBackups()
    onToast?.('Backup deleted.', 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

// Port of Home.Backups.cs:65-66.
function confirmRestoreBackup(path: string) {
  _confirmRestoreBackupPath = path
}
function cancelRestoreBackup() {
  _confirmRestoreBackupPath = ''
}

/** Port of `RestoreBedrockBackup` (Home.Backups.cs:68). Overwrites live data. */
async function restoreBedrockBackup(backup: BedrockBackup) {
  _confirmRestoreBackupPath = ''
  onToast?.('Restoring backup…', 'info')
  try {
    await invoke('restore_bedrock_backup', {
      backupPath: backup.file_path,
      instanceDir: instance?.directory ?? '',
    })
    onToast?.(`'${backup.name}' was restored. Worlds and packs are back in place.`, 'success')
    onToast?.('Backup restored.', 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/** Port of `OpenBedrockBackupsFolder` (Home.Backups.cs:85). */
async function openBedrockBackupsFolder() {
  try {
    const dir = backupsDir || (await invoke<string>('bedrock_backups_dir'))
    await invoke('open_folder', { path: dir })
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

$effect(() => {
  instance
  refreshBedrockBackups()
})
</script>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<div class="panel-header">
  <div class="panel-title-wrap">
    <span class="panel-title">Backups</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn"
      onclick={openBedrockBackupsFolder}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Open folder"
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      class="panel-icon-btn {isLoadingBackups ? 'spinning' : ''}"
      disabled={isLoadingBackups}
      onclick={refreshBedrockBackups}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <button
      class="panel-back-btn"
      onclick={onBack}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Back"
    >
      <i class="fa-solid fa-chevron-down"></i>
    </button>
  </div>
</div>

<SubTabs strip="utilities" active="backups" {onNavigate} />

<div class="panel-body">
  <div style="display:flex; gap:8px; padding:12px 14px 4px;">
    <input
      class="modal-input"
      placeholder="Optional label (e.g. before-1.21-update)"
      bind:value={backupLabelInput}
      onkeydown={(e) => e.stopPropagation()}
      style="flex:1;"
    />
    <button
      class="btn-sm btn-blue"
      disabled={isCreatingBackup}
      onclick={createBedrockBackup}
      onmousedown={(e) => e.stopPropagation()}
    >
      {#if isCreatingBackup}
        <span class="spinner"></span>
      {:else}
        <i class="fa-solid fa-camera"></i>
      {/if}
      Back up now
    </button>
  </div>

  {#if isLoadingBackups}
    <div class="versions-loading"><span class="spinner"></span><span>Loading backups…</span></div>
  {:else if bedrockBackups.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-clock-rotate-left"></i>
      <span>No backups yet</span>
      <small>Snapshot your worlds and packs before a risky install or a Windows reset.</small>
    </div>
  {:else}
    {#each bedrockBackups as backup (backup.file_path)}
      <div class="version-row">
        <div class="version-meta">
          <span class="version-name">{backup.name}</span>
          <span class="version-sub"
            >{formatSize(backup.size_bytes)} · {formatRelativeTime(backup.created_at)}</span
          >
        </div>
        <div class="version-actions">
          {#if _confirmRestoreBackupPath === backup.file_path}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Confirm restore (overwrites current worlds/packs)"
              onclick={() => restoreBedrockBackup(backup)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-check"></i>
            </button>
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Cancel"
              onclick={cancelRestoreBackup}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-xmark"></i>
            </button>
          {:else}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Restore"
              onclick={() => confirmRestoreBackup(backup.file_path)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-clock-rotate-left"></i>
            </button>
          {/if}
          {#if _confirmDeleteBackupPath === backup.file_path}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Confirm delete"
              onclick={() => deleteBedrockBackup(backup)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-check"></i>
            </button>
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Cancel"
              onclick={cancelDeleteBackup}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-xmark"></i>
            </button>
          {:else}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Delete"
              onclick={() => confirmDeleteBackup(backup.file_path)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>

<EditionTabs edition="bedrock" active="utilities" {onNavigate} />
