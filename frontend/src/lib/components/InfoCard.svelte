<script lang="ts">
  import type { Snippet } from 'svelte'
  import Card from './Card.svelte'

  let {
    icon,
    iconVariant = 'latite',
    name,
    sub,
    active = false,
    actions,
  }: {
    icon: string
    iconVariant?: 'latite' | 'flarial' | 'oderso'
    name: string
    sub: string
    active?: boolean
    actions?: Snippet
  } = $props()
</script>

<Card {active}>
  <div class="client-card-header">
    <div class="client-card-icon {iconVariant}-icon"><i class={icon}></i></div>
    <div class="client-card-meta">
      <span class="client-card-name">{name}</span>
      <span class="client-card-sub">{sub}</span>
    </div>
    {#if actions}
      <div class="client-card-actions">{@render actions()}</div>
    {/if}
  </div>
</Card>

<!-- .client-card-* classes are defined in app.css (ported 1:1) — no scoped
     styles here so there's one source of truth. -->
