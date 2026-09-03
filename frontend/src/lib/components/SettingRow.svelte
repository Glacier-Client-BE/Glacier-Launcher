<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    label,
    hint = '',
    clickable = false,
    onClick,
    onRowClick,
    hintContent,
    trailing,
    hiddenBySearch = false,
  }: {
    label: string
    hint?: string
    clickable?: boolean
    onClick?: () => void
    /** Alias for onClick — some ports read more naturally with this name. */
    onRowClick?: () => void
    hintContent?: Snippet
    trailing?: Snippet
    /** Mirrors the original's SH()/hidden-by-search class for the settings search filter. */
    hiddenBySearch?: boolean
  } = $props()

  function fire() {
    onClick?.()
    onRowClick?.()
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -- role is conditional but always paired with tabindex/onkeydown when set -->
<div
  class="setting-row"
  class:clickable
  class:hidden-by-search={hiddenBySearch}
  role={clickable ? 'button' : undefined}
  tabindex={clickable ? 0 : undefined}
  onclick={() => clickable && fire()}
  onkeydown={(e) => clickable && (e.key === 'Enter' || e.key === ' ') && fire()}
>
  <div class="setting-meta">
    <span class="setting-label">{label}</span>
    {#if hint}
      <span class="setting-hint">{hint}</span>
    {:else if hintContent}
      <span class="setting-hint">{@render hintContent()}</span>
    {/if}
  </div>
  {#if trailing}{@render trailing()}{/if}
</div>

<!-- .setting-row / .setting-label / .setting-hint / .setting-meta are
     defined in app.css (ported 1:1) — no scoped styles here. -->
