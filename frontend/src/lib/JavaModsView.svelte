<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import EditionTabs from './components/EditionTabs.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import IconBtn from './components/IconBtn.svelte'
  import type { LauncherSettings, JavaInstance } from './settings'

  /**
   * 1:1 port of the Java (`IsJava`) half of the `addons` panel -
   * `Pages/Home.razor:2362-2987` ("Mods & Addons", the largest panel in the
   * app). The Bedrock (`IsBedrock`) branches of that same panel live in
   * `BedrockPacksView.svelte` and are deliberately NOT duplicated here.
   *
   * All seven sub-tabs of `javaModsTab` are present: loaders, mods, assets,
   * datapacks, tools, curseforge, modrinth (Home.razor:2392-2419).
   *
   * Backend reality check - the loaders/mods tabs are fully wired to the
   * java_mods.rs commands. Several other controls have NO backend command in
   * `src-tauri/src/lib.rs`; per the porting rules those are rendered visibly
   * disabled with an explanatory tooltip rather than faked. Each is marked
   * with a `MISSING_*` constant below naming the command it needs.
   */
  let {
    settings,
    onBack,
    onNavigate = () => {},
  }: {
    settings: LauncherSettings
    onBack?: () => void
    onNavigate?: (view: string) => void
  } = $props()

  type ModInfo = {
    file_name: string
    mod_id: string
    name: string
    provides: string[]
    required_deps: string[]
    loader: string
    disabled: boolean
  }
  type ModReport = { mods: ModInfo[]; warnings: string[] }

  type JavaModsTab =
    | 'loaders'
    | 'mods'
    | 'assets'
    | 'datapacks'
    | 'tools'
    | 'curseforge'
    | 'modrinth'

  // ---------------------------------------------------------------------
  // Missing backend commands. Each string is used as the `data-tooltip` on
  // the control the original would have wired up.
  // ---------------------------------------------------------------------
  const MISSING_FILE_LIST =
    'Unavailable: needs a list_java_instance_files command (resourcepacks/shaderpacks/schematics)'
  const MISSING_WORLDS = 'Unavailable: needs a list_java_worlds command'
  const MISSING_DATAPACKS =
    'Unavailable: needs list_java_datapacks / import_java_datapack / delete_java_datapack / open_java_datapacks_folder commands'
  const MISSING_BACKUP = 'Unavailable: needs a backup_java_saves command'
  const MISSING_EXPORT = 'Unavailable: needs an export_java_modpack command'
  const MISSING_DUPLICATE = 'Unavailable: needs a duplicate_java_instance command'
  const MISSING_CF_SEARCH =
    'Unavailable: needs a search_curseforge_addons command (search_curseforge_modpacks is modpack-only) plus a CurseForge API key'
  const MISSING_CF_INSTALL = 'Unavailable: needs an install_curseforge_file command'
  const MISSING_MR_SEARCH =
    'Unavailable: needs a search_modrinth_projects command (search_modrinth_modpacks is modpack-only)'
  const MISSING_MR_INSTALL = 'Unavailable: needs an install_modrinth_version command'

  let javaModsTab = $state<JavaModsTab>('loaders')
  let instances = $state<JavaInstance[]>([])
  let activeInstanceId = $state('')
  let report = $state<ModReport>({ mods: [], warnings: [] })
  let error = $state('')
  let javaUtilityBusy = $state(false)

  // Assets tab counts. Only screenshots have a backing command
  // (list_java_screenshots); the rest stay null and render as "-".
  let javaScreenshotsCount = $state<number | null>(null)

  // Datapacks tab state (Home.Datapacks.cs). No backing commands exist, so
  // the world list is always empty and the picker is disabled.
  let datapacksWorldName = $state('')
  const javaWorldsList: { Name: string }[] = []
  const javaDatapacks: { Name: string; SizeBytes: number; ModifiedAt: string }[] = []

  // CurseForge browse state (Home.Clients.cs:839-930).
  const CF_CATEGORIES: { key: string; label: string; icon: string }[] = [
    { key: 'all', label: 'All', icon: 'fa-solid fa-layer-group' },
    { key: 'mods', label: 'Mods', icon: 'fa-solid fa-puzzle-piece' },
    { key: 'modpacks', label: 'Modpacks', icon: 'fa-solid fa-cubes' },
    { key: 'texturepacks', label: 'Resource Packs', icon: 'fa-solid fa-palette' },
    { key: 'shaders', label: 'Shaders', icon: 'fa-solid fa-droplet' },
    { key: 'worlds', label: 'Worlds', icon: 'fa-solid fa-globe' },
  ]
  const MR_CATEGORIES: { key: string; label: string; icon: string }[] = [
    { key: 'mod', label: 'Mods', icon: 'fa-solid fa-puzzle-piece' },
    { key: 'modpack', label: 'Modpacks', icon: 'fa-solid fa-cubes' },
    { key: 'resourcepack', label: 'Resource Packs', icon: 'fa-solid fa-palette' },
    { key: 'shader', label: 'Shaders', icon: 'fa-solid fa-droplet' },
  ]

  type CfAddon = {
    Id: number
    Name: string
    AuthorName: string
    Summary: string
    ThumbnailUrl: string
    DownloadCount: number
    CategoryName: string
  }
  type MrProject = {
    Id: string
    Title: string
    Author: string
    Description: string
    IconUrl: string
    Downloads: number
    CategoryName: string
  }

  let cfCategory = $state('all')
  let cfSearchQuery = $state('')
  const cfSearching = false
  const cfError = ''
  const cfHasSearched = false
  const cfResults: CfAddon[] = []
  const cfTotalCount = 0
  const cfDownloadingId: number | null = null
  const cfDownloadProgress = 0

  let mrCategory = $state('mod')
  let mrSearchQuery = $state('')
  const mrSearching = false
  const mrError = ''
  const mrHasSearched = false
  const mrResults: MrProject[] = []
  const mrTotalCount = 0
  const mrDownloadingId: string | null = null
  const mrDownloadProgress = 0

  const activeVersion = $derived(settings.JavaActiveVersion || '')
  const activeInstance = $derived(instances.find((i) => i.id === activeInstanceId))

  /** Port of `FormatDownloads` (Home.razor.cs:729-734). */
  function formatDownloads(count: number): string {
    if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M downloads`
    if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K downloads`
    return `${count} downloads`
  }

  /** Port of `FormatSize`. */
  function formatSize(bytes: number): string {
    if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
    if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${bytes} B`
  }

  async function loadInstances() {
    try {
      instances = await invoke<JavaInstance[]>('list_java_instances')
      if (!activeInstanceId || !instances.some((i) => i.id === activeInstanceId)) {
        activeInstanceId =
          instances.find((i) => i.id === settings.JavaActiveInstanceId)?.id ?? instances[0]?.id ?? ''
      }
    } catch (e) {
      error = String(e)
    }
  }

  async function loadMods() {
    if (!activeInstanceId) {
      report = { mods: [], warnings: [] }
      return
    }
    error = ''
    try {
      report = await invoke<ModReport>('list_java_mods', { instanceId: activeInstanceId })
    } catch (e) {
      error = String(e)
    }
  }

  async function loadScreenshotCount() {
    if (!activeInstanceId) {
      javaScreenshotsCount = null
      return
    }
    try {
      const shots = await invoke<unknown[]>('list_java_screenshots', {
        instanceId: activeInstanceId,
      })
      javaScreenshotsCount = shots.length
    } catch {
      javaScreenshotsCount = null
    }
  }

  async function toggleJavaMod(mod: ModInfo) {
    try {
      await invoke('set_java_mod_enabled', {
        instanceId: activeInstanceId,
        fileName: mod.file_name,
        enabled: mod.disabled,
      })
      await loadMods()
    } catch (e) {
      error = String(e)
    }
  }

  async function deleteJavaFile(mod: ModInfo) {
    try {
      await invoke('delete_java_mod', { instanceId: activeInstanceId, fileName: mod.file_name })
      await loadMods()
    } catch (e) {
      error = String(e)
    }
  }

  /**
   * Port of `OpenJavaFolder(kind)` (Home.Clients.cs:572). The original had a
   * JavaInstanceService.OpenFolder(kind) helper; here the instance directory
   * comes from `list_java_instances` and the subfolder is opened with the
   * existing generic `open_folder` command (which creates it if missing,
   * matching the original's behaviour). `mods` keeps the dedicated
   * `open_java_mods_folder` command.
   */
  async function openJavaFolder(kind: string) {
    try {
      if (kind === 'mods') {
        await invoke('open_java_mods_folder', { instanceId: activeInstanceId })
        return
      }
      if (kind === 'screenshots') {
        await invoke('open_java_screenshots_folder', { instanceId: activeInstanceId })
        return
      }
      const dir = activeInstance?.directory
      if (!dir) {
        error = 'No active Java instance.'
        return
      }
      await invoke('open_folder', { path: `${dir}\\${kind}` })
    } catch (e) {
      error = String(e)
    }
  }

  async function installFabricForActive() {
    if (!activeVersion) return
    javaUtilityBusy = true
    error = ''
    try {
      await invoke('install_fabric_loader', { minecraftVersion: activeVersion, loaderVersion: null })
    } catch (e) {
      error = String(e)
    } finally {
      javaUtilityBusy = false
    }
  }

  async function installQuiltForActive() {
    if (!activeVersion) return
    javaUtilityBusy = true
    error = ''
    try {
      await invoke('install_quilt_loader', { minecraftVersion: activeVersion, loaderVersion: null })
    } catch (e) {
      error = String(e)
    } finally {
      javaUtilityBusy = false
    }
  }

  async function downloadForgeInstaller() {
    if (!activeVersion) return
    javaUtilityBusy = true
    error = ''
    try {
      const path = await invoke<string>('download_forge_installer', {
        minecraftVersion: activeVersion,
        forgeVersion: null,
      })
      alert(`Forge installer downloaded to:\n${path}\n\nRun it to complete the install.`)
    } catch (e) {
      error = String(e)
    } finally {
      javaUtilityBusy = false
    }
  }

  async function downloadNeoForgeInstaller() {
    if (!activeVersion) return
    javaUtilityBusy = true
    error = ''
    try {
      const path = await invoke<string>('download_neoforge_installer', {
        minecraftVersion: activeVersion,
      })
      alert(`NeoForge installer downloaded to:\n${path}\n\nRun it to complete the install.`)
    } catch (e) {
      error = String(e)
    } finally {
      javaUtilityBusy = false
    }
  }

  /** Port of `OnOpenDatapacksTab` (Home.Datapacks.cs:17). */
  function onOpenDatapacksTab() {
    javaModsTab = 'datapacks'
    // The original refreshed javaWorldsList / javaDatapacks here; no
    // list_java_worlds or list_java_datapacks command exists yet.
  }

  loadInstances().then(() => {
    loadMods()
    loadScreenshotCount()
  })

  $effect(() => {
    activeInstanceId
    loadMods()
    loadScreenshotCount()
  })
</script>

<PanelHeader title="Mods & Addons" {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="btn-sm"
      onclick={() => onNavigate('modpacks')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-cubes"></i> Modpacks
    </button>
  {/snippet}
</PanelHeader>

<SubTabs strip="packs-addons" active="addons" onNavigate={(key) => onNavigate(key)} />

<div class="versions-client-switcher">
  <button
    class="vcs-btn {javaModsTab === 'loaders' ? 'active' : ''}"
    onclick={() => (javaModsTab = 'loaders')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-screwdriver-wrench"></i> Loaders
  </button>
  <button
    class="vcs-btn {javaModsTab === 'mods' ? 'active' : ''}"
    onclick={() => (javaModsTab = 'mods')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-puzzle-piece"></i> Mods ({report.mods.length})
  </button>
  <button
    class="vcs-btn {javaModsTab === 'assets' ? 'active' : ''}"
    onclick={() => (javaModsTab = 'assets')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-image"></i> Assets
  </button>
  <button
    class="vcs-btn {javaModsTab === 'datapacks' ? 'active' : ''}"
    onclick={() => onOpenDatapacksTab()}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-cubes-stacked"></i> Datapacks
  </button>
  <button
    class="vcs-btn {javaModsTab === 'tools' ? 'active' : ''}"
    onclick={() => (javaModsTab = 'tools')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-toolbox"></i> Tools
  </button>
  <button
    class="vcs-btn {javaModsTab === 'curseforge' ? 'active' : ''}"
    onclick={() => (javaModsTab = 'curseforge')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-fire"></i> CurseForge
  </button>
  <button
    class="vcs-btn {javaModsTab === 'modrinth' ? 'active' : ''}"
    onclick={() => (javaModsTab = 'modrinth')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-leaf"></i> Modrinth
  </button>
</div>

{#if javaModsTab !== 'curseforge' && javaModsTab !== 'modrinth'}
  <div class="panel-body" style="padding-top:6px;">
    {#if error}<p class="error-text">{error}</p>{/if}

    {#if instances.length > 1}
      <select
        class="setting-select"
        bind:value={activeInstanceId}
        style="width:100%; margin-bottom:8px;"
      >
        {#each instances as inst (inst.id)}
          <option value={inst.id}>{inst.name}</option>
        {/each}
      </select>
    {/if}

    {#if javaModsTab === 'loaders'}
      {#if !activeVersion}
        <div class="empty-state">
          <i class="fa-solid fa-triangle-exclamation"></i>
          <span>No version selected</span>
          <small
            >Pick a Minecraft version from the Versions panel first, then come back to install a mod
            loader.</small
          >
        </div>
      {:else}
        <div
          class="version-row"
          style="border-left-width:2px; border-left-color:var(--accent); margin-bottom:4px;"
        >
          <div class="version-meta">
            <span class="version-name" style="font-size:12px; color:var(--text-dim);"
              >Installing for</span
            >
            <span class="version-sub" style="font-size:13px; font-weight:600; color:var(--text);"
              >{activeVersion}</span
            >
          </div>
        </div>

        <div class="client-card {activeVersion.includes('fabric') ? 'client-active' : ''}">
          <div class="client-card-header">
            <div
              class="client-card-icon"
              style="background:rgba(219,185,137,0.15); color:#dbb989; padding:8px;"
            >
              <i class="fa-solid fa-feather"></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name">Fabric</span>
              <span class="client-card-sub">
                {#if activeVersion.includes('fabric')}
                  <span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Active</span>
                {:else}
                  <span style="opacity:0.5;">Lightweight mod loader</span>
                {/if}
              </span>
            </div>
            <div class="client-card-actions">
              <IconBtn
                icon="fa-solid fa-download"
                title="Install Fabric"
                disabled={javaUtilityBusy}
                onClick={installFabricForActive}
              />
            </div>
          </div>
          <p class="client-card-desc">
            Fast, lightweight mod loader. One click installs the latest stable Fabric loader for your
            active version.
          </p>
        </div>

        <div class="client-card {activeVersion.includes('quilt') ? 'client-active' : ''}">
          <div class="client-card-header">
            <div
              class="client-card-icon"
              style="background:rgba(147,130,210,0.15); color:#9382d2; padding:8px;"
            >
              <i class="fa-solid fa-layer-group"></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name">Quilt</span>
              <span class="client-card-sub">
                {#if activeVersion.includes('quilt')}
                  <span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Active</span>
                {:else}
                  <span style="opacity:0.5;">Fabric-compatible fork</span>
                {/if}
              </span>
            </div>
            <div class="client-card-actions">
              <IconBtn
                icon="fa-solid fa-download"
                title="Install Quilt"
                disabled={javaUtilityBusy}
                onClick={installQuiltForActive}
              />
            </div>
          </div>
          <p class="client-card-desc">
            Community-driven fork of Fabric with enhanced APIs. Compatible with most Fabric mods out
            of the box.
          </p>
        </div>

        <div class="client-card">
          <div class="client-card-header">
            <div
              class="client-card-icon"
              style="background:rgba(66,133,244,0.15); color:#4285f4; padding:8px;"
            >
              <i class="fa-solid fa-hammer"></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name">Forge</span>
              <span class="client-card-sub"><span style="opacity:0.5;">The classic mod loader</span></span>
            </div>
            <div class="client-card-actions">
              <IconBtn
                icon="fa-solid fa-download"
                title="Download Forge installer"
                disabled={javaUtilityBusy}
                onClick={downloadForgeInstaller}
              />
            </div>
          </div>
          <p class="client-card-desc">
            Auto-detects the recommended Forge version for your active Minecraft version and
            downloads the installer.
          </p>
        </div>

        <div class="client-card">
          <div class="client-card-header">
            <div
              class="client-card-icon"
              style="background:rgba(255,120,50,0.15); color:#ff7832; padding:8px;"
            >
              <i class="fa-solid fa-fire-flame-curved"></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name">NeoForge</span>
              <span class="client-card-sub"
                ><span style="opacity:0.5;">Modern Forge fork (1.20.2+)</span></span
              >
            </div>
            <div class="client-card-actions">
              <IconBtn
                icon="fa-solid fa-download"
                title="Download NeoForge installer"
                disabled={javaUtilityBusy}
                onClick={downloadNeoForgeInstaller}
              />
            </div>
          </div>
          <p class="client-card-desc">
            Auto-detects the latest NeoForge version for your active Minecraft version and downloads
            the installer.
          </p>
        </div>
      {/if}
    {:else if javaModsTab === 'mods'}
      {#if report.warnings.length > 0}
        <div class="mod-warnings">
          <div class="mod-warnings-head">
            <i class="fa-solid fa-triangle-exclamation"></i>
            {report.warnings.length} potential issue{report.warnings.length === 1 ? '' : 's'}
          </div>
          {#each report.warnings as w}
            <div class="mod-warning-item">{w}</div>
          {/each}
        </div>
      {/if}
      {#if report.mods.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-puzzle-piece"></i>
          <span>No local mods</span>
          <small>Drop jars into the active instance mods folder, or browse CurseForge below.</small>
          <button
            class="btn-sm"
            style="margin-top:6px;"
            onclick={() => openJavaFolder('mods')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-folder-open"></i> Open mods folder
          </button>
        </div>
      {:else}
        {#each report.mods as mod (mod.file_name)}
          <div class="version-row">
            <div class="version-meta">
              <span class="version-name" style={mod.disabled ? 'opacity:.55;' : ''}>{mod.name}</span>
              <span class="version-sub"
                >{mod.disabled ? 'Disabled' : 'Enabled'}
                {mod.required_deps.length === 0 ? '' : `· Requires ${mod.required_deps.join(', ')}`}</span
              >
            </div>
            <button
              class="vcs-btn vcs-btn-ghost"
              onclick={() => toggleJavaMod(mod)}
              onmousedown={(e) => e.stopPropagation()}
              aria-label={mod.disabled ? `Enable ${mod.name}` : `Disable ${mod.name}`}
            >
              <i class="fa-solid {mod.disabled ? 'fa-toggle-off' : 'fa-toggle-on'}"></i>
            </button>
            <button
              class="vcs-btn vcs-btn-ghost"
              onclick={() => deleteJavaFile(mod)}
              onmousedown={(e) => e.stopPropagation()}
              aria-label={`Delete ${mod.name}`}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          </div>
        {/each}
        <div style="text-align:center; padding:4px;">
          <button
            class="btn-sm"
            onclick={() => openJavaFolder('mods')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-folder-open"></i> Open mods folder
          </button>
        </div>
      {/if}
    {:else if javaModsTab === 'assets'}
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon"
            style="background:rgba(67,181,129,0.15); color:var(--green); padding:8px;"
          >
            <i class="fa-solid fa-image"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">Resource Packs</span>
            <span class="client-card-sub" style="opacity:0.5;" data-tooltip={MISSING_FILE_LIST}
              >— pack(s)</span
            >
          </div>
          <div class="client-card-actions">
            <button
              class="icon-btn"
              data-tooltip="Open folder"
              aria-label="Open folder"
              onclick={() => openJavaFolder('resourcepacks')}
              onmousedown={(e) => e.stopPropagation()}><i class="fa-solid fa-folder-open"></i></button
            >
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon"
            style="background:rgba(255,200,50,0.15); color:#ffc832; padding:8px;"
          >
            <i class="fa-solid fa-wand-magic-sparkles"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">Shader Packs</span>
            <span class="client-card-sub" style="opacity:0.5;" data-tooltip={MISSING_FILE_LIST}
              >— pack(s)</span
            >
          </div>
          <div class="client-card-actions">
            <button
              class="icon-btn"
              data-tooltip="Open folder"
              aria-label="Open folder"
              onclick={() => openJavaFolder('shaderpacks')}
              onmousedown={(e) => e.stopPropagation()}><i class="fa-solid fa-folder-open"></i></button
            >
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon"
            style="background:rgba(114,137,218,0.15); color:var(--accent); padding:8px;"
          >
            <i class="fa-solid fa-map"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">Saves / Worlds</span>
            <span class="client-card-sub" style="opacity:0.5;">Open your worlds folder</span>
          </div>
          <div class="client-card-actions">
            <button
              class="icon-btn"
              data-tooltip="Open folder"
              aria-label="Open folder"
              onclick={() => openJavaFolder('saves')}
              onmousedown={(e) => e.stopPropagation()}><i class="fa-solid fa-folder-open"></i></button
            >
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon"
            style="background:rgba(240,71,71,0.15); color:var(--red); padding:8px;"
          >
            <i class="fa-solid fa-camera"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">Screenshots</span>
            <span class="client-card-sub" style="opacity:0.5;"
              >{javaScreenshotsCount ?? '—'} screenshot(s)</span
            >
          </div>
          <div class="client-card-actions">
            <button
              class="icon-btn"
              data-tooltip="Open folder"
              aria-label="Open folder"
              onclick={() => openJavaFolder('screenshots')}
              onmousedown={(e) => e.stopPropagation()}><i class="fa-solid fa-folder-open"></i></button
            >
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon"
            style="background:rgba(250,166,26,0.15); color:var(--orange); padding:8px;"
          >
            <i class="fa-solid fa-drafting-compass"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">Schematics</span>
            <span class="client-card-sub" style="opacity:0.5;" data-tooltip={MISSING_FILE_LIST}
              >— file(s)</span
            >
          </div>
          <div class="client-card-actions">
            <button
              class="icon-btn"
              data-tooltip="Open folder"
              aria-label="Open folder"
              onclick={() => openJavaFolder('schematics')}
              onmousedown={(e) => e.stopPropagation()}><i class="fa-solid fa-folder-open"></i></button
            >
          </div>
        </div>
      </div>
    {:else if javaModsTab === 'datapacks'}
      <label class="modal-input-label" for="datapacks-world">World</label>
      <select
        id="datapacks-world"
        class="setting-select"
        bind:value={datapacksWorldName}
        disabled
        data-tooltip={MISSING_WORLDS}
        onclick={(e) => e.stopPropagation()}
        style="width:100%; margin-bottom:8px;"
      >
        <option value="">Select a world…</option>
        {#each javaWorldsList as w (w.Name)}
          <option value={w.Name}>{w.Name}</option>
        {/each}
      </select>

      {#if !datapacksWorldName}
        <div class="empty-state">
          <i class="fa-solid fa-cubes-stacked"></i>
          <span>Pick a world above</span>
          <small>Datapacks are installed per-world, not shared across all your saves.</small>
        </div>
      {:else if javaDatapacks.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-cubes-stacked"></i>
          <span>No datapacks in "{datapacksWorldName}"</span>
          <button class="btn-sm" style="margin-top:6px;" disabled data-tooltip={MISSING_DATAPACKS}>
            <i class="fa-solid fa-upload"></i> Add datapack (.zip)
          </button>
        </div>
      {:else}
        {#each javaDatapacks as pack (pack.Name)}
          <div class="version-row">
            <div class="version-meta">
              <span class="version-name">{pack.Name}</span>
              <span class="version-sub">{formatSize(pack.SizeBytes)} · {pack.ModifiedAt}</span>
            </div>
            <button
              class="vcs-btn vcs-btn-ghost"
              disabled
              data-tooltip={MISSING_DATAPACKS}
              aria-label={`Delete ${pack.Name}`}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          </div>
        {/each}
        <div style="text-align:center; padding:8px;">
          <button class="btn-sm" disabled data-tooltip={MISSING_DATAPACKS}>
            <i class="fa-solid fa-upload"></i> Add datapack (.zip)
          </button>
          <button class="btn-sm" disabled data-tooltip={MISSING_DATAPACKS}>
            <i class="fa-solid fa-folder-open"></i> Open folder
          </button>
        </div>
      {/if}
    {:else if javaModsTab === 'tools'}
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon"
            style="background:rgba(114,137,218,0.15); color:var(--accent); padding:8px;"
          >
            <i class="fa-solid fa-box-archive"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">Backup Saves</span>
            <span class="client-card-sub" style="opacity:0.5;"
              >Zip your worlds before a risky update</span
            >
          </div>
          <div class="client-card-actions">
            <button class="icon-btn" disabled data-tooltip={MISSING_BACKUP} aria-label="Create backup">
              <i class="fa-solid fa-download"></i>
            </button>
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon"
            style="background:rgba(67,181,129,0.15); color:var(--green); padding:8px;"
          >
            <i class="fa-solid fa-file-zipper"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">Export Modpack</span>
            <span class="client-card-sub" style="opacity:0.5;"
              >Bundle mods + config into a shareable zip</span
            >
          </div>
          <div class="client-card-actions">
            <button class="icon-btn" disabled data-tooltip={MISSING_EXPORT} aria-label="Export">
              <i class="fa-solid fa-file-export"></i>
            </button>
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon"
            style="background:rgba(250,166,26,0.15); color:var(--orange); padding:8px;"
          >
            <i class="fa-solid fa-clone"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">Duplicate Instance</span>
            <span class="client-card-sub" style="opacity:0.5;"
              >Clone your active instance with all mods &amp; config</span
            >
          </div>
          <div class="client-card-actions">
            <button class="icon-btn" disabled data-tooltip={MISSING_DUPLICATE} aria-label="Duplicate">
              <i class="fa-solid fa-copy"></i>
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}

{#if javaModsTab === 'curseforge'}
  <div class="panel-body">
    <div class="versions-client-switcher">
      {#each CF_CATEGORIES as cat (cat.key)}
        <button
          class="vcs-btn {cfCategory === cat.key ? 'active' : ''}"
          onclick={() => (cfCategory = cat.key)}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class={cat.icon}></i> {cat.label}
        </button>
      {/each}
    </div>

    <div class="panel-search-wrap">
      <i class="fa-solid fa-magnifying-glass"></i>
      <input
        class="panel-search-input"
        placeholder="Search CurseForge for Java mods, modpacks, shaders..."
        bind:value={cfSearchQuery}
        disabled
        data-tooltip={MISSING_CF_SEARCH}
        onkeydown={(e) => e.stopPropagation()}
      />
    </div>

    {#if cfSearching}
      <div style="display:flex; align-items:center; justify-content:center; padding:30px;">
        <span class="spinner" style="width:18px;height:18px;border-width:2px;"></span>
        <span style="margin-left:10px; font-size:12px; color:var(--text-dim);"
          >Searching CurseForge...</span
        >
      </div>
    {:else if cfError}
      <div style="padding:20px; text-align:center;">
        <span class="error-text">{cfError}</span>
        <br />
        <button class="btn-sm" style="margin-top:10px;" disabled data-tooltip={MISSING_CF_SEARCH}>
          <i class="fa-solid fa-rotate"></i> Retry
        </button>
      </div>
    {:else if cfResults.length === 0 && cfHasSearched}
      <div class="empty-state">
        <i class="fa-solid fa-box-open"></i>
        <span>No addons found</span>
        <small>Try a different search term or category.</small>
      </div>
    {:else if cfResults.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-magnifying-glass"></i>
        <span>Browse CurseForge</span>
        <small>Search for Bedrock addons, resource packs, worlds, or skins above.</small>
      </div>
    {:else}
      {#each cfResults as a (a.Id)}
        <div class="client-card" style="margin-bottom:6px;">
          <div class="client-card-header">
            <div class="client-card-icon" style="padding:3px; border-radius:6px; overflow:hidden;">
              {#if a.ThumbnailUrl}
                <img
                  src={a.ThumbnailUrl}
                  style="width:100%;height:100%;object-fit:cover;border-radius:4px;"
                  alt=""
                />
              {:else}
                <i class="fa-solid fa-cube" style="font-size:18px; color:var(--accent);"></i>
              {/if}
            </div>
            <div class="client-card-meta" style="min-width:0;">
              <span
                class="client-card-name"
                style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">{a.Name}</span
              >
              <span class="client-card-sub" style="display:flex; gap:8px; align-items:center;">
                <span style="opacity:0.6;">{a.AuthorName}</span>
                <span style="opacity:0.4;">{formatDownloads(a.DownloadCount)}</span>
                {#if a.CategoryName}
                  <span class="tag-uptodate" style="font-size:9px; padding:1px 5px;"
                    >{a.CategoryName}</span
                  >
                {/if}
              </span>
            </div>
            <div class="client-card-actions">
              {#if cfDownloadingId === a.Id}
                <InlineProgress progress={cfDownloadProgress} />
              {:else}
                <button
                  class="icon-btn"
                  disabled
                  data-tooltip={MISSING_CF_INSTALL}
                  aria-label="Download & Install"
                >
                  <i class="fa-solid fa-download"></i>
                </button>
              {/if}
            </div>
          </div>
          <p
            class="client-card-desc"
            style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;"
          >
            {a.Summary}
          </p>
        </div>
      {/each}

      {#if cfResults.length < cfTotalCount}
        <div style="text-align:center; padding:8px;">
          <button class="btn-sm" disabled data-tooltip={MISSING_CF_SEARCH}>
            <i class="fa-solid fa-angles-down"></i> Load more ({cfResults.length} / {cfTotalCount})
          </button>
        </div>
      {/if}
    {/if}
  </div>
{/if}

{#if javaModsTab === 'modrinth'}
  <div class="panel-body">
    <div class="versions-client-switcher">
      {#each MR_CATEGORIES as cat (cat.key)}
        <button
          class="vcs-btn {mrCategory === cat.key ? 'active' : ''}"
          onclick={() => (mrCategory = cat.key)}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class={cat.icon}></i> {cat.label}
        </button>
      {/each}
    </div>

    <div class="panel-search-wrap">
      <i class="fa-solid fa-magnifying-glass"></i>
      <input
        class="panel-search-input"
        placeholder="Search Modrinth for mods, modpacks, shaders..."
        bind:value={mrSearchQuery}
        disabled
        data-tooltip={MISSING_MR_SEARCH}
        onkeydown={(e) => e.stopPropagation()}
      />
    </div>

    {#if mrSearching}
      <div style="display:flex; align-items:center; justify-content:center; padding:30px;">
        <span class="spinner" style="width:18px;height:18px;border-width:2px;"></span>
        <span style="margin-left:10px; font-size:12px; color:var(--text-dim);"
          >Searching Modrinth...</span
        >
      </div>
    {:else if mrError}
      <div style="padding:20px; text-align:center;">
        <span class="error-text">{mrError}</span>
        <br />
        <button class="btn-sm" style="margin-top:10px;" disabled data-tooltip={MISSING_MR_SEARCH}>
          <i class="fa-solid fa-rotate"></i> Retry
        </button>
      </div>
    {:else if mrResults.length === 0 && mrHasSearched}
      <div class="empty-state">
        <i class="fa-solid fa-box-open"></i>
        <span>No results found</span>
        <small>Try a different search term or category.</small>
      </div>
    {:else if mrResults.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-leaf"></i>
        <span>Browse Modrinth</span>
        <small>Search for mods, modpacks, resource packs, or shaders above.</small>
      </div>
    {:else}
      {#each mrResults as p (p.Id)}
        <div class="client-card" style="margin-bottom:6px;">
          <div class="client-card-header">
            <div class="client-card-icon" style="padding:3px; border-radius:6px; overflow:hidden;">
              {#if p.IconUrl}
                <img
                  src={p.IconUrl}
                  style="width:100%;height:100%;object-fit:cover;border-radius:4px;"
                  alt=""
                />
              {:else}
                <i class="fa-solid fa-leaf" style="font-size:18px; color:#1bd96a;"></i>
              {/if}
            </div>
            <div class="client-card-meta" style="min-width:0;">
              <span
                class="client-card-name"
                style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">{p.Title}</span
              >
              <span class="client-card-sub" style="display:flex; gap:8px; align-items:center;">
                <span style="opacity:0.6;">{p.Author}</span>
                <span style="opacity:0.4;">{formatDownloads(p.Downloads)}</span>
                {#if p.CategoryName}
                  <span class="tag-uptodate" style="font-size:9px; padding:1px 5px;"
                    >{p.CategoryName}</span
                  >
                {/if}
              </span>
            </div>
            <div class="client-card-actions">
              {#if mrDownloadingId === p.Id}
                <InlineProgress progress={mrDownloadProgress} />
              {:else}
                <button
                  class="icon-btn"
                  disabled
                  data-tooltip={MISSING_MR_INSTALL}
                  aria-label="Download & Install"
                >
                  <i class="fa-solid fa-download"></i>
                </button>
              {/if}
            </div>
          </div>
          <p
            class="client-card-desc"
            style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;"
          >
            {p.Description}
          </p>
        </div>
      {/each}

      {#if mrResults.length < mrTotalCount}
        <div style="text-align:center; padding:8px;">
          <button class="btn-sm" disabled data-tooltip={MISSING_MR_SEARCH}>
            <i class="fa-solid fa-angles-down"></i> Load more ({mrResults.length} / {mrTotalCount})
          </button>
        </div>
      {/if}
    {/if}
  </div>
{/if}

<EditionTabs edition="java" active="mods" onNavigate={(view) => onNavigate(view)} />
