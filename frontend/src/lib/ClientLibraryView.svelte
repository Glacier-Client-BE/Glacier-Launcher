<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * Literal port of the `clientlibrary` panel — Home.razor:4022-4185, with
   * behaviour from Pages/Home.ClientLibrary.cs (108 lines) and
   * Services/ClientLibraryService.cs.
   *
   * This is the GitHub-backed community catalog (Indigo-Tools/ClientLibrary) —
   * distinct from the "Utilities > Library" tab, which manages the user's own
   * local .dll/.exe files.
   *
   * Same DOM order, class names, tooltips and strings as the original. The
   * three distinct states are preserved exactly: loading
   * (`_clLoading && _clPacks.Count == 0`), error
   * (`_clError != "" && _clPacks.Count == 0`) and no-match
   * (`clFiltered.Count == 0`).
   *
   * `instanceDir` of "" means "the live game install" — the backend resolves
   * the real per-account com.mojang path itself, so it is NOT a disabled state.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { onDestroy, onMount } from 'svelte'
  import SubTabs from './components/SubTabs.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'

  let {
    instanceDir = '',
    onBack,
    onNavigate,
    onToast,
  }: {
    instanceDir?: string
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  /** Port of `ClientLibraryDownload`. */
  type ClientLibraryDownload = {
    label: string
    file_name: string
    repo_path: string
    is_extension: boolean
  }
  /** Port of `ClientLibraryPack`. */
  type ClientLibraryPack = {
    version: string
    name: string
    icon_url: string
    banner_url: string
    description: string
    downloads: ClientLibraryDownload[]
  }

  // ── Port of Home.ClientLibrary.cs:12-17. ────────────────────────────────
  let _clPacks = $state<ClientLibraryPack[]>([])
  let _clLoading = $state(false)
  let _clError = $state('')
  let _clFilter = $state('')
  // The search box updates `_clFilter` ~150ms after typing so the derived
  // list is not rebuilt on every keystroke.
  let _clFilterInput = $state('')
  let _clFilterTimer: ReturnType<typeof setTimeout> | undefined
  function onClFilterInput(e: Event) {
    _clFilterInput = (e.currentTarget as HTMLInputElement).value
    clearTimeout(_clFilterTimer)
    _clFilterTimer = setTimeout(() => (_clFilter = _clFilterInput), 150)
  }
  onDestroy(() => clearTimeout(_clFilterTimer))
  let _clVersion = $state('all')
  let _clExpanded = $state<Set<string>>(new Set())
  /** Port of `ClientLibVersions()` (Home.ClientLibrary.cs:46). */
  let _clVersions = $state<string[]>([])

  let _clInstalled = $state<Set<string>>(new Set())
  let _clBusyPath = $state<string | null>(null)
  let _clProgress = $state(0)

  /** Port of `ClientLibraryPack.Key`. */
  function packKey(p: ClientLibraryPack) {
    return `${p.version}/${p.name}`
  }
  /** Port of `ClientLibraryPack.MainDownloads`. */
  function mainDownloads(p: ClientLibraryPack) {
    return p.downloads.filter((d) => !d.is_extension)
  }
  /** Port of `ClientLibraryPack.Extensions`. */
  function extensions(p: ClientLibraryPack) {
    return p.downloads.filter((d) => d.is_extension)
  }

  /** Port of `LoadClientLibraryAsync(bool refresh = false)` (Home.ClientLibrary.cs:27). */
  async function loadClientLibrary(refresh = false) {
    if (_clLoading) return
    _clLoading = true
    _clError = ''
    try {
      _clPacks = await invoke<ClientLibraryPack[]>('get_client_library_packs', { refresh })
      if (_clPacks.length === 0) _clError = 'The library index came back empty.'
      // `ClientLibVersions()` is `ClientLib2.Versions()`, which orders the
      // distinct versions numerically descending ("1.21" above "1.9"); do the
      // same ordering on the backend rather than reimplementing it here.
      try {
        _clVersions = await invoke<string[]>('client_library_versions', { packs: _clPacks })
      } catch {
        _clVersions = Array.from(new Set(_clPacks.map((p) => p.version)))
      }
      await refreshInstalled()
    } catch (e) {
      _clPacks = []
      _clVersions = []
      _clError = String(e)
    } finally {
      _clLoading = false
    }
  }

  /** Port of `IsClientDownloadInstalled` (Home.ClientLibrary.cs:75), pre-resolved. */
  async function refreshInstalled() {
    // Probes run concurrently; sequential awaits made N*M IPC round-trips
    // block the panel on the N5100 for no benefit.
    const downloads = _clPacks.flatMap((p) => p.downloads)
    const results = await Promise.all(
      downloads.map((d) =>
        invoke<boolean>('is_client_library_download_installed', { instanceDir, download: d })
          .then((yes) => (yes ? d.repo_path : null))
          // a probe failure just means "not installed"
          .catch(() => null),
      ),
    )
    _clInstalled = new Set(results.filter((r): r is string => r !== null))
  }


  /** Port of `FilteredClientLibrary` (Home.ClientLibrary.cs:48). */
  const clFiltered = $derived(
    _clPacks.filter((p) => {
      if (_clVersion !== 'all' && p.version !== _clVersion) return false
      const f = _clFilter.trim().toLowerCase()
      if (f) {
        if (!p.name.toLowerCase().includes(f) && !p.description.toLowerCase().includes(f)) return false
      }
      return true
    }),
  )

  /** Port of `IsClientPackExpanded` (Home.ClientLibrary.cs:61). */
  function isClientPackExpanded(p: ClientLibraryPack) {
    return _clExpanded.has(packKey(p))
  }

  /** Port of `ToggleClientPack` (Home.ClientLibrary.cs:63). */
  function toggleClientPack(p: ClientLibraryPack) {
    const next = new Set(_clExpanded)
    const key = packKey(p)
    if (next.has(key)) next.delete(key)
    else next.add(key)
    _clExpanded = next
  }

  /** Port of `SetClientLibVersion` (Home.ClientLibrary.cs:69). */
  function setClientLibVersion(v: string) {
    _clVersion = v
  }

  /** Port of `InstallClientPackAsync` (Home.ClientLibrary.cs:78). */
  async function installClientPack(d: ClientLibraryDownload) {
    if (_clBusyPath) return
    _clBusyPath = d.repo_path
    _clProgress = 0
    try {
      await invoke('install_client_library_download', { instanceDir, download: d })
      _clInstalled = new Set([..._clInstalled, d.repo_path])
      onToast?.(t('toast.installedLabelToPacks', { label: d.label }), 'success')
    } catch (e) {
      onToast?.(t('toast.installFailed', { error: String(e) }), 'error')
    } finally {
      _clBusyPath = null
      _clProgress = 0
    }
  }

  /** Port of `UninstallClientPack` (Home.ClientLibrary.cs:95). */
  async function uninstallClientPack(d: ClientLibraryDownload) {
    try {
      await invoke('uninstall_client_library_download', { instanceDir, download: d })
      const next = new Set(_clInstalled)
      next.delete(d.repo_path)
      _clInstalled = next
      onToast?.(t('toast.removedLabel', { label: d.label }), 'info')
    } catch (e) {
      onToast?.(t('toast.couldnTRemoveIt') + String(e), 'error')
    }
  }

  /** Port of `OpenUrl(ClientLibraryService.SiteUrl)`. */
  async function openSiteUrl() {
    try {
      const url = await invoke<string>('client_library_site_url')
      if (url) invoke('open_url', { url }).catch(() => {})
    } catch {
      /* ignore */
    }
  }

  // Port of `OpenClientLibrary` (Home.ClientLibrary.cs:21) — loads without refresh.
  onMount(() => loadClientLibrary(false))
</script>

<style>
  .cl-retry { margin-top: 10px; }
  .cl-progress { margin-top: 4px; }
</style>

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
    <span class="panel-title">{t('x.clientLibrary')}</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-spacer"></div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn"
      onclick={openSiteUrl}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.indigoToolsClientlibraryGithub')}
      aria-label={t('tt.openGithub')}
    >
      <i class="fa-brands fa-github"></i>
    </button>
    <button
      class="panel-icon-btn {_clLoading ? 'spinning' : ''}"
      disabled={_clLoading}
      onclick={() => loadClientLibrary(true)}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  </div>
</div>

<SubTabs strip="packs-addons" active="clientlib" edition="bedrock" onNavigate={(k) => onNavigate?.(k)} />

<!--
  IA change: the version filter was a `.mcv-channel-tabs` row that has to
  wrap onto more and more lines as the community catalog grows Minecraft
  versions — unlike the fixed 2-4 item strips elsewhere, this list is
  unbounded. Same master-detail rail as Settings/Java Mods/Tweaks fixes
  that; the accordion pack list becomes a `.client-card-cols` grid so
  browsing many packs isn't one long column of full-width rows either.
-->
<div class="settings-layout">
  <nav class="settings-nav-rail" aria-label={t('tt.minecraftVersion')}>
    <button
      type="button"
      class="settings-nav-btn {_clVersion === 'all' ? 'active' : ''}"
      onclick={() => setClientLibVersion('all')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-layer-group"></i> <span>{t('x.all')}</span>
      <span class="settings-nav-btn-count">{_clPacks.length}</span>
    </button>
    {#each _clVersions as v (v)}
      <button
        type="button"
        class="settings-nav-btn {_clVersion === v ? 'active' : ''}"
        onclick={() => setClientLibVersion(v)}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-cube"></i> <span>MC {v}</span>
        <span class="settings-nav-btn-count">{_clPacks.filter((p) => p.version === v).length}</span>
      </button>
    {/each}
  </nav>

  <div class="settings-content">
    <div class="panel-search-wrap settings-search-wrap">
      <i class="fa-solid fa-magnifying-glass"></i>
      <input
        class="panel-search-input"
        aria-label={t('ph.searchClients')}
        placeholder={t('ph.searchClients')}
        value={_clFilterInput}
        oninput={onClFilterInput}
        onkeydown={(e) => e.stopPropagation()}
      />
    </div>

    <div class="panel-body">
      {#if _clLoading && _clPacks.length === 0}
        <ListSkeleton rows={5} />
      {:else if _clError && _clPacks.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-triangle-exclamation"></i>
          <span>{t('empty.couldnTLoadLibrary')}</span>
          <small>{_clError}</small>
          <button
            class="btn-sm cl-retry"
            onclick={() => loadClientLibrary(true)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-rotate"></i> {t('common.retry')}
          </button>
        </div>
      {:else}
        {#if _clError}
          <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>{_clError}</span></div>
        {/if}

        {#if clFiltered.length === 0}
          <div class="empty-state">
            <i class="fa-solid fa-magnifying-glass"></i>
            <span>{t('empty.noClientsMatch')}</span>
            <small>{t('empty.tryAnotherMinecraftVersionOrClear')}</small>
          </div>
        {/if}

        <div class="client-card-cols">
        {#each clFiltered as pack (packKey(pack))}
          {@const open = isClientPackExpanded(pack)}
          <div
            class="cl-pack {open ? 'open' : ''} {pack.banner_url ? 'textured' : ''}"
            style={pack.banner_url ? `--cl-banner:url('${pack.banner_url}')` : ''}
          >
            <div
              class="cl-pack-head"
              role="button"
              tabindex="0"
              aria-expanded={open}
              onclick={() => toggleClientPack(pack)}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') toggleClientPack(pack)
              }}
              onmousedown={(e) => e.stopPropagation()}
            >
              {#if pack.icon_url}
                <img class="cl-pack-icon" src={pack.icon_url} alt="" loading="lazy" decoding="async" />
              {:else}
                <div class="cl-pack-icon cl-pack-icon-ph"><i class="fa-solid fa-wand-sparkles"></i></div>
              {/if}
              <div class="version-meta">
                <span class="version-name">{pack.name}</span>
                <span class="version-sub">
                  MC {pack.version} · {mainDownloads(pack).length} download{mainDownloads(pack).length === 1 ? '' : 's'}
                  {#if extensions(pack).length > 0} · {extensions(pack).length} extension{extensions(pack).length === 1 ? '' : 's'}{/if}
                </span>
              </div>
              <i class="fa-solid fa-chevron-down cl-pack-caret"></i>
            </div>

            {#if open}
              <div class="cl-pack-body">
                {#if pack.description}<p class="cl-pack-desc">{pack.description}</p>{/if}
                {#each [...mainDownloads(pack), ...extensions(pack)] as dl (dl.repo_path)}
                  {@const isInstalled = _clInstalled.has(dl.repo_path)}
                  {@const busy = _clBusyPath === dl.repo_path}
                  <div class="cl-dl-row">
                    <i class={dl.is_extension ? 'fa-solid fa-puzzle-piece' : 'fa-solid fa-box'}></i>
                    <div class="version-meta">
                      <span class="version-name">{dl.label}</span>
                      <span class="version-sub">
                        {dl.is_extension ? 'Extension' : 'Client pack'}
                        {#if isInstalled} · installed{/if}
                      </span>
                      {#if busy}
                        <div class="cl-progress">
                          <InlineProgress progress={_clProgress} showSpinner={false} />
                        </div>
                      {/if}
                    </div>
                    <div class="version-actions">
                      {#if isInstalled}
                        <button
                          class="icon-btn icon-btn-ghost"
                          data-tooltip={t('tt.removeFromResourcePacks')}
                          aria-label={t('tt.removeFromResourcePacks')}
                          onclick={() => uninstallClientPack(dl)}
                          onmousedown={(e) => e.stopPropagation()}
                        >
                          <i class="fa-solid fa-trash"></i>
                        </button>
                      {/if}
                      <button
                        class="icon-btn icon-btn-ghost"
                        data-tooltip={isInstalled ? 'Reinstall' : 'Install to resource packs'}
                        aria-label={isInstalled ? 'Reinstall' : 'Install to resource packs'}
                        disabled={_clBusyPath !== null}
                        onclick={() => installClientPack(dl)}
                        onmousedown={(e) => e.stopPropagation()}
                      >
                        {#if busy}<span class="spinner"></span>{:else}<i class="fa-solid fa-download"></i>{/if}
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
