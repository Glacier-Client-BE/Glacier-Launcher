<script lang="ts">
  import { t } from './i18nStore.svelte'
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
import { UTILITIES } from './components/SubTabs.svelte'
import IconBtn from './components/IconBtn.svelte'
import ListSkeleton from './components/ListSkeleton.svelte'
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
  /** Routes through the parent's Utilities-strip resolution — this view
   *  never navigates itself. */
  onNavigate: (view: string, strip?: string) => void
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
    onToast?.(t('toast.savedName', { name: path.split(/[\\/]/).pop() ?? '' }), 'success')
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
    onToast?.(t('toast.backupDeleted'), 'success')
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
  onToast?.(t('toast.restoringBackup'), 'info')
  try {
    await invoke('restore_bedrock_backup', {
      backupPath: backup.file_path,
      instanceDir: instance?.directory ?? '',
    })
    onToast?.(t('toast.restoredBackup', { name: backup.name }), 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/** QOL: restore into a brand-new instance instead of overwriting the live
 *  install — for peeking at an old snapshot or recovering from a bad
 *  restore without risking what's currently being played. */
async function restoreBackupToNewInstance(backup: BedrockBackup) {
  _confirmRestoreBackupPath = ''
  onToast?.(t('toast.restoringNewInstance'), 'info')
  try {
    const created = await invoke<{ name: string }>('restore_bedrock_backup_to_new_instance', {
      backupPath: backup.file_path,
      name: `Restored - ${backup.name}`,
    })
    onToast?.(t('toast.restoredIntoNewInstance', { name: created.name }), 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

// QOL: backup diff — pick two backups, see what actually changed between
// them (files added/removed/changed-size) instead of comparing by eye.
let compareMode = $state(false)
let compareSelection = $state<string[]>([])
let compareResult = $state<{ added: string[]; removed: string[]; changed: string[]; unchanged_count: number } | null>(null)
let comparing = $state(false)

function toggleCompareMode() {
  compareMode = !compareMode
  compareSelection = []
  compareResult = null
}

function toggleCompareSelect(path: string) {
  if (compareSelection.includes(path)) {
    compareSelection = compareSelection.filter((p) => p !== path)
  } else if (compareSelection.length < 2) {
    compareSelection = [...compareSelection, path]
  }
  compareResult = null
}

async function runCompare() {
  if (compareSelection.length !== 2) return
  comparing = true
  try {
    compareResult = await invoke('diff_bedrock_backups', {
      pathA: compareSelection[0],
      pathB: compareSelection[1],
    })
  } catch (e) {
    onToast?.(String(e), 'error')
  } finally {
    comparing = false
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

<style>
  .bb-notice {
    margin: 10px 14px 0;
  }
  .bb-grow { flex: 1; }
  .bb-toolbar {
    display: flex;
    gap: 8px;
    padding: 12px 14px 4px;
  }
  .bb-toolbar-2 {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 14px 12px;
  }
  .bb-num { width: 70px; }
  .bb-wrap {
    gap: 8px;
    padding: 8px 0;
    flex-wrap: wrap;
  }
  .bb-mb { margin-bottom: 8px; }
  .bb-mt { margin-top: 6px; }
  .bb-icon-accent {
    background: var(--accent-bg);
    color: var(--accent);
  }
</style>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<div class="panel-header has-back">
  <button
    type="button"
    class="panel-back-btn"
    onclick={onBack}
    onmousedown={(e) => e.stopPropagation()}
    data-tooltip={t('tt.back')}
    aria-label={t('tt.back')}
  >
    <i class="fa-solid fa-arrow-left"></i>
  </button>
  <div class="panel-title-wrap">
    <span class="panel-title">{t('label.backups')}</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-spacer"></div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn"
      onclick={openBedrockBackupsFolder}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.openFolder')}
      aria-label={t('tt.openFolder')}
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      class="panel-icon-btn {isLoadingBackups ? 'spinning' : ''}"
      disabled={isLoadingBackups}
      onclick={refreshBedrockBackups}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <button
      class="panel-icon-btn {compareMode ? 'active' : ''}"
      onclick={toggleCompareMode}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={compareMode ? 'Cancel compare' : 'Compare two backups'}
      aria-label={t('tt.compareBackups')}
    >
      <i class="fa-solid fa-code-compare"></i>
    </button>
  </div>
</div>

<div class="settings-layout">
  <nav class="settings-nav-rail" aria-label={t('tt.utilitiesSections')}>
    {#each UTILITIES as nav (nav.key)}
      <button
        type="button"
        class="settings-nav-btn {nav.key === 'backups' ? 'active' : ''}"
        onclick={() => onNavigate(nav.key, 'utilities')}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class={nav.icon}></i> {nav.label}
      </button>
    {/each}
  </nav>

  <div class="settings-content">
<div class="panel-body">
  {#if !marketplaceReminderDismissed}
    <!-- Feature 23: one-time reminder, static UI/copy — no backend beyond a
         dismissed flag persisted in settings. -->
    <div class="notice-bar is-info bb-notice">
      <i class="fa-solid fa-circle-info"></i>
      <span class="bb-grow"
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

  <div class="bb-toolbar">
    <input
      class="modal-input bb-grow"
      placeholder={t('ph.optionalLabel')}
      bind:value={backupLabelInput}
      onkeydown={(e) => e.stopPropagation()}
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
  <div class="bb-toolbar-2">
    <label for="auto-backup-interval" class="version-sub">{t('x.autoBackupEvery')}</label>
    <input
      id="auto-backup-interval"
      class="modal-input bb-num"
      type="number"
      min="0"
      step="1"
      bind:value={autoBackupInterval}
      onkeydown={(e) => e.stopPropagation()}
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
    <ListSkeleton rows={4} />
  {:else if bedrockBackups.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-clock-rotate-left"></i>
      <span>{t('empty.noBackupsYet')}</span>
      <small>{t('empty.snapshotYourWorldsPacksBeforeRisky')}</small>
    </div>
  {:else}
    {#if compareMode}
      <div class="version-row bb-wrap">
        <span class="version-sub">Pick two backups to compare ({compareSelection.length}/2)</span>
        <button class="btn-sm" onclick={runCompare} disabled={compareSelection.length !== 2 || comparing}>
          {#if comparing}<span class="spinner"></span>{:else}<i class="fa-solid fa-code-compare"></i>{/if} Compare
        </button>
      </div>
      {#if compareResult}
        <div class="mod-warnings bb-mb">
          <div class="mod-warnings-head"><i class="fa-solid fa-code-compare"></i> {compareResult.unchanged_count} unchanged, {compareResult.added.length} added, {compareResult.removed.length} removed, {compareResult.changed.length} changed</div>
          {#if compareResult.added.length > 0}
            <p class="version-sub bb-mt"><strong>Added:</strong> {compareResult.added.slice(0, 20).join(', ')}{compareResult.added.length > 20 ? '…' : ''}</p>
          {/if}
          {#if compareResult.removed.length > 0}
            <p class="version-sub"><strong>Removed:</strong> {compareResult.removed.slice(0, 20).join(', ')}{compareResult.removed.length > 20 ? '…' : ''}</p>
          {/if}
          {#if compareResult.changed.length > 0}
            <p class="version-sub"><strong>Changed:</strong> {compareResult.changed.slice(0, 20).join(', ')}{compareResult.changed.length > 20 ? '…' : ''}</p>
          {/if}
        </div>
      {/if}
    {/if}
    <div class="client-card-cols">
    {#each bedrockBackups as backup (backup.file_path)}
      <div class="client-card {compareMode && compareSelection.includes(backup.file_path) ? 'client-active' : ''}">
        <div class="client-card-header">
          {#if compareMode}
            <button
              type="button"
              class="vcs-btn vcs-btn-ghost"
              onclick={() => toggleCompareSelect(backup.file_path)}
              aria-label={compareSelection.includes(backup.file_path) ? 'Deselect' : 'Select'}
            >
              <i class="fa-solid {compareSelection.includes(backup.file_path) ? 'fa-square-check' : 'fa-square'}"></i>
            </button>
          {/if}
          <div class="client-card-icon bb-icon-accent">
            <i class="fa-solid fa-box-archive"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{backup.name}</span>
            <span class="client-card-sub"
              >{formatSize(backup.size_bytes)} · {formatRelativeTime(backup.created_at)}</span
            >
          </div>
          <div class="client-card-actions">
            {#if _confirmRestoreBackupPath === backup.file_path}
              <IconBtn
                ghost
                icon="fa-solid fa-check"
                title="Confirm restore (overwrites current worlds/packs)"
                onClick={() => restoreBedrockBackup(backup)}
                onMouseDown={(e) => e.stopPropagation()}
              />
              <IconBtn
                ghost
                icon="fa-solid fa-layer-group"
                title="Restore to a new instance instead (doesn't touch the live install)"
                onClick={() => restoreBackupToNewInstance(backup)}
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
        {#if _confirmRestoreBackupPath === backup.file_path}
          <p class="client-card-desc warning-text">This overwrites your current worlds &amp; packs.</p>
        {/if}
      </div>
    {/each}
    </div>
  {/if}
</div>
  </div>
</div>
