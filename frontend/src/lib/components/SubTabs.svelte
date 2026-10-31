<script module lang="ts">
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

  export type SubTab = { key: string; icon: string; img?: string; label: string }

  /**
   * Port of `UtilitiesTabs`. Exported so the panels it routes to (Bedrock
   * Backups, Bedrock Screenshots, UtilitiesView itself) can all render the
   * same vertical `.settings-nav-rail` from one source of truth instead of
   * three hand-copied arrays drifting apart.
   */
  export const UTILITIES: SubTab[] = [
    { key: 'options', icon: 'fa-solid fa-sliders', label: 'Options' },
    { key: 'shaders', icon: 'fa-solid fa-droplet', label: 'Shaders' },
    { key: 'clients', icon: 'fa-solid fa-file-code', label: 'Library' },
    { key: 'system', icon: 'fa-solid fa-microchip', label: 'System' },
    { key: 'backups', icon: 'fa-solid fa-clock-rotate-left', label: 'Backups' },
    { key: 'photos', icon: 'fa-solid fa-images', label: 'Photos' },
  ]
</script>

<script lang="ts">


  let {
    strip,
    active,
    onNavigate,
    edition,
  }: {
    strip: SubTabKey
    active: string
    /**
     * The second argument is the strip this came from. It matters because the
     * same key means different things in different strips: `clients` is the
     * Clients *panel* in `BedrockTabs` but an internal Utilities tab here.
     */
    onNavigate: (key: string, strip?: string) => void
    /**
     * Only meaningful for `packs-addons`: picks the real VanillaTweaks /
     * BedrockTweaks and LeviLamina brand art for the Tweaks and Clients
     * pills instead of a generic wand/puzzle glyph. Omit for strips that
     * don't vary by edition.
     */
    edition?: 'java' | 'bedrock'
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

  /**
   * Port of `PacksAddonsTabs`. Tweaks and Clients carry the real brand art
   * for whichever edition is active (VanillaTweaks/BedrockTweaks,
   * LeviLamina) instead of a generic wand/puzzle glyph - falls back to the
   * glyph when no edition is given.
   *
   * `clientlib` (the Indigo-Tools/ClientLibrary DLL/EXE catalog) and `packs`
   * ("Manage" — Bedrock's resource/behavior/skin pack manager) are both
   * Bedrock-only destinations with no Java equivalent to route to;
   * JavaModsView's own nav rail (Mods/Assets/Loaders/etc.) already covers
   * both roles for Java internally. Showing them on the Java edition of this
   * strip sent Java users into Bedrock-only panels — dropped for `edition
   * === 'java'` instead.
   */
  const PACKS_ADDONS: SubTab[] = $derived(
    [
      { key: 'addons', icon: 'fa-solid fa-cloud-arrow-down', label: 'Browse' },
      edition === 'java'
        ? null
        : {
            key: 'clientlib',
            icon: 'fa-solid fa-wand-sparkles',
            img: edition === 'bedrock' ? '/images/clients/levi-lamina.svg' : undefined,
            label: 'Clients',
          },
      {
        key: 'tweaks',
        icon: 'fa-solid fa-wand-magic-sparkles',
        img: edition === 'java' ? '/images/vanilla-tweaks.svg' : edition === 'bedrock' ? '/images/bedrock-tweaks.svg' : undefined,
        label: 'Tweaks',
      },
      edition === 'java' ? null : { key: 'packs', icon: 'fa-solid fa-boxes-stacked', label: 'Manage' },
    ].filter((t): t is SubTab => t !== null),
  )

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

<!--
  interop.js's tab-cycler (prev/next arrow paging) only ever attaches to
  `.versions-client-switcher:not(.switcher-wrap)` — it exists for strips with
  more tabs than can ever reasonably fit (Utilities' 6). Every strip here
  with 4 tabs or fewer should always show all of its tabs in one view rather
  than being paged behind arrows, so it opts out via `.switcher-wrap`
  (wrapping to a second row in the rare case it doesn't fit, instead of
  hiding tabs).
-->
<div class="versions-client-switcher {tabs.length <= 4 ? 'switcher-wrap' : ''}">
  {#each tabs as tab (tab.key)}
    <button
      class="vcs-btn {active === tab.key ? 'active' : ''}"
      onclick={() => onNavigate(tab.key, strip)}
    >
      {#if tab.img}
        <img class="vcs-btn-img" src={tab.img} alt="" />
      {:else}
        <i class={tab.icon}></i>
      {/if}
      &nbsp;{tab.label}
    </button>
  {/each}
</div>
