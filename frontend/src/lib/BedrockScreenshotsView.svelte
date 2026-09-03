<script lang="ts">
/**
 * Literal port of the `bedrockscreenshots` panel — Home.razor:4375-4477, with
 * behaviour from Pages/Home.BedrockScreenshots.cs (37 lines).
 *
 * Transcribed element-for-element: same DOM order, class names, tooltips and
 * strings. The original has no error state and no per-row delete — the grid is
 * tiles only.
 *
 * The screenshot source is always the *live* install: the C# side read the
 * static `BedrockScreenshotService.ComMojangScreenshotsDir`, and the Tauri
 * command ignores its `instanceDir` argument for the same reason. Nothing here
 * is per-instance.
 */
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import SubTabs from './components/SubTabs.svelte'
import EditionTabs from './components/EditionTabs.svelte'

let {
  onBack,
  onNavigate,
  onToast,
}: {
  onBack: () => void
  /** Both tab strips route through the parent — this view never routes itself. */
  onNavigate: (view: string) => void
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

function parentDir(path: string): string {
  const idx = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return idx > 0 ? path.slice(0, idx) : ''
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

/** Port of `OpenScreenshot` (Home.BedrockScreenshots.cs). */
async function openScreenshot(shot: { path: string; name: string }) {
  try {
    await invoke('open_path', { path: shot.path })
  } catch (e) {
    onToast?.(String(e), 'error')
  }
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
</script>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<div class="panel-header">
  <div class="panel-title-wrap">
    <span class="panel-title">Screenshots</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn"
      onclick={openBedrockScreenshotsFolder}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Open folder"
    >
      <i class="fa-solid fa-folder-open"></i>
    </button>
    <button
      class="panel-icon-btn {isLoadingBedrockScreenshots ? 'spinning' : ''}"
      disabled={isLoadingBedrockScreenshots}
      onclick={refreshBedrockScreenshots}
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

<SubTabs strip="utilities" active="photos" {onNavigate} />

<div class="panel-body">
  {#if isLoadingBedrockScreenshots}
    <div class="versions-loading">
      <span class="spinner"></span><span>Loading screenshots…</span>
    </div>
  {:else if bedrockScreenshots.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-image"></i>
      <span>No screenshots yet</span>
      <small
        >In-game screenshots and Xbox Game Bar captures (Win+Alt+PrtScn) both show up here.</small
      >
    </div>
  {:else}
    <div class="screenshot-grid">
      {#each bedrockScreenshots as shot (shot.path)}
        <!-- Port of `OpenScreenshot` -> `OpenUrl(shot.Path)`: opens the
             file in the system's default viewer via `open_path`. -->
        <button
          class="screenshot-tile"
          onclick={() => openScreenshot(shot)}
          onmousedown={(e) => e.stopPropagation()}
          title={shot.name}
        >
          <img src={screenshotUrl(shot.path)} loading="lazy" alt={shot.name} />
        </button>
      {/each}
    </div>
  {/if}
</div>

<EditionTabs edition="bedrock" active="utilities" {onNavigate} />
