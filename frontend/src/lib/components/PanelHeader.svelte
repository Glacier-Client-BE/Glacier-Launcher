<script lang="ts">
  import type { Snippet } from 'svelte'

  /**
   * `backIcon` exists because the original is not uniform: 20 of the 23
   * `.panel-back-btn` occurrences in Home.razor use `fa-chevron-down`, but
   * `versions` (2214), `levimods` (3003) and the level.dat editor (4494) use
   * `fa-chevron-left`. Defaulting to the majority keeps every other panel
   * unchanged.
   */
  let {
    title,
    onBack,
    extraActions,
    backIcon = 'fa-chevron-down',
  }: { title: string; onBack?: () => void; extraActions?: Snippet; backIcon?: string } = $props()
</script>

<div class="panel-header">
  <div class="panel-title-wrap">
    <span class="panel-title">{title}</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-actions">
    {#if extraActions}{@render extraActions()}{/if}
    {#if onBack}
      <button type="button" class="panel-back-btn" data-tooltip="Back" aria-label="Back" onclick={onBack}>
        <i class="fa-solid {backIcon}"></i>
      </button>
    {/if}
  </div>
</div>

<!-- .panel-header / .panel-title / .panel-back-btn etc. are defined in
     app.css (ported 1:1) — no scoped styles here. Note: the old scoped
     version referenced an invented `--text-h` token that doesn't exist in
     the real design system, which silently broke the title color; using
     the global `.panel-title` rule (var(--text)) fixes that too. -->
