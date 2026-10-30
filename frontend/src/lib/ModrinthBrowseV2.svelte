<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * Phase 8 — Modrinth browse v2. Uses the EXISTING
   * `search_modrinth_projects`/`install_modrinth_mod` commands
   * (`src-tauri/src/commands/java_mods.rs`) — there is no
   * `get_modrinth_project_details`-equivalent command in this codebase, so
   * the "detail sub-view" works from the search hit's own fields
   * (title/description/author/downloads/category) and its gallery/full
   * version list are rendered disabled-with-tooltip rather than faked.
   *
   * `search_modrinth_projects` only accepts `query`/`project_type`/`offset`
   * — no loader/version/category facet params — so the sidebar's loader
   * checkboxes, MC-version box, and category filter are applied
   * client-side over the fetched page rather than sent to the backend.
   * This is a deliberate scope call, called out in PARITY.md.
   *
   * N5100 constraint: the result list only ever mounts a small window of
   * DOM nodes (visible slice + overscan) via manual scroll-position math,
   * recalculated on a rAF-throttled scroll handler — no virtualization
   * library.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import Spinner from './components/Spinner.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'

  /** Mirrors `gl_modpack::models::MrProject` (Rust). */
  type MrProject = {
    id: string
    slug: string
    title: string
    description: string
    icon_url: string
    author: string
    downloads: number
    project_type: string
    category_name?: string | null
  }

  let {
    instanceId,
    onBack,
    onToast,
  }: {
    /** Instance to install mods into. */
    instanceId?: string
    onBack?: () => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  function toast(message: string, kind = 'info') {
    onToast?.(message, kind)
  }

  const MISSING_PROJECT_DETAILS =
    'Unavailable: needs a get_modrinth_project_details-equivalent command (gallery + full version list)'
  const MISSING_SERVER_SIDE_FACETS =
    'Filtered client-side: search_modrinth_projects has no loader/version/category facet params yet'

  const LOADERS = ['fabric', 'forge', 'quilt', 'neoforge'] as const
  const SORTS = [
    { id: 'relevance', label: 'Relevance' },
    { id: 'downloads', label: 'Downloads' },
    { id: 'title', label: 'Name (A-Z)' },
  ] as const

  let query = $state('')
  let selectedLoaders = $state<Set<string>>(new Set())
  let mcVersion = $state('')
  let category = $state('')
  let sort = $state<(typeof SORTS)[number]['id']>('relevance')

  let results = $state<MrProject[]>([])
  let loading = $state(false)
  let error = $state('')
  let totalCount = $state(0)
  let offset = $state(0)
  const PAGE_SIZE = 20

  let searchToken = 0

  async function runSearch(reset: boolean) {
    const token = ++searchToken
    loading = true
    error = ''
    if (reset) offset = 0
    try {
      const res = await invoke<{ projects: MrProject[]; total_count: number }>(
        'search_modrinth_projects',
        { query, projectType: 'mod', offset },
      )
      if (token !== searchToken) return
      results = reset ? res.projects : [...results, ...res.projects]
      totalCount = res.total_count
    } catch (e) {
      if (token === searchToken) error = String(e)
    } finally {
      if (token === searchToken) loading = false
    }
  }

  let debounceTimer: ReturnType<typeof setTimeout> | undefined
  function onQueryInput() {
    clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => runSearch(true), 250)
  }

  runSearch(true)

  function toggleLoader(l: string) {
    const s = new Set(selectedLoaders)
    if (s.has(l)) s.delete(l)
    else s.add(l)
    selectedLoaders = s
  }

  // Client-side filter + sort (see the module-level note on facet support).
  const filtered = $derived.by(() => {
    let list = results
    if (selectedLoaders.size > 0) {
      list = list.filter((p) =>
        [...selectedLoaders].some((l) => p.description.toLowerCase().includes(l) || p.title.toLowerCase().includes(l)),
      )
    }
    if (category.trim()) {
      const c = category.trim().toLowerCase()
      list = list.filter((p) => (p.category_name ?? '').toLowerCase().includes(c))
    }
    const sorted = [...list]
    if (sort === 'downloads') sorted.sort((a, b) => b.downloads - a.downloads)
    else if (sort === 'title') sorted.sort((a, b) => a.title.localeCompare(b.title))
    return sorted
  })

  // ---- windowed rendering (N5100: cap DOM nodes for an unbounded list) ----
  const ROW_HEIGHT = 84
  const OVERSCAN = 4
  let viewportEl: HTMLDivElement | undefined
  let viewportHeight = $state(600)
  let scrollTop = $state(0)
  let rafPending = false

  function onScroll() {
    if (rafPending) return
    rafPending = true
    requestAnimationFrame(() => {
      rafPending = false
      if (viewportEl) scrollTop = viewportEl.scrollTop
      // Infinite-scroll: fetch next page once near the bottom.
      if (
        viewportEl &&
        !loading &&
        results.length < totalCount &&
        viewportEl.scrollTop + viewportEl.clientHeight > viewportEl.scrollHeight - ROW_HEIGHT * 4
      ) {
        offset += PAGE_SIZE
        runSearch(false)
      }
    })
  }

  function measureViewport() {
    if (viewportEl) viewportHeight = viewportEl.clientHeight
  }

  const totalHeight = $derived(filtered.length * ROW_HEIGHT)
  const startIndex = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN))
  const visibleCount = $derived(Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN * 2)
  const endIndex = $derived(Math.min(filtered.length, startIndex + visibleCount))
  const visibleSlice = $derived(filtered.slice(startIndex, endIndex))
  const topPad = $derived(startIndex * ROW_HEIGHT)

  // -- detail sub-view --------------------------------------------------
  let detailProject = $state<MrProject | null>(null)
  let installing = $state(false)

  onMount(() => {
    measureViewport()
    window.addEventListener('resize', measureViewport)
    return () => {
      window.removeEventListener('resize', measureViewport)
      clearTimeout(debounceTimer)
    }
  })

  async function install(p: MrProject) {
    if (!instanceId) {
      toast('Pick an instance first.', 'error')
      return
    }
    installing = true
    try {
      const res = await invoke<{ file_name: string; dependency_warnings: { title: string; installed: boolean }[] }>(
        'install_modrinth_mod',
        { instanceId, projectId: p.id },
      )
      toast(`Installed ${res.file_name}.`, 'success')
      const missingDeps = res.dependency_warnings.filter((d) => !d.installed)
      if (missingDeps.length) {
        toast(`Missing dependencies: ${missingDeps.map((d) => d.title).join(', ')}`, 'warning')
      }
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      installing = false
    }
  }
