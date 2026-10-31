<script lang="ts">
  import { t } from './i18nStore.svelte'
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
    activeClient = 'Vanilla',
    onSelectClient = () => {},
  }: {
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
    /** The Java client the top-bar chip shows (App's `activeJavaClient`). */
    activeClient?: string
    /** Sets the Java client choice, the same one the top-bar dropdown sets. */
    onSelectClient?: (name: string) => void
  } = $props()

  /** Standalone mods App installs into the active instance on launch (MOD_CLIENTS). */
  const MOD_CARDS = [
    { id: 'Essential', logo: '/images/clients/essential.svg', desc: 'Social, cosmetics and screenshots mod — installed into your instance' },
    { id: 'Dawn', logo: '/images/clients/dawn.svg', desc: "Feather's successor — standalone Fabric mod (1.17.1+)" },
    { id: 'NoRisk', logo: '/images/clients/norisk.svg', desc: 'PvP client — standalone mod for Fabric, Forge and NeoForge' },
  ] as const

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
  /** Live per-version download counts from glacierclient.xyz's own public
   * counter (the same one the website displays) — best-effort, keyed by
   * `tag`. Empty map just means "don't show a count," not an error. */
  let glacierDownloadCounts = $state<Record<string, number>>({})

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
    // Best-effort, never blocks the manifest load or shows an error of its
    // own — an empty result just means no count badge renders.
    try {
      glacierDownloadCounts = await invoke<Record<string, number>>('get_glacier_download_counts')
    } catch {
      glacierDownloadCounts = {}
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
      onToast(t('toast.installFailed', { error: String(e) }), 'error')
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

  type LunarBadlionStatus = {
    lunar_installed: boolean
    badlion_installed: boolean
    can_direct_launch_lunar: boolean
    lunar_installed_versions: string[]
    lunar_exe_path: string | null
    badlion_exe_path: string | null
  }

  let status = $state<LunarBadlionStatus | null>(null)

  const lbInstalled = $derived(!!status && (status.lunar_installed || status.badlion_installed))

  /** Port of `RefreshJavaClients` (and the detect half of `OpenJavaClientsAsync`). */
  async function refreshJavaClients() {
    try {
      const next = await invoke<LunarBadlionStatus>('detect_lunar_badlion')
      status = next
    } catch (e) {
      onToast(String(e), 'error')
    }
  }

  /** Port of `LaunchLunar` - starts the installed Lunar launcher app. */
  async function launchLunar() {
    const exe = status?.lunar_exe_path ?? status?.badlion_exe_path
    if (!exe) {
      onToast(t('toast.lunarClientIsnTInstalledGet'), 'error')
      return
    }
    try {
      await invoke('launch_client_exe', { exePath: exe })
      onToast(t('toast.lunarClientLaunching'), 'success')
    } catch (e) {
      onToast(String(e), 'error')
    }
  }

  function openUrl(url: string) {
    invoke('open_url', { url }).catch((e) => onToast(String(e), 'error'))
  }

  // ---- Horizon (horizon-rust/horizon): native Rust Java 26.3 rewrite ----
  type HorizonBuild = {
    id: string
    name: string
    zipSha256: string
    installedAt: number
    publishedAt: string
    notes: string
    playable: boolean
  }
  type HorizonLatest = {
    name: string
    publishedAt: string
    notes: string
    zipSha256: string
    updateAvailable: boolean
  }
  type HorizonOfficialNotes = { name: string; notes: string }
  type HorizonStatus = {
    installed: boolean
    officialBuild: string | null
    officialNotes: HorizonOfficialNotes | null
    officialNotesError: string | null
    current: HorizonBuild | null
    previous: HorizonBuild | null
    canRollback: boolean
    latest: HorizonLatest | null
    latestError: string | null
    stateError: string | null
    busy: boolean
    running: boolean
    repoUrl: string
    websiteUrl: string
  }

  let horizon = $state<HorizonStatus | null>(null)
  let horizonLoading = $state(false)
  /** Set while an install, update, rollback or uninstall is in flight. */
  let horizonWorking = $state(false)
  let horizonStage = $state('')
  let horizonStageProgress = $state(0)
  let horizonNotesOpen = $state(false)
  /** Which installed build the version picker points at. */
  let horizonPick = $state('')
  let launchingHorizon = $state(false)

  /** Download progress for the Horizon zip, from the shared download queue. */
  const horizonDownloadProgress = $derived(
    queue.find((e) => e.label === 'Horizon' && e.status === 'Downloading')?.progress ?? 0,
  )
  const horizonBusy = $derived(horizonWorking || !!horizon?.busy)
  const horizonRunning = $derived(!!horizon?.running || launchingHorizon)
  const horizonPickedPrevious = $derived(
    !!horizon?.previous && horizonPick === horizon.previous.id,
  )

  /** Reads Horizon's install state and the published nightly. */
  async function loadHorizon() {
    horizonLoading = true
    try {
      horizon = await invoke<HorizonStatus>('horizon_status')
      horizonPick = horizon.current?.id ?? ''
    } catch (e) {
      onToast(String(e), 'error')
    } finally {
      horizonLoading = false
    }
  }

  /** Runs one of the long Horizon operations and keeps the card in step. */
  async function runHorizon(
    command: 'horizon_install' | 'horizon_update' | 'horizon_rollback',
    doneMessage: string,
  ) {
    if (horizonBusy) return
    horizonWorking = true
    horizonStage = ''
    horizonStageProgress = 0
    try {
      horizon = await invoke<HorizonStatus>(command)
      horizonPick = horizon.current?.id ?? ''
      onToast(doneMessage, 'success')
    } catch (e) {
      onToast(String(e), 'error')
      await loadHorizon()
    } finally {
      horizonWorking = false
    }
  }

  async function launchHorizon() {
    if (horizonBusy || horizonRunning) return
    launchingHorizon = true
    try {
      await invoke<number>('horizon_launch')
      onToast('Horizon is starting…', 'success')
      await loadHorizon()
    } catch (e) {
      onToast(String(e), 'error')
    } finally {
      launchingHorizon = false
    }
  }

  async function uninstallHorizon() {
    if (horizonBusy || horizonRunning) return
    if (!window.confirm('Uninstall Horizon? Its builds are removed. Your worlds and options in its game folder are kept.')) {
      return
    }
    horizonWorking = true
    try {
      await invoke('horizon_uninstall')
      onToast('Horizon removed. Worlds were kept.', 'info')
      await loadHorizon()
    } catch (e) {
      onToast(String(e), 'error')
    } finally {
      horizonWorking = false
    }
  }

  onMount(() => {
    listen<{ stage: string; percent: number }>('horizon-progress', (e) => {
      horizonStage = e.payload.stage
      horizonStageProgress = e.payload.percent
    }).then((fn) => {
      unlistenHorizon.push(fn)
    })
    listen<{ instance_id: string; status: string }>('java-process-changed', (e) => {
      if (e.payload.instance_id === 'horizon' && e.payload.status === 'exited') {
        loadHorizon()
      }
    }).then((fn) => {
      unlistenHorizon.push(fn)
    })
  })
  const unlistenHorizon: UnlistenFn[] = []
  onDestroy(() => unlistenHorizon.forEach((fn) => fn()))

  loadHorizon()

  refreshJavaClients()
</script>

<style>
  .client-icon-accent {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .client-icon-brand {
    background: transparent;
    padding: 0;
    overflow: hidden;
  }
  .client-brand-logo {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: inherit;
  }
  .client-logo {
    width: 18px;
    height: 18px;
    object-fit: contain;
  }
  .client-variant {
    color: var(--text-dim);
    font-weight: 500;
  }
  .client-card-note {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--red);
  }
  .client-update-flag {
    color: var(--accent-hover);
    font-weight: 600;
  }
  .horizon-build-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }
  .horizon-notes-toggle {
    align-self: flex-start;
    margin-top: 6px;
  }
  .horizon-notes {
    white-space: pre-wrap;
    max-height: 220px;
    overflow: auto;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-dim);
    margin-top: 6px;
    padding: 10px 12px;
    border-radius: var(--r-sm);
    background: var(--bg-inset);
  }
  .horizon-notes b {
    color: var(--text);
  }
