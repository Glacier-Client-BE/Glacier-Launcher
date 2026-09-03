<script lang="ts">
  import type { Snippet } from 'svelte'

  let { active = false, children }: { active?: boolean; children?: Snippet } = $props()
</script>

<div class="client-card" class:client-active={active}>
  {#if children}{@render children()}{/if}
</div>

<!-- .client-card / .client-card.client-active are defined in app.css
     (ported 1:1 from the original wwwroot/css/app.css) — no scoped styles
     here so there's one source of truth. -->
