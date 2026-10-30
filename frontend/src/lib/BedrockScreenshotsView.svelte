<script lang="ts">
  import { t } from './i18nStore.svelte'
/**
 * Literal port of the `bedrockscreenshots` panel — Home.razor:4375-4477, with
 * behaviour from Pages/Home.BedrockScreenshots.cs (37 lines).
 *
 * Transcribed element-for-element: same DOM order, class names, tooltips and
 * strings. The original has no error state and no per-row delete — the grid is
 * tiles only.
 *
 * TODO(feature 29, unified screenshot/clip hotkey passthrough): skipped.
 * There is no existing folder-watch here to extend — this panel lists the
 * screenshots directory on demand (see `list_bedrock_screenshots` below),
 * it does not watch it. Building real passthrough (a Win32 hotkey hook plus
 * an fs watcher over a client-specific output folder, wired through a new
 * Tauri event) is a new subsystem, not a small addition, so it's left for a
 * dedicated pass rather than bolted on here.
 *
 * The screenshot source is always the *live* install: the C# side read the
 * static `BedrockScreenshotService.ComMojangScreenshotsDir`, and the Tauri
 * command ignores its `instanceDir` argument for the same reason. Nothing here
 * is per-instance.
 */
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { UTILITIES } from './components/SubTabs.svelte'
import PanelHeader from './components/PanelHeader.svelte'
import ListSkeleton from './components/ListSkeleton.svelte'
import { formatSize, formatRelativeTime, parentDir } from './format'

let {
  onBack,
  onNavigate,
  onToast,
}: {
  onBack: () => void
  /** Routes through the parent's Utilities-strip resolution — this view
   *  never navigates itself. */
  onNavigate: (view: string, strip?: string) => void
  onToast?: (message: string, kind: 'info' | 'success' | 'error') => void
} = $props()

/** Serde emits snake_case (crates/gl-bedrock/src/model.rs:70+). */
type ScreenshotFile = {
  name: string
  path: string
  kind: string
  size_bytes: number
  modified_at: string
}

// Port of Home.BedrockScreenshots.cs:10-11.
let bedrockScreenshots = $state<ScreenshotFile[]>([])
let isLoadingBedrockScreenshots = $state(false)

// QOL: per-screenshot favorite (persisted in settings, see
// gl-settings::LauncherSettings::bedrock_favorite_screenshots) and delete.
let favoritePaths = $state<Set<string>>(new Set())
let confirmDeletePath = $state('')

/** Favorited tiles first, otherwise the backend's already-sorted order. */
const sortedScreenshots = $derived(
  [...bedrockScreenshots].sort((a, b) => {
    const fa = favoritePaths.has(a.path) ? 1 : 0
    const fb = favoritePaths.has(b.path) ? 1 : 0
    return fb - fa
  }),
)

async function loadFavorites() {
  try {
    favoritePaths = new Set(await invoke<string[]>('list_favorite_screenshots'))
  } catch {
    // Best-effort — tiles just render unfavorited.
  }
}

async function toggleFavorite(shot: ScreenshotFile) {
  const wasFavorited = favoritePaths.has(shot.path)
  // Optimistic update — a starred state that lags a round trip behind a
  // click feels broken on a grid of tiles you're rapidly clicking through.
  const next = new Set(favoritePaths)
  if (wasFavorited) next.delete(shot.path)
  else next.add(shot.path)
  favoritePaths = next
  try {
    const nowFavorited = await invoke<boolean>('toggle_favorite_screenshot', { path: shot.path })
    if (nowFavorited !== !wasFavorited) await loadFavorites()
  } catch (e) {
    await loadFavorites()
    onToast?.(String(e), 'error')
  }
}

function confirmDelete(path: string) {
  confirmDeletePath = path
}
function cancelDelete() {
  confirmDeletePath = ''
}

// QOL: bulk select — export selected to a folder, or delete them all at
// once, instead of one-at-a-time only.
let selectMode = $state(false)
let selectedPaths = $state<Set<string>>(new Set())

function toggleSelectMode() {
  selectMode = !selectMode
  selectedPaths = new Set()
}

function toggleSelected(path: string) {
  const next = new Set(selectedPaths)
  if (next.has(path)) next.delete(path)
  else next.add(path)
  selectedPaths = next
}

async function deleteSelected() {
  const paths = [...selectedPaths]
  let failed = 0
  for (const path of paths) {
    try {
      await invoke('delete_bedrock_screenshot', { path })
    } catch {
      failed++
    }
  }
  bedrockScreenshots = bedrockScreenshots.filter((s) => !selectedPaths.has(s.path))
  selectedPaths = new Set()
  selectMode = false
  onToast?.(
    failed > 0
      ? `Deleted ${paths.length - failed} screenshot(s), ${failed} failed.`
      : `Deleted ${paths.length} screenshot(s).`,
    failed > 0 ? 'error' : 'success',
  )
}

