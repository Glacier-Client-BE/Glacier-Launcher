<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { untrack } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'
  import ViewModeToggle from './components/ViewModeToggle.svelte'
  import { loadViewMode, saveViewMode, type ViewMode } from './viewMode'
  import type { LauncherSettings } from './settings'

  // Ported from Components/ModpacksPanel.razor. Search hits
  // `search_curseforge_modpacks` / `search_modrinth_modpacks`. Install hits
  // the new `install_curseforge_modpack` / `install_modrinth_modpack`
  // commands (src-tauri/src/commands/modpack.rs): download the pack
  // archive, extract overrides into a freshly created Java instance, and
  // download every mod file. Simplified vs. the C# original: no automatic
  // mod-loader install (Fabric/Forge/Quilt/NeoForge) — the instance is
  // created with the pack's Minecraft version, and the user installs/selects
  // the loader afterward via the existing Java mod-loader install commands.
  let { settings, onBack }: { settings: LauncherSettings; onBack?: () => void } = $props()

  let installingId = $state<string | null>(null)
  let installMsg = $state('')

  type PackResult = {
    id: string
    numericId: number
    title: string
    author: string
    summary: string
    icon: string
    downloads: number
    source: 'cf' | 'mr'
  }

  let source = $state<'cf' | 'mr'>(untrack(() => settings.CurseForgeApiKey)?.trim() ? 'cf' : 'mr')
  let sourceTouched = $state(false)
  let query = $state('')
  let searching = $state(false)
  let searched = $state(false)
  let error = $state('')
  let results = $state<PackResult[]>([])
  let modpacksViewMode = $state<ViewMode>(loadViewMode('modpacks'))
  function setModpacksViewMode(m: ViewMode) {
    modpacksViewMode = m
    saveViewMode('modpacks', m)
  }

  const cfAvailable = $derived(!!settings.CurseForgeApiKey?.trim())

  function truncate(s: string, n: number): string {
    if (!s) return ''
    return s.length <= n ? s : s.slice(0, n).trimEnd() + '…'
  }

  function formatDownloads(count: number): string {
    if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M downloads`
    if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K downloads`
    return `${count} downloads`
  }

  function setSource(s: 'cf' | 'mr') {
    sourceTouched = true
    if (source === s) return
    source = s
    search()
  }

  // Resync the default source if the CurseForge API key becomes available/unavailable
  // while this view is mounted (e.g. user edits it in Settings), but only until the
  // user manually picks a source themselves.
  $effect(() => {
    if (!sourceTouched) source = settings.CurseForgeApiKey?.trim() ? 'cf' : 'mr'
  })

  async function search() {
    searching = true
    error = ''
    searched = true
    results = []
    try {
      if (source === 'cf') {
        if (!cfAvailable) {
          searching = false
          return
        }
        const res = await invoke<{
          addons: {
            id: number
            name: string
            summary: string
            thumbnail_url: string
            author_name: string
            download_count: number
          }[]
        }>('search_curseforge_modpacks', { apiKey: settings.CurseForgeApiKey, query, page: 0 })
        results = res.addons.map((a) => ({
          id: String(a.id),
          numericId: a.id,
          title: a.name,
          author: a.author_name,
          summary: a.summary,
          icon: a.thumbnail_url,
          downloads: a.download_count,
          source: 'cf',
        }))
      } else {
        const res = await invoke<{
          projects: {
            id: string
            title: string
            description: string
            icon_url: string
            author: string
            downloads: number
          }[]
        }>('search_modrinth_modpacks', { query, offset: 0 })
        results = res.projects.map((p) => ({
          id: p.id,
          numericId: 0,
          title: p.title,
          author: p.author,
          summary: p.description,
          icon: p.icon_url,
          downloads: p.downloads,
          source: 'mr',
        }))
      }
    } catch (e) {
      error = String(e)
    } finally {
      searching = false
    }
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'Enter') search()
  }

  async function install(r: PackResult) {
    installingId = r.source + r.id
    installMsg = ''
    try {
      const result =
        r.source === 'cf'
          ? await invoke<{ instance_name: string; files_installed: number; files_failed: number }>(
              'install_curseforge_modpack',
              { apiKey: settings.CurseForgeApiKey, modId: r.numericId, packName: r.title },
            )
          : await invoke<{ instance_name: string; files_installed: number; files_failed: number }>(
              'install_modrinth_modpack',
              { projectId: r.id, packName: r.title },
            )
      installMsg = `Installed "${result.instance_name}" — ${result.files_installed} file(s) downloaded${
        result.files_failed ? `, ${result.files_failed} failed` : ''
      }. Set a mod loader for it in the Java instance settings if needed.`
    } catch (e) {
      installMsg = `Install failed: ${e}`
    } finally {
      installingId = null
    }
  }

  search()
</script>

<PanelHeader title="Modpacks" {onBack}>
  {#snippet extraActions()}
    <ViewModeToggle mode={modpacksViewMode} onChange={setModpacksViewMode} />
  {/snippet}
</PanelHeader>

<div class="panel-body">
  <div class="modpack-source-tabs">
    <button class="vcs-btn" class:active={source === 'cf'} onclick={() => setSource('cf')}>CurseForge</button>
    <button class="vcs-btn" class:active={source === 'mr'} onclick={() => setSource('mr')}>Modrinth</button>
  </div>

  <div class="modpack-search">
    <input
      class="panel-search-input"
      placeholder="Search modpacks…"
      bind:value={query}
      onkeydown={onSearchKey}
    />
    <button class="btn-sm" onclick={search} aria-label="Search modpacks"><i class="fa-solid fa-magnifying-glass"></i></button>
  </div>

  {#if source === 'cf' && !cfAvailable}
    <div class="empty-state">
      <i class="fa-solid fa-key"></i>
      <span>A CurseForge API key is required to browse CurseForge. Add one in Settings, or use Modrinth (no key needed).</span>
    </div>
  {/if}

  {#if searching}
    <div class="versions-loading"><Spinner /><span>Searching…</span></div>
  {:else if error}
    <div class="versions-loading versions-error">
      <i class="fa-solid fa-triangle-exclamation"></i><span>{error}</span>
    </div>
  {:else if results.length === 0 && searched}
    <div class="empty-state">
      <i class="fa-solid fa-magnifying-glass"></i>
      <span>No modpacks found for "{query}".</span>
    </div>
  {/if}

  {#if installMsg}
    <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>{installMsg}</span></div>
  {/if}

  <div class="version-list {modpacksViewMode === 'grid' ? 'grid-view' : ''}">
  {#each results as r (r.source + r.id)}
    <div class="modpack-row">
      {#if r.icon}
        <img class="modpack-icon" src={r.icon} loading="lazy" alt="" />
      {:else}
        <div class="modpack-icon modpack-icon-fallback"><i class="fa-solid fa-cubes"></i></div>
      {/if}
      <div class="modpack-meta">
        <span class="version-name">{r.title}</span>
        <span class="version-sub">{r.author} · {formatDownloads(r.downloads)}</span>
        <span class="modpack-summary">{truncate(r.summary, 90)}</span>
      </div>
      <button
        class="vcs-btn"
        disabled={installingId !== null}
        onclick={() => install(r)}
        data-tooltip="Download the pack and install it into a new Java instance."
      >
        {#if installingId === r.source + r.id}
          <Spinner />
        {:else}
          <i class="fa-solid fa-download"></i>&nbsp;Install
        {/if}
      </button>
    </div>
  {/each}
  </div>
</div>
