<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * `addons` panel — real CurseForge-backed Bedrock addon browser, replacing
   * the former stub. Hits `search_bedrock_addons` / `install_bedrock_addon`
   * (src-tauri/src/commands/bedrock_addons.rs), which mirror the C#
   * `CurseForgeService` Bedrock game/class ids and `GetDirectoryForClass`
   * install routing. The API key comes from Settings, same as `ModpacksView`.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import ViewModeToggle from './components/ViewModeToggle.svelte'
  import { loadViewMode, saveViewMode, type ViewMode } from './viewMode'
  import { loadFilterPref, saveFilterPref } from './filterMemory'
  import type { LauncherSettings } from './settings'

  let {
    settings,
    onBack,
    onNavigate,
    onToast,
  }: {
    settings: LauncherSettings
    onBack?: () => void
    onNavigate?: (view: string, strip?: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  /** Port of the C# `BedrockClass*` constants (CurseForgeService.cs). */
  type Category = { key: string; label: string; icon: string; classId: number }
  const CATEGORIES: Category[] = [
    { key: 'addons', label: 'Add-ons', icon: 'fa-solid fa-cubes', classId: 4984 },
    { key: 'maps', label: 'Maps', icon: 'fa-solid fa-map', classId: 6913 },
    { key: 'skins', label: 'Skins', icon: 'fa-solid fa-shirt', classId: 6925 },
    { key: 'texturepacks', label: 'Texture Packs', icon: 'fa-solid fa-swatchbook', classId: 6929 },
    { key: 'scripts', label: 'Scripts', icon: 'fa-solid fa-code', classId: 6940 },
  ]

  /** Translated category name; `label` above stays as the English fallback. */
  function categoryLabel(c: Category): string {
    return t(`category.${c.key}`)
  }

  type CfAddon = {
    id: number
    name: string
    summary: string
    thumbnail_url: string
    author_name: string
    download_count: number
    class_id: number
    latest_file_id: number
    category_name: string
  }
  type CfSearchResult = { addons: CfAddon[]; total_count: number }

  const apiKey = $derived(settings.CurseForgeApiKey?.trim() ?? '')

  // QOL: remembers the last-selected category across restarts instead of
  // always resetting to the first tab.
  const savedCategoryKey = loadFilterPref('bedrock-addons.category', CATEGORIES[0].key)
  let category = $state<Category>(CATEGORIES.find((c) => c.key === savedCategoryKey) ?? CATEGORIES[0])
  let query = $state('')
  let loading = $state(false)
  let error = $state('')
  let results = $state<CfAddon[]>([])
  let addonsViewMode = $state<ViewMode>(loadViewMode('bedrock-addons'))
  function setAddonsViewMode(m: ViewMode) {
    addonsViewMode = m
    saveViewMode('bedrock-addons', m)
  }
  let searchedOnce = $state(false)

  async function search() {
    if (!apiKey) {
      error = t('error.noCurseforgeKeyInSettings')
      return
    }
    loading = true
    error = ''
    searchedOnce = true
    try {
      const res = await invoke<CfSearchResult>('search_bedrock_addons', {
        apiKey,
        classId: category.classId,
        query,
        page: 0,
      })
      results = res.addons
    } catch (e) {
      results = []
      error = String(e)
    } finally {
      loading = false
    }
  }

  function selectCategory(c: Category) {
    if (category.key === c.key) return
    category = c
    saveFilterPref('bedrock-addons.category', c.key)
    void search()
  }

  onMount(() => void search())

  // ── Install progress ─────────────────────────────────────────────────
  // Same `downloads-changed` snapshot pattern as BedrockVanillaVersionsView:
  // the queue pushes the whole download list, matched here by label.
  type DownloadEntry = { id: string; label: string; status: string; progress: number }
  let downloadEntries = $state<DownloadEntry[]>([])
  let unlistenDownloads: UnlistenFn | undefined

  let downloadsDisposed = false
  onMount(() => {
    listen<DownloadEntry[]>('downloads-changed', (e) => {
      downloadEntries = e.payload
    }).then((fn) => {
      if (downloadsDisposed) fn()
      else unlistenDownloads = fn
    })
  })
  onDestroy(() => {
    downloadsDisposed = true
    unlistenDownloads?.()
  })

  let installingId = $state<number | null>(null)

  function progressFor(addon: CfAddon): number {
    const entry = downloadEntries.find(
      (e) => e.label === addon.name && (e.status === 'Downloading' || e.status === 'Queued'),
    )
    return entry?.progress ?? 0
  }

  async function installAddon(addon: CfAddon) {
    if (!apiKey || installingId !== null) return
    installingId = addon.id
    try {
      const result = await invoke<{ installed_paths: string[]; warnings: string[] }>(
        'install_bedrock_addon',
        {
          apiKey,
          modId: addon.id,
          fileId: addon.latest_file_id,
          classId: category.classId,
        },
      )
      onToast?.(t('toast.installedName', { name: addon.name }), 'success')
      // Feature 21: the install still succeeds when a conflict is found —
      // it's a warning, not a block — but the user should see it.
      for (const warning of result.warnings ?? []) {
        onToast?.(warning, 'error')
      }
    } catch (e) {
      onToast?.(t('toast.failedToInstallName', { name: addon.name, error: String(e) }), 'error')
    } finally {
      installingId = null
    }
  }

  function formatDownloads(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
    return String(n)
  }
</script>

<style>
  .ba-retry { margin-top: 10px; }
  .ba-progress { margin-top: 4px; }
</style>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<PanelHeader title={t('panel.browseAddons')} {onBack}>
  {#snippet extraActions()}
    <button
      aria-label={t('tt.refresh')}
      class="panel-icon-btn {loading ? 'spinning' : ''}"
      disabled={loading}
      onclick={search}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <ViewModeToggle mode={addonsViewMode} onChange={setAddonsViewMode} />
  {/snippet}
</PanelHeader>

<SubTabs strip="packs-addons" active="addons" edition="bedrock" onNavigate={(k, s) => onNavigate?.(k, s)} />

<!--
  IA change: was a horizontal category strip (`.mcv-channel-tabs`, 5 items)
  above a single results list. Same master-detail rail as Settings/Java
  Mods/Tweaks — a category is a persistent choice while browsing, not
  something that needs to sit above a search box that scrolls out of view.
-->
<div class="settings-layout">
  <nav class="settings-nav-rail" aria-label={t('tt.addCategories')}>
    {#each CATEGORIES as c (c.key)}
      <button
        type="button"
        class="settings-nav-btn {category.key === c.key ? 'active' : ''}"
        onclick={() => selectCategory(c)}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class={c.icon}></i> {categoryLabel(c)}
      </button>
    {/each}
  </nav>

  <div class="settings-content">
    <div class="panel-search-wrap settings-search-wrap">
      <i class="fa-solid fa-magnifying-glass"></i>
      <input
        class="panel-search-input"
        aria-label={t('ph.searchCurseforge')}
        placeholder={t('ph.searchCurseforge')}
        bind:value={query}
        onkeydown={(e) => {
          e.stopPropagation()
          if (e.key === 'Enter') void search()
        }}
      />
    </div>

    <div class="panel-body">
      {#if !apiKey}
        <div class="empty-state">
          <i class="fa-solid fa-key"></i>
          <span>{t('empty.noCurseforgeApiKeySet')}</span>
          <small>{t('empty.addOneSettingsBrowseInstallBedrock')}</small>
        </div>
      {:else if loading && results.length === 0}
        <ListSkeleton rows={6} />
      {:else if error && results.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-triangle-exclamation"></i>
          <span>{t('empty.couldnTLoadResults')}</span>
          <small>{error}</small>
          <button class="btn-sm ba-retry" onclick={search} onmousedown={(e) => e.stopPropagation()}>
            <i class="fa-solid fa-rotate"></i> {t('common.retry')}
          </button>
        </div>
      {:else if searchedOnce && results.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-magnifying-glass"></i>
          <span>{t('empty.noCategoryMatch', { category: categoryLabel(category).toLowerCase() })}</span>
          <small>{t('empty.tryAnotherSearchTermOrCategory')}</small>
        </div>
      {:else}
        {#if error}
          <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>{error}</span></div>
        {/if}
        <div class="version-list {addonsViewMode === 'grid' ? 'grid-view' : ''}">
        {#each results as addon (addon.id)}
          {@const busy = installingId === addon.id}
          <div class="version-row">
            {#if addon.thumbnail_url}
              <img class="version-icon" src={addon.thumbnail_url} alt="" loading="lazy" decoding="async" />
            {:else}
              <div class="version-icon version-icon-placeholder">
                <i class={category.icon}></i>
              </div>
            {/if}
            <div class="version-meta">
              <span class="version-name">{addon.name}</span>
              <span class="version-sub">
                {addon.author_name} · {t('addon.downloadCount', { count: formatDownloads(addon.download_count) })}
                {#if addon.category_name} · {addon.category_name}{/if}
              </span>
              {#if addon.summary}<span class="version-sub">{addon.summary}</span>{/if}
              {#if busy}
                <div class="ba-progress">
                  <InlineProgress progress={progressFor(addon)} showSpinner={false} />
                </div>
              {/if}
            </div>
            <div class="version-actions">
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={addon.latest_file_id ? t('tt.install') : t('tt.noDownloadableFile')}
                aria-label={addon.latest_file_id ? t('tt.install') : t('tt.noDownloadableFile')}
                disabled={installingId !== null || !addon.latest_file_id}
                onclick={() => installAddon(addon)}
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
  </div>
</div>