async function exportSelected() {
  const dir = await openDialog({ directory: true, title: 'Export screenshots to…' })
  if (typeof dir !== 'string') return
  const paths = [...selectedPaths]
  let failed = 0
  for (const path of paths) {
    const name = path.split(/[\\/]/).pop() ?? 'screenshot.jpg'
    try {
      await invoke('copy_file', { src: path, dest: `${dir}/${name}` })
    } catch {
      failed++
    }
  }
  onToast?.(
    failed > 0
      ? `Exported ${paths.length - failed} screenshot(s), ${failed} failed.`
      : `Exported ${paths.length} screenshot(s).`,
    failed > 0 ? 'error' : 'success',
  )
}

/** QOL: copy the image itself to the clipboard (not just its path), so it
 *  can be pasted directly into Discord/etc. without opening the file. */
async function copyScreenshotToClipboard(shot: ScreenshotFile) {
  try {
    const res = await fetch(screenshotUrl(shot.path))
    const blob = await res.blob()
    await navigator.clipboard.write([new ClipboardItem({ [blob.type]: blob })])
    onToast?.(t('toast.copiedClipboard'), 'success')
  } catch (e) {
    onToast?.(t('toast.copyFailed', { error: String(e) }), 'error')
  }
}

async function deleteScreenshot(shot: ScreenshotFile) {
  try {
    await invoke('delete_bedrock_screenshot', { path: shot.path })
    bedrockScreenshots = bedrockScreenshots.filter((s) => s.path !== shot.path)
    favoritePaths = new Set([...favoritePaths].filter((p) => p !== shot.path))
    confirmDeletePath = ''
    onToast?.(t('toast.screenshotDeleted'), 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

/**
 * `BedrockScreenshotService.OpenFolder` had the screenshots directory to hand;
 * no Tauri command exposes it, so it is recovered from a listed screenshot's
 * path. With nothing listed the header's "Open folder" button is rendered
 * disabled rather than made to silently do nothing.
 */
const screenshotsDir = $derived(
  bedrockScreenshots.length > 0 ? parentDir(bedrockScreenshots[0].path) : '',
)

/**
 * Port of `FileToCapturesUrl` (Home.QuickActions.cs:269). The WPF build served
 * these over custom local schemes; the Tauri equivalent is `convertFileSrc`.
 */
function screenshotUrl(path: string): string {
  try {
    return convertFileSrc(path)
  } catch {
    return ''
  }
}

/** Port of `RefreshBedrockScreenshotsAsync` (Home.BedrockScreenshots.cs:23). */
async function refreshBedrockScreenshots() {
  isLoadingBedrockScreenshots = true
  try {
    // The backend always reads the live install and ignores this argument.
    bedrockScreenshots = await invoke<ScreenshotFile[]>('list_bedrock_screenshots', {
      instanceDir: '',
    })
  } catch (e) {
    bedrockScreenshots = []
    onToast?.(String(e), 'error')
  } finally {
    isLoadingBedrockScreenshots = false
  }
}

/** Port of `OpenScreenshot` (Home.BedrockScreenshots.cs) — now opens the
 *  in-app lightbox instead of shelling out to the OS's default viewer,
 *  which lost focus/context on the launcher and had no next/prev. */
function openScreenshot(shot: ScreenshotFile) {
  lightboxPath = shot.path
}

async function openInSystemViewer(shot: { path: string; name: string }) {
  try {
    await invoke('open_path', { path: shot.path })
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

// QOL: lightbox with arrow-key/click next-prev, instead of every screenshot
// click shelling out to the OS's default image viewer (losing focus on the
// launcher, no easy way to flip through the set).
let lightboxPath = $state<string | null>(null)
const lightboxIndex = $derived(sortedScreenshots.findIndex((s) => s.path === lightboxPath))
const lightboxShot = $derived(lightboxIndex >= 0 ? sortedScreenshots[lightboxIndex] : null)
function closeLightbox() {
  lightboxPath = null
}
function lightboxNext() {
  if (lightboxIndex < 0 || sortedScreenshots.length === 0) return
  lightboxPath = sortedScreenshots[(lightboxIndex + 1) % sortedScreenshots.length].path
}
function lightboxPrev() {
  if (lightboxIndex < 0 || sortedScreenshots.length === 0) return
  lightboxPath = sortedScreenshots[(lightboxIndex - 1 + sortedScreenshots.length) % sortedScreenshots.length].path
}
function lightboxKeydown(e: KeyboardEvent) {
  if (lightboxPath === null) return
  if (e.key === 'Escape') closeLightbox()
  else if (e.key === 'ArrowRight') lightboxNext()
  else if (e.key === 'ArrowLeft') lightboxPrev()
}

/** Port of `OpenBedrockScreenshotsFolder` (Home.BedrockScreenshots.cs:32). */
async function openBedrockScreenshotsFolder() {
  try {
    const dir = screenshotsDir || (await invoke<string>('bedrock_screenshots_dir'))
    await invoke('open_folder', { path: dir })
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

refreshBedrockScreenshots()
loadFavorites()
</script>

<style>
  .bs-bulk {
    gap: 8px;
    padding: 8px 14px;
    flex-wrap: wrap;
  }
  .bs-danger { color: var(--red); }
  .bs-star { color: var(--yellow); }
  .bs-lightbox-actions {
    display: flex;
    gap: 6px;
  }
</style>

<svelte:window onkeydown={lightboxKeydown} />

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<PanelHeader title={t('panel.screenshots')} {onBack}>
  {#snippet extraActions()}
    <button
      aria-label={t('tt.openFolder')}
      class="panel-icon-btn"
      onclick={openBedrockScreenshotsFolder}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.openFolder')}
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      aria-label={t('tt.refresh')}
      class="panel-icon-btn {isLoadingBedrockScreenshots ? 'spinning' : ''}"
      disabled={isLoadingBedrockScreenshots}
      onclick={refreshBedrockScreenshots}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refresh')}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    {#if bedrockScreenshots.length > 0}
      <button
        aria-label={t('tt.select')}
        class="panel-icon-btn {selectMode ? 'active' : ''}"
        onclick={toggleSelectMode}
        onmousedown={(e) => e.stopPropagation()}
        data-tooltip={selectMode ? 'Cancel selection' : 'Select multiple'}
      >
        <i class="fa-solid fa-check-double"></i>
      </button>
    {/if}
  {/snippet}
</PanelHeader>

{#if selectMode}
  <div class="version-row bs-bulk">
    <span class="version-sub">{selectedPaths.size} selected</span>
    <button class="btn-sm" onclick={exportSelected} disabled={selectedPaths.size === 0}>
      <i class="fa-solid fa-file-export"></i> Export selected
    </button>
    <button class="btn-sm bs-danger" onclick={deleteSelected} disabled={selectedPaths.size === 0}>
      <i class="fa-solid fa-trash"></i> Delete selected
    </button>
  </div>
{/if}

<div class="settings-layout">
  <nav class="settings-nav-rail" aria-label={t('tt.utilitiesSections')}>
    {#each UTILITIES as nav (nav.key)}
      <button
        type="button"
        class="settings-nav-btn {nav.key === 'photos' ? 'active' : ''}"
        onclick={() => onNavigate(nav.key, 'utilities')}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class={nav.icon}></i> {nav.label}
      </button>
    {/each}
  </nav>

  <div class="settings-content">
<div class="panel-body">
  {#if isLoadingBedrockScreenshots}
    <ListSkeleton rows={4} />
  {:else if bedrockScreenshots.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-image"></i>
      <span>{t('empty.noScreenshotsYet')}</span>
      <small
        >In-game screenshots and Xbox Game Bar captures (Win+Alt+PrtScn) both show up here.</small
      >
    </div>
  {:else}
    <span class="panel-section-label">{bedrockScreenshots.length} screenshot{bedrockScreenshots.length === 1 ? '' : 's'}</span>
    <div class="screenshot-grid">
      {#each sortedScreenshots as shot (shot.path)}
        <!-- Was a single <button> for the whole tile (`OpenScreenshot` ->
             `OpenUrl`); now a div with the same role so the favorite/delete
             buttons below can nest inside it without an invalid
             button-inside-button. -->
        <div
          class="screenshot-tile {selectMode && selectedPaths.has(shot.path) ? 'screenshot-tile-selected' : ''}"
          role="button"
          tabindex="0"
          onclick={() => (selectMode ? toggleSelected(shot.path) : openScreenshot(shot))}
          onkeydown={(e) => { if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) (selectMode ? toggleSelected(shot.path) : openScreenshot(shot)) }}
          onmousedown={(e) => e.stopPropagation()}
          title={shot.name}
        >
          {#if selectMode}
            <div class="screenshot-tile-select-check">
              <i class="fa-solid {selectedPaths.has(shot.path) ? 'fa-square-check' : 'fa-square'}"></i>
            </div>
          {/if}
          <img src={screenshotUrl(shot.path)} loading="lazy" decoding="async" alt={shot.name} />
          <div class="screenshot-tile-actions">
            <button
              type="button"
              class="panel-icon-btn {favoritePaths.has(shot.path) ? 'active' : ''}"
              data-tooltip={favoritePaths.has(shot.path) ? 'Unfavorite' : 'Favorite'}
              aria-label={favoritePaths.has(shot.path) ? 'Unfavorite' : 'Favorite'}
              onclick={(e) => { e.stopPropagation(); toggleFavorite(shot) }}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="{favoritePaths.has(shot.path) ? 'fa-solid' : 'fa-regular'} fa-star"></i>
            </button>
            <button
              type="button"
              class="panel-icon-btn"
              data-tooltip={t('tt.copyImage')}
              aria-label={t('tt.copyImage')}
              onclick={(e) => { e.stopPropagation(); copyScreenshotToClipboard(shot) }}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-copy"></i>
            </button>
            {#if confirmDeletePath === shot.path}
              <button
                type="button"
                class="panel-icon-btn bs-danger"
                data-tooltip={t('tt.confirmDelete')}
                aria-label={t('tt.confirmDelete')}
                onclick={(e) => { e.stopPropagation(); deleteScreenshot(shot) }}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-check"></i>
              </button>
              <button
                type="button"
                class="panel-icon-btn"
                data-tooltip={t('tt.cancel')}
                aria-label={t('tt.cancel')}
                onclick={(e) => { e.stopPropagation(); cancelDelete() }}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-xmark"></i>
              </button>
            {:else}
              <button
                type="button"
                class="panel-icon-btn"
                data-tooltip={t('tt.delete')}
                aria-label={t('tt.delete')}
                onclick={(e) => { e.stopPropagation(); confirmDelete(shot.path) }}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid fa-trash"></i>
              </button>
            {/if}
          </div>
          <div class="screenshot-tile-overlay">
            {#if favoritePaths.has(shot.path)}
              <i class="fa-solid fa-star bs-star"></i>
            {/if}
            <span class="screenshot-tile-name">{shot.name}</span>
            <span class="screenshot-tile-meta">{formatSize(shot.size_bytes)} · {formatRelativeTime(shot.modified_at)}</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
  </div>
</div>

{#if lightboxShot}
  <div class="lightbox-overlay" role="presentation" onclick={closeLightbox}>
    <button type="button" class="lightbox-nav lightbox-nav-prev" onclick={(e) => { e.stopPropagation(); lightboxPrev() }} aria-label={t('tt.previous')}>
      <i class="fa-solid fa-chevron-left"></i>
    </button>
    <div class="lightbox-body" role="presentation" onclick={(e) => e.stopPropagation()}>
      <img src={screenshotUrl(lightboxShot.path)} alt={lightboxShot.name} />
      <div class="lightbox-caption">
        <div>
          <span class="lightbox-caption-name">{lightboxShot.name}</span>
          <span class="lightbox-caption-meta">{formatSize(lightboxShot.size_bytes)} · {formatRelativeTime(lightboxShot.modified_at)} · {lightboxIndex + 1} / {sortedScreenshots.length}</span>
        </div>
        <div class="bs-lightbox-actions">
          <button type="button" class="panel-icon-btn" data-tooltip={t('tt.favorite')} aria-label={t('tt.favorite')} onclick={() => toggleFavorite(lightboxShot)}>
            <i class="{favoritePaths.has(lightboxShot.path) ? 'fa-solid' : 'fa-regular'} fa-star"></i>
          </button>
          <button type="button" class="panel-icon-btn" data-tooltip={t('tt.copyImage')} aria-label={t('tt.copyImage')} onclick={() => copyScreenshotToClipboard(lightboxShot)}>
            <i class="fa-solid fa-copy"></i>
          </button>
          <button type="button" class="panel-icon-btn" data-tooltip={t('tt.openSystemViewer')} aria-label={t('tt.openSystemViewer')} onclick={() => openInSystemViewer(lightboxShot)}>
            <i class="fa-solid fa-up-right-from-square"></i>
          </button>
          <button type="button" class="panel-icon-btn" data-tooltip={t('tt.close')} aria-label={t('tt.close')} onclick={closeLightbox}>
            <i class="fa-solid fa-xmark"></i>
          </button>
        </div>
      </div>
    </div>
    <button type="button" class="lightbox-nav lightbox-nav-next" onclick={(e) => { e.stopPropagation(); lightboxNext() }} aria-label={t('tt.next')}>
      <i class="fa-solid fa-chevron-right"></i>
    </button>
  </div>
{/if}

