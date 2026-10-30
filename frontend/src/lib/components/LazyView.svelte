<script lang="ts">
  import type { Component } from 'svelte'

  // Wraps a dynamically imported panel view so it is code-split out of the
  // main bundle. The import promise is cached by the module system, so
  // re-opening a panel does not refetch it; props are forwarded unchanged.
  // A failed chunk load shows an error with a retry instead of a blank panel.
  let {
    loader,
    ...props
  }: {
    loader: () => Promise<{ default: Component<any> }>
    [key: string]: any
  } = $props()

  let mod = $state(loader())

  function retry() {
    mod = loader()
  }
</script>

{#await mod}
  <!-- loading: keep the panel blank, as before -->
{:then m}
  {@const View = m.default}
  <View {...props} />
{:catch err}
  <div class="lazy-view-error" role="alert" style="padding:24px;display:flex;flex-direction:column;gap:10px;align-items:flex-start;">
    <p style="margin:0;">Could not load this panel.</p>
    <small style="opacity:0.7;word-break:break-word;">{String(err)}</small>
    <button type="button" class="btn-sm" onclick={retry}>Retry</button>
  </div>
{/await}
