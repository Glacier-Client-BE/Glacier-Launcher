<script lang="ts">
  import { t } from './i18nStore.svelte'
/**
 * `bedrockpacks` panel — literal port of Home.razor:3929-4021 plus its
 * code-behind Pages/Home.Packs.cs (whole file).
 *
 * Transcription notes (deviations are called out, nothing is invented):
 *  - The original markup renders NO pack-kind switcher. `SwitchBedrockPacksKind`
 *    (Home.Packs.cs:25) exists in the code-behind but has no call site anywhere
 *    in the WPF app (`grep -rn "SwitchBedrockPacksKind"` → only its definition;
 *    `OpenBedrockPacks()` is always invoked via the zero-arg overload, which
 *    hard-codes "resource"). So the kind is state + an exported switch
 *    function, exactly as in the original, and no UI element was added.
 *  - The original's kind strings are already `resource` / `behavior` / `skin` /
 *    `resource-dev` / `behavior-dev` / `skin-dev` (Services/BedrockPackService.cs
 *    DirFor), which is exactly what the Tauri backend accepts — no mapping needed.
 */
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import SubTabs from './components/SubTabs.svelte'
import IconBtn from './components/IconBtn.svelte'
import type { BedrockInstance } from './settings'
  import ListSkeleton from './components/ListSkeleton.svelte'

/** Port of Models/BedrockPack.cs (Rust: gl-bedrock model.rs:52). */
type BedrockPack = {
  name: string
  folder_path: string
  kind: string
  size_bytes: number
  modified_at: string
  icon_path: string | null
}

export type BedrockPacksKind =
  | 'resource'
  | 'behavior'
  | 'skin'
  | 'resource-dev'
  | 'behavior-dev'
  | 'skin-dev'

let {
  instance = null,
  initialKind = 'resource',
  onNavigate,
  onBack,
  onToast,
}: {
  instance?: BedrockInstance | null
  /** Port of `OpenBedrockPacks(string kind)` — the entry kind. Default "resource". */
  initialKind?: BedrockPacksKind
  onNavigate: (view: string) => void
  onBack: () => void
  onToast?: (message: string, kind: 'success' | 'error') => void
} = $props()

// Home.Packs.cs:10-13
let bedrockPacks = $state<BedrockPack[]>([])
let isLoadingBedrockPacks = $state(false)
// svelte-ignore state_referenced_locally -- mirrors OpenBedrockPacks(kind) seeding the field once
let bedrockPacksKind = $state<BedrockPacksKind>(initialKind)
let confirmDeletePackPath = $state('')

/** Port of `SwitchBedrockPacksKind` (Home.Packs.cs:25). Exported: the original
 *  has no in-panel call site, so the only way to reach it is from outside. */
export async function switchBedrockPacksKind(kind: BedrockPacksKind) {
  if (bedrockPacksKind === kind) return
  bedrockPacksKind = kind
  await refreshBedrockPacks()
}

/** Port of `RefreshBedrockPacksAsync` (Home.Packs.cs:33). */
export async function refreshBedrockPacks() {
  isLoadingBedrockPacks = true
  const kind = bedrockPacksKind
  try {
    bedrockPacks = await invoke<BedrockPack[]>('list_bedrock_packs', {
      // An empty instanceDir means "the live game install"; the backend
      // resolves the real per-account com.mojang path itself.
      instanceDir: instance?.directory ?? '',
      kind,
    })
  } catch (e) {
    bedrockPacks = []
    onToast?.(String(e), 'error')
  }
  isLoadingBedrockPacks = false
}

$effect(() => {
  void instance
  untrack(() => refreshBedrockPacks())
})

/**
 * Port of `OpenBedrockPacksFolder` (Home.Packs.cs:43), which opens
 * `BedrockPackService.DirFor(kind)`. There is no backend command that returns
 * that directory: `list_bedrock_packs` resolves it internally and only
 * `open_folder(path)` exists, so the path can only be derived from a listed
 * pack's parent. When the list is empty the control is rendered disabled with
 * an explanatory tooltip rather than silently doing nothing.
 */
