<script lang="ts">
  /** Placeholder rows shown in place of a spinner while a list loads. */
  let { rows = 5 }: { rows?: number } = $props()
</script>

{#each Array(rows) as _, i (i)}
  <div class="skeleton-row">
    <div class="skeleton-block skeleton-icon"></div>
    <div class="skeleton-meta">
      <div class="skeleton-block skeleton-line" style="width:{58 + ((i * 13) % 30)}%"></div>
      <div class="skeleton-block skeleton-line skeleton-line-sm" style="width:{28 + ((i * 17) % 24)}%"></div>
    </div>
  </div>
{/each}
