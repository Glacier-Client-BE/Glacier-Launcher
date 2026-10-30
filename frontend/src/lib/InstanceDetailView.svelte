<script lang="ts">
  /**
   * Phase 8 — instance detail view: a left rail (icon/name/loader
   * badge/play/playtime) plus a `PanelTabs` strip whose bodies reuse the
   * EXISTING standalone panels in "embedded" mode (Screenshots/Worlds/Logs)
   * rather than duplicating their logic. Content/Sync/Export don't have a
   * real backend yet (Phase 2/6/7) so they render "coming soon"
   * placeholders instead of faking functionality.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { untrack } from 'svelte'
  import { t } from './i18nStore.svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import PanelTabs from './components/PanelTabs.svelte'
  import JavaScreenshotsView from './JavaScreenshotsView.svelte'
  import JavaBackupsView from './JavaBackupsView.svelte'
  import LogsPanel from './LogsPanel.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import type { EffectiveLaunchConfig } from './settings'

  type DetailInstance = {
    id: string
    name?: string
    directory: string
    version_id?: string
    accent_color?: string | null
    group?: string | null
    loader?: string | null
    loader_version?: string | null
    playtime_seconds?: number
  }

  let {
    instance,
    onBack,
    onToast,
    onLaunch,
  }: {
    instance: DetailInstance
    onBack?: () => void
    onToast?: (message: string, kind?: string) => void
    onLaunch?: (id: string) => void
  } = $props()

  function toast(message: string, kind = 'info') {
    onToast?.(message, kind)
  }

  type TabDef = { id: string; label: string; icon: string }
  const TABS: TabDef[] = [
    { id: 'content', label: 'Content', icon: 'fa-solid fa-cubes' },
    { id: 'worlds', label: 'Worlds', icon: 'fa-solid fa-earth-americas' },
    { id: 'screenshots', label: 'Screenshots', icon: 'fa-solid fa-image' },
    { id: 'logs', label: 'Logs', icon: 'fa-solid fa-file-lines' },
    { id: 'settings', label: 'Settings', icon: 'fa-solid fa-gear' },
    { id: 'sync', label: 'Sync', icon: 'fa-solid fa-rotate' },
    { id: 'export', label: 'Export', icon: 'fa-solid fa-file-export' },
  ]
  let activeTab = $state('content')

  const MISSING_CONTENT_TAB =
    'Unavailable: needs a per-instance content-list command (Phase 2 loaders/mods aggregation)'
  const MISSING_SYNC =
    'Unavailable: needs SyncPacksView/gl-sync commands (Phase 7)'
  const MISSING_EXPORT_MODPACK =
    'Unavailable: needs an export_java_modpack command'

  let effective = $state<EffectiveLaunchConfig | null>(null)
  let loadingEffective = $state(false)

  async function loadEffective() {
    loadingEffective = true
    try {
      effective = await invoke<EffectiveLaunchConfig>('get_effective_launch_config', {
        id: instance.id,
      })
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      loadingEffective = false
    }
  }

  $effect(() => {
    if (activeTab === 'settings') untrack(() => { if (!effective && !loadingEffective) loadEffective() })
  })

  function fmtPlaytime(seconds: number): string {
    const h = Math.floor(seconds / 3600)
    const m = Math.floor((seconds % 3600) / 60)
    if (h > 0) return `${h}h ${m}m played`
    if (m > 0) return `${m}m played`
    return 'Just started'
  }

  let exporting = $state(false)

  async function exportZip() {
    if (exporting) return
    exporting = true
    try {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const dest = await save({ defaultPath: `${instance.name ?? "instance"}.zip` })
      if (!dest) return
      await invoke('export_java_instance', { id: instance.id, destPath: dest })
      toast('Instance exported.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      exporting = false
    }
  }
</script>

<PanelHeader title={instance.name ?? "Instance"} {onBack} />

<div class="panel-body instance-detail">
  <div class="instance-detail-rail" style={instance.accent_color ? `--accent:${instance.accent_color}` : ''}>
    <div class="instance-detail-icon">
      <i class="fa-solid fa-cube"></i>
    </div>
    <div class="instance-detail-name" title={instance.name ?? 'Instance'}>{instance.name ?? "Instance"}</div>
    {#if instance.loader && instance.loader !== 'vanilla'}
      <span class="instance-loader-badge">{instance.loader}{instance.loader_version ? ` ${instance.loader_version}` : ''}</span>
    {/if}
    <div class="instance-detail-version" title={instance.version_id ?? ''}>{instance.version_id ?? ''}</div>
    <button type="button" class="btn-sm btn-primary instance-detail-play" onclick={() => onLaunch?.(instance.id)}>
      <i class="fa-solid fa-play"></i> Play
    </button>
    {#if instance.playtime_seconds && instance.playtime_seconds > 0}
      <div class="instance-detail-playtime">{fmtPlaytime(instance.playtime_seconds)}</div>
    {/if}
    {#if instance.group}
      <div class="instance-detail-group" title={instance.group}><i class="fa-solid fa-folder"></i> {instance.group}</div>
    {/if}
  </div>

  <div class="instance-detail-main">
    <PanelTabs tabs={TABS} active={activeTab} onSelect={(id) => (activeTab = id)} />

    <div class="instance-detail-tab-body">
      {#if activeTab === 'content'}
        <div class="empty-state">
          <i class="fa-solid fa-cubes"></i>
          <span>Content browser coming soon</span>
          <small data-tooltip={MISSING_CONTENT_TAB}>Pending a per-instance mods and loader list.</small>
        </div>
      {:else if activeTab === 'worlds'}
        <JavaBackupsView instanceName={instance.name ?? ''} embedded onToast={toast} />
      {:else if activeTab === 'screenshots'}
        <JavaScreenshotsView embedded instanceId={instance.id} />
      {:else if activeTab === 'logs'}
        <LogsPanel instanceDir={instance.directory} instanceId={instance.id} embedded />
      {:else if activeTab === 'settings'}
        <div class="instance-settings-summary">
          {#if loadingEffective}
            <ListSkeleton rows={4} />
          {:else if effective}
            <div class="wizard-review">
              <div><strong>{t('label.ram')}</strong><span>{effective.min_ram_mb} – {effective.max_ram_mb} MB</span></div>
              <div><strong>Java path</strong><span>{effective.java_path ?? 'Global default'}</span></div>
              <div><strong>{t('label.fullscreen')}</strong><span>{effective.fullscreen ? 'Yes' : 'No'}</span></div>
              <div>
                <strong>{t('x.resolution')}</strong>
                <span>{effective.use_custom_resolution ? `${effective.window_width}×${effective.window_height}` : 'Global default'}</span>
              </div>
              <div><strong>{t('x.overridden')}</strong><span>{effective.overridden.length ? effective.overridden.join(', ') : 'None'}</span></div>
            </div>
            <p class="wizard-hint">
              Read-only view of the resolved settings. Edit them in the Profile panel's per-instance
              controls.
            </p>
          {:else}
            <div class="empty-state">
              <i class="fa-solid fa-triangle-exclamation"></i>
              <span>{t('x.couldNotLoadSettings')}</span>
            </div>
          {/if}
        </div>
      {:else if activeTab === 'sync'}
        <div class="empty-state">
          <i class="fa-solid fa-rotate"></i>
          <span>Sync is coming soon</span>
          <small data-tooltip={MISSING_SYNC}>Sync packs ship in a later release.</small>
        </div>
      {:else if activeTab === 'export'}
        <div class="instance-export-tab">
          <button class="btn-sm btn-primary" onclick={exportZip} disabled={exporting} aria-busy={exporting}>
            <i class="fa-solid fa-file-zipper"></i> Export as .zip
          </button>
          <button class="btn-sm" disabled data-tooltip={MISSING_EXPORT_MODPACK}>
            <i class="fa-solid fa-box-archive"></i> Export as modpack
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  /* Detail rhythm: section gutter, band gaps between stacked blocks,
     card padding on the left rail. Disabled export button dims to .5. */
  .instance-detail {
    gap: var(--panel-gutter);
  }
  .instance-detail-rail {
    padding: var(--card-pad-y) var(--card-pad-x);
    gap: var(--panel-band);
    min-width: 0;
  }
  .instance-detail-name,
  .instance-detail-version,
  .instance-detail-group {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .instance-detail-version,
  .instance-detail-group {
    color: var(--text-dim);
    opacity: 1;
  }
  .instance-detail-play {
    margin-top: var(--panel-band);
    transition: background-color var(--t), opacity var(--t);
  }
  .instance-detail-play:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .instance-detail-main {
    gap: var(--panel-band);
  }
  .instance-export-tab {
    gap: var(--panel-band);
    flex-wrap: wrap;
  }
  .instance-export-tab button:not(.btn-primary):hover:not(:disabled) {
    background: var(--bg-item-hover);
  }
  .instance-export-tab button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .instance-settings-summary .wizard-review > div {
    padding: var(--row-pad-y) 0;
  }
  .instance-settings-summary .wizard-hint {
    margin-top: var(--panel-band);
  }
</style>