</style>

<PanelHeader title={t('panel.launchers')} {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn"
      data-tooltip={t('tt.reDetect')}
      aria-label={t('tt.reDetect')}
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
  <span class="panel-section-label">{t('x.builtIn')}</span>
  <div class="credits-card credits-card-glacier">
    <div class="credits-card-icon client-icon-accent">
      <img class="chip-icon client-logo" src="images/mc-java-logo.svg" alt="">
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
    <div class="credits-card-icon client-icon-accent">
      <img class="chip-icon client-logo" src="images/icon.svg" alt="">
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
          {#if glacierDownloadCounts[glacierLatest.tag] > 0}
            · {glacierDownloadCounts[glacierLatest.tag].toLocaleString()} downloads
          {/if}
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
          <i class="fa-solid fa-rotate"></i> {t('common.retry')}
        </button>
      {:else if glacierLatest}
        {#if glacierInstalled}
          <button class="vcs-btn" onmousedown={(e) => e.stopPropagation()} onclick={launchGlacier}>
            <i class="fa-solid fa-play"></i> {t('common.launch')}
          </button>
          <button
            class="vcs-btn vcs-btn-ghost"
            onmousedown={(e) => e.stopPropagation()}
            onclick={uninstallGlacier}
            data-tooltip={t('tt.uninstall')} aria-label={t('tt.uninstall')}
          ><i class="fa-solid fa-trash"></i></button>
        {:else}
          <button class="vcs-btn" onmousedown={(e) => e.stopPropagation()} onclick={installGlacier}>
            <i class="fa-solid fa-download"></i> {t('common.install')}
          </button>
        {/if}
      {/if}
    </div>
  </div>

  <span class="panel-section-label">{t('x.thirdParty')}</span>

  {#if lbInstalled}
  <div class="credits-card">
    <div class="credits-card-icon client-icon-brand">
      <img class="client-brand-logo" src="/images/clients/lunar.svg" alt="" />
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name"
        >Lunar Client <span class="client-variant">incl. Badlion</span></span
      >
      <span class="credits-card-sub" title={status?.lunar_exe_path ?? status?.badlion_exe_path ?? ''}>
        Opens the Lunar launcher, which signs you in and starts the game
      </span>
    </div>
    <div class="credits-card-actions">
      <button class="vcs-btn" onmousedown={(e) => e.stopPropagation()} onclick={launchLunar}>
        <i class="fa-solid fa-play"></i> {t('common.launch')}
      </button>
    </div>
  </div>
  {/if}

  {#if horizon?.installed}
  <div class="credits-card horizon-card">
    <div class="credits-card-icon client-icon-brand">
      <img class="client-brand-logo" src="/images/clients/horizon.svg" alt="" />
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name"
        >Horizon <span class="client-variant">native Java 26.3 rewrite · nightly</span></span
      >
      <span class="credits-card-sub">
        {#if horizonLoading && !horizon}
          Checking Horizon…
        {:else if horizon?.officialBuild}
          Using Horizon launcher's install · {horizon.officialBuild}
          {#if horizon.latest?.updateAvailable}· <b class="client-update-flag">Update available in Horizon launcher</b>{/if}
        {:else if horizon?.installed && horizon.current}
          Installed · {horizon.current.name}
          {#if horizon.latest?.updateAvailable}· <b class="client-update-flag">update available</b>{/if}
        {:else}
          Not installed. Horizon plays Minecraft Java 26.3 natively. It needs a Microsoft account
          that owns Minecraft: Java Edition, and Java mods don't run in it.
        {/if}
      </span>
      {#if horizonWorking || horizonStage}
        <span class="client-card-note">{horizonStage}</span>
      {/if}
      {#if horizon?.latestError}
        <span class="client-card-note"><i class="fa-solid fa-triangle-exclamation"></i> Could not check for updates: {horizon.latestError}</span>
      {/if}
      {#if horizon?.stateError}
        <span class="client-card-note">{horizon.stateError}</span>
      {/if}
      {#if horizon && !horizon.officialBuild && horizon.installed && horizon.previous && horizonPick}
        <span class="credits-card-sub horizon-build-row">
          Build
          <select class="setting-select" bind:value={horizonPick} disabled={horizonBusy}>
            <option value={horizon.current?.id}>{horizon.current?.name} (current)</option>
            <option value={horizon.previous.id}>{horizon.previous.name} (previous)</option>
          </select>
          {#if horizonPickedPrevious}
            <button class="vcs-btn vcs-btn-ghost" disabled={horizonBusy} onclick={() => runHorizon('horizon_rollback', 'Switched Horizon to the previous build')}>
              <i class="fa-solid fa-clock-rotate-left"></i> Use this build
            </button>
          {/if}
        </span>
      {/if}
      {#if horizon && (horizon.officialBuild || horizon.latest?.notes || horizon.current?.notes)}
        <button class="vcs-btn vcs-btn-ghost horizon-notes-toggle" onclick={() => (horizonNotesOpen = !horizonNotesOpen)}>
          <i class="fa-solid fa-scroll"></i> {horizonNotesOpen ? 'Hide patch notes' : 'Patch notes'}
        </button>
        {#if horizonNotesOpen}
          <div class="horizon-notes">
            {#if horizon.officialBuild}
              {#if horizon.officialNotes?.notes}
                <b>{horizon.officialNotes.name}</b>{'\n'}{horizon.officialNotes.notes}
              {:else}
                <b>{horizon.officialNotes?.name ?? horizon.officialBuild}</b>{'\n'}No patch notes found for this build.
                {#if horizon.officialNotesError}{'\n'}({horizon.officialNotesError}){/if}
              {/if}
            {:else if horizon?.latest?.updateAvailable && horizon.latest.notes}
              <b>{horizon.latest.name}</b>{'\n'}{horizon.latest.notes}
            {:else if horizon?.current?.notes}
              <b>{horizon.current.name}</b>{'\n'}{horizon.current.notes}
            {:else}
              No patch notes for this build.
            {/if}
          </div>
        {/if}
      {/if}
    </div>
    <div class="credits-card-actions">
      {#if horizonWorking || (horizonLoading && !horizon)}
        <InlineProgress progress={horizonDownloadProgress || horizonStageProgress / 100} showSpinner />
      {:else if horizon?.installed}
        <button class="vcs-btn" disabled={horizonBusy || horizonRunning} onmousedown={(e) => e.stopPropagation()} onclick={launchHorizon}>
          <i class="fa-solid fa-play"></i> {horizonRunning ? 'Running' : t('common.launch')}
        </button>
        {#if horizon.latest?.updateAvailable && !horizon.officialBuild}
          <button class="vcs-btn vcs-btn-ghost" disabled={horizonBusy || horizonRunning} onclick={() => runHorizon('horizon_update', 'Horizon updated')}>
            <i class="fa-solid fa-arrow-up"></i> Update
          </button>
        {/if}
        <button class="vcs-btn vcs-btn-ghost" onclick={() => openUrl('https://playhorizon.dev')} data-tooltip="playhorizon.dev" aria-label="playhorizon.dev">
          <i class="fa-solid fa-globe"></i>
        </button>
        {#if !horizon.officialBuild}
        <button class="vcs-btn vcs-btn-ghost" disabled={horizonBusy || horizonRunning} onclick={uninstallHorizon} data-tooltip={t('tt.uninstall')} aria-label={t('tt.uninstall')}>
          <i class="fa-solid fa-trash"></i>
        </button>
        {/if}
      {:else if horizon}
        <button class="vcs-btn" disabled={horizonBusy} onclick={() => runHorizon('horizon_install', 'Horizon installed')}>
          <i class="fa-solid fa-download"></i> {t('common.install')}
        </button>
        <button class="vcs-btn vcs-btn-ghost" onclick={() => openUrl('https://github.com/horizon-rust/horizon')} data-tooltip="GitHub" aria-label="GitHub">
          <i class="fa-brands fa-github"></i>
        </button>
      {/if}
    </div>
  </div>
  {/if}

  {#each MOD_CARDS as card (card.id)}
  <div class="credits-card">
    <div class="credits-card-icon client-icon-brand">
      <img class="client-brand-logo" src={card.logo} alt="" />
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name">{card.id}</span>
      <span class="credits-card-sub">{card.desc}</span>
    </div>
    <div class="credits-card-actions">
      {#if activeClient === card.id}
        <button class="vcs-btn vcs-btn-ghost" disabled>
          <i class="fa-solid fa-check"></i> Active
        </button>
      {:else}
        <button class="vcs-btn" onmousedown={(e) => e.stopPropagation()} onclick={() => onSelectClient(card.id)}>
          <i class="fa-solid fa-check"></i> Use
        </button>
      {/if}
    </div>
  </div>
  {/each}
</div>

