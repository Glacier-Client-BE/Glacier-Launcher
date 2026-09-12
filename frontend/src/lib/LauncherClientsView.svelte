<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import PanelHeader from './components/PanelHeader.svelte'
  import InlineProgress from './components/InlineProgress.svelte'

  /**
   * 1:1 port of the `javaclients` panel - `Pages/Home.razor:4849-5017`
   * ("Launchers"). Driving code-behind: `RefreshJavaClients` /
   * `LaunchLunar` / `LaunchLunarDirect` (Pages/Home.Panels.cs:529-572) and
   * the whole of `Pages/Home.Glacier.cs`.
   *
   * Structure follows the original exactly: info bar, a "Built-in" section
   * with the Vanilla card and the Glacier Client card, then a "Third-party"
   * section with the single combined Lunar/Badlion card, then the Java
   * bottom tab strip with `launchers` active. The original has NO separate
   * Badlion card and NO empty state - a previous rewrite invented both.
   *
   * `detect_lunar_badlion`, `launch_client_exe`, `launch_lunar_direct`, and
   * now `get_glacier_manifest`/`install_glacier_client`/
   * `uninstall_glacier_client`/`launch_glacier_client` all exist in
   * `src-tauri/src/lib.rs`.
   */
  let {
    onBack,
    onNavigate = () => {},
    onToast = () => {},
  }: {
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  type GlacierVersion = {
    id: string
    name: string
    releaseDate: string
    tag: string
    url: string
    sha256: string
    fabric: boolean
    forge: boolean
    javaVersion: number
    changelog: string
  }
  type GlacierManifest = {
    schemaVersion: number
    latestRelease: string
    releaseDate: string
    versions: GlacierVersion[]
  }

  let glacierManifest = $state<GlacierManifest | null>(null)
  let glacierLoading = $state(false)
  let glacierError = $state('')
  let glacierBusy = $state(false)
  let glacierInstalled = $state(false)

  const glacierLatest = $derived.by(() => {
    if (!glacierManifest) return null
    return (
      glacierManifest.versions.find((v) => v.id === glacierManifest!.latestRelease) ??
      glacierManifest.versions[0] ??
      null
    )
  })

  /** Live download queue, for real progress on the Glacier install. */
  type QueueEntry = { id: string; label: string; status: string; progress: number }
  let queue = $state<QueueEntry[]>([])
  let unlistenQueue: UnlistenFn | undefined
  const glacierProgress = $derived(
    glacierLatest
      ? (queue.find((e) => e.label === `Glacier ${glacierLatest!.name}` && e.status === 'Downloading')
          ?.progress ?? 0)
      : 0,
  )

  onMount(() => {
    listen<QueueEntry[]>('downloads-changed', (e) => {
      queue = e.payload
    }).then((fn) => {
      unlistenQueue = fn
    })
  })
  onDestroy(() => unlistenQueue?.())

  /** Port of `EnsureGlacierManifestAsync`. */
  async function loadGlacierManifest(force = false) {
    if (glacierManifest && !force) return
    glacierLoading = true
    glacierError = ''
    try {
      glacierManifest = await invoke<GlacierManifest>('get_glacier_manifest', { forceRefresh: force })
      await refreshGlacierInstalled()
    } catch (e) {
      glacierError = String(e)
    } finally {
      glacierLoading = false
    }
  }

  async function refreshGlacierInstalled() {
    if (!glacierLatest) {
      glacierInstalled = false
      return
    }
    try {
      const size = await invoke<number>('glacier_installed_size_bytes')
      // No per-version installed probe exists; a nonzero total is a good
      // enough signal for the single "latest" card this panel shows.
      glacierInstalled = size > 0
    } catch {
      glacierInstalled = false
    }
  }

  /** Port of `InstallGlacierAsync`. */
  async function installGlacier() {
    if (glacierBusy || !glacierLatest) return
    glacierBusy = true
    glacierError = ''
    try {
      await invoke('install_glacier_client', { version: glacierLatest })
      glacierInstalled = true
      onToast(`Glacier ${glacierLatest.name} installed!`, 'success')
    } catch (e) {
      glacierError = String(e)
      onToast('Install failed: ' + String(e), 'error')
    } finally {
      glacierBusy = false
    }
  }

  /** Port of `UninstallGlacier`. */
  async function uninstallGlacier() {
    if (!glacierLatest) return
    try {
      await invoke('uninstall_glacier_client', { version: glacierLatest })
      glacierInstalled = false
      onToast(`Glacier ${glacierLatest.name} removed.`, 'info')
    } catch (e) {
      onToast(String(e), 'error')
    }
  }

  /** Port of `LaunchGlacierAsync`. */
  async function launchGlacier() {
    if (glacierBusy || !glacierLatest) return
    glacierBusy = true
    glacierError = ''
    try {
      await invoke('launch_glacier_client', { version: glacierLatest })
      onToast(`Glacier ${glacierLatest.name} launching…`, 'success')
    } catch (e) {
      glacierError = String(e)
      onToast(String(e), 'error')
    } finally {
      glacierBusy = false
    }
  }

  loadGlacierManifest()

  let launchingLunarDirect = $state(false)

  type LunarBadlionStatus = {
    lunar_installed: boolean
    badlion_installed: boolean
    can_direct_launch_lunar: boolean
    lunar_installed_versions: string[]
    lunar_exe_path: string | null
    badlion_exe_path: string | null
  }

  let status = $state<LunarBadlionStatus | null>(null)
  let lunarSelectedVersion = $state('')

  const lbInstalled = $derived(!!status && (status.lunar_installed || status.badlion_installed))
  const canDirect = $derived(!!status?.can_direct_launch_lunar)

  /** Port of `RefreshJavaClients` (and the detect half of `OpenJavaClientsAsync`). */
  async function refreshJavaClients() {
    try {
      const next = await invoke<LunarBadlionStatus>('detect_lunar_badlion')
      status = next
      if (!next.lunar_installed_versions.includes(lunarSelectedVersion))
        lunarSelectedVersion = next.lunar_installed_versions[0] ?? ''
    } catch (e) {
      onToast(String(e), 'error')
    }
  }

  /** Port of `LaunchLunar` - starts the installed Lunar launcher app. */
  async function launchLunar() {
    const exe = status?.lunar_exe_path ?? status?.badlion_exe_path
    if (!exe) {
      onToast("Lunar Client isn't installed. Get it from lunarclient.com.", 'error')
      return
    }
    try {
      await invoke('launch_client_exe', { exePath: exe })
      onToast('Lunar Client launching…', 'success')
    } catch (e) {
      onToast(String(e), 'error')
    }
  }

  /** Port of `LaunchLunarDirect` - spawns the JVM straight against Lunar's
   *  offline multiversion install, bypassing the Lunar Electron app. */
  async function launchLunarDirect() {
    if (!lunarSelectedVersion || launchingLunarDirect) return
    launchingLunarDirect = true
    try {
      await invoke('launch_lunar_direct', { versionId: lunarSelectedVersion })
      onToast(`Launching Lunar ${lunarSelectedVersion}…`, 'success')
    } catch (e) {
      onToast(String(e), 'error')
    } finally {
      launchingLunarDirect = false
    }
  }

  function openUrl(url: string) {
    invoke('open_url', { url }).catch((e) => onToast(String(e), 'error'))
  }

  refreshJavaClients()
</script>

<PanelHeader title="Launchers" {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn"
      data-tooltip="Re-detect"
      aria-label="Re-detect"
      onmousedown={(e) => e.stopPropagation()}
      onclick={refreshJavaClients}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="mcv-info-bar">
  <i class="fa-solid fa-circle-info"></i>
  <span
    >Vanilla launches through Glacier directly. Lunar and Badlion launch via their own installed apps
    — we don't proxy through them.</span
  >
</div>

<div class="panel-body">
  <span class="panel-section-label">Built-in</span>
  <div class="credits-card credits-card-glacier">
    <div class="credits-card-icon" style="background:var(--accent-bg); color:var(--accent);">
      <i class="fa-brands fa-java"></i>
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name">Vanilla (Glacier built-in)</span>
      <span class="credits-card-sub"
        >Launches Mojang's Java client directly. Pick a version in the Versions panel.</span
      >
    </div>
    <div class="credits-card-actions">
      <button
        class="vcs-btn"
        onmousedown={(e) => e.stopPropagation()}
        onclick={() => onNavigate('versions')}
      >
        <i class="fa-solid fa-box-archive"></i> Versions
      </button>
    </div>
  </div>

  <div class="credits-card">
    <div class="credits-card-icon" style="background:var(--accent-bg); color:var(--accent);">
      <i class="fa-solid fa-snowflake"></i>
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name">Glacier Client</span>
      <span class="credits-card-sub">
        {#if glacierLoading}
          Loading manifest…
        {:else if glacierError}
          <span class="client-card-note">{glacierError}</span>
        {:else if glacierLatest}
          {glacierLatest.name} · {glacierLatest.fabric ? 'Fabric' : glacierLatest.forge ? 'Forge' : 'Unknown'}
          {glacierInstalled ? '· Installed' : ''}
        {:else}
          No Glacier Client releases published yet.
        {/if}
      </span>
    </div>
    <div class="credits-card-actions">
      {#if glacierBusy}
        <InlineProgress progress={glacierProgress} showSpinner />
      {:else if glacierError}
        <button class="vcs-btn vcs-btn-ghost" onclick={() => loadGlacierManifest(true)}>
          <i class="fa-solid fa-rotate"></i> Retry
        </button>
      {:else if glacierLatest}
        {#if glacierInstalled}
          <button class="vcs-btn" onmousedown={(e) => e.stopPropagation()} onclick={launchGlacier}>
            <i class="fa-solid fa-play"></i> Launch
          </button>
          <button
            class="vcs-btn vcs-btn-ghost"
            onmousedown={(e) => e.stopPropagation()}
            onclick={uninstallGlacier}
            data-tooltip="Uninstall" aria-label="Uninstall"
          ><i class="fa-solid fa-trash"></i></button>
        {:else}
          <button class="vcs-btn" onmousedown={(e) => e.stopPropagation()} onclick={installGlacier}>
            <i class="fa-solid fa-download"></i> Install
          </button>
        {/if}
      {/if}
    </div>
  </div>

  <span class="panel-section-label">Third-party</span>

  <div class="credits-card">
    <div class="credits-card-icon" style="background:#06b6d422; color:#22d3ee;">
      <i class="fa-solid fa-moon"></i>
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name"
        >Lunar Client <span style="opacity:0.55; font-weight:500;">incl. Badlion</span></span
      >
      <span class="credits-card-sub">
        {#if lbInstalled && canDirect}
          Direct launch ready · {status?.lunar_installed_versions.length} version(s) installed offline
        {:else if lbInstalled}
          Detected · {status?.lunar_exe_path ?? status?.badlion_exe_path} — first launch may show the
          Lunar splash
        {:else}
          Not installed — download from lunarclient.com (Lunar's purchase of Badlion lets one app
          cover both)
        {/if}
      </span>
    </div>
    <div class="credits-card-actions">
      {#if lbInstalled && canDirect}
        <select
          class="setting-select"
          bind:value={lunarSelectedVersion}
          onclick={(e) => e.stopPropagation()}
        >
          {#each status?.lunar_installed_versions ?? [] as lv (lv)}
            <option value={lv}>{lv}</option>
          {/each}
        </select>
        <button
          class="vcs-btn"
          disabled={launchingLunarDirect}
          onmousedown={(e) => e.stopPropagation()}
          onclick={launchLunarDirect}
          data-tooltip="Direct launch"
        >
          {#if launchingLunarDirect}
            <span class="spinner"></span>
          {:else}
            <i class="fa-solid fa-bolt"></i>
          {/if}
          Launch
        </button>
      {:else if lbInstalled}
        <button class="vcs-btn" onmousedown={(e) => e.stopPropagation()} onclick={launchLunar}>
          <i class="fa-solid fa-play"></i> Launch
        </button>
      {:else}
        <button
          class="vcs-btn vcs-btn-ghost"
          onmousedown={(e) => e.stopPropagation()}
          onclick={() => openUrl('https://www.lunarclient.com')}
        >
          <i class="fa-solid fa-arrow-up-right-from-square"></i> Get it
        </button>
      {/if}
    </div>
  </div>
</div>

