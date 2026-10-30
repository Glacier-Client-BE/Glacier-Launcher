<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * Port of the "levimods" panel — Home.razor:2988-3101 with its code-behind in
   * Home.LeviLaminaMods.cs, backed by `Services/LeviLaminaModsService.cs`
   * (ported as `gl_clients::levilamina_mods`).
   *
   * Bedrock-side LeviLamina mod browser — distinct from the Java Fabric/Forge
   * panel (JavaModsView.svelte). `SearchAsync` filters the (cached) full
   * listing rather than hitting a search endpoint, so the debounced re-search
   * below reproduces the original's behaviour exactly: spinner, refetch,
   * filter.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import ViewModeToggle from './components/ViewModeToggle.svelte'
  import { loadViewMode, saveViewMode, type ViewMode } from './viewMode'
  import ListSkeleton from './components/ListSkeleton.svelte'

  let { onBack, onToast }: { onBack?: () => void; onToast?: (m: string, kind?: string) => void } =
    $props()

  type LeviLaminaMod = {
    repo_owner: string
    repo_name: string
    name: string
    description: string
    avatar_url: string
    latest_version: string
    stars: number
  }

  let levModsQuery = $state('')
  let levModsSearching = $state(false)
  let levModsError = $state('')
  let levModsResults = $state<LeviLaminaMod[]>([])
  let levModsViewMode = $state<ViewMode>(loadViewMode('levilamina-mods'))
  function setLevModsViewMode(m: ViewMode) {
    levModsViewMode = m
    saveViewMode('levilamina-mods', m)
  }
  /**
   * Port of `levModsInstallingRepo` — the original keys this on `RepoName`
   * alone, not on owner/name, so two same-named repos from different owners
   * would both show progress. Kept as-is rather than "fixed".
   */
  let levModsInstallingRepo = $state('')
  let installed = $state<Record<string, boolean>>({})

  function key(m: LeviLaminaMod) {
    return `${m.repo_owner}/${m.repo_name}`
  }

  /** Port of `LevModsSearchAsync`. */
  async function levModsSearchAsync() {
    levModsSearching = true
    levModsError = ''
    try {
      const all = await invoke<LeviLaminaMod[]>('fetch_levilamina_mods')
      const q = levModsQuery.trim().toLowerCase()
      levModsResults = q
        ? all.filter(
            (m) =>
              m.name.toLowerCase().includes(q) || m.description.toLowerCase().includes(q),
          )
        : all
      for (const m of levModsResults) {
        installed[key(m)] = await invoke<boolean>('levilamina_mod_installed', {
          repoOwner: m.repo_owner,
          repoName: m.repo_name,
        }).catch(() => false)
      }
    } catch (e) {
      levModsError = String(e)
    } finally {
      levModsSearching = false
    }
  }

  /** Port of `DebouncedLevModsSearch` — 350ms, latest input wins. */
  let debounceTimer: ReturnType<typeof setTimeout> | undefined
  function onLevModsSearchInput(value: string) {
    levModsQuery = value
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(levModsSearchAsync, 350)
  }

  /** Port of `HandleLevModInstall` — same toast texts. */
  async function handleLevModInstall(m: LeviLaminaMod) {
    if (levModsInstallingRepo) return
    levModsInstallingRepo = m.repo_name
    try {
      await invoke('download_levilamina_mod', {
        repoOwner: m.repo_owner,
        repoName: m.repo_name,
        latestVersion: m.latest_version,
        modName: m.name,
      })
      installed[key(m)] = true
      onToast?.(t('toast.nameInstalledExcl', { name: m.name }), 'success')
    } catch (e) {
      onToast?.(t('toast.installFailed', { error: String(e) }), 'error')
    } finally {
      levModsInstallingRepo = ''
    }
  }

  /** Port of `HandleLevModDelete`. */
  async function handleLevModDelete(m: LeviLaminaMod) {
    try {
      await invoke('delete_levilamina_mod', { repoOwner: m.repo_owner, repoName: m.repo_name })
      installed[key(m)] = false
      onToast?.(t('toast.nameRemoved', { name: m.name }), 'info')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  /**
   * Live install progress, replacing the original's `levModsInstallProgress`
   * + `ThrottledProgress`. The queue entry is labelled with the mod's display
   * name, matching the C#'s `label: mod.Name`.
   */
  type QueueEntry = { id: string; label: string; status: string; progress: number }
  let queue = $state<QueueEntry[]>([])
  let unlisten: UnlistenFn | undefined
  let levDisposed = false

  function progressFor(m: LeviLaminaMod): number {
    return queue.find((e) => e.status === 'Downloading' && e.label === m.name)?.progress ?? 0
  }

  onMount(() => {
    listen<QueueEntry[]>('downloads-changed', (e) => {
      queue = e.payload
    }).then((fn) => {
      if (levDisposed) fn()
      else unlisten = fn
    })
  })
  onDestroy(() => {
    levDisposed = true
    if (debounceTimer) clearTimeout(debounceTimer)
    unlisten?.()
  })

  // Port of `OpenLeviLaminaMods`: search once on open.
  levModsSearchAsync()
</script>

<style>
  .lev-list:not(.grid-view) {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .lev-card {
    margin: 0;
  }
  .lev-avatar-wrap {
    padding: 3px;
    border-radius: var(--r-sm);
    overflow: hidden;
  }
  .lev-avatar {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 8px;
  }
  .lev-fallback-icon {
    font-size: 18px;
    color: var(--accent);
  }
  .lev-name {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lev-sub {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .lev-dim {
    font-size: 11px;
    color: var(--text-dim);
  }
  .lev-error-block {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 20px;
    text-align: center;
  }
  .lev-inline-error {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
  }
</style>

<PanelHeader title={t('panel.leviLaminaMods')} {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn {levModsSearching ? 'spinning' : ''}"
      disabled={levModsSearching}
      onclick={levModsSearchAsync}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <ViewModeToggle mode={levModsViewMode} onChange={setLevModsViewMode} />
  {/snippet}
</PanelHeader>

<div class="panel-search-wrap">
  <i class="fa-solid fa-magnifying-glass"></i>
  <input
    class="panel-search-input"
    aria-label={t('ph.searchLevilamina')}
    placeholder={t('ph.searchLevilamina')}
    value={levModsQuery}
    oninput={(e) => onLevModsSearchInput(e.currentTarget.value)}
    onkeydown={(e) => e.stopPropagation()}
  />
</div>

<div class="panel-body">
  {#if levModsSearching}
    <ListSkeleton rows={4} />
  {:else if levModsError && levModsResults.length === 0}
    <div class="lev-error-block">
      <span class="error-text"><i class="fa-solid fa-triangle-exclamation"></i> {levModsError}</span>
      <button
        class="btn-sm"
        onclick={levModsSearchAsync}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-rotate"></i> {t('common.retry')}
      </button>
    </div>
  {:else if levModsResults.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-box-open"></i>
      <span>{t('empty.noModsFound')}</span>
      <small>{t('empty.tryDifferentSearchTerm')}</small>
    </div>
  {:else}
    {#if levModsError}
      <span class="error-text lev-inline-error"><i class="fa-solid fa-triangle-exclamation"></i> {levModsError}</span>
    {/if}
    <div class="version-list lev-list {levModsViewMode === 'grid' ? 'grid-view' : ''}">
    {#each levModsResults as m (key(m))}
      <div class="client-card lev-card">
        <div class="client-card-header">
          <div class="client-card-icon lev-avatar-wrap">
            {#if m.avatar_url}
              <img
                src={m.avatar_url}
                loading="lazy"
                decoding="async"
                class="lev-avatar"
                alt=""
              />
            {:else}
              <i class="fa-solid fa-puzzle-piece lev-fallback-icon"></i>
            {/if}
          </div>
          <div class="client-card-meta">
            <span class="client-card-name lev-name">{m.name}</span>
            <span class="client-card-sub lev-sub">
              <span class="lev-dim">v{m.latest_version}</span>
              <span class="lev-dim"><i class="fa-solid fa-star"></i> {m.stars}</span>
            </span>
          </div>
          <div class="client-card-actions">
            {#if levModsInstallingRepo === m.repo_name}
              <InlineProgress progress={progressFor(m)} />
            {:else if installed[key(m)]}
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip={t('tt.remove')}
                aria-label={t('tt.remove')}
                onclick={() => handleLevModDelete(m)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-trash"></i>
              </button>
            {:else}
              <button
                class="icon-btn"
                data-tooltip={t('tt.install')}
                aria-label={t('tt.install')}
                onclick={() => handleLevModInstall(m)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-download"></i>
              </button>
            {/if}
          </div>
        </div>
        {#if m.description}
          <p class="client-card-desc">{m.description}</p>
        {/if}
      </div>
    {/each}
    </div>
  {/if}
</div>
