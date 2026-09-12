<script lang="ts">
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

  let category = $state<Category>(CATEGORIES[0])
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
      error = 'No CurseForge API key set in Settings.'
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
    void search()
  }

  onMount(() => void search())

  // ── Install progress ─────────────────────────────────────────────────
  // Same `downloads-changed` snapshot pattern as BedrockVanillaVersionsView:
  // the queue pushes the whole download list, matched here by label.
  type DownloadEntry = { id: string; label: string; status: string; progress: number }
  let downloadEntries = $state<DownloadEntry[]>([])
  let unlistenDownloads: UnlistenFn | undefined

  onMount(() => {
    listen<DownloadEntry[]>('downloads-changed', (e) => {
      downloadEntries = e.payload
    }).then((fn) => {
      unlistenDownloads = fn
    })
  })
  onDestroy(() => unlistenDownloads?.())

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
      onToast?.(`Installed '${addon.name}'.`, 'success')
      // Feature 21: the install still succeeds when a conflict is found —
      // it's a warning, not a block — but the user should see it.
      for (const warning of result.warnings ?? []) {
        onToast?.(warning, 'error')
      }
    } catch (e) {
      onToast?.(`Failed to install '${addon.name}': ${String(e)}`, 'error')
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

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<PanelHeader title="Browse Addons" {onBack}>
  {#snippet extraActions()}
    <button
      aria-label="Refresh"
      class="panel-icon-btn {loading ? 'spinning' : ''}"
      disabled={loading}
      onclick={search}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <ViewModeToggle mode={addonsViewMode} onChange={setAddonsViewMode} />
  {/snippet}
</PanelHeader>

<SubTabs strip="packs-addons" active="addons" onNavigate={(k, s) => onNavigate?.(k, s)} />

<div class="mcv-filter-bar">
  <div class="panel-search-wrap">
    <i class="fa-solid fa-magnifying-glass"></i>
    <input
      class="panel-search-input"
      placeholder="Search CurseForge..."
      bind:value={query}
      onkeydown={(e) => {
        e.stopPropagation()
        if (e.key === 'Enter') void search()
      }}
    />
  </div>
  <div class="mcv-channel-tabs">
    {#each CATEGORIES as c (c.key)}
      <button
        class="mcv-channel-tab {category.key === c.key ? 'active' : ''}"
        onclick={() => selectCategory(c)}
        onmousedown={(e) => e.stopPropagation()}
      ><i class={c.icon}></i> {c.label}</button>
    {/each}
  </div>
</div>

<div class="panel-body">
  {#if !apiKey}
    <div class="empty-state">
      <i class="fa-solid fa-key"></i>
      <span>No CurseForge API key set</span>
      <small>Add one in Settings to browse and install Bedrock add-ons from CurseForge.</small>
    </div>
  {:else if loading && results.length === 0}
    <ListSkeleton rows={6} />
  {:else if error && results.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-triangle-exclamation"></i>
      <span>Couldn't load results</span>
      <small>{error}</small>
      <button class="btn-sm" style="margin-top:10px;" onclick={search} onmousedown={(e) => e.stopPropagation()}>
        <i class="fa-solid fa-rotate"></i> Retry
      </button>
    </div>
  {:else if searchedOnce && results.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-magnifying-glass"></i>
      <span>No {category.label.toLowerCase()} match</span>
      <small>Try another search term or category.</small>
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
          <img class="version-icon" src={addon.thumbnail_url} alt="" loading="lazy" />
        {:else}
          <div class="version-icon version-icon-placeholder">
            <i class={category.icon}></i>
          </div>
        {/if}
        <div class="version-meta">
          <span class="version-name">{addon.name}</span>
          <span class="version-sub">
            {addon.author_name} · {formatDownloads(addon.download_count)} downloads
            {#if addon.category_name} · {addon.category_name}{/if}
          </span>
          {#if addon.summary}<span class="version-sub">{addon.summary}</span>{/if}
          {#if busy}
            <div style="margin-top:4px;">
              <InlineProgress progress={progressFor(addon)} showSpinner={false} />
            </div>
          {/if}
        </div>
        <div class="version-actions">
          <button
            class="icon-btn icon-btn-ghost"
            data-tooltip={addon.latest_file_id ? 'Install' : 'No downloadable file'}
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
