<script lang="ts">
  import { t } from '../i18nStore.svelte'
  /**
   * HSV colour picker - 1:1 port of `Components/ColorPicker.razor` (198 lines).
   *
   * Three sliders (hue / shade / alpha) plus a hex field. "Shade" is the
   * original's single axis combining saturation and value: it is read back as
   * `s * v` and written as a straight multiply of the pure hue, so the picker
   * moves along the black-to-pure-hue line rather than a 2D square.
   *
   * Emitted colours are `#rrggbb` when opaque and `rgba(r,g,b,a)` otherwise,
   * matching `ComposeColor`.
   */
  let {
    value = '#000000',
    onChange,
    onClose,
  }: {
    value?: string
    onChange?: (hex: string) => void
    onClose?: () => void
  } = $props()

  let h = $state(0) // hue 0..360
  let shade = $state(0) // 0 (black) .. 1 (pure hue)
  let a = $state(1) // alpha 0..1
  let lastEmitted = ''

  /** `.ToString("0.###")`. */
  function fmt3(v: number): string {
    const s = v.toFixed(3).replace(/0+$/, '').replace(/\.$/, '')
    return s === '' || s === '-' ? '0' : s
  }

  function hexToRgb(hex: string): [number, number, number] {
    let s = (hex ?? '').trim().replace(/^#/, '')
    if (s.length === 3) s = s[0] + s[0] + s[1] + s[1] + s[2] + s[2]
    if (s.length < 6) return [0, 0, 0]
    const r = parseInt(s.slice(0, 2), 16)
    const g = parseInt(s.slice(2, 4), 16)
    const b = parseInt(s.slice(4, 6), 16)
    return [Number.isNaN(r) ? 0 : r, Number.isNaN(g) ? 0 : g, Number.isNaN(b) ? 0 : b]
  }

  function rgbToHsv(r: number, g: number, b: number): [number, number, number] {
    const rf = r / 255,
      gf = g / 255,
      bf = b / 255
    const max = Math.max(rf, gf, bf)
    const min = Math.min(rf, gf, bf)
    const d = max - min
    let hue = 0
    if (d > 0) {
      if (max === rf) hue = ((gf - bf) / d) % 6
      else if (max === gf) hue = (bf - rf) / d + 2
      else hue = (rf - gf) / d + 4
      hue *= 60
      if (hue < 0) hue += 360
    }
    const s = max <= 0 ? 0 : d / max
    return [hue, s, max]
  }

  function hsvToHex(hue: number, s: number, v: number): string {
    hue = ((hue % 360) + 360) % 360
    const c = v * s
    const x = c * (1 - Math.abs(((hue / 60) % 2) - 1))
    const m = v - c
    let r = 0,
      g = 0,
      b = 0
    if (hue < 60) [r, g, b] = [c, x, 0]
    else if (hue < 120) [r, g, b] = [x, c, 0]
    else if (hue < 180) [r, g, b] = [0, c, x]
    else if (hue < 240) [r, g, b] = [0, x, c]
    else if (hue < 300) [r, g, b] = [x, 0, c]
    else [r, g, b] = [c, 0, x]
    const to = (n: number) =>
      Math.min(255, Math.max(0, Math.round((n + m) * 255)))
        .toString(16)
        .padStart(2, '0')
        .toUpperCase()
    return `#${to(r)}${to(g)}${to(b)}`
  }

  function shadeHex(hue: number, sh: number): string {
    const [r, g, b] = hexToRgb(hsvToHex(hue, 1, 1))
    const l = (c: number) =>
      Math.round(c * sh)
        .toString(16)
        .padStart(2, '0')
        .toUpperCase()
    return `#${l(r)}${l(g)}${l(b)}`
  }

  function composeColor(hue: number, sh: number, alpha: number): string {
    const hex = shadeHex(hue, sh)
    if (alpha >= 0.999) return hex
    const [r, g, b] = hexToRgb(hex)
    return `rgba(${r},${g},${b},${fmt3(alpha)})`
  }

  function parseColor(input: string): [number, number, number, number] {
    const s = (input ?? '').trim()
    if (s.slice(0, 3).toLowerCase() === 'rgb') {
      const open = s.indexOf('(')
      const close = s.indexOf(')')
      if (open < 0 || close <= open) return [0, 0, 0, 1]
      const parts = s.slice(open + 1, close).split(',').map((p) => p.trim())
      if (parts.length < 3) return [0, 0, 0, 1]
      const r = parseInt(parts[0], 10)
      const g = parseInt(parts[1], 10)
      const b = parseInt(parts[2], 10)
      if ([r, g, b].some(Number.isNaN)) return [0, 0, 0, 1]
      const alpha = parts.length > 3 ? parseFloat(parts[3]) : 1
      return [r, g, b, Math.min(1, Math.max(0, Number.isNaN(alpha) ? 1 : alpha))]
    }
    let hx = s.replace(/^#/, '')
    if (hx.length === 3) hx = hx[0] + hx[0] + hx[1] + hx[1] + hx[2] + hx[2]
    if (hx.length < 6) return [0, 0, 0, 1]
    const [r, g, b] = hexToRgb('#' + hx.slice(0, 6))
    const alpha = hx.length >= 8 ? parseInt(hx.slice(6, 8), 16) / 255 : 1
    return [r, g, b, Number.isNaN(alpha) ? 1 : alpha]
  }

  /**
   * Port of `OnParametersSet`: re-derive the sliders from an incoming value,
   * but only when it isn't the value this picker itself last emitted -
   * otherwise round-tripping would fight the user's dragging.
   */
  $effect(() => {
    if (value.toLowerCase() === lastEmitted.toLowerCase()) return
    const [r, g, b, alpha] = parseColor(value)
    const [hue, s, v] = rgbToHsv(r, g, b)
    h = hue
    shade = Math.min(1, Math.max(0, s * v))
    a = alpha
  })

  function emit() {
    const color = composeColor(h, shade, a)
    lastEmitted = color
    onChange?.(color)
  }

  function onHex(e: Event) {
    const raw = (e.currentTarget as HTMLInputElement).value ?? ''
    let s = raw.trim().replace(/^#/, '')
    if (s.length === 3) s = s[0] + s[0] + s[1] + s[1] + s[2] + s[2]
    if (s.length !== 6 && s.length !== 8) return
    if (!/^[0-9a-fA-F]+$/.test(s)) return
    const [r, g, b] = hexToRgb('#' + s.slice(0, 6))
    const [hue, sat, val] = rgbToHsv(r, g, b)
    h = hue
    shade = Math.min(1, Math.max(0, sat * val))
    a = s.length === 8 ? parseInt(s.slice(6, 8), 16) / 255 : 1
    emit()
  }

  const pureHueHex = $derived(hsvToHex(h, 1, 1))
  const opaqueHex = $derived(shadeHex(h, shade))
  const currentColor = $derived(composeColor(h, shade, a))
  const hexBody = $derived.by(() => {
    const [r, g, b] = hexToRgb(opaqueHex)
    const hh = (n: number) => n.toString(16).padStart(2, '0').toUpperCase()
    return a >= 0.999
      ? `${hh(r)}${hh(g)}${hh(b)}`
      : `${hh(r)}${hh(g)}${hh(b)}${hh(Math.round(a * 255))}`
  })
</script>

<div class="cpick" role="presentation" onmousedown={(e) => e.stopPropagation()}>
  <input
    class="cpick-slider cpick-hue"
    type="range"
    min="0"
    max="360"
    step="1"
    aria-label={t('tt.hue')}
    value={Math.round(h)}
    oninput={(e) => { h = Math.min(360, Math.max(0, +e.currentTarget.value)); emit() }}
  />

  <input
    class="cpick-slider cpick-shade"
    type="range"
    min="0"
    max="100"
    step="1"
    aria-label={t('tt.shade')}
    style="--cpick-grad: linear-gradient(to right, #000, {pureHueHex});"
    value={Math.round(shade * 100)}
    oninput={(e) => { shade = Math.min(1, Math.max(0, +e.currentTarget.value / 100)); emit() }}
  />

  <input
    class="cpick-slider cpick-alpha"
    type="range"
    min="0"
    max="100"
    step="1"
    aria-label={t('tt.opacity')}
    style="--cpick-grad: linear-gradient(to right, transparent, {opaqueHex});"
    value={Math.round(a * 100)}
    oninput={(e) => { a = Math.min(1, Math.max(0, +e.currentTarget.value / 100)); emit() }}
  />

  <div class="cpick-foot">
    <span class="cpick-preview" style="--dot-color:{currentColor};"></span>
    <div class="cpick-hexwrap">
      <span class="cpick-hash">#</span>
      <input
        class="cpick-hex"
        value={hexBody}
        onchange={onHex}
        spellcheck="false"
        autocomplete="off"
        maxlength="8"
        aria-label={t('tt.hexColour')}
      />
    </div>
    <button type="button" class="btn-sm btn-accent" onclick={() => onClose?.()}>{t('x.done')}</button>
  </div>
</div>
