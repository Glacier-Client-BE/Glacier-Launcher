<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * 1:1 port of the `mcversions` panel ("MC Versions — Vanilla Version
   * Switcher") in `Pages/Home.razor`, together with its code-behind in
   * `Pages/Home.Panels.cs`: `OpenMcVersions` / `RefreshMcVersionsAsync` /
   * `LoadMcVersionsAsync` / `HandleMcVersionDownload` /
   * `HandleMcVersionSwitch` / `HandleMcVersionDelete` / the
   * `McVersionActions` RenderFragment, `FilteredMcVersions`
   * (Home.razor.cs:101-121) and `Models/VanillaVersion.cs`.
   *
   * Backed by gl_bedrock::wu through list_vanilla_bedrock_versions /
   * vanilla_bedrock_version_installed / download_vanilla_bedrock_version /
   * delete_vanilla_bedrock_version / switch_vanilla_bedrock_version.
   *
   * No scoped <style> block: every class below is defined in app.css (a
   * verbatim copy of the original stylesheet), and Svelte's scoping would
   * make local rules win over it.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { invokeCached } from './invokeCache'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import IconBtn from './components/IconBtn.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'

  let {
    onBack,
    onNavigate,
    onToast,
  }: {
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  function toast(message: string, kind = 'info') {
    onToast?.(message, kind)
  }

  /** Port of `Models/VanillaVersion.cs`. */
  type VanillaVersion = {
    version: string
    update_id: string
    package_name: string
    channel: string
    size_bytes: number
    /** `"gdk"` for community-archive entries; `IsGdk` is `Source == "gdk"`. */
    source: string
    friendly_version: string
    download_url: string
    sha256: string
    is_downloaded: boolean
    is_downloading: boolean
    is_switching: boolean
    is_active: boolean
    progress: number
    error_message: string
  }

  type WuVersion = Pick<
    VanillaVersion,
    | 'version'
    | 'update_id'
    | 'package_name'
    | 'channel'
    | 'size_bytes'
    | 'source'
    | 'friendly_version'
    | 'download_url'
    | 'sha256'
  >

  /** Port of `VanillaVersion.IsGdk`. */
  function isGdk(v: VanillaVersion): boolean {
    return v.source === 'gdk'
  }

  // Legacy Bedrock versions are all "1.x.y.z"; the newer calendar-based
  // scheme (e.g. "26.1") uses a non-1 major, so we prefix those with "v" to
  // match Mojang's own marketing naming. (`VanillaVersion.DisplayName`.)
  function displayName(v: VanillaVersion): string {
    const baseName = v.version.startsWith('1.') ? v.version : `v${v.version}`
    return v.channel === 'Preview' ? `${baseName} Preview` : baseName
  }

  /** Port of `VanillaVersion.SizeLabel`. */
  function sizeLabel(bytes: number): string {
    if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`
    if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(0)} MB`
    if (bytes > 0) return `${(bytes / 1024).toFixed(0)} KB`
    return ''
  }

  let mcVersionsList = $state<VanillaVersion[]>([])
  /** Installed build, for the GDK-cutoff note in the info popover. */
  let installedVersion = $state<{ installed: boolean; display_version: string }>({
    installed: false,
    display_version: '',
  })
  invoke<{ installed: boolean; display_version: string; is_preview: boolean }>(
    'installed_bedrock_version',
  )
    .then((v) => (installedVersion = v))
    .catch(() => {})

  /** Port of `mcvInfoOpen` - the About-versions popover. */
  let mcvInfoOpen = $state(false)

  let isLoadingMcVersions = $state(false)
  let mcVersionsError = $state('')
  let mcVersionsFilter = $state('')
  let mcVersionsChannel = $state('all')

  /** Port of `FilteredMcVersions` (Home.razor.cs:101-121). */
  const filteredMcVersions = $derived.by(() => {
    let q = mcVersionsList
    if (mcVersionsChannel === 'release') q = q.filter((v) => v.channel !== 'Preview')
    else if (mcVersionsChannel === 'preview') q = q.filter((v) => v.channel === 'Preview')
    if (mcVersionsFilter.trim()) {
      const filter = mcVersionsFilter.trim().toLowerCase()
      q = q.filter((v) => v.version.toLowerCase().includes(filter))
    }
    return q
  })

  const activeList = $derived(filteredMcVersions.filter((v) => v.is_active))
  const restList = $derived(filteredMcVersions.filter((v) => !v.is_active))

  /** Port of `LoadMcVersionsAsync` / `RefreshMcVersionsAsync`. */
  async function refreshMcVersionsAsync() {
    isLoadingMcVersions = true
    mcVersionsError = ''
    let active = ''
    try {
      const settings = await invoke<{ ActiveVanillaVersion?: string }>('get_settings')
      active = settings?.ActiveVanillaVersion ?? ''
    } catch {
      active = ''
    }
    try {
      // Cached briefly: the Windows Update list barely changes between opens.
      const versions = await invokeCached<WuVersion[]>('list_vanilla_bedrock_versions', undefined, 10 * 60 * 1000)
      mcVersionsList = await Promise.all(
        versions.map(async (v) => ({
          ...v,
          is_downloaded: await invoke<boolean>('vanilla_bedrock_version_installed', {
            version: v.version,
          }).catch(() => false),
          is_downloading: false,
          is_switching: false,
          is_active: v.version === active,
          progress: 0,
          error_message: '',
        })),
      )
    } catch (e) {
      mcVersionsError = String(e)
    } finally {
      isLoadingMcVersions = false
    }
  }

  // ── Download progress ────────────────────────────────────────────────
  // The queue pushes `downloads-changed` (the port of
  // `DownloadService.Changed`) carrying the whole snapshot, so this listens
  // rather than polling. Each Bedrock job is labelled "Bedrock <version>".
  type DownloadEntry = { id: string; label: string; status: string; progress: number }

  let downloadEntries = $state<DownloadEntry[]>([])
  let unlistenDownloads: UnlistenFn | undefined

  function entryFor(v: VanillaVersion): DownloadEntry | undefined {
    return downloadEntries.find(
      (e) => e.label === `Bedrock ${v.version}` && (e.status === 'Downloading' || e.status === 'Queued'),
    )
  }

  let downloadsDisposed = false
  onMount(() => {
    listen<DownloadEntry[]>('downloads-changed', (e) => {
      downloadEntries = e.payload
    }).then((fn) => {
      // Destroyed before the listener resolved: release it now, don't leak it.
      if (downloadsDisposed) fn()
      else unlistenDownloads = fn
    })
  })
  onDestroy(() => {
    downloadsDisposed = true
    unlistenDownloads?.()
  })

  /** Port of `HandleMcVersionDownload`. */
  async function handleMcVersionDownload(v: VanillaVersion) {
    if (v.is_downloading) return
    v.is_downloading = true
    v.error_message = ''
    v.progress = 0
    try {
      await invoke('download_vanilla_bedrock_version', {
        version: {
          version: v.version,
          update_id: v.update_id,
          package_name: v.package_name,
          size_bytes: v.size_bytes,
          channel: v.channel,
          // Archive entries carry their URL and hash on the row; without
          // these the backend has nothing to download or verify against.
          source: v.source,
          friendly_version: v.friendly_version,
          download_url: v.download_url,
          sha256: v.sha256,
        },
      })
      v.is_downloaded = true
      toast(`Minecraft ${v.version} downloaded!`, 'success')
    } catch (e) {
      const message = String(e)
      if (/cancel/i.test(message)) toast(`Minecraft ${v.version} download cancelled.`, 'info')
      else v.error_message = message
    } finally {
      v.is_downloading = false
      v.progress = 0
    }
  }

  function cancelMcVersionDownload(v: VanillaVersion) {
    const entry = entryFor(v)
    if (entry) invoke('cancel_download', { id: entry.id }).catch(() => {})
  }

  /** Port of `HandleMcVersionSwitch`. */
  async function handleMcVersionSwitch(v: VanillaVersion) {
    v.is_switching = true
    v.error_message = ''
    try {
      await invoke('switch_vanilla_bedrock_version', { version: v.version })
      const settings = await invoke<Record<string, unknown>>('get_settings')
      await invoke('set_settings', { settings: { ...settings, ActiveVanillaVersion: v.version } })
      for (const mv of mcVersionsList) mv.is_active = mv.version === v.version
      toast(`Switched to Minecraft ${v.version}!`, 'success')
    } catch (e) {
      v.error_message = String(e)
      toast(String(e), 'error')
    } finally {
      v.is_switching = false
    }
  }

  /** Port of `HandleMcVersionDelete`. */
  async function handleMcVersionDelete(v: VanillaVersion) {
    try {
      await invoke('delete_vanilla_bedrock_version', { version: v.version })
      v.is_downloaded = false
      if (v.is_active) for (const mv of mcVersionsList) mv.is_active = false
    } catch (e) {
      v.error_message = String(e)
    }
  }

  refreshMcVersionsAsync()
</script>

{#snippet mcVersionActions(v: VanillaVersion)}
  {#if v.is_downloading}
    <InlineProgress progress={v.progress || (entryFor(v)?.progress ?? 0)} onCancel={() => cancelMcVersionDownload(v)} />
  {:else if v.is_switching}
    <div class="version-actions">
      <IconBtn disabled loading title="Switching" />
    </div>
  {:else}
    <div class="version-actions">
      {#if v.is_downloaded}
        <IconBtn
          ghost
          icon="fa-solid fa-trash"
          title="Delete"
          onClick={() => handleMcVersionDelete(v)}
        />
        {#if !v.is_active}
          <IconBtn
            class="mcv-switch-btn"
            icon="fa-solid fa-right-left"
            title="Switch to this version"
            onClick={() => handleMcVersionSwitch(v)}
          />
        {/if}
      {:else}
        <IconBtn
          icon="fa-solid fa-download"
          title="Download"
          onClick={() => handleMcVersionDownload(v)}
        />
      {/if}
    </div>
  {/if}
{/snippet}

<PanelHeader title={t('panel.mcVersions')} {onBack}>
  {#snippet extraActions()}
    <div style="position:relative;">
      <button
        type="button"
        class="panel-icon-btn {mcvInfoOpen ? 'active' : ''}"
        onclick={() => (mcvInfoOpen = !mcvInfoOpen)}
        onmousedown={(e) => e.stopPropagation()}
        data-tooltip={t('tt.aboutVersions')}
        aria-label={t('tt.aboutVersions')}
      >
        <i class="fa-solid fa-circle-info"></i>
      </button>
      {#if mcvInfoOpen}
        <div class="mcv-info-pop" role="presentation" onclick={(e) => e.stopPropagation()}>
          <p>
            <i class="fa-solid fa-circle-info"></i>
            Switch between Minecraft Bedrock versions. Sideloading a specific version
            requires <b>{t('x.developerMode')}</b>; the Microsoft Store option below does not.
          </p>
          <p class="mcv-info-pop-warn">
            <i class="fa-solid fa-triangle-exclamation"></i>
            Microsoft's own appx/Windows-Update channel stops at <b>1.21.120</b> — after
            the <b>GDK</b> move, newer builds
            ({installedVersion.installed ? `like your ${installedVersion.display_version}` : '1.26 / v26 and up'})
            were never published there. Glacier fills the gap with the
            <b>community GDK unpack archive</b>, which mirrors those builds pre-extracted;
            each download is checked against the archive's published SHA-256. Rows sourced
            that way are marked <b>GDK</b>. They're third-party mirrors, not a Microsoft
            endpoint — for a first-party install use
            <b>Install latest from Microsoft Store</b> below.
          </p>
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="panel-icon-btn {isLoadingMcVersions ? 'spinning' : ''}"
      disabled={isLoadingMcVersions}
      onclick={refreshMcVersionsAsync}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>



<SubTabs strip="versions-instances" active="versions" onNavigate={(k) => onNavigate?.(k)} />

<!-- ── Microsoft Store install (BedrockUpdater method) ──
     `InstallFromStoreAsync` is backed by `StoreInstallService`, which wraps
     WinRT's `AppInstallManager`
     (`Windows.ApplicationModel.Store.Preview.InstallControl`). The `windows`
     crate ships no bindings for that namespace — it is a restricted preview
     contract, and there is no
     `ApplicationModel_Store_Preview_InstallControl` feature to enable — so
     there is no honest way to drive an in-app install with real progress
     from Rust today. Both buttons render disabled and say so, rather than
     pretending to work or silently substituting a different behaviour (e.g.
     shelling out to a Store deep link, which installs without any of the
     progress reporting the original shows). See PARITY.md. -->
<div class="mcv-store-row">
  <div class="mcv-store-meta">
    <span class="mcv-store-title"
      ><i class="fa-brands fa-microsoft" style="margin-right:6px;"></i>Install latest from Microsoft
      Store</span
    >
    <span class="mcv-store-sub">No Developer Mode required. Installs the current Release or Preview.</span>
  </div>
  <button
    type="button"
    class="vcs-btn"
    disabled
    data-tooltip={t('tt.unavailableNeedsWinrtAppinstallmanagerWhichHas')}
    onmousedown={(e) => e.stopPropagation()}>{t('x.release')}</button
  >
  <button
    type="button"
    class="vcs-btn vcs-btn-ghost"
    disabled
    data-tooltip={t('tt.unavailableNeedsWinrtAppinstallmanagerWhichHas')}
    onmousedown={(e) => e.stopPropagation()}>{t('x.preview')}</button
  >
</div>

<div class="panel-search-wrap">
  <i class="fa-solid fa-magnifying-glass"></i>
  <input
    class="panel-search-input"
    placeholder={t('ph.filterMcVersions')}
    bind:value={mcVersionsFilter}
    onkeydown={(e) => e.stopPropagation()}
  />
</div>

<div class="mcv-channel-tabs">
  <button
    type="button"
    class="mcv-channel-tab {mcVersionsChannel === 'all' ? 'active' : ''}"
    onclick={() => (mcVersionsChannel = 'all')}
    onmousedown={(e) => e.stopPropagation()}>{t('x.all')}</button
  >
  <button
    type="button"
    class="mcv-channel-tab {mcVersionsChannel === 'release' ? 'active' : ''}"
    onclick={() => (mcVersionsChannel = 'release')}
    onmousedown={(e) => e.stopPropagation()}>{t('x.releases')}</button
  >
  <button
    type="button"
    class="mcv-channel-tab {mcVersionsChannel === 'preview' ? 'active' : ''}"
    onclick={() => (mcVersionsChannel = 'preview')}
    onmousedown={(e) => e.stopPropagation()}>{t('x.previews')}</button
  >
</div>

<div class="panel-body">
  {#if isLoadingMcVersions}
    <ListSkeleton rows={6} />
  {:else if mcVersionsList.length === 0 && mcVersionsError}
    <div class="versions-loading versions-error">
      <i class="fa-solid fa-triangle-exclamation"></i>
      <span>{mcVersionsError}</span>
      <button class="vcs-btn" style="margin-top:8px;" onclick={refreshMcVersionsAsync}>
        <i class="fa-solid fa-rotate"></i> {t('common.retry')}
      </button>
    </div>
  {:else if mcVersionsList.length === 0}
    <div class="versions-loading"><span class="version-sub">{t('x.noVersionsFound')}</span></div>
  {:else}
    {#if mcVersionsError && mcVersionsList.length > 0}
      <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>{mcVersionsError}</span></div>
    {/if}

    {#if filteredMcVersions.length === 0}
      <div class="versions-loading"><span class="version-sub">{t('empty.noResultsFound')}</span></div>
    {/if}

    {#if activeList.length > 0}
      <span class="panel-section-label">{t('label.active')}</span>
      <div class="client-card-cols">
      {#each activeList as v (`${v.version}|${v.channel}`)}
        <div class="client-card client-active">
          <div class="client-card-header">
            <div class="client-card-icon oderso-icon"><img src="images/mc-logo.svg" alt=""></div>
            <div class="client-card-meta">
              <span class="client-card-name">Minecraft {displayName(v)}</span>
              <span class="client-card-sub">
                <span class="tag-active">{t('label.active')}</span>
                {#if isGdk(v)}
                  <span class="mcv-source-tag" data-tooltip={t('tt.communityGdkUnpackArchive')}>GDK</span>
                {/if}
              </span>
            </div>
            {@render mcVersionActions(v)}
          </div>
          {#if v.error_message}
            <p class="client-card-desc error-text">{v.error_message}</p>
          {:else if v.is_downloaded}
            <p class="client-card-desc">Ready · {sizeLabel(v.size_bytes)}</p>
          {/if}
        </div>
      {/each}
      </div>
      <span class="panel-section-label">{t('x.allVersions')}</span>
    {/if}

    <div class="client-card-cols">
    {#each restList as v (`${v.version}|${v.channel}`)}
      <div class="client-card {v.error_message ? 'error-row' : ''}">
        <div class="client-card-header">
          <div class="client-card-icon oderso-icon"><img src="images/mc-logo.svg" alt=""></div>
          <div class="client-card-meta">
            <span class="client-card-name">Minecraft {displayName(v)}</span>
            {#if isGdk(v)}
              <span class="client-card-sub">
                <span
                  class="mcv-source-tag"
                  data-tooltip={t('tt.communityGdkUnpackArchiveVerifiedAgainst')}
                  >GDK</span
                >
              </span>
            {/if}
          </div>
          {@render mcVersionActions(v)}
        </div>
        {#if v.error_message}
          <p class="client-card-desc error-text">{v.error_message}</p>
        {:else if v.is_downloaded && !v.is_downloading}
          <p class="client-card-desc">Ready · {sizeLabel(v.size_bytes)}</p>
        {:else if !v.is_downloaded && !v.is_downloading}
          <p class="client-card-desc">{sizeLabel(v.size_bytes)}</p>
        {/if}
      </div>
    {/each}
    </div>
  {/if}
</div>
