<script lang="ts">
  let { size, borderWidth, class: cls = '' }: { size?: number; borderWidth?: number; class?: string } =
    $props()

  const style = $derived(
    [size ? `width:${size}px;height:${size}px;` : '', borderWidth ? `border-width:${borderWidth}px;` : ''].join(''),
  )
</script>

<span class="spinner {cls}" {style} role="status" aria-live="polite"></span>

<!-- .spinner + @keyframes spin-pulse are defined in app.css (ported 1:1) —
     no scoped styles here. -->
