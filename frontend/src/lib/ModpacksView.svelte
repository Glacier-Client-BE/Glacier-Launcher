<script lang="ts">
  import { t } from './i18nStore.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { untrack } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'
  import type { LauncherSettings } from './settings'
  import ListSkeleton from './components/ListSkeleton.svelte'

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
          searched = false
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

<style>
  .modpack-search {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0 12px;
  }
  .modpack-search .panel-search-input {
    flex: 1;
    min-width: 0;
  }
  .modpack-search-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 32px;
    min-height: 32px;
  }
  .modpack-icon-fallback {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-bg);
    color: var(--accent);
  }
  .modpack-status {
    margin-top: 8px;
  }
</style>

<PanelHeader title={t('panel.modpacks')} {onBack} />

<div class="panel-body">
  <div class="versions-client-switcher switcher-wrap">
    <button class="vcs-btn" class:active={source === 'cf'} aria-pressed={source === 'cf'} onclick={() => setSource('cf')}>CurseForge</button>
    <button class="vcs-btn" class:active={source === 'mr'} aria-pressed={source === 'mr'} onclick={() => setSource('mr')}>Modrinth</button>
  </div>

  <div class="modpack-search">
    <input
      class="panel-search-input"
      aria-label={t('ph.searchModpacks')}
      placeholder={t('ph.searchModpacks')}
      bind:value={query}
      onkeydown={onSearchKey}
    />
    <button class="btn-sm modpack-search-btn" onclick={search} aria-label={t('tt.searchModpacks')}><i class="fa-solid fa-magnifying-glass"></i></button>
  </div>

  {#if source === 'cf' && !cfAvailable}
    <div class="empty-state">
      <i class="fa-solid fa-key"></i>
      <span>{t('empty.curseforgeApiKeyIsRequiredBrowse')}</span>
    </div>
  {/if}

  {#if searching}
    <ListSkeleton rows={4} />
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
    <div class="cache-notice modpack-status"><i class="fa-solid fa-circle-info"></i><span>{installMsg}</span></div>
  {/if}

  <div class="client-card-cols">
  {#each results as r (r.source + r.id)}
    <div class="client-card">
      <div class="client-card-header">
        {#if r.icon}
          <img class="client-card-icon client-card-icon-img" src={r.icon} loading="lazy" alt="" />
        {:else}
          <div class="client-card-icon modpack-icon-fallback"><i class="fa-solid fa-cubes"></i></div>
        {/if}
        <div class="client-card-meta">
          <span class="client-card-name">{r.title}</span>
          <span class="client-card-sub">{r.author} · {formatDownloads(r.downloads)}</span>
        </div>
        <div class="client-card-actions">
          <button
            class="vcs-btn"
            disabled={installingId !== null}
            onclick={() => install(r)}
            data-tooltip={t('tt.downloadPackInstallItIntoNew')}
          >
            {#if installingId === r.source + r.id}
              <Spinner />
            {:else}
              <i class="fa-solid fa-download"></i>&nbsp;Install
            {/if}
          </button>
        </div>
      </div>
      <p class="client-card-desc">{truncate(r.summary, 140)}</p>
    </div>
  {/each}
  </div>
</div>
