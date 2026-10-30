<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * Port of the "tweaks" panel - Home.razor:3102-3266 with its code-behind in
   * Home.Tweaks.cs, backed by `Services/TweaksService.cs`: the VanillaTweaks
   * build endpoint for Java, and a BedrockTweaks GitHub-repo `.mcpack`
   * assembly for Bedrock.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount, untrack } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'

  let {
    edition,
    minecraftDir,
    onBack,
    onNavigate,
    onToast,
  }: {
    edition: 'Java' | 'Bedrock'
    minecraftDir: string
    onBack?: () => void
    onNavigate?: (view: string, strip?: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  const isJava = $derived(edition === 'Java')

  type Pack = {
    id: string
    name: string
    description: string
    category_id: string
    category: string
    icon_url: string
  }
  type Category = { id: string; name: string; packs: Pack[] }
  /** BedrockTweaks' `packs.json` splits what's conceptually one category into
   *  a parent plus several "Parent / Sub-thing" children (e.g. "Aesthetic"
   *  and "Aesthetic / More Zombies", or "GUI" with five+ children) — each of
   *  those rendered as its own top-level rail entry, so the rail had 27
   *  buttons where ~13 real topics exist. Java's flat VanillaTweaks
   *  categories have no "/" and end up one group per category, unaffected. */
  type CategoryGroup = { id: string; name: string; categories: Category[] }

  /** Fetched rather than hardcoded - `TweaksService.JavaVersions` is the source. */
  let javaVersions = $state<string[]>([])
  let javaVersion = $state('')
  /** Port of `_javaVerOpen` - the custom `.tw-dropdown`, not a native select. */
  let javaVerOpen = $state(false)
  let categories = $state<Category[]>([])
  let engineVersion = $state<[number, number, number]>([1, 20, 0])
  let versionLabel = $state('')
  /** IA change: was an accordion of categories you expanded one at a time,
   *  each pushing the next one further down. Now that the panel opens wide
   *  (WIDE_DASHBOARD_VIEWS), this is master-detail instead — a category rail
   *  on the left, the selected category's packs as a card grid on the
   *  right, same pattern as Settings/Java Mods. */
  let selectedCategoryId = $state('')
  let selected = $state<Map<string, Pack>>(new Map())
  let loading = $state(true)
  let error = $state('')
  let building = $state(false)
  /** Port of `_tweaksEditionIsJava` - which edition the loaded list belongs to. */
  let loadedEditionIsJava = $state(false)

  /** Port of `TweakKey` (Home.Tweaks.cs:36) - `CategoryId|Category|Id`, verbatim. */
  function packKey(p: Pack) {
    return `${p.category_id}|${p.category}|${p.id}`
  }

  /** Port of `TweaksTitle` - NOT "Java Tweaks". */
  const tweaksTitle = $derived(isJava ? 'Vanilla Tweaks' : 'Bedrock Tweaks')

  /** Port of `TweaksVersionLabel`. */
  const tweaksVersionLabel = $derived(
    loadedEditionIsJava ? javaVersion : versionLabel ? `MC ${versionLabel}` : '',
  )

  /** Port of `LoadTweaksAsync` - clears selection AND expansion each time. */
  async function load() {
    loading = true
    error = ''
    selected = new Map()
    selectedCategoryId = ''
    loadedEditionIsJava = isJava
    try {
      if (isJava) {
        categories = await invoke<Category[]>('list_vanilla_tweaks', { version: javaVersion })
      } else {
        const idx = await invoke<{ categories: Category[]; engine_version: [number, number, number]; version_label: string }>(
          'list_bedrock_tweaks',
        )
        categories = idx.categories
        engineVersion = idx.engine_version
        versionLabel = idx.version_label
      }
    } catch (e) {
      error = String(e)
      categories = []
    } finally {
      loading = false
    }
  }

  // vanillatweaks.net hotlink-blocks cross-origin <img> requests based on
  // Referer (confirmed via direct testing: 403 with a foreign Referer, 200
  // with none/same-origin) — breaks icon loading in both the original
  // WebView2 app and this one. Fetching server-side via `fetch_image_bytes`
  // (reqwest sends no Referer) and rendering as a data URL sidesteps it.
  let iconCache = $state<Map<string, string>>(new Map())

  async function resolveIcon(url: string) {
    if (!url || iconCache.has(url)) return
    try {
      const dataUrl = await invoke<string>('fetch_image_bytes', { url })
      iconCache = new Map(iconCache).set(url, dataUrl)
    } catch (e) {
      // Leave unresolved — the CSS fallback icon behind it stays visible.
      // Logged (not just swallowed): a silent catch here is exactly why a
      // real backend fetch failure looked identical to "icons just don't
      // exist" instead of a diagnosable error.
      console.warn(`Tweak pack icon failed to load: ${url}`, e)
    }
  }

  /** Groups categories that share a "/"-namespaced parent id into one rail
   *  entry, preserving each sub-category's own name/packs for section
   *  headers in the detail pane. Order follows first-seen order in
   *  `categories` (already the source's own ordering). */
  const categoryGroups = $derived.by(() => {
    const groups = new Map<string, Category[]>()
    for (const c of categories) {
      const topId = c.id.split('/')[0]
      if (!groups.has(topId)) groups.set(topId, [])
      groups.get(topId)!.push(c)
    }
    const result: CategoryGroup[] = []
    for (const [topId, cats] of groups) {
      const parent = cats.find((c) => c.id === topId)
      const name = parent ? parent.name : (cats[0].name.split(' / ')[0] ?? cats[0].name)
      result.push({ id: topId, name, categories: cats })
    }
    return result
  })

  /** A sub-category's own display label within its group's detail pane —
   *  drops the "Parent / " prefix `packs.json` puts on every child's name,
   *  since the parent is already the rail's active label. */
  function subCategoryLabel(g: CategoryGroup, c: Category): string {
    if (c.id === g.id) return ''
    const idx = c.name.indexOf(' / ')
    return idx === -1 ? c.name : c.name.slice(idx + 3)
  }

  /** Was `ToggleCategory` (accordion expand/collapse, keyed on `c.Name`).
   *  Master-detail replaces expand/collapse with selection — picking a
   *  category shows its packs in the detail pane instead of pushing every
   *  other category down the page. */
  function selectCategory(g: CategoryGroup) {
    selectedCategoryId = g.id
    for (const c of g.categories) for (const p of c.packs) resolveIcon(p.icon_url)
  }

  const selectedCategory = $derived(
    categoryGroups.find((g) => g.id === selectedCategoryId) ?? categoryGroups[0] ?? null,
  )

  $effect(() => {
    if (categoryGroups.length === 0) return
    if (!categoryGroups.some((g) => g.id === selectedCategoryId)) {
      selectCategory(categoryGroups[0])
    }
  })

  function isTweakSelected(p: Pack) {
    return selected.has(packKey(p))
  }

  function toggleTweak(p: Pack) {
    const next = new Map(selected)
    const key = packKey(p)
    if (next.has(key)) next.delete(key)
    else next.set(key, p)
    selected = next
  }

  function categorySelectedCount(g: CategoryGroup) {
    return g.categories.reduce(
      (sum, c) => sum + c.packs.filter((p) => isTweakSelected(p)).length,
      0,
    )
  }

  function groupPackCount(g: CategoryGroup) {
    return g.categories.reduce((sum, c) => sum + c.packs.length, 0)
  }

  /** Port of `InstallTweaksAsync` - same toast texts, selection cleared on success. */
  async function buildAndInstall() {
    if (selected.size === 0 || building) return
    building = true
    error = ''
    try {
      if (loadedEditionIsJava) {
        const bySelectionCat = new Map<string, string[]>()
        for (const p of selected.values()) {
          const arr = bySelectionCat.get(p.category) ?? []
          arr.push(p.id)
          bySelectionCat.set(p.category, arr)
        }
        const selection = Object.fromEntries(bySelectionCat)
        const path = await invoke<string>('build_vanilla_tweaks_pack', {
          minecraftDir,
          version: javaVersion,
          selection,
        })
        onToast?.(t('toast.installedToResourcepacksDash', { file: baseName(path) }), 'success')
      } else {
        const path = await invoke<string>('build_bedrock_tweaks_pack', {
          destDir: `${minecraftDir}\\resource_packs`,
          engineVersion,
          selection: Array.from(selected.values()),
        })
        onToast?.(t('toast.installedFileToPacks', { file: baseName(path) }), 'success')
      }
      selected = new Map()
    } catch (e) {
      onToast?.(String(e), 'error')
      error = String(e)
    } finally {
      building = false
    }
  }

  /** `Path.GetFileName` equivalent, for the install toasts. */
  function baseName(path: string): string {
    const parts = path.split(/[\\/]/)
    return parts[parts.length - 1] || path
  }

  /** Port of `ToggleJavaVerDropdown` - inert while loading or building. */
  function toggleJavaVerDropdown() {
    if (loading || building) return
    javaVerOpen = !javaVerOpen
  }

  /** Port of `SelectJavaTweaksVersion` - reselecting the same version only closes. */
  function selectJavaTweaksVersion(v: string) {
    javaVerOpen = false
    if (v === javaVersion) return
    javaVersion = v
    load()
  }

  /**
   * Real build progress, replacing the original's `_tweaksProgress` +
   * `IProgress<double>`. Both build paths run through the shared download
   * queue - Java submits one `VanillaTweaks pack` entry, Bedrock a batch all
   * labelled `BedrockTweaks file` - so the share of those entries that have
   * finished is the honest figure, pushed by `downloads-changed`.
   */
  type QueueEntry = { id: string; label: string; status: string; progress: number }
  let queue = $state<QueueEntry[]>([])
  let unlisten: UnlistenFn | undefined
  let tweaksDisposed = false

  const tweaksProgress = $derived.by(() => {
    const label = loadedEditionIsJava ? 'VanillaTweaks pack' : 'BedrockTweaks file'
    const mine = queue.filter((e) => e.label === label)
    if (mine.length === 0) return 0
    const done = mine.filter((e) => e.status === 'Completed').length
    const partial = mine
      .filter((e) => e.status === 'Downloading')
      .reduce((sum, e) => sum + e.progress, 0)
    return Math.min(1, (done + partial) / mine.length)
  })

  onMount(() => {
    invoke<string[]>('tweaks_java_versions')
      .then((list) => {
        javaVersions = list
        // Port of `_javaTweaksVersion = TweaksService.JavaVersions[0]`.
        if (!javaVersion && list.length > 0) {
          javaVersion = list[0]
          // The Java list can only be fetched once a version is known: the
          // endpoint is `.../json/<version>/rpcategories.json`, so loading
          // before this resolves requests `.../json//rpcategories.json` and
          // 404s. The C# has no such window because `_javaTweaksVersion` is
          // initialised synchronously from `TweaksService.JavaVersions[0]`.
          if (isJava) load()
        }
      })
      .catch(() => {})
    listen<QueueEntry[]>('downloads-changed', (e) => {
      queue = e.payload
    }).then((fn) => {
      if (tweaksDisposed) fn()
      else unlisten = fn
    })
  })
  onDestroy(() => {
    tweaksDisposed = true
    unlisten?.()
  })

  // Bedrock needs no version, so it can load immediately; Java waits for the
  // version list above.
  if (!untrack(() => isJava)) load()
</script>

<PanelHeader title={tweaksTitle} {onBack}>
  {#snippet extraActions()}
    {#if isJava}
      <div class="tw-dropdown {javaVerOpen ? 'open' : ''}">
        <button
          class="tw-dropdown-btn"
          onclick={toggleJavaVerDropdown}
          onmousedown={(e) => e.stopPropagation()}
          disabled={loading || building}
        >
          <i class="fa-solid fa-cube"></i>
          <span>MC {javaVersion}</span>
          <i class="fa-solid fa-chevron-down tw-dropdown-caret"></i>
        </button>
        {#if javaVerOpen}
          <div class="tw-dropdown-menu">
            {#each javaVersions as ver (ver)}
              <button
                class="tw-dropdown-item {javaVersion === ver ? 'active' : ''}"
                onclick={() => selectJavaTweaksVersion(ver)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <span>MC {ver}</span>
                {#if javaVersion === ver}
                  <i class="fa-solid fa-check"></i>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {:else if tweaksVersionLabel}
      <span class="tweaks-version-pill" data-tooltip={t('tt.bedrocktweaksShipsOneCurrentPackSet')}>
        <i class="fa-solid fa-cube"></i> {tweaksVersionLabel}
      </span>
    {/if}
    <button
      type="button"
      class="panel-icon-btn {loading ? 'spinning' : ''}"
      disabled={loading}
      onclick={load}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<!-- Home.razor:3156-3159: the Bedrock edition gets the shared
     Packs/Addons cross-nav strip here; Java does not. -->
{#if !isJava}
  <SubTabs strip="packs-addons" active="tweaks" edition="bedrock" onNavigate={(k, s) => onNavigate?.(k, s)} />
{/if}

{#if loading}
  <div class="panel-body"><ListSkeleton rows={5} /></div>
{:else if error && categories.length === 0}
  <div class="panel-body">
    <div class="empty-state">
      <i class="fa-solid fa-triangle-exclamation"></i>
      <span>{t('empty.couldnTLoadTweaks')}</span>
      <small>{error}</small>
      <button
        type="button"
        class="btn-sm retry-btn"
        onclick={load}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-rotate"></i> {t('common.retry')}
      </button>
    </div>
  </div>
{:else if categories.length === 0}
  <div class="panel-body">
    <div class="empty-state">
      <i class="fa-solid fa-wand-magic-sparkles"></i>
      <span>{t('empty.noTweaksAvailable')}</span>
      <small>{t('empty.tryDifferentMinecraftVersion')}</small>
    </div>
  </div>
{:else}
  <div class="settings-layout">
    <nav class="settings-nav-rail" aria-label={t('tt.tweakCategories')}>
      {#each categoryGroups as g (g.id)}
        {@const selCount = categorySelectedCount(g)}
        <button
          type="button"
          class="settings-nav-btn {selectedCategory?.id === g.id ? 'active' : ''}"
          onclick={() => selectCategory(g)}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-wand-magic-sparkles"></i>
          <span class="nav-label truncate" title={g.name}>{g.name}</span>
          <span class="settings-nav-btn-count {selCount > 0 ? 'count-active' : ''}"
            >{selCount > 0 ? selCount : groupPackCount(g)}</span
          >
        </button>
      {/each}
    </nav>

    <div class="settings-content">
      <div class="panel-body">
        <p class="setting-hint intro">
          {isJava
            ? 'Pick tweaks, then build & install them straight into your resourcepacks folder.'
            : 'Pick tweaks — Glacier assembles a single .mcpack and imports it into your resource packs.'}
        </p>
        {#if selectedCategory}
          {#each selectedCategory.categories as c (c.id)}
            {@const subLabel = subCategoryLabel(selectedCategory, c)}
            {#if subLabel}
              <span class="panel-section-label">{subLabel}</span>
            {/if}
            <div class="client-card-cols {subLabel ? 'has-sub' : ''}">
              {#each c.packs as p (packKey(p))}
                {@const sel = isTweakSelected(p)}
                <button
                  type="button"
                  class="client-card tweak-pack-card {sel ? 'client-active' : ''}"
                  onclick={() => toggleTweak(p)}
                >
                  <div class="client-card-header">
                    <span class="tweak-icon client-card-icon tweak-icon-box">
                      <i class="fa-solid fa-wand-magic-sparkles tweak-icon-fallback"></i>
                      {#if iconCache.has(p.icon_url)}
                        <img class="tweak-icon-img" src={iconCache.get(p.icon_url)} alt="" loading="lazy" decoding="async" />
                      {/if}
                    </span>
                    <div class="client-card-meta">
                      <span class="client-card-name truncate" title={p.name}>{p.name}</span>
                    </div>
                    <div
                      class="toggle {sel ? 'on' : ''}"
                      role="switch"
                      aria-checked={sel}
                      tabindex="0"
                      onclick={(e) => {
                        e.stopPropagation()
                        toggleTweak(p)
                      }}
                      onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && toggleTweak(p)}
                    ></div>
                  </div>
                  {#if p.description}
                    <p class="client-card-desc" title={p.description}>{p.description}</p>
                  {/if}
                </button>
              {/each}
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if categories.length > 0}
  <div class="panel-footer-bar">
    {#if building}
      <div class="progress-bar-wrap progress-grow">
        <div class="progress-bar-fill" style="width:{Math.round(tweaksProgress * 100)}%"></div>
      </div>
      <span class="dl-pct">{Math.floor(tweaksProgress * 100)}%</span>
    {:else}
      <span class="footer-count">{selected.size} selected</span>
      <button
        type="button"
        class="action-btn"
        disabled={selected.size === 0}
        onclick={buildAndInstall}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-download"></i> Build &amp; Install
      </button>
    {/if}
  </div>
{/if}

<style>
  /* Pack cards sit on the shared client-card surface; this file only adds the
     spacing and small text roles its markup needs. Colours come from tokens. */
  .retry-btn {
    margin-top: 10px;
  }
  .nav-label {
    flex: 1;
    min-width: 0;
  }
  .count-active {
    color: var(--accent-hover);
    background: var(--accent-bg);
  }
  .intro {
    margin: 0 0 8px;
  }
  .has-sub {
    margin-bottom: 14px;
  }
  .tweak-icon-box {
    padding: 6px;
  }
  .progress-grow {
    flex: 1;
    min-width: 0;
  }
  .footer-count {
    font-size: 12px;
    color: var(--text-dim);
  }
  .truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .client-card-desc {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>

