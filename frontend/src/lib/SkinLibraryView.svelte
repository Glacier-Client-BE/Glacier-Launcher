<script lang="ts">
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
      onToast?.('Skin added to library.', 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  /** Port of `SaveCurrent`. */
  async function saveCurrent() {
    const name = settings?.JavaUsername ?? ''
    if (!name.trim()) {
      onToast?.('Sign in with Microsoft (Java) first to save your current skin.', 'error')
      return
    }
    busy = true
    try {
      await invoke('add_skin_from_username', { username: name })
      await reload()
      onToast?.('Your current skin was saved to the library.', 'success')
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
      onToast?.('Skin saved to library.', 'success')
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
      onToast?.('Sign in with Microsoft (Java) first to apply skins.', 'error')
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
    onToast?.('Applying skin…', 'info')

    try {
      await invoke('apply_skin', { path: s.path, slim })
      onToast?.(`Applied "${s.name}" as ${slim ? 'slim' : 'classic'}.`, 'success')
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
    await invoke('delete_saved_skin', { path: s.path })
    await reload()
    onToast?.('Skin removed.', 'info')
  }

  reload()
</script>

<PanelHeader title="Skin Library" {onBack}>
  {#snippet extraActions()}
    <button
      class="btn-sm"
      onclick={saveCurrent}
      disabled={busy}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Save your account's current skin into the library"
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
      placeholder="Add by username (grabs their current skin)…"
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
      aria-label="Add by username"
    >
      <i class="fa-solid fa-user-plus"></i>
    </button>
  </div>

  {#if skins.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-shirt"></i>
      <span>No saved skins yet. Add a PNG or pull one in by username, then apply it to your signed-in
      account.</span>
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
              data-tooltip="Remove from library"
              aria-label="Remove from library"><i class="fa-solid fa-trash"></i></button
            >
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
