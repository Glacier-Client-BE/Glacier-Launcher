<script lang="ts">
  import type { Snippet } from 'svelte'

  /**
   * Every panel's back control now uses the same glyph and the same
   * position (leading, before the title) — matching the mockup's
   * slideOutFrame header. `backIcon` stays overridable for the rare case
   * that genuinely needs a different affordance, but nothing passes it
   * anymore.
   */
  let {
    title,
    onBack,
    extraActions,
    backIcon = 'fa-arrow-left',
    breadcrumb = [],
  }: {
    title: string
    onBack?: () => void
    extraActions?: Snippet
    backIcon?: string
    /** Optional trail e.g. ['Home', 'Utilities', 'Shaders']. Empty/omitted
     * renders nothing, so panels that don't opt in look exactly as before. */
    breadcrumb?: string[]
  } = $props()
</script>

<div class="panel-header {onBack ? 'has-back' : ''}">
  {#if onBack}
    <button type="button" class="panel-back-btn" data-tooltip="Back" aria-label="Back" onclick={onBack}>
      <i class="fa-solid {backIcon}"></i>
    </button>
  {/if}
  <div class="panel-title-wrap">
    {#if breadcrumb.length > 0}
      <div class="panel-breadcrumb">
        {#each breadcrumb as segment, i (i)}
          {#if i > 0}<i class="fa-solid fa-chevron-right panel-breadcrumb-sep"></i>{/if}
          <span class="panel-breadcrumb-seg {i === breadcrumb.length - 1 ? 'current' : ''}">{segment}</span>
        {/each}
      </div>
    {/if}
    <span class="panel-title">{title}</span>
    <div class="panel-title-underline"></div>
  </div>
  {#if onBack}<div class="panel-header-spacer"></div>{/if}
  {#if extraActions}
    <div class="panel-header-actions">{@render extraActions()}</div>
  {/if}
</div>

<!-- .panel-header / .panel-title / .panel-back-btn etc. are defined in
     app.css (ported 1:1) — no scoped styles here. Note: the old scoped
     version referenced an invented `--text-h` token that doesn't exist in
     the real design system, which silently broke the title color; using
     the global `.panel-title` rule (var(--text)) fixes that too. -->
