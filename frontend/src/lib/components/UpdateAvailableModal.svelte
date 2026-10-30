<script lang="ts">
  /**
   * The launcher-update flow used to be buried in Settings — the only visible
   * signal was a small top-bar pill that, when clicked, opened the *News*
   * panel (its changelog tab) rather than anything that could actually start
   * an update. Finding "Check for updates" / "Download and install" meant
   * knowing to go into Settings yourself. This surfaces the same
   * check/download/install flow (still backed by `check_launcher_update` /
   * `download_launcher_update` / `install_launcher_update`) as a modal
   * reachable directly from the pill, with the release's changelog inline.
   */
  import { invoke } from '@tauri-apps/api/core'
  import Modal from './Modal.svelte'
  import { formatChangelog } from '../format'
  import { t } from '../i18nStore.svelte'

  type UpdateInfo = { tag: string; download_url: string; changelog: string; asset_size: number }

  let {
    open,
    update,
    currentVersion,
    onClose,
    onToast,
  }: {
    open: boolean
    update: UpdateInfo | null
    currentVersion: string
    onClose: () => void
    onToast?: (message: string, kind?: 'info' | 'success' | 'error') => void
  } = $props()

  let downloading = $state(false)
  let installing = $state(false)
  let progressPct = $state(0)
  let error = $state('')

  function formatBytes(bytes: number): string {
    if (!bytes || bytes <= 0) return ''
    const units = ['B', 'KB', 'MB', 'GB']
    let v = bytes
    let i = 0
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024
      i++
    }
    return `${Math.round(v * 10) / 10} ${units[i]}`
  }

  async function downloadAndInstall() {
    if (!update) return
    error = ''
    downloading = true
    progressPct = 0
    try {
      const installerPath = await invoke<string>('download_launcher_update', {
        downloadUrl: update.download_url,
        tag: update.tag,
      })
      downloading = false
      installing = true
      // Success exits the app (installer takes over); failure keeps the
      // downloaded installer around via gl-update's pending marker, so the
      // very next launch retries automatically before showing any UI.
      await invoke('install_launcher_update', { installerPath })
    } catch (e) {
      error = String(e)
      onToast?.(t('toast.updateFailed', { error: String(e) }), 'error')
    } finally {
      downloading = false
      installing = false
    }
  }
</script>

<Modal
  {open}
  title={update ? 'Update available' : "You're up to date"}
  icon={update ? 'fa-solid fa-arrow-up' : 'fa-solid fa-circle-check'}
  iconColor={update ? undefined : 'var(--green)'}
  closeOnBackdrop={!downloading && !installing}
  onClose={() => { if (!downloading && !installing) onClose() }}
>
  {#if !update}
    <p class="setting-hint">Glacier Launcher v{currentVersion} is the latest version.</p>
    <div class="modal-footer-actions" style="margin-top:16px;">
      <button class="onboarding-continue-btn" onclick={onClose}>{t('common.close')}</button>
    </div>
  {:else}
    <div class="update-modal-version-row">
      <span class="update-modal-current">v{currentVersion}</span>
      <i class="fa-solid fa-arrow-right" style="color:var(--text-dim);"></i>
      <span class="update-modal-new">{update.tag}</span>
      {#if update.asset_size}
        <span class="update-modal-size">{formatBytes(update.asset_size)}</span>
      {/if}
    </div>

    {#if update.changelog}
      <div class="update-modal-changelog">{@html formatChangelog(update.changelog)}</div>
    {:else}
      <p class="setting-hint">{t('x.noChangelogWasProvidedForThisRelease')}</p>
    {/if}

    {#if error}
      <p class="warning-text" style="margin-top:10px;">{error}</p>
    {/if}

    <div class="modal-footer-actions" style="margin-top:16px;">
      <button class="modal-btn modal-btn-cancel" onclick={onClose} disabled={downloading || installing}>
        Remind me later
      </button>
      <button class="onboarding-continue-btn" onclick={downloadAndInstall} disabled={downloading || installing}>
        {#if installing}
          <span class="spinner"></span> Launching installer…
        {:else if downloading}
          <span class="spinner"></span> Downloading…
        {:else}
          <i class="fa-solid fa-download"></i> Download &amp; install
        {/if}
      </button>
    </div>
  {/if}
</Modal>

<style>
  .update-modal-version-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--r-md);
    background: var(--bg-item);
    margin-bottom: 12px;
  }

  .update-modal-current {
    color: var(--text-dim);
    font-weight: 600;
    font-size: 13px;
  }

  .update-modal-new {
    color: var(--accent);
    font-weight: 800;
    font-size: 14px;
  }

  .update-modal-size {
    margin-left: auto;
    color: var(--text-dim);
    font-size: 11.5px;
  }

  /* .update-modal-changelog and .modal-footer-actions are shared globals in
     app.css - App.svelte's What's New modal reuses both. */
</style>
