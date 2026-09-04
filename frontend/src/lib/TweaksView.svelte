<script lang="ts">
  /**
   * Port of the "tweaks" panel - Home.razor:3102-3266 with its code-behind in
   * Home.Tweaks.cs, backed by `Services/TweaksService.cs`: the VanillaTweaks
   * build endpoint for Java, and a BedrockTweaks GitHub-repo `.mcpack`
   * assembly for Bedrock.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount } from 'svelte'
  import EditionTabs from './components/EditionTabs.svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'

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
    onNavigate?: (view: string) => void
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

  /** Fetched rather than hardcoded - `TweaksService.JavaVersions` is the source. */
  let javaVersions = $state<string[]>([])
  let javaVersion = $state('')
  /** Port of `_javaVerOpen` - the custom `.tw-dropdown`, not a native select. */
  let javaVerOpen = $state(false)
  let categories = $state<Category[]>([])
  let engineVersion = $state<[number, number, number]>([1, 20, 0])
  let versionLabel = $state('')
  let expanded = $state<Set<string>>(new Set())
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
    expanded = new Set()
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
      const bytes = await invoke<number[]>('fetch_image_bytes', { url })
      let binary = ''
      for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i])
      const dataUrl = `data:image/png;base64,${btoa(binary)}`
      iconCache = new Map(iconCache).set(url, dataUrl)
    } catch {
      // Leave unresolved — the CSS fallback icon behind it stays visible.
    }
  }

  /** Port of `ToggleCategory` - keyed on `c.Name`, as the original is. */
  function toggleCategory(c: Category) {
    const next = new Set(expanded)
    if (next.has(c.name)) {
      next.delete(c.name)
    } else {
      next.add(c.name)
      for (const p of c.packs) resolveIcon(p.icon_url)
    }
    expanded = next
  }

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

  function categorySelectedCount(c: Category) {
    return c.packs.filter((p) => isTweakSelected(p)).length
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
        onToast?.(`Installed to resourcepacks — ${baseName(path)}`, 'success')
      } else {
        const path = await invoke<string>('build_bedrock_tweaks_pack', {
          destDir: `${minecraftDir}\\resource_packs`,
          engineVersion,
          selection: Array.from(selected.values()),
        })
        onToast?.(`Installed ${baseName(path)} to your resource packs.`, 'success')
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
      unlisten = fn
    })
  })
  onDestroy(() => {
    unlisten?.()
  })

  // Bedrock needs no version, so it can load immediately; Java waits for the
  // version list above.
  if (!isJava) load()
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
      <span class="tweaks-version-pill" data-tooltip="BedrockTweaks ships one current pack set">
        <i class="fa-solid fa-cube"></i> {tweaksVersionLabel}
      </span>
    {/if}
    <button
      type="button"
      class="panel-icon-btn {loading ? 'spinning' : ''}"
      disabled={loading}
      onclick={load}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
      aria-label="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<!-- Home.razor:3156-3159: the Bedrock edition gets the shared
     Packs/Addons cross-nav strip here; Java does not. -->
{#if !isJava}
  <SubTabs strip="packs-addons" active="tweaks" onNavigate={(k) => onNavigate?.(k)} />
{/if}

<div class="panel-body">
  {#if loading}
    <div class="versions-loading"><span class="spinner"></span><span>Loading tweaks…</span></div>
  {:else if error && categories.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-triangle-exclamation"></i>
      <span>Couldn't load tweaks</span>
      <small>{error}</small>
      <button
        type="button"
        class="btn-sm"
        style="margin-top:10px;"
        onclick={load}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-rotate"></i> Retry
      </button>
    </div>
  {:else if categories.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-wand-magic-sparkles"></i>
      <span>No tweaks available</span>
      <small>Try a different Minecraft version.</small>
    </div>
  {:else}
    <p class="setting-hint" style="margin:0 0 8px;">
      {isJava
        ? 'Pick tweaks, then build & install them straight into your resourcepacks folder.'
        : 'Pick tweaks — Glacier assembles a single .mcpack and imports it into your resource packs.'}
    </p>
    {#each categories as c (c.id)}
      {@const isExpanded = expanded.has(c.name)}
      {@const selCount = categorySelectedCount(c)}
      <button
        type="button"
        class="tweak-cat-header {isExpanded ? 'expanded' : ''}"
        onclick={() => toggleCategory(c)}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-chevron-right tweak-cat-chevron"></i>
        <span class="tweak-cat-name">{c.name}</span>
        {#if selCount > 0}
          <span class="tweak-cat-badge">{selCount}</span>
        {/if}
        <span class="tweak-cat-count">{c.packs.length}</span>
      </button>
      {#if isExpanded}
        {#each c.packs as p (packKey(p))}
          {@const sel = isTweakSelected(p)}
          <div class="setting-row clickable tweak-row" role="button" tabindex="0" onclick={() => toggleTweak(p)} onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && toggleTweak(p)}>
            <span class="tweak-icon">
              <i class="fa-solid fa-wand-magic-sparkles tweak-icon-fallback"></i>
              {#if iconCache.has(p.icon_url)}
                <img class="tweak-icon-img" src={iconCache.get(p.icon_url)} alt="" />
              {/if}
            </span>
            <div class="setting-meta" style="min-width:0;">
              <span class="setting-label">{p.name}</span>
              {#if p.description}
                <span class="setting-hint">{p.description}</span>
              {/if}
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
        {/each}
      {/if}
    {/each}
  {/if}

</div>

{#if categories.length > 0}
  <div class="panel-footer-bar">
    {#if building}
      <div class="progress-bar-wrap" style="flex:1;">
        <div class="progress-bar-fill" style="width:{Math.round(tweaksProgress * 100)}%"></div>
      </div>
      <span class="dl-pct">{Math.floor(tweaksProgress * 100)}%</span>
    {:else}
      <span style="font-size:12px;color:var(--text-dim);">{selected.size} selected</span>
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

<!-- Home.razor:3253-3262: Bedrock shows the *addons* tab as active here,
     Java shows *tweaks*. -->
<EditionTabs
  active={isJava ? 'tweaks' : 'addons'}
  edition={isJava ? 'java' : 'bedrock'}
  onNavigate={(v) => onNavigate?.(v)}
/>
