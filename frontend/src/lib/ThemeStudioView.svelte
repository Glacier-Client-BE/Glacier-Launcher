<script lang="ts">
/**
 * Theme Studio - 1:1 port of `Components/ThemeStudioPanel.razor` (539 lines).
 *
 * This panel had NO counterpart in the rewrite at all, which is why the
 * Settings panel's two Theme Studio entry points navigated nowhere.
 *
 * Applying a theme stays on the frontend, exactly as the original's
 * `ThemeService.ApplyAsync` did over JS interop: `setTheme`, `setThemeVars`,
 * `setFont`, `setBlurIntensity`, `setAnimationSpeed`, `setCustomCss`,
 * `setBackgroundFit`, `setCustomBackground`. Persistence and the CSS
 * variable computation live in Rust (`gl_settings::themes`) so a live
 * preview and a saved theme can never disagree.
 *
 * Target hardware is a Celeron N5100, so the live preview only ever swaps
 * CSS custom properties - no animated box-shadow or filter work.
 */
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import ColorPicker from './components/ColorPicker.svelte'
import PanelHeader from './components/PanelHeader.svelte'
import type { LauncherSettings } from './settings'

let {
  settings,
  onChange,
  onBack,
  onToast,
  breadcrumb = [],
}: {
  settings: LauncherSettings
  onChange: (next: LauncherSettings) => void
  onBack?: () => void
  onToast?: (message: string, kind: 'info' | 'success' | 'error') => void
  breadcrumb?: string[]
} = $props()

/** Mirrors the Rust `ThemeDefinition` (PascalCase over the IPC boundary). */
type ThemeDefinition = {
  SchemaVersion: number
  Id: string
  Name: string
  BasePreset: string
  Accent: string
  Bg: string
  BgPanel: string
  Text: string
  TextDim: string
  Red: string
  Green: string
  Orange: string
  RadiusSm: number
  RadiusMd: number
  Blur: number
  OverlayStrength: number
  FontFamily: string
  AnimationSpeed: number
  BackgroundImage: string
  BackgroundFit: string
  BackgroundOpacity: number
  CustomCss: string
}

let themes = $state<ThemeDefinition[]>([])
let sel = $state<ThemeDefinition | null>(null)
let presets = $state<[string, string][]>([])
let appliedPreview = false
let saveTimer: ReturnType<typeof setTimeout> | null = null

const isActive = (t: ThemeDefinition) => settings.ActiveThemeId === t.Id

