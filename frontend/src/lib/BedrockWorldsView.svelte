<script lang="ts">
/**
 * Literal port of the `bedrockworlds` panel — Home.razor:3829-3928, with
 * behaviour from Pages/Home.Worlds.cs (69 lines).
 *
 * Transcribed element-for-element: same DOM order, class names, tooltips and
 * strings. The delete confirmation is INLINE in the row
 * (`_confirmDeleteWorldPath`), never a modal.
 */
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import SubTabs from './components/SubTabs.svelte'
import IconBtn from './components/IconBtn.svelte'
import type { BedrockInstance } from './settings'
import { formatSize, formatRelativeTime, parentDir } from './format'

let {
  instance,
  onBack,
  onNavigate,
  onEditLevelDat,
  onToast,
}: {
  instance: BedrockInstance | null
  onBack: () => void
  /** Both tab strips route through the parent — this view never routes itself. */
  onNavigate: (view: string) => void
  onEditLevelDat: (worldDir: string) => void
  onToast?: (message: string, kind: 'info' | 'success' | 'error') => void
} = $props()

/** Serde emits snake_case (crates/gl-bedrock/src/model.rs:18-24). */
type BedrockWorld = {
  name: string
  folder_path: string
  size_bytes: number
  modified_at: string
  icon_path: string | null
}

// Port of Home.Worlds.cs:10-12.
let bedrockWorlds = $state<BedrockWorld[]>([])
let isLoadingBedrockWorlds = $state(false)
let _confirmDeleteWorldPath = $state('')

/**
 * Feature 22 — seed display. `read_level_dat_summary` already exists (backs
 * the level.dat editor's NBT parsing) and reads `RandomSeed` directly, so
 * this reuses it rather than adding a second NBT reader. Keyed by
 * folder_path and fetched best-effort per row: a world with an unreadable
 * or missing level.dat simply shows no seed instead of failing the list.
 */
let worldSeeds = $state<Record<string, string>>({})

async function loadWorldSeed(world: BedrockWorld) {
  try {
    const summary = await invoke<{ random_seed: number; has_random_seed: boolean }>(
      'read_level_dat_summary',
      { worldDir: world.folder_path },
    )
    if (summary.has_random_seed) {
      worldSeeds[world.folder_path] = String(summary.random_seed)
    }
  } catch {
    // Best-effort — no seed shown for this row.
  }
}

/** Port of Home.razor.cs:692 `FormatSize`. */
/**
 * Port of `FileToMojangUrl` (Home.QuickActions.cs:286). The WPF build served
 * world icons over a custom `glacier-mojang.local` scheme; the Tauri
 * equivalent is `convertFileSrc`.
 */
function worldIconUrl(path: string | null): string {
  if (!path) return ''
  try {
    return convertFileSrc(path)
  } catch {
    return ''
  }
}

/**
 * `BedrockWorldService.WorldsDir` was a static in the WPF build; no Tauri
 * command exposes it, so it is recovered from a listed world's folder. With
 * no worlds listed the header's "Open folder" button is rendered disabled
 * rather than made to silently do nothing.
 */
const worldsDir = $derived(
  bedrockWorlds.length > 0 ? parentDir(bedrockWorlds[0].folder_path) : '',
)

/** Port of `RefreshBedrockWorldsAsync` (Home.Worlds.cs:21). */
async function refreshBedrockWorlds() {
  isLoadingBedrockWorlds = true
  try {
    // An empty instanceDir means "the live install": the backend resolves the
    // real per-account com.mojang path itself.
    bedrockWorlds = await invoke<BedrockWorld[]>('list_bedrock_worlds', {
      instanceDir: instance?.directory ?? '',
    })
    worldSeeds = {}
    for (const world of bedrockWorlds) {
      loadWorldSeed(world)
    }
  } catch (e) {
    bedrockWorlds = []
    onToast?.(String(e), 'error')
  } finally {
    isLoadingBedrockWorlds = false
  }
}

