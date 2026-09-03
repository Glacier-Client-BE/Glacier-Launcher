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
  import PanelHeader from './components/PanelHeader.svelte'
  import EditionTabs from './components/EditionTabs.svelte'
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
   * No backend counterpart for `JavaInstallerService.InstallAsync` /
   * `JavaLauncherService.LaunchAsync(versionId)` exists yet: `lib.rs`
   * registers `create_java_instance` and `launch_java_instance` (both
   * instance-scoped), not a per-version install or launch. Per the
   * established pattern, the controls stay visible but disabled.
   */
  const MISSING_INSTALL = 'Unavailable: needs an install_java_version command'
  const MISSING_LAUNCH = 'Unavailable: needs a launch_java_version command (launch_java_instance is instance-scoped)'

  type Row = JavaVersion & { is_active: boolean }

  let versionsList = $state<Row[]>([])
  let isLoading = $state(false)
  let loadError = $state('')
  let versionsFilter = $state('')

  // Mirrors of the two persisted settings. They are held locally as well
  // because the shell currently mounts this panel without an `onSaveSettings`
  // callback, so the prop object would not re-render on toggle.
  let showSnapshots = $state(settings?.JavaShowSnapshots ?? false)
  let showHistorical = $state(settings?.JavaShowHistorical ?? false)

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
  {/snippet}
</PanelHeader>

<div class="versions-client-switcher">
  <button class="vcs-btn active" onmousedown={(e) => e.stopPropagation()}>
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
    <div class="versions-loading"><span class="spinner"></span><span>Fetching Java versions...</span></div>
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
            disabled
            data-tooltip={MISSING_LAUNCH}
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
    {/if}

    {#if others.length > 0}
      <span class="panel-section-label">Available ({others.length})</span>
      {#each others as v (v.id)}
        <div class="version-row">
          <div class="version-meta">
            <span class="version-name">Minecraft {v.id}</span>
            <span class="version-sub" style="opacity:0.6;"
              >{typeLabel(v.type)}
              {v.release_time ? `· ${v.release_time.slice(0, 10)}` : ''}</span
            >
          </div>
          <button
            class="vcs-btn"
            disabled
            data-tooltip={MISSING_INSTALL}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-download"></i>&nbsp;Install
          </button>
        </div>
      {/each}
    {/if}

    {#if installed.length === 0 && others.length === 0}
      <div class="versions-loading"><span style="opacity:0.5;">No results for "{versionsFilter}"</span></div>
    {/if}
  {/if}
</div>

<EditionTabs edition="java" active="versions" onNavigate={(view) => onNavigate?.(view)} />
