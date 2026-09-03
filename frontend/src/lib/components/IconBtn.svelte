<script lang="ts">
  import type { Snippet } from 'svelte'
  import Spinner from './Spinner.svelte'

  let {
    icon = '',
    title = '',
    class: cls = '',
    ghost = false,
    disabled = false,
    loading = false,
    onClick,
    children,
  }: {
    icon?: string
    title?: string
    class?: string
    ghost?: boolean
    disabled?: boolean
    loading?: boolean
    onClick?: () => void
    children?: Snippet
  } = $props()
</script>

<button
  type="button"
  class="icon-btn {ghost ? 'icon-btn-ghost' : ''} {cls}"
  data-tooltip={title}
  aria-label={title}
  aria-busy={loading}
  {disabled}
  onclick={(e) => {
    e.stopPropagation()
    onClick?.()
  }}
>
  {#if loading}
    <Spinner size={14} />
  {:else if icon}
    <i class={icon}></i>
  {:else if children}
    {@render children()}
  {/if}
</button>

<!-- .icon-btn / .icon-btn-ghost are defined in app.css (ported 1:1 from the
     original — confirmed against Home.razor's own `class="icon-btn"` /
     `class="icon-btn icon-btn-ghost"` markup) — no scoped styles here. -->
