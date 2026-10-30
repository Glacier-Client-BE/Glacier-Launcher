<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * Port of `Components/SkinLibraryPanel.razor` (172 lines) — the "Skin
   * Library" panel, reached from `currentView == "skinlibrary"`
   * (Home.razor:1003).
   *
   * This panel had **no counterpart at all** in the rewrite: no view, no
   * route, and no commands — even though `gl_integrations::skin_library` and
   * `::skins` were both fully ported. Those are now exposed as
   * `list_saved_skins` / `add_skin_from_file` / `add_skin_from_username` /
   * `delete_saved_skin` / `apply_skin`.
   */
  import { invoke, convertFileSrc } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import PanelHeader from './components/PanelHeader.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import type { LauncherSettings } from './settings'

  let {
    settings,
    onBack,
    onSaveSettings,
    onToast,
  }: {
    settings: LauncherSettings | null
    onBack?: () => void
    onSaveSettings?: (next: LauncherSettings) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  /** Port of `SkinLibraryService.SavedSkin`. */
  type SavedSkin = { name: string; path: string; url: string; slim: boolean }

  let skins = $state<SavedSkin[]>([])
  let username = $state('')
  let busy = $state(false)
  let loaded = $state(false)

  /**
   * The crate fills `SavedSkin.url` for the remote case, but a library entry
   * is a local file. WebView2 blocks `file://`, so local previews go through
   * `convertFileSrc` — the same approach `BedrockScreenshotsView` uses.
   */
  function previewSrc(s: SavedSkin): string {
    if (s.url && /^https?:/i.test(s.url)) return s.url
    return s.path ? convertFileSrc(s.path) : ''
  }

  /** Port of `Reload`. */
  async function reload() {
    try {
      skins = await invoke<SavedSkin[]>('list_saved_skins')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      loaded = true
    }
  }

  /**
   * Port of `AddFromFile`. The original calls
   * `LauncherUtilityService.PickFileAsync("Choose a skin PNG", …)`; the host
   * has to own the pick because WebView2 exposes no real path for a file
   * chosen inside the page.
   */
  async function addFromFile() {
    const path = await open({
      title: 'Choose a skin PNG',
      multiple: false,
      directory: false,
      filters: [
        { name: 'Minecraft skin (PNG)', extensions: ['png'] },
        { name: 'All files', extensions: ['*'] },
      ],
    })
    if (!path || typeof path !== 'string') return
    try {
      await invoke('add_skin_from_file', { sourcePath: path, slim: false })
      await reload()
      onToast?.(t('toast.skinAddedLibrary'), 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  /** Port of `SaveCurrent`. */
  async function saveCurrent() {
    const name = settings?.JavaUsername ?? ''
    if (!name.trim()) {
      onToast?.(t('toast.signWithMicrosoftJavaFirstSave'), 'error')
      return
    }
    busy = true
    try {
      await invoke('add_skin_from_username', { username: name })
      await reload()
      onToast?.(t('toast.yourCurrentSkinWasSavedLibrary'), 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }

  /** Port of `AddFromUsername`. */
  async function addFromUsername() {
    if (!username.trim()) return
    busy = true
    try {
      await invoke('add_skin_from_username', { username })
      username = ''
      await reload()
      onToast?.(t('toast.skinSavedLibrary'), 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }

  /**
   * Port of `Apply`. The original optimistically writes
   * `LastAppliedSkinPath` / `SkinChangedTicks` before uploading (so the footer
   * re-renders immediately) and rolls both back if the upload fails; that
   * rollback is preserved here.
   *
   * The access token is NOT passed from here — `apply_skin` reads it from
   * settings in the backend, so it never reaches page script.
   */
  async function apply(s: SavedSkin, slim: boolean) {
    if (!settings?.JavaAccessToken) {
      onToast?.(t('toast.signWithMicrosoftJavaFirstApply'), 'error')
      return
    }
    busy = true
    const prevPath = settings.LastAppliedSkinPath
    const prevTicks = settings.SkinChangedTicks
    onSaveSettings?.({
      ...settings,
      LastAppliedSkinPath: s.path,
      // .NET ticks: 100ns units since 0001-01-01, which is 621355968000000000
      // ticks before the Unix epoch. Matching the original's `DateTime.UtcNow.Ticks`
      // keeps the value comparable with anything written by the old app.
      SkinChangedTicks: Date.now() * 10000 + 621355968000000000,
    })
    onToast?.(t('toast.applyingSkin'), 'info')

    try {
      await invoke('apply_skin', { path: s.path, slim })
      onToast?.(t('toast.appliedNameAsVariant', { name: s.name, variant: slim ? t('x.slim') : t('x.classic') }), 'success')
    } catch (e) {
      onSaveSettings?.({
        ...settings,
        LastAppliedSkinPath: prevPath,
        SkinChangedTicks: prevTicks,
      })
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }

  /** Port of `Delete`. */
  async function remove(s: SavedSkin) {
    try {
      await invoke('delete_saved_skin', { path: s.path })
      await reload()
      onToast?.(t('toast.skinRemoved'), 'info')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  reload()
</script>

<PanelHeader title={t('panel.skinLibrary')} {onBack}>
  {#snippet extraActions()}
    <button
      class="btn-sm"
      onclick={saveCurrent}
      disabled={busy}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.saveYourAccountSCurrentSkin')}
    >
      <i class="fa-solid fa-floppy-disk"></i> Save current
    </button>
    <button class="btn-sm" onclick={addFromFile} onmousedown={(e) => e.stopPropagation()}>
      <i class="fa-solid fa-upload"></i> Add PNG
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  <div class="skinlib-add">
    <input
      class="panel-search-input"
      aria-label={t('ph.addByUsername')}
      placeholder={t('ph.addByUsername')}
      bind:value={username}
      onkeydown={(e) => {
        e.stopPropagation()
        if (e.key === 'Enter') addFromUsername()
      }}
      onmousedown={(e) => e.stopPropagation()}
    />
    <button
      class="btn-sm"
      onclick={addFromUsername}
      disabled={busy}
      onmousedown={(e) => e.stopPropagation()}
      aria-label={t('tt.addByUsername')}
    >
      <i class="fa-solid fa-user-plus"></i>
    </button>
  </div>

  {#if !loaded}
    <ListSkeleton rows={3} />
  {:else if skins.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-shirt"></i>
      <span>{t('empty.noSavedSkinsYetAddPng')}</span>
    </div>
  {:else}
    <div class="skinlib-grid">
      {#each skins as s (s.path)}
        <div class="skinlib-card">
          <div class="skinlib-preview">
            <img src={previewSrc(s)} loading="lazy" style="image-rendering:pixelated;" alt="" />
          </div>
          <span class="skinlib-name">{s.name}</span>
          <div class="skinlib-actions">
            <button
              class="btn-sm btn-accent"
              onclick={() => apply(s, s.slim)}
              disabled={busy}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Apply to your account as {s.slim ? 'slim (Alex)' : 'classic (Steve)'}"
            >
              <i class="fa-solid fa-shirt"></i> Apply
            </button>
            <button
              class="btn-sm"
              onclick={() => apply(s, !s.slim)}
              disabled={busy}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Apply as {s.slim ? 'classic (Steve)' : 'slim (Alex)'} instead"
              >{s.slim ? 'Classic' : 'Slim'}</button
            >
            <button
              class="btn-sm ts-danger"
              onclick={() => remove(s)}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.removeFromLibrary')}
              aria-label={t('tt.removeFromLibrary')}><i class="fa-solid fa-trash"></i></button
            >
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
