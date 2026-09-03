<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import EditionTabs from './components/EditionTabs.svelte'

  /**
   * 1:1 port of the `javaclients` panel - `Pages/Home.razor:4849-5017`
   * ("Launchers"). Driving code-behind: `RefreshJavaClients` /
   * `LaunchLunar` / `LaunchLunarDirect` (Pages/Home.Panels.cs:529-572) and
   * the whole of `Pages/Home.Glacier.cs`.
   *
   * Structure follows the original exactly: info bar, a "Built-in" section
   * with the Vanilla card and the Glacier Client card, then a "Third-party"
   * section with the single combined Lunar/Badlion card, then the Java
   * bottom tab strip with `launchers` active. The original has NO separate
   * Badlion card and NO empty state - a previous rewrite invented both.
   *
   * Backend reality check - `detect_lunar_badlion` and `launch_client_exe`
   * exist in `src-tauri/src/lib.rs`; nothing backing `Home.Glacier.cs` does,
   * and there is no command that spawns the direct-launch JVM. Per the
   * porting rules those controls are rendered visibly disabled with an
   * explanatory tooltip rather than faked.
   */
  const MISSING_GLACIER =
    'Unavailable: needs get_glacier_manifest / install_glacier_client / launch_glacier_client commands'
  const MISSING_LUNAR_DIRECT = 'Unavailable: needs a launch_lunar_direct command'

  let {
    onBack,
    onNavigate = () => {},
    onToast = () => {},
  }: {
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  type LunarBadlionStatus = {
    lunar_installed: boolean
    badlion_installed: boolean
    can_direct_launch_lunar: boolean
    lunar_installed_versions: string[]
    lunar_exe_path: string | null
    badlion_exe_path: string | null
  }

  let status = $state<LunarBadlionStatus | null>(null)
  let lunarSelectedVersion = $state('')

  const lbInstalled = $derived(!!status && (status.lunar_installed || status.badlion_installed))
  const canDirect = $derived(!!status?.can_direct_launch_lunar)

  /** Port of `RefreshJavaClients` (and the detect half of `OpenJavaClientsAsync`). */
  async function refreshJavaClients() {
    try {
      const next = await invoke<LunarBadlionStatus>('detect_lunar_badlion')
      status = next
      if (!next.lunar_installed_versions.includes(lunarSelectedVersion))
        lunarSelectedVersion = next.lunar_installed_versions[0] ?? ''
    } catch (e) {
      onToast(String(e), 'error')
    }
  }

  /** Port of `LaunchLunar` - starts the installed Lunar launcher app. */
  async function launchLunar() {
    const exe = status?.lunar_exe_path ?? status?.badlion_exe_path
    if (!exe) {
      onToast("Lunar Client isn't installed. Get it from lunarclient.com.", 'error')
      return
    }
    try {
      await invoke('launch_client_exe', { exePath: exe })
      onToast('Lunar Client launching…', 'success')
    } catch (e) {
      onToast(String(e), 'error')
    }
  }

  function openUrl(url: string) {
    invoke('open_url', { url }).catch((e) => onToast(String(e), 'error'))
  }

  refreshJavaClients()
</script>

<PanelHeader title="Launchers" {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn"
      data-tooltip="Re-detect"
      aria-label="Re-detect"
      onmousedown={(e) => e.stopPropagation()}
      onclick={refreshJavaClients}
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="mcv-info-bar">
  <i class="fa-solid fa-circle-info"></i>
  <span
    >Vanilla launches through Glacier directly. Lunar and Badlion launch via their own installed apps
    — we don't proxy through them.</span
  >
</div>

<div class="panel-body">
  <span class="panel-section-label">Built-in</span>
  <div class="credits-card credits-card-glacier">
    <div class="credits-card-icon" style="background:var(--accent-bg); color:var(--accent);">
      <i class="fa-brands fa-java"></i>
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name">Vanilla (Glacier built-in)</span>
      <span class="credits-card-sub"
        >Launches Mojang's Java client directly. Pick a version in the Versions panel.</span
      >
    </div>
    <div class="credits-card-actions">
      <button
        class="vcs-btn"
        onmousedown={(e) => e.stopPropagation()}
        onclick={() => onNavigate('versions')}
      >
        <i class="fa-solid fa-box-archive"></i> Versions
      </button>
    </div>
  </div>

  <div class="credits-card">
    <div class="credits-card-icon" style="background:var(--accent-bg); color:var(--accent);">
      <i class="fa-solid fa-snowflake"></i>
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name">Glacier Client</span>
      <span class="credits-card-sub">
        <!-- The original renders the manifest here (name · loader · installed
             state). No backend command exposes the Glacier manifest, so the
             state is reported honestly rather than faked. -->
        <span class="client-card-note">{MISSING_GLACIER}</span>
      </span>
    </div>
    <div class="credits-card-actions">
      <button class="vcs-btn vcs-btn-ghost" disabled data-tooltip={MISSING_GLACIER}>
        <i class="fa-solid fa-rotate"></i> Retry
      </button>
    </div>
  </div>

  <span class="panel-section-label">Third-party</span>

  <div class="credits-card">
    <div class="credits-card-icon" style="background:#06b6d422; color:#22d3ee;">
      <i class="fa-solid fa-moon"></i>
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name"
        >Lunar Client <span style="opacity:0.55; font-weight:500;">incl. Badlion</span></span
      >
      <span class="credits-card-sub">
        {#if lbInstalled && canDirect}
          Direct launch ready · {status?.lunar_installed_versions.length} version(s) installed offline
        {:else if lbInstalled}
          Detected · {status?.lunar_exe_path ?? status?.badlion_exe_path} — first launch may show the
          Lunar splash
        {:else}
          Not installed — download from lunarclient.com (Lunar's purchase of Badlion lets one app
          cover both)
        {/if}
      </span>
    </div>
    <div class="credits-card-actions">
      {#if lbInstalled && canDirect}
        <select
          class="setting-select"
          bind:value={lunarSelectedVersion}
          onclick={(e) => e.stopPropagation()}
        >
          {#each status?.lunar_installed_versions ?? [] as lv (lv)}
            <option value={lv}>{lv}</option>
          {/each}
        </select>
        <button class="vcs-btn" disabled data-tooltip={MISSING_LUNAR_DIRECT}>
          <i class="fa-solid fa-bolt"></i> Launch
        </button>
      {:else if lbInstalled}
        <button class="vcs-btn" onmousedown={(e) => e.stopPropagation()} onclick={launchLunar}>
          <i class="fa-solid fa-play"></i> Launch
        </button>
      {:else}
        <button
          class="vcs-btn vcs-btn-ghost"
          onmousedown={(e) => e.stopPropagation()}
          onclick={() => openUrl('https://www.lunarclient.com')}
        >
          <i class="fa-solid fa-arrow-up-right-from-square"></i> Get it
        </button>
      {/if}
    </div>
  </div>
</div>

<EditionTabs edition="java" active="launchers" onNavigate={(view) => onNavigate(view)} />
