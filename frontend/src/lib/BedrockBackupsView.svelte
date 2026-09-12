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
import IconBtn from './components/IconBtn.svelte'
import type { BedrockInstance } from './settings'
import { formatSize, formatRelativeTime, parentDir } from './format'

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

// Feature 20 — auto-backup-every-N-launches. `0` disables it.
let autoBackupInterval = $state(0)
let isSavingAutoBackupInterval = $state(false)

// Feature 23 — one-time dismissible marketplace-content reminder.
let marketplaceReminderDismissed = $state(true)

async function loadAutoBackupInterval() {
  try {
    autoBackupInterval = await invoke<number>('bedrock_auto_backup_interval')
  } catch {
    // Best-effort — leave the control at its default.
  }
}

async function saveAutoBackupInterval() {
  isSavingAutoBackupInterval = true
  try {
    await invoke('set_bedrock_auto_backup_interval', { everyNLaunches: autoBackupInterval })
    onToast?.(
      autoBackupInterval > 0
        ? `Auto-backup will run every ${autoBackupInterval} launch${autoBackupInterval === 1 ? '' : 'es'}.`
        : 'Auto-backup disabled.',
      'success',
    )
  } catch (e) {
    onToast?.(String(e), 'error')
  } finally {
    isSavingAutoBackupInterval = false
  }
}

async function loadMarketplaceReminderDismissed() {
  try {
    marketplaceReminderDismissed = await invoke<boolean>('bedrock_marketplace_reminder_dismissed')
  } catch {
    marketplaceReminderDismissed = true
  }
}

async function dismissMarketplaceReminder() {
  marketplaceReminderDismissed = true
  try {
    await invoke('dismiss_bedrock_marketplace_reminder')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/** Port of Home.razor.cs:692 `FormatSize`. */
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

loadAutoBackupInterval()
loadMarketplaceReminderDismissed()
</script>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<div class="panel-header has-back">
  <button
    type="button"
    class="panel-back-btn"
    onclick={onBack}
    onmousedown={(e) => e.stopPropagation()}
    data-tooltip="Back"
    aria-label="Back"
  >
    <i class="fa-solid fa-arrow-left"></i>
  </button>
  <div class="panel-title-wrap">
    <span class="panel-title">Backups</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-spacer"></div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn"
      onclick={openBedrockBackupsFolder}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Open folder"
      aria-label="Open folder"
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      class="panel-icon-btn {isLoadingBackups ? 'spinning' : ''}"
      disabled={isLoadingBackups}
      onclick={refreshBedrockBackups}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
      aria-label="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  </div>
</div>

<SubTabs strip="utilities" active="backups" {onNavigate} />

<div class="panel-body">
  {#if !marketplaceReminderDismissed}
    <!-- Feature 23: one-time reminder, static UI/copy — no backend beyond a
         dismissed flag persisted in settings. -->
    <div
      class="warning-text"
      style="display:flex; align-items:center; gap:8px; padding:10px 14px; margin:10px 14px 0;"
    >
      <i class="fa-solid fa-circle-info"></i>
      <span style="flex:1;"
        >Heads up: marketplace-purchased content (maps, skin packs, texture packs bought in-game)
        isn't included when you export or back up a world locally — only your own saves and
        installed add-ons are.</span
      >
      <IconBtn
        ghost
        icon="fa-solid fa-xmark"
        title="Dismiss"
        onClick={dismissMarketplaceReminder}
        onMouseDown={(e) => e.stopPropagation()}
      />
    </div>
  {/if}

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

  <!-- Feature 20: auto-backup-every-N-launches. -->
  <div style="display:flex; align-items:center; gap:8px; padding:4px 14px 12px;">
    <label for="auto-backup-interval" class="version-sub">Auto-backup every</label>
    <input
      id="auto-backup-interval"
      class="modal-input"
      type="number"
      min="0"
      step="1"
      bind:value={autoBackupInterval}
      onkeydown={(e) => e.stopPropagation()}
      style="width:70px;"
    />
    <span class="version-sub">launch(es) — 0 disables it</span>
    <button
      class="btn-sm"
      disabled={isSavingAutoBackupInterval}
      onclick={saveAutoBackupInterval}
      onmousedown={(e) => e.stopPropagation()}
    >
      Save
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
            <span class="warning-text">This overwrites your current worlds &amp; packs.</span>
            <IconBtn
              ghost
              icon="fa-solid fa-check"
              title="Confirm restore (overwrites current worlds/packs)"
              onClick={() => restoreBedrockBackup(backup)}
              onMouseDown={(e) => e.stopPropagation()}
            />
            <IconBtn
              ghost
              icon="fa-solid fa-xmark"
              title="Cancel"
              onClick={cancelRestoreBackup}
              onMouseDown={(e) => e.stopPropagation()}
            />
          {:else}
            <IconBtn
              ghost
              icon="fa-solid fa-clock-rotate-left"
              title="Restore"
              onClick={() => confirmRestoreBackup(backup.file_path)}
              onMouseDown={(e) => e.stopPropagation()}
            />
          {/if}
          {#if _confirmDeleteBackupPath === backup.file_path}
            <IconBtn
              ghost
              icon="fa-solid fa-check"
              title="Confirm delete"
              onClick={() => deleteBedrockBackup(backup)}
              onMouseDown={(e) => e.stopPropagation()}
            />
            <IconBtn
              ghost
              icon="fa-solid fa-xmark"
              title="Cancel"
              onClick={cancelDeleteBackup}
              onMouseDown={(e) => e.stopPropagation()}
            />
          {:else}
            <IconBtn
              ghost
              icon="fa-solid fa-trash"
              title="Delete"
              onClick={() => confirmDeleteBackup(backup.file_path)}
              onMouseDown={(e) => e.stopPropagation()}
            />
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>
