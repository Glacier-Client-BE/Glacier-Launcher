<script lang="ts">
  export interface TabDef {
    id: string
    label: string
    icon: string
  }

  let { tabs, active, onSelect }: { tabs: TabDef[]; active: string; onSelect: (id: string) => void } =
    $props()
</script>

<div class="panel-tabs" role="tablist">
  {#each tabs as tab (tab.id)}
    <button
      type="button"
      class="panel-tab"
      class:active={active === tab.id}
      role="tab"
      aria-selected={active === tab.id}
      onclick={() => onSelect(tab.id)}
    >
      <i class={tab.icon}></i>{tab.label}
    </button>
  {/each}
</div>

<!-- .panel-tabs / .panel-tab are defined in app.css (ported 1:1) — no
     scoped styles here. The old scoped version diverged from the real
     values (padding, border-top, background, overflow-x) and, because
     Svelte's scoping attribute makes scoped selectors more specific than
     plain global ones, silently won over the real CSS. -->

<!-- interop.js's tab-cycler (arrow buttons for overflowing tab rows) reads
     the tabs from `.panel-tabs > .panel-tab` and only activates when they
     overflow — nothing extra needed here for it to work. -->