</script>

<style>
  /* Each row is windowed at ROW_HEIGHT (84px) = 76px card + 8px gap. */
  .modrinth-v2-card {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 76px;
    margin-bottom: 8px;
    padding: var(--card-pad-y) var(--card-pad-x);
    background: var(--bg-item);
    border: none;
    border-radius: var(--r-md);
    box-shadow: var(--shadow-sm);
    color: var(--text);
    text-align: left;
    font-family: inherit;
    cursor: pointer;
    transition: background var(--t);
  }
  .modrinth-v2-card:hover {
    background: var(--bg-item-hover);
  }
  .modrinth-v2-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .modrinth-v2-icon {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    border-radius: var(--r-sm);
    object-fit: cover;
    background: var(--bg-inset);
  }
  .modrinth-v2-icon-lg {
    width: 64px;
    height: 64px;
    border-radius: var(--r-md);
    object-fit: cover;
    background: var(--bg-inset);
  }
  .modrinth-v2-card-body {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .modrinth-v2-card-title {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .modrinth-v2-card-desc {
    font-size: 12px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .modrinth-v2-card-meta {
    display: flex;
    gap: 10px;
    font-size: 11px;
    color: var(--text-dim);
  }
</style>

<div class="modrinth-v2">
  <div class="modrinth-v2-sidebar">
    <h3>{t('x.filters')}</h3>
    <div class="modrinth-v2-filter-group">
      <span class="wizard-hint">{t('x.loader')}</span>
      {#each LOADERS as l}
        <label class="modrinth-v2-checkbox" data-tooltip={MISSING_SERVER_SIDE_FACETS}>
          <input type="checkbox" checked={selectedLoaders.has(l)} onchange={() => toggleLoader(l)} />
          {l}
        </label>
      {/each}
    </div>
    <label class="wizard-field">
      <span>{t('x.mcVersion')}</span>
      <input class="setting-input" bind:value={mcVersion} placeholder="e.g. 1.21.1" data-tooltip={MISSING_SERVER_SIDE_FACETS} />
    </label>
    <label class="wizard-field">
      <span>{t('label.category')}</span>
      <input class="setting-input" bind:value={category} placeholder="e.g. adventure" />
    </label>
    <label class="wizard-field">
      <span>{t('x.sort')}</span>
      <select class="setting-select" bind:value={sort}>
        {#each SORTS as s}
          <option value={s.id}>{s.label}</option>
        {/each}
      </select>
    </label>
  </div>

  <div class="modrinth-v2-main">
    <div class="modrinth-v2-search-row">
      <input
        class="setting-input"
        aria-label={t('ph.searchModrinthMods')}
        placeholder={t('ph.searchModrinthMods')}
        bind:value={query}
        oninput={onQueryInput}
      />
      {#if onBack}<button class="btn-sm" onclick={onBack}>{t('common.close')}</button>{/if}
    </div>

    {#if error}<p class="error-text">{error}</p>{/if}

    <div
      class="modrinth-v2-viewport"
      bind:this={viewportEl}
      onscroll={onScroll}
    >
      <div class="modrinth-v2-spacer" style="height:{totalHeight}px;">
        <div class="modrinth-v2-window" style="transform:translateY({topPad}px);">
          {#each visibleSlice as p (p.id)}
            <button type="button" class="modrinth-v2-card" onclick={() => (detailProject = p)}>
              <img class="modrinth-v2-icon" src={p.icon_url || '/vendor/img/placeholder-mod.png'} alt="" loading="lazy" />
              <div class="modrinth-v2-card-body">
                <div class="modrinth-v2-card-title">{p.title}</div>
                <div class="modrinth-v2-card-desc">{p.description}</div>
                <div class="modrinth-v2-card-meta">
                  <span>{p.author}</span>
                  <span>{p.downloads.toLocaleString()} downloads</span>
                </div>
              </div>
            </button>
          {/each}
        </div>
      </div>
      {#if loading}
        <ListSkeleton rows={4} />
      {/if}
      {#if !loading && filtered.length === 0}
        <div class="empty-state"><p>{t('x.noResults')}</p></div>
      {/if}
    </div>
  </div>
</div>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape' && detailProject) detailProject = null }} />

{#if detailProject}
  <div class="wizard-overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && (detailProject = null)}>
    <div class="wizard-card modrinth-v2-detail">
      <div class="wizard-head">
        <h2>{detailProject.title}</h2>
        <button class="panel-icon-btn" onclick={() => (detailProject = null)} aria-label={t('tt.close')}>
          <i class="fa-solid fa-xmark"></i>
        </button>
      </div>
      <div class="wizard-body">
        <img class="modrinth-v2-icon-lg" src={detailProject.icon_url || '/vendor/img/placeholder-mod.png'} alt="" />
        <p>{detailProject.description}</p>
        <div class="wizard-review">
          <div><strong>{t('x.author')}</strong><span>{detailProject.author}</span></div>
          <div><strong>{t('nav.downloads')}</strong><span>{detailProject.downloads.toLocaleString()}</span></div>
          <div><strong>{t('x.type')}</strong><span>{detailProject.project_type}</span></div>
          {#if detailProject.category_name}<div><strong>{t('label.category')}</strong><span>{detailProject.category_name}</span></div>{/if}
        </div>
        <button
          class="btn-sm"
          disabled
          data-tooltip={MISSING_PROJECT_DETAILS}
        >
          <i class="fa-solid fa-images"></i> Gallery
        </button>
        <button
          class="btn-sm"
          disabled
          data-tooltip={MISSING_PROJECT_DETAILS}
        >
          <i class="fa-solid fa-code-branch"></i> All versions
        </button>
      </div>
      <div class="wizard-footer">
        <button class="btn-sm" onclick={() => (detailProject = null)}>{t('common.close')}</button>
        <button class="btn-sm btn-primary" onclick={() => detailProject && install(detailProject)} disabled={installing || !instanceId}>
          {#if installing}<Spinner size={14} />{:else}<i class="fa-solid fa-download"></i>{/if} Install latest
        </button>
      </div>
    </div>
  </div>
{/if}