/** Port of `OpenBedrockWorldsFolder` (Home.Worlds.cs:63). */
async function openBedrockWorldsFolder() {
  try {
    const dir = worldsDir || (await invoke<string>('bedrock_worlds_dir'))
    await invoke('open_folder', { path: dir })
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/** Port of `OpenBedrockWorldFolder` (Home.Worlds.cs:30). */
async function openBedrockWorldFolder(world: BedrockWorld) {
  try {
    await invoke('open_folder', { path: world.folder_path })
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

// Port of Home.Worlds.cs:36-37.
function confirmDeleteWorld(folderPath: string) {
  _confirmDeleteWorldPath = folderPath
}
function cancelDeleteWorld() {
  _confirmDeleteWorldPath = ''
}

/** Port of `DeleteBedrockWorld` (Home.Worlds.cs:39). */
async function deleteBedrockWorld(world: BedrockWorld) {
  try {
    await invoke('delete_bedrock_world', { worldPath: world.folder_path })
    _confirmDeleteWorldPath = ''
    await refreshBedrockWorlds()
    onToast?.(`Deleted '${world.name}'.`, 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/** Port of `ExportBedrockWorld` (Home.Worlds.cs:51). */
async function exportBedrockWorld(world: BedrockWorld) {
  try {
    onToast?.(`Exporting '${world.name}'\u2026`, 'info')
    // `export_bedrock_world` owns the destination, matching the C# service
    // (always `%USERPROFILE%\Glacier Launcher\exported-worlds`).
    const path = await invoke<string>('export_bedrock_world', {
      worldPath: world.folder_path,
    })
    onToast?.(`'${world.name}' was exported to ${path}.`, 'success')
    onToast?.('World exported.', 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

$effect(() => {
  instance
  refreshBedrockWorlds()
})

// Feature 23 — one-time dismissible marketplace-content reminder, shared
// with BedrockBackupsView via the same two settings-backed commands.
let marketplaceReminderDismissed = $state(true)

async function loadMarketplaceReminderDismissed() {
  try {
    marketplaceReminderDismissed = await invoke<boolean>('bedrock_marketplace_reminder_dismissed')
  } catch {
    marketplaceReminderDismissed = true
  }
}

async function dismissMarketplaceReminder() {
  marketplaceReminderDismissed = true
  try {
    await invoke('dismiss_bedrock_marketplace_reminder')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

loadMarketplaceReminderDismissed()
</script>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<div class="panel-header has-back">
  <button
    type="button"
    class="panel-back-btn"
    onclick={onBack}
    onmousedown={(e) => e.stopPropagation()}
    data-tooltip="Back"
    aria-label="Back"
  >
    <i class="fa-solid fa-arrow-left"></i>
  </button>
  <div class="panel-title-wrap">
    <span class="panel-title">Worlds</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-spacer"></div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn"
      disabled={!worldsDir}
      onclick={openBedrockWorldsFolder}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={worldsDir
        ? 'Open folder'
        : 'Open folder — unavailable until worlds are listed'}
      aria-label="Open folder"
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      class="panel-icon-btn {isLoadingBedrockWorlds ? 'spinning' : ''}"
      disabled={isLoadingBedrockWorlds}
      onclick={refreshBedrockWorlds}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
      aria-label="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  </div>
</div>

<SubTabs strip="worlds-servers" active="worlds" {onNavigate} />

<div class="panel-body">
  {#if !marketplaceReminderDismissed}
    <div
      class="warning-text"
      style="display:flex; align-items:center; gap:8px; padding:10px 14px; margin:10px 14px 0;"
    >
      <i class="fa-solid fa-circle-info"></i>
      <span style="flex:1;"
        >Heads up: marketplace-purchased content (maps, skin packs, texture packs bought in-game)
        isn't included when you export or back up a world locally — only your own saves and
        installed add-ons are.</span
      >
      <IconBtn
        ghost
        icon="fa-solid fa-xmark"
        title="Dismiss"
        onClick={dismissMarketplaceReminder}
        onMouseDown={(e) => e.stopPropagation()}
      />
    </div>
  {/if}
  {#if isLoadingBedrockWorlds}
    <div class="versions-loading"><span class="spinner"></span><span>Loading worlds…</span></div>
  {:else if bedrockWorlds.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-globe"></i>
      <span>No worlds yet</span>
      <small>Worlds you create in Minecraft, or import as .mcworld, show up here.</small>
    </div>
  {:else}
    {#each bedrockWorlds as world (world.folder_path)}
      {@const iconUrl = worldIconUrl(world.icon_path)}
      <div class="version-row">
        {#if iconUrl}
          <img class="version-icon" src={iconUrl} alt="" />
        {:else}
          <div class="version-icon version-icon-placeholder"><i class="fa-solid fa-globe"></i></div>
        {/if}
        <div class="version-meta">
          <span class="version-name">{world.name}</span>
          <span class="version-sub"
            >{formatSize(world.size_bytes)} · {formatRelativeTime(world.modified_at)}{worldSeeds[
              world.folder_path
            ]
              ? ` · Seed: ${worldSeeds[world.folder_path]}`
              : ''}</span
          >
        </div>
        <div class="version-actions">
          <IconBtn
            ghost
            icon="fa-solid fa-folder-open"
            title="Open folder"
            onClick={() => openBedrockWorldFolder(world)}
            onMouseDown={(e) => e.stopPropagation()}
          />
          <IconBtn
            ghost
            icon="fa-solid fa-file-export"
            title="Export as .mcworld"
            onClick={() => exportBedrockWorld(world)}
            onMouseDown={(e) => e.stopPropagation()}
          />
          <IconBtn
            ghost
            icon="fa-solid fa-sliders"
            title="Edit level.dat"
            onClick={() => onEditLevelDat(world.folder_path)}
            onMouseDown={(e) => e.stopPropagation()}
          />
          {#if _confirmDeleteWorldPath === world.folder_path}
            <IconBtn
              ghost
              icon="fa-solid fa-check"
              title="Confirm delete"
              onClick={() => deleteBedrockWorld(world)}
              onMouseDown={(e) => e.stopPropagation()}
            />
            <IconBtn
              ghost
              icon="fa-solid fa-xmark"
              title="Cancel"
              onClick={cancelDeleteWorld}
              onMouseDown={(e) => e.stopPropagation()}
            />
          {:else}
            <IconBtn
              ghost
              icon="fa-solid fa-trash"
              title="Delete"
              onClick={() => confirmDeleteWorld(world.folder_path)}
              onMouseDown={(e) => e.stopPropagation()}
            />
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>