async function load() {
  try {
    presets = await invoke<[string, string][]>('theme_presets')
    themes = await invoke<ThemeDefinition[]>('list_themes')
    sel = themes.find((t) => t.Id === settings.ActiveThemeId) ?? themes[0] ?? null
    if (sel) await preview()
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}
load()

/** Port of `ThemeService.ApplyAsync`. */
async function preview() {
  if (!sel) return
  appliedPreview = true
  try {
    const vars = await invoke<Record<string, string>>('theme_css_vars', { theme: sel })
    window.setTheme?.(sel.BasePreset)
    window.setThemeVars?.(vars)
    window.setFont?.(sel.FontFamily)
    window.setBlurIntensity?.(sel.Blur)
    window.setAnimationSpeed?.(sel.AnimationSpeed)
    window.setCustomCss?.(sel.CustomCss)
    window.setBackgroundFit?.(sel.BackgroundFit, sel.BackgroundOpacity)
    if (sel.BackgroundImage) window.setCustomBackground?.(sel.BackgroundImage)
  } catch {
    /* the original swallows apply failures too - a bad theme must not brick the UI */
  }
}

/** Port of `RestoreNonThemeLookAsync`. */
async function restoreNonThemeLook() {
  try {
    window.clearThemeVars?.()
    window.setFont?.('')
    window.setCustomCss?.('')
    window.setBackgroundFit?.('cover', 1.0)
    window.setTheme?.(settings.ThemePreset)
    window.setAccentColor?.(settings.AccentColor)
    window.setBlurIntensity?.(settings.BlurIntensity)
    window.setAnimationSpeed?.(settings.AnimationSpeed)
    window.setCustomBackground?.(settings.CustomBackgroundPath ?? '')
    window.setFont?.(settings.FontFamily ?? '')
    window.setBackgroundFit?.(settings.BackgroundFit ?? 'cover', 1.0)
  } catch {
    /* ignore */
  }
}

/**
 * Port of `MarkDirty`: debounce the write by 700ms so dragging a slider
 * doesn't hammer the disk, optionally re-applying the live preview.
 */
function markDirty(applyLive = true) {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(() => {
    if (sel) invoke('update_theme', { theme: sel }).catch(() => {})
  }, 700)
  if (applyLive) preview()
}

async function markDirtyAsync() {
  markDirty(false)
  await preview()
}

async function selectTheme(t: ThemeDefinition) {
  sel = t
  await preview()
}

async function newTheme() {
  try {
    const created = await invoke<ThemeDefinition>('create_theme', {
      preset: settings.ThemePreset,
      accent: settings.AccentColor,
    })
    // The original seeds blur and animation speed from current settings.
    created.Blur = settings.BlurIntensity
    created.AnimationSpeed = settings.AnimationSpeed
    await invoke('update_theme', { theme: created })
    themes = await invoke<ThemeDefinition[]>('list_themes')
    sel = themes.find((t) => t.Id === created.Id) ?? created
    await preview()
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

async function duplicateTheme() {
  if (!sel) return
  const copy = await invoke<ThemeDefinition | null>('duplicate_theme', { id: sel.Id })
  if (!copy) return
  themes = await invoke<ThemeDefinition[]>('list_themes')
  sel = copy
  await preview()
}

async function deleteTheme() {
  if (!sel) return
  const wasActive = isActive(sel)
  await invoke('delete_theme', { id: sel.Id })
  if (wasActive) {
    onChange({ ...settings, ActiveThemeId: '' })
    await restoreNonThemeLook()
  }
  themes = await invoke<ThemeDefinition[]>('list_themes')
  sel = themes[0] ?? null
  if (sel) await preview()
  onToast?.('Theme deleted.', 'info')
}

async function activateTheme() {
  if (!sel) return
  await invoke('update_theme', { theme: sel })
  onChange({ ...settings, ActiveThemeId: sel.Id })
  await preview()
  onToast?.(`Theme "${sel.Name}" active.`, 'success')
}

async function deactivateTheme() {
  onChange({ ...settings, ActiveThemeId: '' })
  await restoreNonThemeLook()
  onToast?.('Back to the standard preset.', 'info')
}

async function exportTheme() {
  if (!sel) return
  try {
    await invoke('update_theme', { theme: sel })
    const path = await invoke<string>('export_theme', { id: sel.Id })
    const name = path.split(/[\\/]/).pop() ?? path
    onToast?.(`Exported ${name}`, 'success')
    // The original opens the exports folder afterwards.
    const dir = path.slice(0, path.length - name.length - 1)
    await invoke('open_folder', { path: dir }).catch(() => {})
  } catch (e) {
    onToast?.(`Export failed: ${e}`, 'error')
  }
}

async function importTheme() {
  // File pickers must originate in the native host - a webview has no
  // File.path - so this uses the dialog plugin rather than an <input type=file>.
  const picked = await open({
    title: 'Import a theme',
    multiple: false,
    filters: [
      { name: 'Glacier theme', extensions: ['json', 'glaciertheme'] },
      { name: 'All files', extensions: ['*'] },
    ],
  })
  if (typeof picked !== 'string' || !picked) return
  const imported = await invoke<ThemeDefinition | null>('import_theme', { filePath: picked })
  if (!imported) {
    onToast?.("That file isn't a valid Glacier theme.", 'error')
    return
  }
  themes = await invoke<ThemeDefinition[]>('list_themes')
  sel = imported
  await preview()
  onToast?.(`Imported "${imported.Name}".`, 'success')
}

async function pickBackground() {
  if (!sel) return
  const picked = await open({
    title: 'Choose a background',
    multiple: false,
    filters: [
      { name: 'Image files', extensions: ['png', 'jpg', 'jpeg', 'bmp', 'gif', 'webp'] },
      { name: 'All files', extensions: ['*'] },
    ],
  })
  if (typeof picked !== 'string' || !picked) return
  const stored = await invoke<string | null>('stash_theme_background', {
    sourcePath: picked,
    themeId: sel.Id,
  })
  if (!stored) {
    onToast?.("Couldn't use that image (missing or over 20 MB).", 'error')
    return
  }
  sel.BackgroundImage = stored
  markDirty(false)
  await preview()
}

async function clearBackground() {
  if (!sel) return
  sel.BackgroundImage = ''
  markDirty(false)
  window.setCustomBackground?.('')
  await preview()
}

/** Port of `ReseedFromPreset`: reset only the four palette colours. */
async function reseedFromPreset() {
  if (!sel) return
  const seed = await invoke<ThemeDefinition>('create_theme', {
    preset: sel.BasePreset,
    accent: sel.Accent,
  })
  sel.Bg = seed.Bg
  sel.BgPanel = seed.BgPanel
  sel.Text = seed.Text
  sel.TextDim = seed.TextDim
  // create_theme persisted a throwaway seed; drop it again.
  await invoke('delete_theme', { id: seed.Id }).catch(() => {})
  themes = await invoke<ThemeDefinition[]>('list_themes')
  await markDirtyAsync()
}

async function applyCustomCss() {
  await markDirtyAsync()
}

async function resetCustomCss() {
  if (!sel) return
  sel.CustomCss = ''
  await markDirtyAsync()
}

/** Port of `ClosePanel`: save, then restore whatever should actually be showing. */
async function closePanel() {
  if (saveTimer) clearTimeout(saveTimer)
  if (sel) await invoke('update_theme', { theme: sel }).catch(() => {})
  const active = themes.find((t) => t.Id === settings.ActiveThemeId) ?? null
  if (active) {
    if (sel?.Id !== active.Id) {
      sel = active
      await preview()
    }
  } else if (appliedPreview) {
    await restoreNonThemeLook()
  }
  onBack?.()
}

// ---- colour swatch editor (port of ColorSwatch / OpenColorEditor) ----
type ColorKey = 'accent' | 'bg' | 'panels' | 'text' | 'textdim' | 'red' | 'green' | 'orange'
const SWATCHES: { label: string; key: ColorKey; field: keyof ThemeDefinition }[] = [
  { label: 'Accent', key: 'accent', field: 'Accent' },
  { label: 'Background', key: 'bg', field: 'Bg' },
  { label: 'Panels', key: 'panels', field: 'BgPanel' },
  { label: 'Text', key: 'text', field: 'Text' },
  { label: 'Muted text', key: 'textdim', field: 'TextDim' },
  { label: 'Error', key: 'red', field: 'Red' },
  { label: 'Success', key: 'green', field: 'Green' },
  { label: 'Warning', key: 'orange', field: 'Orange' },
]

let editKey = $state<ColorKey | null>(null)
let editLabel = $state('')
let editField = $state<keyof ThemeDefinition | null>(null)

function openColorEditor(label: string, key: ColorKey, field: keyof ThemeDefinition) {
  // Clicking the active swatch again closes it, as in the original.
  if (editKey === key) {
    closeColorEditor()
    return
  }
  editKey = key
  editLabel = label
  editField = field
}

function closeColorEditor() {
  editKey = null
  editField = null
}

async function onColorChanged(hex: string) {
  if (!sel || !editField) return
  ;(sel as unknown as Record<string, string>)[editField as string] = hex
  await markDirtyAsync()
}

const editValue = $derived(
  sel && editField ? ((sel as unknown as Record<string, string>)[editField as string] ?? '#000000') : '#000000',
)

const FONT_SUGGESTIONS = [
  'Segoe UI', 'Inter', 'Roboto', 'Arial', 'Verdana', 'Tahoma',
  'Trebuchet MS', 'Georgia', 'Cascadia Code', 'Consolas', 'JetBrains Mono', 'Comic Sans MS',
]

function fileName(p: string): string {
  return p.split(/[\\/]/).pop() ?? p
}
</script>

<!-- `.panel-overlay` and `.panel-handle` are supplied once by App.svelte
   around whichever panel is active, so this panel must not emit its own
   or the two nest and render a panel inside a panel. -->
<PanelHeader title="Theme Studio" onBack={closePanel} {breadcrumb}>
  {#snippet extraActions()}
    <button class="btn-sm" onclick={newTheme} onmousedown={(e) => e.stopPropagation()}>
      <i class="fa-solid fa-plus"></i> New
    </button>
    <button class="btn-sm" onclick={importTheme} onmousedown={(e) => e.stopPropagation()}>
      <i class="fa-solid fa-file-import"></i> Import
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if themes.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-palette"></i>
      <span>No custom themes yet</span>
      <small>Start from the current preset and make it yours — every color, radius and font is editable.</small>
      <button class="btn-sm" onclick={newTheme}>
        <i class="fa-solid fa-plus"></i> Create your first theme
      </button>
    </div>
  {:else}
    <div class="ts-chips">
      {#each themes as t (t.Id)}
        <button
          class="ts-chip {sel?.Id === t.Id ? 'selected' : ''} {isActive(t) ? 'active' : ''}"
          onclick={() => selectTheme(t)}
          onmousedown={(e) => e.stopPropagation()}
        >
          <span class="ts-chip-swatch" style="background:{t.Accent}"></span>
          {t.Name}
          {#if isActive(t)}
            <i class="fa-solid fa-circle-check"></i>
          {/if}
        </button>
      {/each}
    </div>

    {#if sel}
      <div class="ts-preview-card">
        <div class="ts-preview-topbar">
          <img class="ts-preview-logo" src="/images/icon.png" alt="" />
          <span class="ts-preview-title">Glacier Launcher</span>
          <span class="ts-preview-pill">{isActive(sel) ? 'ACTIVE' : 'PREVIEW'}</span>
        </div>
        <div class="ts-preview-body">
          <button class="ts-preview-launch" type="button" tabindex="-1">
            <i class="fa-solid fa-play"></i> Launch
          </button>
          <div class="ts-preview-icons">
            <span><i class="fa-solid fa-gear"></i></span>
            <span><i class="fa-solid fa-puzzle-piece"></i></span>
            <span><i class="fa-solid fa-globe"></i></span>
            <span><i class="fa-solid fa-heart"></i></span>
          </div>
        </div>
      </div>

      <div class="ts-actions">
        {#if isActive(sel)}
          <button class="btn-sm ts-active-btn" onclick={deactivateTheme}>
            <i class="fa-solid fa-circle-check"></i> Active — click to stop using
          </button>
        {:else}
          <button class="btn-sm btn-accent" onclick={activateTheme}>
            <i class="fa-solid fa-wand-magic-sparkles"></i> Use this theme
          </button>
        {/if}
        <button class="btn-sm" onclick={duplicateTheme}><i class="fa-solid fa-copy"></i> Duplicate</button>
        <button class="btn-sm" onclick={exportTheme}><i class="fa-solid fa-file-export"></i> Export</button>
        <button class="btn-sm ts-danger" onclick={deleteTheme}><i class="fa-solid fa-trash"></i> Delete</button>
      </div>

      <div class="panel-section-label">Identity</div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Name</span>
          <span class="setting-hint">Shown in the theme list and export file name</span>
        </div>
        <input
          class="setting-input"
          value={sel.Name}
          maxlength="40"
          oninput={(e) => { if (sel) { sel.Name = e.currentTarget.value; markDirty(false) } }}
          onkeydown={(e) => e.stopPropagation()}
        />
      </div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Base preset</span>
          <span class="setting-hint">Inherits component styling — pick Light for light themes</span>
        </div>
        <div style="display:flex; align-items:center; gap:6px;">
          <select
            class="setting-select"
            value={sel.BasePreset}
            onchange={async (e) => { if (sel) { sel.BasePreset = e.currentTarget.value; await markDirtyAsync() } }}
          >
            {#each presets as [preset, label] (preset)}
              <option value={preset}>{label}</option>
            {/each}
          </select>
          <button class="btn-sm" title="Reset colors to this preset's defaults" onclick={reseedFromPreset}>
            <i class="fa-solid fa-rotate-left"></i>
          </button>
        </div>
      </div>

      <div class="panel-section-label">Colors</div>

      <div class="ts-color-grid">
        {#each SWATCHES as sw (sw.key)}
          <button
            type="button"
            class="ts-color-cell{editKey === sw.key ? ' ts-color-active' : ''}"
            onclick={() => openColorEditor(sw.label, sw.key, sw.field)}
          >
            <span
              class="ts-color-dot"
              style="--dot-color:{(sel as unknown as Record<string, string>)[sw.field as string]};"
            ></span>
            <span>{sw.label}</span>
          </button>
        {/each}
      </div>

      {#if editField}
        <div class="ts-color-editor">
          <div class="ts-color-editor-head">
            <span class="ts-color-editor-title">{editLabel}</span>
          </div>
          <ColorPicker value={editValue} onChange={onColorChanged} onClose={closeColorEditor} />
        </div>
      {/if}

      <div class="panel-section-label">Shape &amp; effects</div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Corner radius</span>
          <span class="setting-hint">Small controls / large cards</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range" min="0" max="24" step="1" value={sel.RadiusSm} aria-label="Small radius"
            oninput={async (e) => { if (sel) { sel.RadiusSm = Math.min(24, Math.max(0, +e.currentTarget.value)); await markDirtyAsync() } }}
          />
          <span class="ts-slider-val">{sel.RadiusSm}px</span>
          <input
            type="range" min="0" max="32" step="1" value={sel.RadiusMd} aria-label="Large radius"
            oninput={async (e) => { if (sel) { sel.RadiusMd = Math.min(32, Math.max(0, +e.currentTarget.value)); await markDirtyAsync() } }}
          />
          <span class="ts-slider-val">{sel.RadiusMd}px</span>
        </div>
      </div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Panel blur</span>
          <span class="setting-hint">Backdrop blur behind panels — 0 disables (fastest)</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range" min="0" max="30" step="2" value={sel.Blur} aria-label="Panel blur"
            oninput={async (e) => { if (sel) { sel.Blur = Math.min(30, Math.max(0, +e.currentTarget.value)); await markDirtyAsync() } }}
          />
          <span class="ts-slider-val">{sel.Blur}px</span>
        </div>
      </div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Overlay strength</span>
          <span class="setting-hint">How much the wallpaper is dimmed toward the background color</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range" min="0" max="100" step="5" value={Math.round(sel.OverlayStrength * 100)} aria-label="Overlay strength"
            oninput={async (e) => { if (sel) { sel.OverlayStrength = Math.min(100, Math.max(0, +e.currentTarget.value)) / 100; await markDirtyAsync() } }}
          />
          <span class="ts-slider-val">{Math.round(sel.OverlayStrength * 100)}%</span>
        </div>
      </div>

      <div class="panel-section-label">Background</div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Wallpaper</span>
          <span class="setting-hint">
            {sel.BackgroundImage ? fileName(sel.BackgroundImage) : 'Default background'}
          </span>
        </div>
        <div style="display:flex; align-items:center; gap:6px;">
          <button class="btn-sm" onclick={pickBackground}><i class="fa-solid fa-image"></i> Choose</button>
          {#if sel.BackgroundImage}
            <button class="btn-sm" onclick={clearBackground} aria-label="Clear wallpaper">
              <i class="fa-solid fa-xmark"></i>
            </button>
          {/if}
        </div>
      </div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Wallpaper fit</span>
          <span class="setting-hint">How the image fills the window</span>
        </div>
        <select
          class="setting-select"
          value={sel.BackgroundFit}
          onchange={async (e) => { if (sel) { sel.BackgroundFit = e.currentTarget.value; await markDirtyAsync() } }}
        >
          <option value="cover">Fill (cover)</option>
          <option value="contain">Fit (contain)</option>
          <option value="tile">Tile</option>
          <option value="center">Center</option>
        </select>
      </div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Wallpaper opacity</span>
          <span class="setting-hint">Fade the image itself</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range" min="10" max="100" step="5" value={Math.round(sel.BackgroundOpacity * 100)} aria-label="Wallpaper opacity"
            oninput={async (e) => { if (sel) { sel.BackgroundOpacity = Math.min(100, Math.max(10, +e.currentTarget.value)) / 100; await markDirtyAsync() } }}
          />
          <span class="ts-slider-val">{Math.round(sel.BackgroundOpacity * 100)}%</span>
        </div>
      </div>

      <div class="panel-section-label">Typography &amp; motion</div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Font family</span>
          <span class="setting-hint">Any installed font — empty uses Segoe UI</span>
        </div>
        <input
          class="setting-input"
          list="ts-font-suggestions"
          placeholder="Segoe UI"
          value={sel.FontFamily}
          onchange={async (e) => { if (sel) { sel.FontFamily = e.currentTarget.value; await markDirtyAsync() } }}
          onkeydown={(e) => e.stopPropagation()}
        />
        <datalist id="ts-font-suggestions">
          {#each FONT_SUGGESTIONS as f (f)}
            <option value={f}></option>
          {/each}
        </datalist>
      </div>

      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">Animation speed</span>
          <span class="setting-hint">Bundled with the theme — 1× is default</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range" min="0.25" max="2" step="0.25" value={sel.AnimationSpeed} aria-label="Animation speed"
            oninput={async (e) => { if (sel) { sel.AnimationSpeed = Math.min(2, Math.max(0.25, +e.currentTarget.value)); await markDirtyAsync() } }}
          />
          <span class="ts-slider-val">{sel.AnimationSpeed}×</span>
        </div>
      </div>

      <div class="panel-section-label">Custom CSS</div>

      <div class="ts-css-editor">
        <textarea
          class="setting-input ts-css-textarea"
          spellcheck="false"
          rows="6"
          placeholder=".action-btn &lbrace; border: 1px solid var(--accent); &rbrace;"
          value={sel.CustomCss}
          onchange={(e) => { if (sel) sel.CustomCss = e.currentTarget.value }}
          onkeydown={(e) => e.stopPropagation()}
        ></textarea>
        <div class="ts-css-actions">
          <button class="btn-sm" onclick={applyCustomCss}><i class="fa-solid fa-play"></i> Apply</button>
          <button class="btn-sm" onclick={resetCustomCss}><i class="fa-solid fa-rotate-left"></i> Reset</button>
          <span class="setting-hint">Injected last, so it wins over everything. Reset removes it instantly if something breaks.</span>
        </div>
      </div>
    {/if}
  {/if}
</div>
