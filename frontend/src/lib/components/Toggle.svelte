<script lang="ts">
  let { on, onChanged }: { on: boolean; onChanged: (v: boolean) => void } = $props()

  function handleClick() {
    onChanged(!on)
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === ' ' || e.key === 'Enter') handleClick()
  }
</script>

<div
  class="toggle"
  class:on
  role="switch"
  aria-checked={on}
  tabindex="0"
  onclick={handleClick}
  onkeydown={handleKeyDown}
></div>

<!-- .toggle is defined in app.css (ported 1:1) — no scoped styles here. -->
