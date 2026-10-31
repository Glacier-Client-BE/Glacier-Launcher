<script lang="ts">
  import { t } from './i18nStore.svelte'
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
  embedded = false,
}: {
  settings: LauncherSettings
  onChange: (next: LauncherSettings) => void
  onBack?: () => void
  onToast?: (message: string, kind: 'info' | 'success' | 'error') => void
  breadcrumb?: string[]
  /** Renders inline inside Settings' Looks category instead of as its own
   *  panel — no PanelHeader/back button, just a section label + toolbar. */
  embedded?: boolean
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

/** Drives each slider's filled-track gradient — see `input[type=range]` in app.css. */
function pct(value: number, min: number, max: number): string {
  return `${max > min ? Math.round(((value - min) / (max - min)) * 100) : 0}%`
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
  try {
    const copy = await invoke<ThemeDefinition | null>('duplicate_theme', { id: sel.Id })
    if (!copy) return
    themes = await invoke<ThemeDefinition[]>('list_themes')
    sel = copy
    await preview()
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

async function deleteTheme() {
  if (!sel) return
  try {
    const wasActive = isActive(sel)
    await invoke('delete_theme', { id: sel.Id })
    if (wasActive) {
      onChange({ ...settings, ActiveThemeId: '' })
      await restoreNonThemeLook()
    }
    themes = await invoke<ThemeDefinition[]>('list_themes')
    sel = themes[0] ?? null
    if (sel) await preview()
    onToast?.(t('toast.themeDeleted'), 'info')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

async function activateTheme() {
  if (!sel) return
  try {
    await invoke('update_theme', { theme: sel })
    onChange({ ...settings, ActiveThemeId: sel.Id })
    await preview()
    onToast?.(t('toast.themeNameActive', { name: sel.Name }), 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
}

async function deactivateTheme() {
  onChange({ ...settings, ActiveThemeId: '' })
  await restoreNonThemeLook()
  onToast?.(t('toast.backStandardPreset'), 'info')
}

async function exportTheme() {
  if (!sel) return
  try {
    await invoke('update_theme', { theme: sel })
    const path = await invoke<string>('export_theme', { id: sel.Id })
    const name = path.split(/[\\/]/).pop() ?? path
    onToast?.(t('toast.exportedName', { name }), 'success')
    // The original opens the exports folder afterwards.
    const dir = path.slice(0, path.length - name.length - 1)
    await invoke('open_folder', { path: dir }).catch(() => {})
  } catch (e) {
    onToast?.(t('toast.exportFailed', { error: String(e) }), 'error')
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
  try {
    const imported = await invoke<ThemeDefinition | null>('import_theme', { filePath: picked })
    if (!imported) {
      onToast?.(t('toast.thatFileIsnTValidGlacier'), 'error')
      return
    }
    themes = await invoke<ThemeDefinition[]>('list_themes')
    sel = imported
    await preview()
    onToast?.(t('toast.importedQuotedName', { name: imported.Name }), 'success')
  } catch (e) {
    onToast?.(String(e), 'error')
  }
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
  try {
    const stored = await invoke<string | null>('stash_theme_background', {
      sourcePath: picked,
      themeId: sel.Id,
    })
    if (!stored) {
      onToast?.(t('toast.couldnTUseThatImageMissing'), 'error')
      return
    }
    sel.BackgroundImage = stored
    markDirty(false)
    await preview()
  } catch (e) {
    onToast?.(String(e), 'error')
  }
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
  try {
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
  } catch (e) {
    onToast?.(String(e), 'error')
  }
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

// ---- tabbed editor sections (split view: rail + tabs on the left, live preview pinned on the right) ----
type TabKey = 'identity' | 'colors' | 'shape' | 'background' | 'typography' | 'css'
const TABS: { key: TabKey; label: string; icon: string }[] = [
  { key: 'identity', label: 'Identity', icon: 'fa-id-badge' },
  { key: 'colors', label: 'Colors', icon: 'fa-palette' },
  { key: 'shape', label: 'Shape & effects', icon: 'fa-shapes' },
  { key: 'background', label: 'Background', icon: 'fa-image' },
  { key: 'typography', label: 'Typography', icon: 'fa-font' },
  { key: 'css', label: 'Custom CSS', icon: 'fa-code' },
]
let activeTab = $state<TabKey>('identity')

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
{#if !embedded}
  <PanelHeader title={t('panel.themeStudio')} onBack={closePanel} {breadcrumb} />
{/if}

<div class="panel-body" style={embedded ? 'padding:0;' : ''}>
  {#if themes.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-palette"></i>
      <span>{t('empty.noCustomThemesYet')}</span>
      <small>{t('empty.startFromCurrentPresetMakeIt')}</small>
      <div style="display:flex; gap:6px; margin-top:6px;">
        <button class="btn-sm btn-accent" onclick={newTheme}>
          <i class="fa-solid fa-plus"></i> Create your first theme
        </button>
        <button class="btn-sm" onclick={importTheme}>
          <i class="fa-solid fa-file-import"></i> {t('common.import')}
        </button>
      </div>
    </div>
  {:else}
    <!--
      Split view: a rail carries the theme picker plus a tab bar that swaps
      in exactly one editor section at a time, so the panel never has to lay
      out six cards' worth of controls at once. The live preview and the
      destructive/activation actions sit alongside it, in sync via the same
      CSS vars `preview()` writes to :root no matter which tab is open.
      Plain flex-wrap (see .ts-shell in app.css) lets the two reflow to
      whatever width this panel actually has instead of relying on a
      breakpoint that can't see it, since it's frequently embedded in a
      narrower Settings sub-column.
    -->
    <div class="ts-shell">
      <div class="ts-rail">
        <div class="ts-picker-row">
          <div class="ts-picker-chips">
            {#each themes as theme (theme.Id)}
              <button
                type="button"
                class="ts-chip {sel?.Id === theme.Id ? 'selected' : ''} {isActive(theme) ? 'active' : ''}"
                onclick={() => selectTheme(theme)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <span class="ts-chip-swatch" style="background:{theme.Accent}"></span>
                {theme.Name}
                {#if isActive(theme)}
                  <i class="fa-solid fa-circle-check"></i>
                {/if}
              </button>
            {/each}
          </div>
          <div class="ts-picker-actions">
            <button class="btn-sm" onclick={newTheme} onmousedown={(e) => e.stopPropagation()} data-tooltip={t('tt.newTheme')} aria-label={t('tt.newTheme')}>
              <i class="fa-solid fa-plus"></i>
            </button>
            <button class="btn-sm" onclick={importTheme} onmousedown={(e) => e.stopPropagation()} data-tooltip={t('tt.importThemeFile')} aria-label={t('tt.importTheme')}>
              <i class="fa-solid fa-file-import"></i>
            </button>
          </div>
        </div>

        {#if sel}
          <div class="ts-tabbar">
            {#each TABS as tab (tab.key)}
              <button
                type="button"
                class="ts-tab {activeTab === tab.key ? 'selected' : ''}"
                onclick={() => (activeTab = tab.key)}
                onmousedown={(e) => e.stopPropagation()}
              >
                <i class="fa-solid {tab.icon}"></i>
                {tab.label}
              </button>
            {/each}
          </div>

          <div class="ts-tab-panel">
            {#if activeTab === 'identity'}
              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('label.name')}</span>
                  <span class="setting-hint">{t('x.shownInTheThemeListAndExportFileName')}</span>
                </div>
                <input
                  class="setting-input"
                  aria-label={t('label.name')}
                  value={sel.Name}
                  maxlength="40"
                  oninput={(e) => { if (sel) { sel.Name = e.currentTarget.value; markDirty(false) } }}
                  onkeydown={(e) => e.stopPropagation()}
                />
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('x.basePreset')}</span>
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
                  <button class="btn-sm" title="Reset colors to this preset's defaults" aria-label="Reset colors to this preset's defaults" onclick={reseedFromPreset}>
                    <i class="fa-solid fa-rotate-left"></i>
                  </button>
                </div>
              </div>
            {:else if activeTab === 'colors'}
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
            {:else if activeTab === 'shape'}
              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('x.cornerRadius')}</span>
                  <span class="setting-hint">{t('x.smallControlsLargeCards')}</span>
                </div>
                <div style="display:flex; align-items:center; gap:8px;">
                  <input
                    type="range" min="0" max="24" step="1" value={sel.RadiusSm} aria-label={t('tt.smallRadius')}
                    style="--pct:{pct(sel.RadiusSm, 0, 24)}"
                    oninput={async (e) => { if (sel) { sel.RadiusSm = Math.min(24, Math.max(0, +e.currentTarget.value)); await markDirtyAsync() } }}
                  />
                  <span class="ts-slider-val">{sel.RadiusSm}px</span>
                  <input
                    type="range" min="0" max="32" step="1" value={sel.RadiusMd} aria-label={t('tt.largeRadius')}
                    style="--pct:{pct(sel.RadiusMd, 0, 32)}"
                    oninput={async (e) => { if (sel) { sel.RadiusMd = Math.min(32, Math.max(0, +e.currentTarget.value)); await markDirtyAsync() } }}
                  />
                  <span class="ts-slider-val">{sel.RadiusMd}px</span>
                </div>
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('x.panelBlur')}</span>
                  <span class="setting-hint">{t('x.backdropBlurBehindPanels')}</span>
                </div>
                <div style="display:flex; align-items:center; gap:8px;">
                  <input
                    type="range" min="0" max="30" step="2" value={sel.Blur} aria-label={t('tt.panelBlur')}
                    style="--pct:{pct(sel.Blur, 0, 30)}"
                    oninput={async (e) => { if (sel) { sel.Blur = Math.min(30, Math.max(0, +e.currentTarget.value)); await markDirtyAsync() } }}
                  />
                  <span class="ts-slider-val">{sel.Blur}px</span>
                </div>
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('x.overlayStrength')}</span>
                  <span class="setting-hint">{t('x.howMuchTheWallpaperIsDimmedTowardTheBack')}</span>
                </div>
                <div style="display:flex; align-items:center; gap:8px;">
                  <input
                    type="range" min="0" max="100" step="5" value={Math.round(sel.OverlayStrength * 100)} aria-label={t('tt.overlayStrength')}
                    style="--pct:{pct(Math.round(sel.OverlayStrength * 100), 0, 100)}"
                    oninput={async (e) => { if (sel) { sel.OverlayStrength = Math.min(100, Math.max(0, +e.currentTarget.value)) / 100; await markDirtyAsync() } }}
                  />
                  <span class="ts-slider-val">{Math.round(sel.OverlayStrength * 100)}%</span>
                </div>
              </div>
            {:else if activeTab === 'background'}
              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('x.wallpaper')}</span>
                  <span class="setting-hint">
                    {sel.BackgroundImage ? fileName(sel.BackgroundImage) : 'Default background'}
                  </span>
                </div>
                <div style="display:flex; align-items:center; gap:6px;">
                  <button class="btn-sm" onclick={pickBackground}><i class="fa-solid fa-image"></i> Choose</button>
                  {#if sel.BackgroundImage}
                    <button class="btn-sm" onclick={clearBackground} aria-label={t('tt.clearWallpaper')}>
                      <i class="fa-solid fa-xmark"></i>
                    </button>
                  {/if}
                </div>
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('x.wallpaperFit')}</span>
                  <span class="setting-hint">{t('x.howTheImageFillsTheWindow')}</span>
                </div>
                <div class="seg-control">
                  {#each [['cover', 'Fill'], ['contain', 'Fit'], ['tile', 'Tile'], ['center', 'Center']] as [v, label] (v)}
                    <button
                      type="button"
                      class="seg-btn {sel.BackgroundFit === v ? 'active' : ''}"
                      onclick={async () => { if (sel) { sel.BackgroundFit = v; await markDirtyAsync() } }}
                    >
                      {label}
                    </button>
                  {/each}
                </div>
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('x.wallpaperOpacity')}</span>
                  <span class="setting-hint">{t('x.fadeTheImageItself')}</span>
                </div>
                <div style="display:flex; align-items:center; gap:8px;">
                  <input
                    type="range" min="10" max="100" step="5" value={Math.round(sel.BackgroundOpacity * 100)} aria-label={t('tt.wallpaperOpacity')}
                    style="--pct:{pct(Math.round(sel.BackgroundOpacity * 100), 10, 100)}"
                    oninput={async (e) => { if (sel) { sel.BackgroundOpacity = Math.min(100, Math.max(10, +e.currentTarget.value)) / 100; await markDirtyAsync() } }}
                  />
                  <span class="ts-slider-val">{Math.round(sel.BackgroundOpacity * 100)}%</span>
                </div>
              </div>
            {:else if activeTab === 'typography'}
              <div class="setting-row">
                <div class="setting-meta">
                  <span class="setting-label">{t('x.fontFamily')}</span>
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
                  <span class="setting-label">{t('x.animationSpeed')}</span>
                  <span class="setting-hint">Bundled with the theme — 1× is default</span>
                </div>
                <div style="display:flex; align-items:center; gap:8px;">
                  <input
                    type="range" min="0.25" max="2" step="0.25" value={sel.AnimationSpeed} aria-label={t('tt.animationSpeed')}
                    style="--pct:{pct(sel.AnimationSpeed, 0.25, 2)}"
                    oninput={async (e) => { if (sel) { sel.AnimationSpeed = Math.min(2, Math.max(0.25, +e.currentTarget.value)); await markDirtyAsync() } }}
                  />
                  <span class="ts-slider-val">{sel.AnimationSpeed}×</span>
                </div>
              </div>
            {:else if activeTab === 'css'}
              <div class="ts-css-editor">
                <textarea
                  class="setting-input ts-css-textarea"
                  spellcheck="false"
                  rows="10"
                  placeholder=".action-btn &lbrace; border: 1px solid var(--accent); &rbrace;"
                  value={sel.CustomCss}
                  onchange={(e) => { if (sel) sel.CustomCss = e.currentTarget.value }}
                  onkeydown={(e) => e.stopPropagation()}
                ></textarea>
                <div class="ts-css-actions">
                  <button class="btn-sm" onclick={applyCustomCss}><i class="fa-solid fa-play"></i> Apply</button>
                  <button class="btn-sm" onclick={resetCustomCss}><i class="fa-solid fa-rotate-left"></i> {t('common.reset')}</button>
                  <span class="setting-hint">Injected last, so it wins over everything. Reset removes it instantly if something breaks.</span>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      {#if sel}
        <div class="ts-preview-pane">
          <div class="ts-preview-card">
            <div class="ts-preview-topbar">
              <img class="ts-preview-logo" src="/images/icon.svg" alt="" />
              <span class="ts-preview-title">Glacier Launcher</span>
              <span class="ts-preview-pill">{isActive(sel) ? 'ACTIVE' : 'PREVIEW'}</span>
            </div>
            <div class="ts-preview-body">
              <button class="ts-preview-launch" type="button" tabindex="-1">
                <i class="fa-solid fa-play"></i> {t('common.launch')}
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
            <button class="btn-sm" onclick={exportTheme}><i class="fa-solid fa-file-export"></i> {t('common.export')}</button>
            <button class="btn-sm ts-danger" onclick={deleteTheme}><i class="fa-solid fa-trash"></i> {t('common.delete')}</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
