<script lang="ts">
  /**
   * 1:1 port of `Components/SkinViewer.razor` (200 lines).
   *
   * Backend/host gap, rendered as a visibly disabled control rather than
   * faked: the original's 3D mode drives a `glacierSkin` JS interop object
   * (three.js) that has no counterpart in the rewrite — there is no
   * `glacierSkin.render` / `setModel` / `setCape` / `dispose`. The "3D" pill
   * is therefore disabled with an explanatory tooltip, and `_mode` is pinned
   * to "2d". Everything else (the pills, the Steve/Alex model choice, the
   * settings persistence, the mc-heads/crafatar URLs and the `onerror`
   * fallback) is transcribed verbatim.
   *
   * Because 3D never engages, the cape pill (`_mode == "3d"` guarded in the
   * original) never renders either — which is faithful, not an omission.
   *
   * No scoped <style>: .skin-viewer/.skin-stage/.skin-body/.skin-pills/
   * .skin-pill/.skin-pill-sep all live in app.css.
   */
  import type { LauncherSettings } from '../settings'

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

  // `_mode` / `_model` seed from settings exactly as `OnInitialized` does.
  // `_mode` is forced to "2d" because the 3D renderer is not ported.
  let mode = $state('2d')
  let model = $state(settings.SkinViewerModel || 'default')

  const cleanUuid = $derived((uuid ?? '').replace(/-/g, '').trim())
  const displayName = $derived(!username || !username.trim() ? 'Player' : username)
  const versionQuery = $derived(version > 0 ? `?v=${version}` : '')
  const bodyRenderUrl = $derived(`https://mc-heads.net/body/${cleanUuid}/right${versionQuery}`)
  const fallbackRenderUrl = $derived(
    `https://crafatar.com/renders/body/${cleanUuid}?overlay&scale=10${version > 0 ? `&v=${version}` : ''}`,
  )

  /** Port of `SetModel` — minus the "jump to 3D so the change is visible" arm,
   *  which cannot run without the 3D renderer. */
  function setModel(next: string) {
    if (model === next) return
    model = next
    onChange({ ...settings, SkinViewerModel: next })
  }
</script>

<div class="skin-viewer">
  <div class="skin-stage">
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
  </div>

  <div class="skin-pills" onmousedown={(e) => e.stopPropagation()} role="presentation">
    <button class="skin-pill {mode === '2d' ? 'active' : ''}">2D</button>
    <button
      class="skin-pill"
      disabled
      data-tooltip="3D preview is not available in this build"
    >3D</button>
    <span class="skin-pill-sep"></span>
    <button
      class="skin-pill {model === 'default' ? 'active' : ''}"
      onclick={() => setModel('default')}
      data-tooltip="Classic model">Steve</button>
    <button
      class="skin-pill {model === 'slim' ? 'active' : ''}"
      onclick={() => setModel('slim')}
      data-tooltip="Slim model">Alex</button>
  </div>
</div>
