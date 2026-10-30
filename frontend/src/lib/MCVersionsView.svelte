<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * 1:1 port of the `javaversions` panel - Home.razor:5018-5166, with its
   * code-behind in Home.Panels.cs (OpenJavaVersions:404, RefreshJavaVersionsAsync:414,
   * LoadJavaVersionsAsync:416, ToggleJavaSnapshots:438, ToggleJavaHistorical:445,
   * SetActiveJavaVersion:452, HandleJavaInstall:662, HandleJavaLaunch:698) plus
   * `FilteredJavaVersions` (Home.razor.cs:141) and `JavaVersion.TypeLabel`
   * (Models/JavaVersion.cs).
   *
   * The previous contents of this file were a loose re-imagining of the same
   * panel: it invented a four-way Releases/Snapshots/Old/All switcher, an
   * invented "New instance" action wired to `create_java_instance`, an
   * invented `.empty-state` block, and dropped the snapshot/beta toggles, the
   * refresh button, the Installed/Available sections, the active-version
   * marker and the bottom Java tab strip. All of that is replaced below by
   * the original markup.
   *
   * Not the Bedrock `mcversions` panel (Home.razor:4653) - that is
   * BedrockVanillaVersionsView.svelte.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount, untrack } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import type { JavaVersion, LauncherSettings } from './settings'

  let {
    settings,
    onBack,
    onNavigate,
    onSaveSettings,
    onToast,
  }: {
    settings: LauncherSettings
    onBack?: () => void
    onNavigate?: (view: string) => void
    onSaveSettings?: (next: LauncherSettings) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  /**
   * `launch_java_version` launches a bare version id directly against the
   * shared versions pool (no instance's mods/saves attached) - mirrors
   * `JavaLauncherService.LaunchAsync(v.Id)`. Distinct from
   * `launch_java_instance`, which the Profile/Instances panel uses for an
   * instance-scoped launch.
   */

  let installingId = $state<string | null>(null)
  let installProgress = $state(0)
  let unlistenInstall: UnlistenFn | undefined

  let installDisposed = false
  onMount(() => {
    listen<{ version_id: string; stage: string; percent: number }>(
      'java-install-progress',
      (e) => {
        if (e.payload.version_id !== installingId) return
        installProgress = e.payload.percent / 100
      },
    ).then((fn) => {
      if (installDisposed) fn()
      else unlistenInstall = fn
    })
  })
  onDestroy(() => {
    installDisposed = true
    unlistenInstall?.()
  })

  async function installJavaVersion(v: Row) {
    if (installingId) return
    installingId = v.id
    installProgress = 0
    try {
      await invoke('install_java_version', { version: v })
      onToast?.(t('toast.minecraftIdInstalled', { id: v.id }), 'success')
      await loadJavaVersions()
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      installingId = null
    }
  }

  let launchingId = $state<string | null>(null)

  /** Port of `JavaLauncherService.LaunchAsync(v.Id)` via `launch_java_version` -
   * a bare version launch with no instance's mods/saves attached. */
  async function launchJavaVersion(v: Row) {
    if (launchingId) return
    launchingId = v.id
    try {
      await invoke('launch_java_version', { versionId: v.id })
      onToast?.(t('toast.launchingMinecraftId', { id: v.id }), 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      launchingId = null
    }
  }

  type Row = JavaVersion & { is_active: boolean }

  let versionsList = $state<Row[]>([])
  let isLoading = $state(false)
  let loadError = $state('')
  let versionsFilter = $state('')

  // Mirrors of the two persisted settings. They are held locally as well
  // because the shell currently mounts this panel without an `onSaveSettings`
  // callback, so the prop object would not re-render on toggle.
  let showSnapshots = $state(untrack(() => settings?.JavaShowSnapshots) ?? false)
  let showHistorical = $state(untrack(() => settings?.JavaShowHistorical) ?? false)

  /** Port of `JavaVersion.TypeLabel`. */
  function typeLabel(type: string): string {
    switch (type) {
      case 'release':
        return 'Release'
      case 'snapshot':
        return 'Snapshot'
      case 'old_beta':
        return 'Beta'
      case 'old_alpha':
        return 'Alpha'
      default:
        return type
    }
  }

  /** Port of `LoadJavaVersionsAsync`. */
  async function loadJavaVersions() {
    isLoading = true
    loadError = ''
    try {
      const list = await invoke<JavaVersion[]>('list_java_versions')
      const active = settings?.JavaActiveVersion ?? ''
      versionsList = list.map((v) => ({ ...v, is_active: v.is_active || v.id === active }))
    } catch (e) {
      loadError = String(e)
    } finally {
      isLoading = false
    }
  }

  /** Port of `RefreshJavaVersionsAsync`. */
  function refreshJavaVersions() {
    loadJavaVersions()
  }

  function persist(next: LauncherSettings) {
    if (onSaveSettings) onSaveSettings(next)
    else invoke('set_settings', { settings: next }).catch(() => {})
  }

  /** Port of `ToggleJavaSnapshots`. */
  function toggleJavaSnapshots() {
    showSnapshots = !showSnapshots
    if (settings) persist({ ...settings, JavaShowSnapshots: showSnapshots })
  }

  /** Port of `ToggleJavaHistorical`. */
  function toggleJavaHistorical() {
    showHistorical = !showHistorical
    if (settings) persist({ ...settings, JavaShowHistorical: showHistorical })
  }

  /** Port of `SetActiveJavaVersion`. */
  function setActiveJavaVersion(v: Row) {
    for (const jv of versionsList) jv.is_active = false
    v.is_active = true
    if (settings) persist({ ...settings, JavaActiveVersion: v.id })
    onToast?.(t('toast.defaultJavaVersion', { id: v.id }), 'success')
  }

  /** Port of `FilteredJavaVersions` (Home.razor.cs:141). */
  const filtered = $derived.by(() => {
    let q = versionsList
    if (!showSnapshots) q = q.filter((v) => v.type !== 'snapshot')
    if (!showHistorical) q = q.filter((v) => v.type !== 'old_beta' && v.type !== 'old_alpha')
    const filter = versionsFilter.trim()
    if (filter) {
      const needle = filter.toLowerCase()
      q = q.filter((v) => v.id.toLowerCase().includes(needle))
    }
    return q
  })

  const installed = $derived(filtered.filter((v) => v.is_installed))
  const others = $derived(filtered.filter((v) => !v.is_installed))

  loadJavaVersions()
</script>

<style>
  .java-logo {
    width: 18px;
    height: 18px;
    object-fit: contain;
  }
  .java-retry {
    margin-top: 8px;
  }
  .java-warn {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--orange);
  }
</style>

<PanelHeader title={t('panel.versions')} {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn {showSnapshots ? 'active' : ''}"
      onclick={toggleJavaSnapshots}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={showSnapshots ? 'Hide snapshots' : 'Show snapshots'}
      aria-label={showSnapshots ? 'Hide snapshots' : 'Show snapshots'}
    >
      <i class="fa-solid fa-flask"></i>
    </button>
    <button
      type="button"
      class="panel-icon-btn {showHistorical ? 'active' : ''}"
      onclick={toggleJavaHistorical}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={showHistorical ? 'Hide beta/alpha' : 'Show beta/alpha'}
      aria-label={showHistorical ? 'Hide beta/alpha' : 'Show beta/alpha'}
    >
      <i class="fa-solid fa-clock-rotate-left"></i>
    </button>
    <button
      type="button"
      class="panel-icon-btn {isLoading ? 'spinning' : ''}"
      disabled={isLoading}
      onclick={refreshJavaVersions}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<!-- Java's counterpart to Bedrock's Versions/Instances cross-nav — Java
     instance management used to live buried in a local tab inside Profile,
     with no link back to this panel at all. -->
<SubTabs strip="versions-instances" active="versions" onNavigate={(k) => onNavigate?.(k)} />

<div class="panel-search-wrap">
  <i class="fa-solid fa-magnifying-glass"></i>
  <input
    class="panel-search-input"
    aria-label={t('ph.filterJavaVersions')}
    placeholder={t('ph.filterJavaVersions')}
    bind:value={versionsFilter}
    onkeydown={(e) => e.stopPropagation()}
  />
</div>

<div class="panel-body">
  {#if isLoading}
    <ListSkeleton rows={6} />
  {:else if versionsList.length === 0 && loadError}
    <div class="versions-loading versions-error">
      <i class="fa-solid fa-triangle-exclamation"></i>
      <span>{loadError}</span>
      <button class="vcs-btn java-retry" onclick={refreshJavaVersions}>
        <i class="fa-solid fa-rotate"></i> {t('common.retry')}
      </button>
    </div>
  {:else}
    {#if loadError && versionsList.length > 0}
      <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>{loadError}</span></div>
    {/if}

    {#if installed.length > 0}
      <span class="panel-section-label">Installed ({installed.length})</span>
      <div class="client-card-cols">
      {#each installed as v (v.id)}
        <div class="client-card {v.is_active ? 'client-active' : ''}">
          <div class="client-card-header">
            <div class="client-card-icon latite-icon"><img class="chip-icon java-logo" src="images/mc-java-logo.png" alt=""></div>
            <div class="client-card-meta">
              <span class="client-card-name">Minecraft {v.id}</span>
              <span class="client-card-sub">
                {#if v.is_active}<span class="tag-active">{t('label.active')}</span>{/if}
                {typeLabel(v.type)}
              </span>
            </div>
            <div class="client-card-actions">
              <button
                class="vcs-btn vcs-btn-ghost"
                onclick={() => setActiveJavaVersion(v)}
                onmousedown={(e) => e.stopPropagation()}
                data-tooltip={t('tt.setAsDefault')}
                aria-label={t('tt.setAsDefault')}
              >
                <i class="fa-solid fa-thumbtack"></i>
              </button>
              <button
                class="vcs-btn"
                disabled={!v.has_jar || launchingId === v.id}
                onclick={() => launchJavaVersion(v)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class={launchingId === v.id ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-play'}></i><span>&nbsp;{launchingId === v.id ? 'Launching…' : 'Launch'}</span>
              </button>
            </div>
          </div>
          {#if v.has_jar}
            <p class="client-card-desc">{t('x.readyToLaunch')}</p>
          {:else}
            <p class="client-card-desc java-warn">
              <i class="fa-solid fa-triangle-exclamation"></i> Profile present but jar missing — open in official launcher once
            </p>
          {/if}
        </div>
      {/each}
      </div>
    {/if}

    {#if others.length > 0}
      <span class="panel-section-label">Available ({others.length})</span>
      <div class="client-card-cols">
      {#each others as v (v.id)}
        <div class="client-card">
          <div class="client-card-header">
            <div class="client-card-icon latite-icon"><img class="chip-icon java-logo" src="images/mc-java-logo.png" alt=""></div>
            <div class="client-card-meta">
              <span class="client-card-name">Minecraft {v.id}</span>
              <span class="client-card-sub"
                >{typeLabel(v.type)}
                {v.release_time ? `· ${v.release_time.slice(0, 10)}` : ''}</span
              >
            </div>
            <div class="client-card-actions">
              {#if installingId === v.id}
                <InlineProgress progress={installProgress} showSpinner={false} />
              {:else}
                <button
                  class="vcs-btn"
                  disabled={installingId !== null}
                  onclick={() => installJavaVersion(v)}
                  onmousedown={(e) => e.stopPropagation()}
                  data-tooltip={t('tt.install')}
                >
                  <i class="fa-solid fa-download"></i>&nbsp;Install {v.id}
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
      </div>
    {/if}

    {#if installed.length === 0 && others.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-filter-circle-xmark"></i>
        <span>{t('empty.noResultsFound')}</span>
      </div>
    {/if}
  {/if}
</div>

