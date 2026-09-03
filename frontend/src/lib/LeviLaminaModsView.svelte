<script lang="ts">
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
   *
   * Not ported: zip-extraction of the downloaded release asset into the mods
   * folder — the raw asset is downloaded as-is (see the backend's
   * `levilamina_mods.rs` doc comment).
   */
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy, onMount } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import InlineProgress from './components/InlineProgress.svelte'

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
    levModsInstallingRepo = m.repo_name
    try {
      await invoke('download_levilamina_mod', {
        repoOwner: m.repo_owner,
        repoName: m.repo_name,
        latestVersion: m.latest_version,
      })
      installed[key(m)] = true
      onToast?.(`${m.name} installed!`, 'success')
    } catch (e) {
      onToast?.(`Install failed: ${String(e)}`, 'error')
    } finally {
      levModsInstallingRepo = ''
    }
  }

  /** Port of `HandleLevModDelete`. */
  async function handleLevModDelete(m: LeviLaminaMod) {
    try {
      await invoke('delete_levilamina_mod', { repoOwner: m.repo_owner, repoName: m.repo_name })
      installed[key(m)] = false
      onToast?.(`${m.name} removed.`, 'info')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  /**
   * Live install progress, replacing the original's `levModsInstallProgress`
   * + `ThrottledProgress`. The backend labels the queue entry
   * `<owner>/<repo> <tag>`; the tag isn't known here, so match on the prefix.
   */
  type QueueEntry = { id: string; label: string; status: string; progress: number }
  let queue = $state<QueueEntry[]>([])
  let unlisten: UnlistenFn | undefined

  function progressFor(m: LeviLaminaMod): number {
    const prefix = `${key(m)} `
    return (
      queue.find((e) => e.status === 'Downloading' && e.label.startsWith(prefix))?.progress ?? 0
    )
  }

  onMount(() => {
    listen<QueueEntry[]>('downloads-changed', (e) => {
      queue = e.payload
    }).then((fn) => {
      unlisten = fn
    })
  })
  onDestroy(() => {
    if (debounceTimer) clearTimeout(debounceTimer)
    unlisten?.()
  })

  // Port of `OpenLeviLaminaMods`: search once on open.
  levModsSearchAsync()
</script>

<PanelHeader title="LeviLamina Mods" {onBack} backIcon="fa-chevron-left">
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn {levModsSearching ? 'spinning' : ''}"
      disabled={levModsSearching}
      onclick={levModsSearchAsync}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
      aria-label="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-search-wrap">
  <i class="fa-solid fa-magnifying-glass"></i>
  <input
    class="panel-search-input"
    placeholder="Search LeviLamina mods..."
    value={levModsQuery}
    oninput={(e) => onLevModsSearchInput(e.currentTarget.value)}
    onkeydown={(e) => e.stopPropagation()}
  />
</div>

<div class="panel-body">
  {#if levModsSearching}
    <div style="display:flex; align-items:center; justify-content:center; padding:30px;">
      <span class="spinner" style="width:18px;height:18px;border-width:2px;"></span>
      <span style="margin-left:10px; font-size:12px; color:var(--text-dim);">Loading mods...</span>
    </div>
  {:else if levModsError && levModsResults.length === 0}
    <div style="padding:20px; text-align:center;">
      <span class="error-text">{levModsError}</span>
      <br />
      <button
        class="btn-sm"
        style="margin-top:10px;"
        onclick={levModsSearchAsync}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-rotate"></i> Retry
      </button>
    </div>
  {:else if levModsResults.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-box-open"></i>
      <span>No mods found</span>
      <small>Try a different search term.</small>
    </div>
  {:else}
    {#if levModsError}
      <span class="error-text" style="display:block; margin-bottom:6px;">{levModsError}</span>
    {/if}
    {#each levModsResults as m (key(m))}
      <div class="client-card" style="margin-bottom:6px;">
        <div class="client-card-header">
          <div class="client-card-icon" style="padding:3px; border-radius:6px; overflow:hidden;">
            {#if m.avatar_url}
              <img
                src={m.avatar_url}
                style="width:100%;height:100%;object-fit:cover;border-radius:4px;"
                alt=""
              />
            {:else}
              <i class="fa-solid fa-puzzle-piece" style="font-size:18px; color:var(--accent);"></i>
            {/if}
          </div>
          <div class="client-card-meta" style="min-width:0;">
            <span
              class="client-card-name"
              style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">{m.name}</span
            >
            <span class="client-card-sub" style="display:flex; gap:8px; align-items:center;">
              <span style="opacity:0.6;">v{m.latest_version}</span>
              <span style="opacity:0.4;"><i class="fa-solid fa-star"></i> {m.stars}</span>
            </span>
          </div>
          <div class="client-card-actions">
            {#if levModsInstallingRepo === m.repo_name}
              <InlineProgress progress={progressFor(m)} />
            {:else if installed[key(m)]}
              <button
                class="icon-btn icon-btn-ghost"
                data-tooltip="Remove"
                aria-label="Remove"
                onclick={() => handleLevModDelete(m)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-trash"></i>
              </button>
            {:else}
              <button
                class="icon-btn"
                data-tooltip="Install"
                aria-label="Install"
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
  {/if}
</div>
