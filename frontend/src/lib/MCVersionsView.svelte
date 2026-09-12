<script lang="ts">
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
  import { untrack } from 'svelte'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import ViewModeToggle from './components/ViewModeToggle.svelte'
  import { loadViewMode, saveViewMode, type ViewMode } from './viewMode'
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
  let installStage = $state('')
  let unlistenInstall: UnlistenFn | undefined

  onMount(() => {
    listen<{ version_id: string; stage: string; percent: number }>(
      'java-install-progress',
      (e) => {
        if (e.payload.version_id !== installingId) return
        installStage = e.payload.stage
        installProgress = e.payload.percent / 100
      },
    ).then((fn) => {
      unlistenInstall = fn
    })
  })
  onDestroy(() => unlistenInstall?.())

  async function installJavaVersion(v: Row) {
    if (installingId) return
    installingId = v.id
    installProgress = 0
    installStage = 'Starting…'
    try {
      await invoke('install_java_version', { version: v })
      onToast?.(`Minecraft ${v.id} installed.`, 'success')
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
      onToast?.(`Launching Minecraft ${v.id}…`, 'success')
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
  let mcVersionsViewMode = $state<ViewMode>(loadViewMode('mc-versions'))
  function setMcVersionsViewMode(m: ViewMode) {
    mcVersionsViewMode = m
    saveViewMode('mc-versions', m)
  }

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
    onToast?.(`Default Java version: ${v.id}`, 'success')
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

<PanelHeader title="Versions" {onBack}>
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
      data-tooltip="Refresh"
      aria-label="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <ViewModeToggle mode={mcVersionsViewMode} onChange={setMcVersionsViewMode} />
  {/snippet}
</PanelHeader>

<div class="versions-client-switcher">
  <button class="vcs-btn active" disabled data-tooltip="Java has one client kind — Vanilla">
    <i class="fa-brands fa-java"></i> Vanilla
  </button>
</div>

<div class="panel-search-wrap">
  <i class="fa-solid fa-magnifying-glass"></i>
  <input
    class="panel-search-input"
    placeholder="Filter Java versions (e.g. 1.21, 1.8.9)..."
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
      <button class="vcs-btn" style="margin-top:8px;" onclick={refreshJavaVersions}>
        <i class="fa-solid fa-rotate"></i> Retry
      </button>
    </div>
  {:else}
    {#if loadError && versionsList.length > 0}
      <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>{loadError}</span></div>
    {/if}

    {#if installed.length > 0}
      <span class="panel-section-label">Installed ({installed.length})</span>
      <div class="version-list {mcVersionsViewMode === 'grid' ? 'grid-view' : ''}">
      {#each installed as v (v.id)}
        <div class="version-row {v.is_active ? 'mcv-active-row' : ''}">
          <div class="version-meta">
            <div style="display:flex; align-items:center; gap:6px;">
              <span class="version-name">Minecraft {v.id}</span>
              {#if v.is_active}<span class="tag-active">Active</span>{/if}
              <span class="version-sub" style="opacity:0.6;">{typeLabel(v.type)}</span>
            </div>
            {#if v.has_jar}
              <span class="version-sub">Ready to launch</span>
            {:else}
              <span class="version-sub" style="color:var(--orange);"
                >Profile present but jar missing — open in official launcher once</span
              >
            {/if}
          </div>
          <button
            class="vcs-btn"
            disabled={!v.has_jar || launchingId === v.id}
            onclick={() => launchJavaVersion(v)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-play"></i><span>&nbsp;Launch</span>
          </button>
          <button
            class="vcs-btn vcs-btn-ghost"
            onclick={() => setActiveJavaVersion(v)}
            onmousedown={(e) => e.stopPropagation()}
            data-tooltip="Set as default"
            aria-label="Set as default"
          >
            <i class="fa-solid fa-thumbtack"></i>
          </button>
        </div>
      {/each}
      </div>
    {/if}

    {#if others.length > 0}
      <span class="panel-section-label">Available ({others.length})</span>
      <div class="version-list {mcVersionsViewMode === 'grid' ? 'grid-view' : ''}">
      {#each others as v (v.id)}
        <div class="version-row">
          <div class="version-meta">
            <span class="version-name">Minecraft {v.id}</span>
            <span class="version-sub" style="opacity:0.6;"
              >{typeLabel(v.type)}
              {v.release_time ? `· ${v.release_time.slice(0, 10)}` : ''}</span
            >
          </div>
          {#if installingId === v.id}
            <InlineProgress progress={installProgress} showSpinner={false} />
          {:else}
            <button
              class="vcs-btn"
              disabled={installingId !== null}
              onclick={() => installJavaVersion(v)}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Install"
            >
              <i class="fa-solid fa-download"></i>&nbsp;Install
            </button>
          {/if}
        </div>
      {/each}
      </div>
    {/if}

    {#if installed.length === 0 && others.length === 0}
      <div class="versions-loading"><span style="opacity:0.5;">No results for "{versionsFilter}"</span></div>
    {/if}
  {/if}
</div>

