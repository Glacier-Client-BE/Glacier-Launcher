<script lang="ts">
  /**
   * The per-panel bottom tab strip (component name: EditionTabs) - port of `BedrockTabs` / `JavaTabs`
   * (Home.BigFeatures.cs:31-75).
   *
   * This is NOT a global shell bar: the original renders this strip *inside*
   * each panel, which is why all 20 `.panel-tabs` occurrences in Home.razor
   * live within panel markup. A previous rewrite hoisted an invented
   * 15-entry version into the shell; that has been deleted.
   *
   * Faithful detail worth keeping: Bedrock omits the click handler on the
   * *active* tab (`if (active != view)`, line 60-61) while Java wires it
   * unconditionally. The difference is preserved below rather than
   * normalised.
   */
  type Tab = { view: string; icon: string; label: string }

  let {
    active,
    edition,
    onNavigate,
  }: {
    active: string
    edition: 'bedrock' | 'java'
    onNavigate: (view: string) => void
  } = $props()

  /** Port of `BedrockTabs` - 7 tabs, in this exact order. */
  const BEDROCK_TABS: Tab[] = [
    { view: 'settings', icon: 'fa-solid fa-gear', label: 'Settings' },
    { view: 'clients', icon: 'fa-solid fa-puzzle-piece', label: 'Clients' },
    { view: 'addons', icon: 'fa-solid fa-boxes-stacked', label: 'Packs' },
    { view: 'mcversions', icon: 'fa-solid fa-box-archive', label: 'Versions' },
    { view: 'worlds', icon: 'fa-solid fa-globe', label: 'Worlds' },
    { view: 'utilities', icon: 'fa-solid fa-screwdriver-wrench', label: 'Utilities' },
    { view: 'credits', icon: 'fa-solid fa-heart', label: 'Credits' },
  ]

  /** Port of `JavaTabs` - 8 tabs. */
  const JAVA_TABS: Tab[] = [
    { view: 'settings', icon: 'fa-solid fa-gear', label: 'Settings' },
    { view: 'launchers', icon: 'fa-solid fa-rocket', label: 'Launchers' },
    { view: 'mods', icon: 'fa-solid fa-puzzle-piece', label: 'Mods' },
    { view: 'versions', icon: 'fa-solid fa-box-archive', label: 'Versions' },
    { view: 'tweaks', icon: 'fa-solid fa-wand-magic-sparkles', label: 'Tweaks' },
    { view: 'profile', icon: 'fa-solid fa-user', label: 'Profile' },
    { view: 'photos', icon: 'fa-solid fa-images', label: 'Photos' },
    { view: 'credits', icon: 'fa-solid fa-heart', label: 'Credits' },
  ]

  const tabs = $derived(edition === 'bedrock' ? BEDROCK_TABS : JAVA_TABS)
</script>

<div class="panel-tabs">
  {#each tabs as tab (tab.view + tab.label)}
    {#if edition === 'bedrock' && active === tab.view}
      <!-- Active Bedrock tab carries no click handler, as in the original. -->
      <button class="panel-tab active">
        <i class={tab.icon}></i>{tab.label}
      </button>
    {:else}
      <button
        class="panel-tab {active === tab.view ? 'active' : ''}"
        onclick={() => onNavigate(tab.view)}
      >
        <i class={tab.icon}></i>{tab.label}
      </button>
    {/if}
  {/each}
</div>
