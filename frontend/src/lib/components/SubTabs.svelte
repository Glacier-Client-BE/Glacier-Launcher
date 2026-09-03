<script lang="ts">
  /**
   * Cross-panel navigation strips - port of `SubTabs` and its four named
   * configurations in Home.MergedPanels.cs:10-51.
   *
   * These are a *different* element from the bottom `.panel-tabs` strip:
   * they render as `.versions-client-switcher` with `.vcs-btn` children, and
   * they link between panels that belong to the same family (Worlds/Servers,
   * Versions/Instances, and so on). None of the four had any counterpart in
   * the rewrite - the original's navigation graph was simply gone.
   *
   * Faithful detail: the original emits `" " + Label`, so each label carries
   * a leading space after the icon.
   */
  export type SubTabKey =
    | 'worlds-servers'
    | 'versions-instances'
    | 'packs-addons'
    | 'utilities'

  type SubTab = { key: string; icon: string; label: string }

  let {
    strip,
    active,
    onNavigate,
  }: {
    strip: SubTabKey
    active: string
    /**
     * The second argument is the strip this came from. It matters because the
     * same key means different things in different strips: `clients` is the
     * Clients *panel* in `BedrockTabs` but an internal Utilities tab here.
     */
    onNavigate: (key: string, strip?: string) => void
  } = $props()

  /** Port of `WorldsServersTabs`. */
  const WORLDS_SERVERS: SubTab[] = [
    { key: 'worlds', icon: 'fa-solid fa-globe', label: 'Worlds' },
    { key: 'servers', icon: 'fa-solid fa-server', label: 'Servers' },
  ]

  /** Port of `VersionsInstancesTabs`. */
  const VERSIONS_INSTANCES: SubTab[] = [
    { key: 'versions', icon: 'fa-solid fa-box-archive', label: 'MC Versions' },
    { key: 'instances', icon: 'fa-solid fa-layer-group', label: 'Instances' },
  ]

  /** Port of `PacksAddonsTabs`. */
  const PACKS_ADDONS: SubTab[] = [
    { key: 'addons', icon: 'fa-solid fa-cloud-arrow-down', label: 'Browse' },
    { key: 'clientlib', icon: 'fa-solid fa-wand-sparkles', label: 'Clients' },
    { key: 'tweaks', icon: 'fa-solid fa-wand-magic-sparkles', label: 'Tweaks' },
    { key: 'packs', icon: 'fa-solid fa-boxes-stacked', label: 'Manage' },
  ]

  /** Port of `UtilitiesTabs`. */
  const UTILITIES: SubTab[] = [
    { key: 'options', icon: 'fa-solid fa-sliders', label: 'Options' },
    { key: 'shaders', icon: 'fa-solid fa-droplet', label: 'Shaders' },
    { key: 'clients', icon: 'fa-solid fa-file-code', label: 'Library' },
    { key: 'system', icon: 'fa-solid fa-microchip', label: 'System' },
    { key: 'backups', icon: 'fa-solid fa-clock-rotate-left', label: 'Backups' },
    { key: 'photos', icon: 'fa-solid fa-images', label: 'Photos' },
  ]

  const tabs = $derived(
    strip === 'worlds-servers'
      ? WORLDS_SERVERS
      : strip === 'versions-instances'
        ? VERSIONS_INSTANCES
        : strip === 'packs-addons'
          ? PACKS_ADDONS
          : UTILITIES,
  )
</script>

<div class="versions-client-switcher">
  {#each tabs as tab (tab.key)}
    <button
      class="vcs-btn {active === tab.key ? 'active' : ''}"
      onclick={() => onNavigate(tab.key, strip)}
    >
      <i class={tab.icon}></i>&nbsp;{tab.label}
    </button>
  {/each}
</div>
