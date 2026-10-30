<script lang="ts">
  import { t } from './i18nStore.svelte'
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
  import ListSkeleton from './components/ListSkeleton.svelte'

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

// QOL: warn before deleting a world with no backup newer than its last
// save. Backups snapshot the whole com.mojang folder rather than one world
// at a time, so "backed up" here means "there's a backup at all newer than
// this world's last modification" - not a precise per-world guarantee, but
// enough to catch the actual danger case: deleting unsaved-anywhere-else
// progress.
let newestBackupAt = $state<string>('')
async function loadNewestBackupTime() {
  try {
    const backups = await invoke<{ created_at: string }[]>('list_bedrock_backups')
    newestBackupAt = backups.reduce((max, b) => (b.created_at > max ? b.created_at : max), '')
  } catch {
    // Best-effort — no warning shown if this fails, rather than blocking deletes.
  }
}
function lacksRecentBackup(world: BedrockWorld): boolean {
  return !newestBackupAt || newestBackupAt < world.modified_at
}

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
 * QOL "world thumbnail generation for worlds missing one": there's no real
 * minimap/screenshot to render without loading the world's chunk data (a
 * genuinely separate subsystem), but a plain repeated globe icon for every
 * world with no `icon.png` makes the grid impossible to tell apart at a
 * glance. This deterministically derives a color and initial from the
 * world's name instead — same idea as a chat app's fallback avatar.
 */
const FALLBACK_PALETTE = ['#7289da', '#43b581', '#faa61a', '#f04747', '#eb459e', '#00d8ff', '#a855f7', '#84cc16']
function worldFallbackColor(name: string): string {
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = (hash * 31 + name.charCodeAt(i)) >>> 0
  return FALLBACK_PALETTE[hash % FALLBACK_PALETTE.length]
}
function worldInitial(name: string): string {
  return (name.trim()[0] ?? '?').toUpperCase()
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
    onToast?.(t('toast.deletedName', { name: world.name }), 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/** Port of `ExportBedrockWorld` (Home.Worlds.cs:51). */
async function exportBedrockWorld(world: BedrockWorld) {
  try {
    onToast?.(t('toast.exportingName', { name: world.name }), 'info')
    // `export_bedrock_world` owns the destination, matching the C# service
    // (always `%USERPROFILE%\Glacier Launcher\exported-worlds`).
    const path = await invoke<string>('export_bedrock_world', {
      worldPath: world.folder_path,
    })
    onToast?.(t('toast.exportedNameTo', { name: world.name, path }), 'success')
    onToast?.(t('toast.worldExported'), 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

$effect(() => {
  instance
  refreshBedrockWorlds()
})

loadNewestBackupTime()

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

<style>
  .bw-notice {
    margin: 10px 14px 0;
  }
  .bw-grow { flex: 1; }
  .bw-initial {
    font-weight: 800;
    font-size: 16px;
  }
</style>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<div class="panel-header has-back">
  <button
    type="button"
    class="panel-back-btn"
    onclick={onBack}
    onmousedown={(e) => e.stopPropagation()}
    data-tooltip={t('tt.back')}
    aria-label={t('tt.back')}
  >
    <i class="fa-solid fa-arrow-left"></i>
  </button>
  <div class="panel-title-wrap">
    <span class="panel-title">{t('label.worlds')}</span>
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
      aria-label={t('tt.openFolder')}
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      class="panel-icon-btn {isLoadingBedrockWorlds ? 'spinning' : ''}"
      disabled={isLoadingBedrockWorlds}
      onclick={refreshBedrockWorlds}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  </div>
</div>

<SubTabs strip="worlds-servers" active="worlds" {onNavigate} />

<div class="panel-body">
  {#if !marketplaceReminderDismissed}
    <div class="notice-bar is-info bw-notice">
      <i class="fa-solid fa-circle-info"></i>
      <span class="bw-grow"
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
    <ListSkeleton rows={4} />
  {:else if bedrockWorlds.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-globe"></i>
      <span>{t('empty.noWorldsYet')}</span>
      <small>{t('empty.worldsYouCreateMinecraftOrImport')}</small>
    </div>
  {:else}
    <div class="client-card-cols">
    {#each bedrockWorlds as world (world.folder_path)}
      {@const iconUrl = worldIconUrl(world.icon_path)}
      <div class="client-card">
        <div class="client-card-header">
          {#if iconUrl}
            <img class="client-card-icon client-card-icon-img" src={iconUrl} alt="" loading="lazy" decoding="async" />
          {:else}
            <div class="client-card-icon bw-initial" style="background:{worldFallbackColor(world.name)}22; color:{worldFallbackColor(world.name)};">
              {worldInitial(world.name)}
            </div>
          {/if}
          <div class="client-card-meta">
            <span class="client-card-name">{world.name}</span>
            <span class="client-card-sub"
              >{formatSize(world.size_bytes)} · {formatRelativeTime(world.modified_at)}{worldSeeds[
                world.folder_path
              ]
                ? ` · Seed: ${worldSeeds[world.folder_path]}`
                : ''}</span
            >
          </div>
        </div>
        <div class="client-card-actions">
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
              title={lacksRecentBackup(world) ? 'Confirm delete (no recent backup!)' : 'Confirm delete'}
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
        {#if _confirmDeleteWorldPath === world.folder_path && lacksRecentBackup(world)}
          <p class="client-card-desc warning-text">
            <i class="fa-solid fa-triangle-exclamation"></i> No backup newer than this world's last save — deleting it may lose unsaved progress.
          </p>
        {/if}
      </div>
    {/each}
    </div>
  {/if}
</div>
