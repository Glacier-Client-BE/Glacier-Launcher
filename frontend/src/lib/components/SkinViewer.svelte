<script lang="ts">
  import { t } from '../i18nStore.svelte'
  /**
   * Port of `Components/SkinViewer.razor` (200 lines).
   *
   * The original drove a `glacierSkin` JS-interop object (three.js) for its
   * 3D mode. This port replaces that bespoke interop with `skinview3d`
   * (MIT, built on three.js) — the standard library for exactly this job —
   * wired directly into the canvas via `$effect` instead of Blazor's
   * OnAfterRenderAsync/JSInvoke dance. Behavior matches the original:
   * 2D/3D toggle, Steve/Alex model toggle, and a cape/elytra/off cycle
   * pill that only appears in 3D mode when a cape is available. All three
   * choices persist to settings exactly as `SetMode`/`SetModel`/`CycleCape`
   * did. The 2D static-image mode (mc-heads.net body render, crafatar
   * fallback) is kept intact as-is.
   *
   * No scoped <style>: .skin-viewer/.skin-stage/.skin-body/.skin-canvas/
   * .skin-loading/.spinner/.skin-pills/.skin-pill/.skin-pill-sep all live
   * in app.css.
   */
  import type { LauncherSettings } from '../settings'
  import { untrack } from 'svelte'
  import { SkinViewer as GlSkinViewer, WalkingAnimation } from 'skinview3d'

  let {
    uuid = '',
    username = '',
    capeUrl = '',
    version = 0,
    localSkinUrl = '',
    settings,
    onChange,
  }: {
    uuid?: string
    username?: string
    capeUrl?: string
    version?: number
    localSkinUrl?: string
    settings: LauncherSettings
    onChange: (next: LauncherSettings) => void
  } = $props()

  // Seeded from settings exactly as `OnInitialized` did.
  let mode = $state(untrack(() => settings.SkinViewerMode) || '2d')
  let model = $state(untrack(() => settings.SkinViewerModel) || 'default')
  let capeMode = $state(untrack(() => settings.SkinViewerCapeMode) || 'cape') // cape | elytra | off
  let ready = $state(false)
  let failed = $state(false)

  const cleanUuid = $derived((uuid ?? '').replace(/-/g, '').trim())
  const displayName = $derived(!username || !username.trim() ? 'Player' : username)
  const versionQuery = $derived(version > 0 ? `?v=${version}` : '')
  const skinTextureUrl = $derived(
    localSkinUrl ? localSkinUrl : `https://mc-heads.net/skin/${cleanUuid}${versionQuery}`,
  )
  const bodyRenderUrl = $derived(`https://mc-heads.net/body/${cleanUuid}/right${versionQuery}`)
  const fallbackRenderUrl = $derived(
    `https://crafatar.com/renders/body/${cleanUuid}?overlay&scale=10${version > 0 ? `&v=${version}` : ''}`,
  )
  const capeLabel = $derived(capeMode === 'elytra' ? 'Elytra' : capeMode === 'off' ? 'No cape' : 'Cape')

  let canvasEl: HTMLCanvasElement | undefined = $state()
  let viewer: GlSkinViewer | null = null

  function disposeViewer() {
    if (viewer) {
      try {
        viewer.dispose()
      } catch {
        /* ignore */
      }
      viewer = null
    }
  }

  // (Re)build the 3D scene whenever we're in 3D mode and the relevant
  // inputs change — mirrors `Init3d`/`_need3dInit` from the original.
  $effect(() => {
    const skinUrl = skinTextureUrl
    const cape = capeMode === 'off' ? '' : capeUrl
    const capeEquip = capeMode === 'elytra' ? 'elytra' : 'cape'
    const currentModel = model

    if (mode !== '3d' || !canvasEl) {
      disposeViewer()
      return
    }

    ready = false
    failed = false

    let cancelled = false
    disposeViewer()

    try {
      const v = new GlSkinViewer({
        canvas: canvasEl,
        width: 300,
        height: 300,
        animation: new WalkingAnimation(),
      })
      v.autoRotate = false
      viewer = v

      v.loadSkin(skinUrl, { model: currentModel === 'slim' ? 'slim' : 'default' })
        .then(() => {
          if (cancelled) return
          if (cape) {
            return v.loadCape(cape, { backEquipment: capeEquip }).catch(() => {
              /* cape load failure shouldn't sink the whole viewer */
            })
          }
        })
        .then(() => {
          if (cancelled) return
          ready = true
        })
        .catch(() => {
          if (cancelled) return
          failed = true
          mode = '2d'
          onChange({ ...settings, SkinViewerMode: '2d' })
        })
    } catch {
      failed = true
      mode = '2d'
      onChange({ ...settings, SkinViewerMode: '2d' })
    }

    return () => {
      cancelled = true
    }
  })

  $effect(() => {
    return () => disposeViewer()
  })

  function setMode(next: string) {
    if (mode === next) return
    mode = next
    onChange({ ...settings, SkinViewerMode: next })
  }

  function setModel(next: string) {
    if (model === next) return
    model = next
    onChange({ ...settings, SkinViewerModel: next })
    if (mode !== '3d') {
      // Jump to 3D so the change is visible, matching the original.
      mode = '3d'
      onChange({ ...settings, SkinViewerModel: next, SkinViewerMode: '3d' })
    }
  }

  function cycleCape() {
    capeMode = capeMode === 'cape' ? 'elytra' : capeMode === 'elytra' ? 'off' : 'cape'
    onChange({ ...settings, SkinViewerCapeMode: capeMode })
  }
</script>

<div class="skin-viewer">
  <div class="skin-stage">
    {#if mode === '3d'}
      <canvas bind:this={canvasEl} class="skin-canvas"></canvas>
      {#if !ready}
        <div class="skin-loading"><span class="spinner"></span></div>
      {/if}
    {:else}
      <img
        class="skin-body"
        src={bodyRenderUrl}
        alt="{displayName} skin"
        onerror={(e) => {
          const img = e.currentTarget as HTMLImageElement
          img.onerror = null
          img.src = fallbackRenderUrl
        }}
      />
    {/if}
  </div>

  <div class="skin-pills" onmousedown={(e) => e.stopPropagation()} role="presentation">
    <button class="skin-pill {mode === '2d' ? 'active' : ''}" onclick={() => setMode('2d')}>2D</button>
    <button class="skin-pill {mode === '3d' ? 'active' : ''}" onclick={() => setMode('3d')}>3D</button>
    <span class="skin-pill-sep"></span>
    <button
      class="skin-pill {model === 'default' ? 'active' : ''}"
      onclick={() => setModel('default')}
      data-tooltip={t('tt.classicModel')}>{t('x.steve')}</button>
    <button
      class="skin-pill {model === 'slim' ? 'active' : ''}"
      onclick={() => setModel('slim')}
      data-tooltip={t('tt.slimModel')}>{t('x.alex')}</button>
    {#if capeUrl && mode === '3d'}
      <button class="skin-pill" onclick={cycleCape} data-tooltip={t('tt.capeElytraOff')}>
        <i class="fa-solid fa-feather"></i> {capeLabel}
      </button>
    {/if}
  </div>
</div>
