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
import EditionTabs from './components/EditionTabs.svelte'
import type { BedrockInstance } from './settings'

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

/** Port of Home.razor.cs:692 `FormatSize`. */
function formatSize(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit++
  }
  return `${value.toFixed(2).replace(/\.?0+$/, '')} ${units[unit]}`
}

/** Port of Home.razor.cs:677 `FormatRelativeTime`. */
function formatRelativeTime(iso: string): string {
  try {
    const dt = new Date(iso)
    if (Number.isNaN(dt.getTime())) return ''
    const diffMs = Date.now() - dt.getTime()
    const minutes = diffMs / 60000
    if (minutes < 2) return 'just now'
    const hours = minutes / 60
    if (hours < 1) return `${Math.floor(minutes)}m ago`
    const days = hours / 24
    if (days < 1) return `${Math.floor(hours)}h ago`
    if (days < 7) return `${Math.floor(days)}d ago`
    return dt.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  } catch {
    return ''
  }
}

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

function parentDir(path: string): string {
  const idx = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return idx > 0 ? path.slice(0, idx) : ''
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
</script>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<div class="panel-header">
  <div class="panel-title-wrap">
    <span class="panel-title">Worlds</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn"
      disabled={!worldsDir}
      onclick={openBedrockWorldsFolder}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={worldsDir
        ? 'Open folder'
        : 'Open folder — unavailable until worlds are listed'}
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      class="panel-icon-btn {isLoadingBedrockWorlds ? 'spinning' : ''}"
      disabled={isLoadingBedrockWorlds}
      onclick={refreshBedrockWorlds}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <button
      class="panel-back-btn"
      onclick={onBack}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Back"
    >
      <i class="fa-solid fa-chevron-down"></i>
    </button>
  </div>
</div>

<SubTabs strip="worlds-servers" active="worlds" {onNavigate} />

<div class="panel-body">
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
            >{formatSize(world.size_bytes)} · {formatRelativeTime(world.modified_at)}</span
          >
        </div>
        <div class="version-actions">
          <button
            class="icon-btn icon-btn-ghost"
            data-tooltip="Open folder"
            onclick={() => openBedrockWorldFolder(world)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-folder-open"></i>
          </button>
          <button
            class="icon-btn icon-btn-ghost"
            data-tooltip="Export as .mcworld"
            onclick={() => exportBedrockWorld(world)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-file-export"></i>
          </button>
          <button
            class="icon-btn icon-btn-ghost"
            data-tooltip="Edit level.dat"
            onclick={() => onEditLevelDat(world.folder_path)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-sliders"></i>
          </button>
          {#if _confirmDeleteWorldPath === world.folder_path}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Confirm delete"
              onclick={() => deleteBedrockWorld(world)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-check"></i>
            </button>
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Cancel"
              onclick={cancelDeleteWorld}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-xmark"></i>
            </button>
          {:else}
            <button
              class="icon-btn icon-btn-ghost"
              data-tooltip="Delete"
              onclick={() => confirmDeleteWorld(world.folder_path)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>

<EditionTabs edition="bedrock" active="worlds" {onNavigate} />