const packsFolderPath = $derived.by(() => {
  const first = bedrockPacks[0]?.folder_path
  if (!first) return ''
  const cut = Math.max(first.lastIndexOf('\\'), first.lastIndexOf('/'))
  return cut > 0 ? first.slice(0, cut) : ''
})

async function openBedrockPacksFolder() {
  if (!packsFolderPath) return
  try {
    await invoke('open_folder', { path: packsFolderPath })
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/** Port of `OpenBedrockPackFolder` (Home.Packs.cs:49). */
async function openBedrockPackFolder(pack: BedrockPack) {
  try {
    await invoke('open_folder', { path: pack.folder_path })
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

// Home.Packs.cs:55-56
function confirmDeletePack(folderPath: string) {
  confirmDeletePackPath = folderPath
}
function cancelDeletePack() {
  confirmDeletePackPath = ''
}

/** Port of `DeleteBedrockPack` (Home.Packs.cs:58). */
async function deleteBedrockPack(pack: BedrockPack) {
  try {
    await invoke('delete_bedrock_pack', { packPath: pack.folder_path })
    confirmDeletePackPath = ''
    await refreshBedrockPacks()
    onToast?.(t('toast.deletedName', { name: pack.name }), 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/** Port of `FormatSize` (Home.razor.cs:692). */
function formatSize(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit++
  }
  // C# "0.##" — up to two decimals, trailing zeros trimmed.
  return `${parseFloat(value.toFixed(2))} ${units[unit]}`
}

/** Port of `FormatRelativeTime` (Home.razor.cs:677). */
function formatRelativeTime(isoTimestamp: string): string {
  const dt = Date.parse(isoTimestamp)
  if (Number.isNaN(dt)) return ''
  const diffMs = Date.now() - dt
  const minutes = diffMs / 60000
  const hours = diffMs / 3600000
  const days = diffMs / 86400000
  if (minutes < 2) return 'just now'
  if (hours < 1) return `${Math.trunc(minutes)}m ago`
  if (days < 1) return `${Math.trunc(hours)}h ago`
  if (days < 7) return `${Math.trunc(days)}d ago`
  return new Date(dt).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
}

/** Port of `FileToMojangUrl` — a local file path rendered as an <img> src. */
function fileToUrl(path: string | null): string {
  return path ? convertFileSrc(path) : ''
}
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
    data-tooltip={t('tt.back')}
    aria-label={t('tt.back')}
  >
    <i class="fa-solid fa-arrow-left"></i>
  </button>
  <div class="panel-title-wrap">
    <span class="panel-title">{t('nav.packs')}</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-spacer"></div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn"
      disabled={!packsFolderPath}
      onclick={openBedrockPacksFolder}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={packsFolderPath
        ? 'Open folder'
        : 'Open folder — unavailable: the packs folder path is resolved by the game install and is only known once packs are listed'}
      aria-label={t('tt.openFolder')}
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      class="panel-icon-btn {isLoadingBedrockPacks ? 'spinning' : ''}"
      disabled={isLoadingBedrockPacks}
      onclick={refreshBedrockPacks}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
      aria-label={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  </div>
</div>

<SubTabs strip="packs-addons" active="packs" edition="bedrock" {onNavigate} />

<!--
  `bedrockPacksKind` and `switchBedrockPacksKind` existed as dead code before
  this: no caller anywhere in the app ever set `initialKind` or called the
  exported switcher, so the panel could only ever show resource packs. A
  first pass added a 3-button strip for Resource/Behavior/Skin, but the
  "-dev" counterparts of all three (`BedrockPacksKind` already had all 6)
  stayed just as unreachable as before. With 6 kinds a strip stops being the
  right shape — same master-detail rail as Settings/Java Mods/Tweaks, so
  every kind is one click away instead of needing a second control to see
  the dev variants at all.
-->
<div class="settings-layout">
  <nav class="settings-nav-rail" aria-label={t('tt.packKind')}>
    <button
      type="button"
      class="settings-nav-btn {bedrockPacksKind === 'resource' ? 'active' : ''}"
      onclick={() => switchBedrockPacksKind('resource')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-palette"></i> Resource
    </button>
    <button
      type="button"
      class="settings-nav-btn {bedrockPacksKind === 'behavior' ? 'active' : ''}"
      onclick={() => switchBedrockPacksKind('behavior')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-gears"></i> Behavior
    </button>
    <button
      type="button"
      class="settings-nav-btn {bedrockPacksKind === 'skin' ? 'active' : ''}"
      onclick={() => switchBedrockPacksKind('skin')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-shirt"></i> Skin
    </button>
    <button
      type="button"
      class="settings-nav-btn {bedrockPacksKind === 'resource-dev' ? 'active' : ''}"
      onclick={() => switchBedrockPacksKind('resource-dev')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-palette"></i> Resource (Dev)
    </button>
    <button
      type="button"
      class="settings-nav-btn {bedrockPacksKind === 'behavior-dev' ? 'active' : ''}"
      onclick={() => switchBedrockPacksKind('behavior-dev')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-gears"></i> Behavior (Dev)
    </button>
    <button
      type="button"
      class="settings-nav-btn {bedrockPacksKind === 'skin-dev' ? 'active' : ''}"
      onclick={() => switchBedrockPacksKind('skin-dev')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-shirt"></i> Skin (Dev)
    </button>
  </nav>

  <div class="settings-content">
    <div class="panel-body">
      {#if isLoadingBedrockPacks}
        <ListSkeleton rows={4} />
      {:else if bedrockPacks.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-boxes-stacked"></i>
          <span>No {bedrockPacksKind} packs yet</span>
          <small
            >Import a .mcpack by dragging it onto the launcher, or install one from
            CurseForge/Modrinth in Addons.</small
          >
        </div>
      {:else}
        <div class="version-list grid-view">
        {#each bedrockPacks as pack (pack.folder_path)}
          {@const packIconUrl = fileToUrl(pack.icon_path)}
          <div class="version-row">
            {#if packIconUrl}
              <img class="version-icon" src={packIconUrl} alt="" loading="lazy" decoding="async" />
            {:else}
              <div class="version-icon version-icon-placeholder">
                <i class="fa-solid fa-boxes-stacked"></i>
              </div>
            {/if}
            <div class="version-meta">
              <span class="version-name">{pack.name}</span>
              <span class="version-sub"
                >{formatSize(pack.size_bytes)} · {formatRelativeTime(pack.modified_at)}</span
              >
            </div>
            <div class="version-actions">
              <IconBtn
                ghost
                icon="fa-solid fa-folder-open"
                title="Open folder"
                onClick={() => openBedrockPackFolder(pack)}
                onMouseDown={(e) => e.stopPropagation()}
              />
              {#if confirmDeletePackPath === pack.folder_path}
                <IconBtn
                  ghost
                  icon="fa-solid fa-check"
                  title="Confirm delete"
                  onClick={() => deleteBedrockPack(pack)}
                  onMouseDown={(e) => e.stopPropagation()}
                />
                <IconBtn
                  ghost
                  icon="fa-solid fa-xmark"
                  title="Cancel"
                  onClick={cancelDeletePack}
                  onMouseDown={(e) => e.stopPropagation()}
                />
              {:else}
                <IconBtn
                  ghost
                  icon="fa-solid fa-trash"
                  title="Delete"
                  onClick={() => confirmDeletePack(pack.folder_path)}
                  onMouseDown={(e) => e.stopPropagation()}
                />
              {/if}
            </div>
          </div>
        {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
