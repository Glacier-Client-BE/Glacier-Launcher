<script lang="ts">
  import { t } from './i18nStore.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { open, save } from '@tauri-apps/plugin-dialog'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import IconBtn from './components/IconBtn.svelte'
  import Spinner from './components/Spinner.svelte'
  import ViewModeToggle from './components/ViewModeToggle.svelte'
  import Modal from './components/Modal.svelte'
  import { loadViewMode, saveViewMode, type ViewMode } from './viewMode'
  import type { LauncherSettings, JavaInstance } from './settings'
  import ListSkeleton from './components/ListSkeleton.svelte'

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
    onToast,
  }: {
    settings: LauncherSettings
    onBack?: () => void
    onNavigate?: (view: string, strip?: string) => void
    onToast?: (message: string, kind?: string) => void
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
  const MISSING_BACKUP = 'Unavailable: needs a backup_java_saves command'
  const MISSING_EXPORT = 'Unavailable: needs an export_java_modpack command'
  const MISSING_FILE_LIST_SCHEMATICS =
    'Unavailable: needs a list_java_instance_files command (schematics)'
  // search_modrinth_projects / install_modrinth_mod now exist
  // (src-tauri/src/commands/java_mods.rs); the Modrinth tab below is wired up
  // for real. CurseForge mod search/install is now wired up for real too, via
  // the unified commands::java_packs::{search_mods,install_curseforge_mod}
  // (Phase 3) — it needs a CurseForge API key in Settings, same as the
  // CurseForge modpack browser.

  let javaModsTab = $state<JavaModsTab>('loaders')
  let javaModsViewMode = $state<ViewMode>(loadViewMode('java-mods'))
  function setJavaModsViewMode(m: ViewMode) {
    javaModsViewMode = m
    saveViewMode('java-mods', m)
  }
  let instances = $state<JavaInstance[]>([])
  let activeInstanceId = $state('')
  let report = $state<ModReport>({ mods: [], warnings: [] })
  let error = $state('')
  let javaUtilityBusy = $state(false)
  let installingLoader = $state<string | null>(null)
  // UI-only loading flags for skeleton states (no data-flow change).
  let modsLoading = $state(false)
  let packsLoading = $state(false)
  let datapacksLoading = $state(false)
  let bulkBusy = $state(false)

  // Assets tab counts. Only screenshots have a backing command
  // (list_java_screenshots); the rest stay null and render as "-".
  let javaScreenshotsCount = $state<number | null>(null)

  // Datapacks tab state (Home.Datapacks.cs), backed by
  // list_java_worlds / list_java_datapacks / import_java_datapack /
  // delete_java_datapack / open_java_datapacks_folder.
  type JavaWorld = { name: string; path: string; size: number; modified_at: string }
  type JavaInstanceFile = {
    name: string
    path: string
    kind: string
    size_bytes: number
    modified_at: string
    is_disabled: boolean
  }
  let datapacksWorldName = $state('')
  let javaWorldsList = $state<JavaWorld[]>([])
  let javaDatapacks = $state<JavaInstanceFile[]>([])
  let datapacksBusy = $state(false)

  async function loadJavaWorldsForDatapacks() {
    try {
      javaWorldsList = await invoke<JavaWorld[]>('list_java_worlds')
    } catch (e) {
      onToast?.(t('toast.failedToListWorlds', { error: String(e) }), 'error')
    }
  }

  async function loadJavaDatapacks() {
    datapacksLoading = true
    try {
      await loadJavaDatapacksInner()
    } finally {
      datapacksLoading = false
    }
  }

  async function loadJavaDatapacksInner() {
    if (!datapacksWorldName) {
      javaDatapacks = []
      return
    }
    try {
      javaDatapacks = await invoke<JavaInstanceFile[]>('list_java_datapacks', {
        worldName: datapacksWorldName,
      })
    } catch (e) {
      onToast?.(t('toast.failedToListDatapacks', { error: String(e) }), 'error')
    }
  }

  $effect(() => {
    void datapacksWorldName
    loadJavaDatapacks()
  })

  async function addJavaDatapack() {
    if (!datapacksWorldName) return
    const path = await open({
      multiple: false,
      title: 'Add datapack',
      filters: [{ name: 'Datapack (zip)', extensions: ['zip'] }],
    })
    if (typeof path !== 'string') return
    datapacksBusy = true
    try {
      await invoke('import_java_datapack', { worldName: datapacksWorldName, zipPath: path })
      await loadJavaDatapacks()
      onToast?.(t('toast.datapackInstalled'), 'success')
    } catch (e) {
      onToast?.(t('toast.failedToAddDatapack', { error: String(e) }), 'error')
    } finally {
      datapacksBusy = false
    }
  }

  async function deleteJavaDatapack(pack: JavaInstanceFile) {
    datapacksBusy = true
    try {
      await invoke('delete_java_datapack', { path: pack.path })
      await loadJavaDatapacks()
    } catch (e) {
      onToast?.(t('toast.failedToDeleteDatapack', { error: String(e) }), 'error')
    } finally {
      datapacksBusy = false
    }
  }

  async function openJavaDatapacksFolder() {
    if (!datapacksWorldName) return
    try {
      await invoke('open_java_datapacks_folder', { worldName: datapacksWorldName })
    } catch (e) {
      onToast?.(t('toast.failedToOpenFolder', { error: String(e) }), 'error')
    }
  }

  let duplicateBusy = $state(false)
  async function duplicateActiveInstance() {
    if (!activeInstanceId) return
    duplicateBusy = true
    try {
      await invoke('duplicate_java_instance', { id: activeInstanceId })
      onToast?.(t('toast.instanceDuplicated'), 'success')
    } catch (e) {
      onToast?.(t('toast.failedToDuplicateInstance', { error: String(e) }), 'error')
    } finally {
      duplicateBusy = false
    }
  }

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
  // Matches gl_modpack::models::MrProject field-for-field (plain serde, no
  // rename_all — snake_case on the wire).
  type MrProject = {
    id: string
    slug: string
    title: string
    author: string
    description: string
    icon_url: string
    downloads: number
    project_type: string
    category_name: string | null
  }
  type MrSearchResultPayload = { projects: MrProject[]; total_count: number }

  type MrDependencyWarning = { project_id: string; title: string; installed: boolean }
  type InstallModrinthModResult = { file_name: string; dependency_warnings: MrDependencyWarning[] }

  // Matches gl_modpack::models::MrVersion.
  type MrVersion = {
    id: string
    name: string
    file_name: string
    url: string
    size: number
    sha1: string
    changelog: string | null
    dependencies: { project_id: string | null; version_id: string | null; dependency_type: string }[]
  }
  type ModUpdateInfo = { file_name: string; mod_id: string; name: string; available: MrVersion }

  // `unified::search_mods` (Phase 3) only searches the "mods" CurseForge
  // class today — the category switcher above the search box is kept for
  // layout parity but only "all"/"mods" actually issue a request; picking
  // another category shows a note rather than a fake empty result.
  let cfCategory = $state('all')
  let cfSearchQuery = $state('')
  let cfSearching = $state(false)
  let cfError = $state('')
  let cfHasSearched = $state(false)
  let cfResults = $state<UnifiedMod[]>([])
  let cfDownloadingId = $state<string | null>(null)
  const cfDownloadProgress = 0

  type UnifiedMod = {
    id: string
    provider: 'modrinth' | 'curseforge'
    name: string
    summary: string
    icon_url: string
    author: string
    downloads: number
    versions: { version_id: string; name: string; file_name: string; download_url: string; size: number; sha1: string }[]
  }

  async function searchCurseForgeMods(loadMore = false) {
    if (!activeInstanceId) {
      cfError = 'Select a Java instance first.'
      return
    }
    if (!settings.CurseForgeApiKey?.trim()) {
      cfError = 'Set a CurseForge API key in Settings first.'
      return
    }
    cfSearching = true
    cfError = ''
    cfHasSearched = true
    const offset = loadMore ? cfResults.length : 0
    try {
      const res = await invoke<UnifiedMod[]>('search_mods', {
        provider: 'curseforge',
        query: cfSearchQuery,
        curseforgeApiKey: settings.CurseForgeApiKey,
        offset,
      })
      cfResults = loadMore ? [...cfResults, ...res] : res
    } catch (e) {
      cfError = String(e)
    } finally {
      cfSearching = false
    }
  }

  async function installCurseForgeMod(mod: UnifiedMod) {
    if (!activeInstanceId || cfDownloadingId) return
    cfDownloadingId = mod.id
    try {
      const versions =
        mod.versions.length > 0
          ? mod.versions
          : (
              await invoke<{ versions: UnifiedMod['versions'] }>('get_mod_versions', {
                provider: 'curseforge',
                projectId: mod.id,
                curseforgeApiKey: settings.CurseForgeApiKey,
              })
            ).versions
      const latest = versions[0]
      if (!latest) {
        onToast?.(t('toast.noDownloadableFile', { name: mod.name }), 'error')
        return
      }
      await invoke('install_curseforge_mod', {
        instanceId: activeInstanceId,
        curseforgeApiKey: settings.CurseForgeApiKey,
        modId: Number(mod.id),
        fileId: Number(latest.version_id),
      })
      onToast?.(t('toast.installedName', { name: mod.name }), 'success')
await loadMods()
    } catch (e) {
      onToast?.(t('toast.failedToInstallName', { name: mod.name, error: String(e) }), 'error')
    } finally {
      cfDownloadingId = null
    }
  }

  let mrCategory = $state('mod')
  let mrSearchQuery = $state('')
  let mrSearching = $state(false)
  let mrError = $state('')
  let mrHasSearched = $state(false)
  let mrResults = $state<MrProject[]>([])
  let mrTotalCount = $state(0)
  let mrDownloadingId = $state<string | null>(null)
  const mrDownloadProgress = 0

  async function searchModrinth(loadMore = false) {
    if (!activeInstanceId) {
      mrError = 'Select a Java instance first.'
      return
    }
    mrSearching = true
    mrError = ''
    mrHasSearched = true
    const offset = loadMore ? mrResults.length : 0
    try {
      const res = await invoke<MrSearchResultPayload>('search_modrinth_projects', {
        query: mrSearchQuery,
        projectType: mrCategory,
        offset,
      })
      mrResults = loadMore ? [...mrResults, ...res.projects] : res.projects
      mrTotalCount = res.total_count
    } catch (e) {
      if (!loadMore) mrResults = []
      mrError = String(e)
    } finally {
      mrSearching = false
    }
  }

  function selectMrCategory(key: string) {
    mrCategory = key
    void searchModrinth()
  }

  // Feature 17: starred/favorited mod project ids, persisted in settings.
  const starredModIds = $derived(new Set(settings.StarredModIds ?? []))
  function isStarred(id: string): boolean {
    return starredModIds.has(id)
  }
  async function toggleStar(project: MrProject) {
    const current = new Set(settings.StarredModIds ?? [])
    if (current.has(project.id)) current.delete(project.id)
    else current.add(project.id)
    try {
      await invoke('set_settings', { settings: { ...settings, StarredModIds: [...current] } })
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  // Feature 12: dependency-resolution warnings surfaced after an install.
  let lastInstallWarnings = $state<{ projectTitle: string; deps: MrDependencyWarning[] } | null>(
    null,
  )

  // ── Dependency-confirm modal (Phase 3: gl_modpack::dependencies) ─────────
  type PlannedFile = {
    provider: 'modrinth' | 'curseforge'
    project_id: string
    version_id: string
    file_url: string
    filename: string
    sha1: string
    is_root: boolean
  }
  type InstallPlan = { files: PlannedFile[]; conflicts: string[] }

  let depModalOpen = $state(false)
  let depPlan = $state<InstallPlan | null>(null)
  let depProject = $state<MrProject | null>(null)
  let depBusy = $state(false)
  let depError = $state('')

  async function openDependencyModal(project: MrProject) {
    if (!activeInstanceId) return
    depProject = project
    depPlan = null
    depError = ''
    depModalOpen = true
    try {
      depPlan = await invoke<InstallPlan>('resolve_mod_dependencies', {
        provider: 'modrinth',
        projectId: project.id,
        versionId: null,
      })
    } catch (e) {
      depError = String(e)
    }
  }

  async function confirmDependencyInstall() {
    if (!activeInstanceId || !depPlan || !depProject) return
    depBusy = true
    try {
      await invoke('install_resolved_mods', { instanceId: activeInstanceId, plan: depPlan })
      onToast?.(
        `Installed '${depProject.title}'${depPlan.files.length > 1 ? ` + ${depPlan.files.length - 1} dependenc${depPlan.files.length - 1 === 1 ? 'y' : 'ies'}` : ''}.`,
        'success',
      )
      depModalOpen = false
      await loadMods()
    } catch (e) {
      depError = String(e)
    } finally {
      depBusy = false
    }
  }

  async function installModrinthMod(project: MrProject) {
    if (!activeInstanceId || mrDownloadingId) return
    mrDownloadingId = project.id
    lastInstallWarnings = null
    try {
      const result = await invoke<InstallModrinthModResult>('install_modrinth_mod', {
        instanceId: activeInstanceId,
        projectId: project.id,
      })
      const missing = result.dependency_warnings.filter((d) => !d.installed)
      if (missing.length > 0) {
        lastInstallWarnings = { projectTitle: project.title, deps: missing }
        onToast?.(
          `Installed '${project.title}', but it needs ${missing.length} mod${missing.length === 1 ? '' : 's'} you don't have.`,
          'info',
        )
      } else {
        onToast?.(t('toast.installedName', { name: project.title }), 'success')
      }
      await loadMods()
    } catch (e) {
      onToast?.(t('toast.failedToInstallName', { name: project.title, error: String(e) }), 'error')
    } finally {
      mrDownloadingId = null
    }
  }

  // ── Update checker + changelog viewer (features 11 & 16) ─────────────────
  let modUpdates = $state<ModUpdateInfo[]>([])
  let checkingUpdates = $state(false)
  let updateCheckError = $state('')
  let updatingFile = $state<string | null>(null)
  let updatingAll = $state(false)
  let changelogFor = $state<ModUpdateInfo | null>(null)

  function loaderGuessFor(instance: JavaInstance | undefined): string[] {
    // No dedicated loader field on JavaInstance yet; version_id conventionally
    // embeds it (e.g. "1.20.1-fabric-0.15.7") the same way the loader-tab
    // "Active" badges above key off `activeVersion.includes('fabric')`.
    const v = (instance?.version_id ?? '').toLowerCase()
    for (const loader of ['fabric', 'quilt', 'forge', 'neoforge']) {
      if (v.includes(loader)) return [loader]
    }
    return []
  }

  function mcVersionGuessFor(instance: JavaInstance | undefined): string[] {
    const v = instance?.version_id ?? ''
    const match = v.match(/\d+\.\d+(\.\d+)?/)
    return match ? [match[0]] : []
  }

  /** QOL: export the active instance's mod list as a shareable text file
   *  (name, mod id, loader, enabled state per line) - handy for posting a
   *  modlist for support/comparison without screenshotting the panel. */
  async function exportModList() {
    if (report.mods.length === 0) return
    const lines = report.mods.map(
      (m) => `${m.disabled ? '[disabled] ' : ''}${m.name} (${m.mod_id}) — ${m.loader} — ${m.file_name}`,
    )
    const text = `Mod list — ${report.mods.length} mod${report.mods.length === 1 ? '' : 's'}\n\n${lines.join('\n')}\n`
    try {
      const path = await save({
        defaultPath: 'mod-list.txt',
        filters: [{ name: 'Text', extensions: ['txt'] }],
      })
      if (!path) return
      await invoke('write_text_file', { path, contents: text })
      onToast?.(t('toast.modListExported'), 'success')
    } catch (e) {
      onToast?.(t('toast.exportFailed', { error: String(e) }), 'error')
    }
  }

  async function checkForModUpdates() {
    if (!activeInstanceId) return
    checkingUpdates = true
    updateCheckError = ''
    modUpdates = []
    try {
      modUpdates = await invoke<ModUpdateInfo[]>('check_java_mod_updates', {
        instanceId: activeInstanceId,
        loaders: loaderGuessFor(activeInstance),
        gameVersions: mcVersionGuessFor(activeInstance),
      })
    } catch (e) {
      updateCheckError = String(e)
    } finally {
      checkingUpdates = false
    }
  }

  async function applyModUpdate(update: ModUpdateInfo) {
    if (updatingFile || updatingAll) return
    updatingFile = update.file_name
    try {
      await invoke('update_java_mod', {
        instanceId: activeInstanceId,
        oldFileName: update.file_name,
        version: update.available,
      })
      modUpdates = modUpdates.filter((u) => u.file_name !== update.file_name)
      if (changelogFor?.file_name === update.file_name) changelogFor = null
      onToast?.(t('toast.updatedName', { name: update.name }), 'success')
      await loadMods()
    } catch (e) {
      onToast?.(t('toast.failedToUpdateName', { name: update.name, error: String(e) }), 'error')
    } finally {
      updatingFile = null
    }
  }

  async function applyAllModUpdates() {
    if (modUpdates.length === 0 || updatingAll || updatingFile) return
    updatingAll = true
    try {
      await invoke('update_all_java_mods', {
        instanceId: activeInstanceId,
        updates: modUpdates.map((u) => ({ oldFileName: u.file_name, version: u.available })),
      })
      onToast?.(t('toast.updatedCountMods', { count: modUpdates.length }), 'success')
      modUpdates = []
      changelogFor = null
      await loadMods()
    } catch (e) {
      onToast?.(t('toast.failedToUpdateAll', { error: String(e) }), 'error')
    } finally {
      updatingAll = false
    }
  }

  // ── Disable-all / bisect mode (feature 14) ────────────────────────────────
  let bisectBusy = $state(false)
  const anyModEnabled = $derived(report.mods.some((m) => !m.disabled))

  async function toggleAllMods() {
    if (!activeInstanceId || bisectBusy) return
    const enable = !anyModEnabled
    bisectBusy = true
    try {
      const count = await invoke<number>('set_all_java_mods_enabled', {
        instanceId: activeInstanceId,
        enabled: enable,
      })
      onToast?.(enable ? t('toast.enabledCountMods', { count }) : t('toast.disabledCountMods', { count }), 'success')
      await loadMods()
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      bisectBusy = false
    }
  }

  // ── Modpack diffing (feature 13) ──────────────────────────────────────────
  type InstanceModDiff = {
    only_a: ModInfo[]
    only_b: ModInfo[]
    different_version: { a: ModInfo; b: ModInfo }[]
    same: ModInfo[]
  }
  let showDiff = $state(false)
  let diffOtherInstanceId = $state('')
  let diffResult = $state<InstanceModDiff | null>(null)
  let diffBusy = $state(false)
  let diffError = $state('')

  async function runDiff() {
    if (!activeInstanceId || !diffOtherInstanceId) return
    diffBusy = true
    diffError = ''
    try {
      diffResult = await invoke<InstanceModDiff>('diff_java_instances', {
        instanceA: activeInstanceId,
        instanceB: diffOtherInstanceId,
      })
    } catch (e) {
      diffError = String(e)
    } finally {
      diffBusy = false
    }
  }

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
    modsLoading = true
    try {
      await loadModsInner()
    } finally {
      modsLoading = false
    }
  }

  async function loadModsInner() {
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

  // ── Resource packs / shader packs (Phase 3: gl_java::packs) ──────────────
  type JavaPackFile = {
    name: string
    path: string
    kind: string
    size_bytes: number
    modified_at: string
    is_disabled: boolean
  }
  let resourcePacks = $state<JavaPackFile[]>([])
  let shaderPacks = $state<JavaPackFile[]>([])

  async function loadPacks() {
    packsLoading = true
    try {
      await loadPacksInner()
    } finally {
      packsLoading = false
    }
  }

  async function loadPacksInner() {
    if (!activeInstanceId) {
      resourcePacks = []
      shaderPacks = []
      return
    }
    try {
      resourcePacks = await invoke<JavaPackFile[]>('list_java_resourcepacks', {
        instanceId: activeInstanceId,
      })
    } catch {
      resourcePacks = []
    }
    try {
      shaderPacks = await invoke<JavaPackFile[]>('list_java_shaderpacks', {
        instanceId: activeInstanceId,
      })
    } catch {
      shaderPacks = []
    }
  }

  async function addPack(kind: 'resourcepack' | 'shaderpack') {
    if (!activeInstanceId) return
    const selected = await open({ multiple: false, directory: false })
    if (!selected || typeof selected !== 'string') return
    try {
      await invoke(kind === 'resourcepack' ? 'import_java_resourcepack' : 'import_java_shaderpack', {
        instanceId: activeInstanceId,
        sourcePath: selected,
      })
      await loadPacks()
      onToast?.(t('toast.packImported'), 'success')
    } catch (e) {
      onToast?.(t('toast.failedToImportPack', { error: String(e) }), 'error')
    }
  }

  async function deletePack(kind: 'resourcepack' | 'shaderpack', pack: JavaPackFile) {
    try {
      await invoke(kind === 'resourcepack' ? 'delete_java_resourcepack' : 'delete_java_shaderpack', {
        path: pack.path,
      })
      await loadPacks()
    } catch (e) {
      onToast?.(t('toast.failedToDeletePack', { error: String(e) }), 'error')
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
      onToast?.(mod.disabled ? t('toast.fileEnabled', { file: mod.file_name }) : t('toast.fileDisabled', { file: mod.file_name }), 'success')
    } catch (e) {
      error = String(e)
      onToast?.(String(e), 'error')
    }
  }

  async function deleteJavaFile(mod: ModInfo) {
    try {
      await invoke('delete_java_mod', { instanceId: activeInstanceId, fileName: mod.file_name })
      await loadMods()
      onToast?.(t('toast.fileDeleted', { file: mod.file_name }), 'success')
    } catch (e) {
      error = String(e)
      onToast?.(String(e), 'error')
    }
  }

  // QOL: bulk-select mods for enable/disable/delete instead of one row at a
  // time — useful for e.g. disabling a whole batch while bisecting a crash.
  let modSelectMode = $state(false)
  let selectedModFiles = $state<Set<string>>(new Set())

  function toggleModSelectMode() {
    modSelectMode = !modSelectMode
    selectedModFiles = new Set()
  }
  function toggleModSelected(fileName: string) {
    const next = new Set(selectedModFiles)
    if (next.has(fileName)) next.delete(fileName)
    else next.add(fileName)
    selectedModFiles = next
  }
  const selectedMods = $derived(report.mods.filter((m) => selectedModFiles.has(m.file_name)))

  async function bulkSetEnabled(enabled: boolean) {
    if (bulkBusy) return
    bulkBusy = true
    try {
      await bulkSetEnabledInner(enabled)
    } finally {
      bulkBusy = false
    }
  }

  async function bulkSetEnabledInner(enabled: boolean) {
    for (const mod of selectedMods) {
      try {
        await invoke('set_java_mod_enabled', { instanceId: activeInstanceId, fileName: mod.file_name, enabled })
      } catch {
        // Best-effort — keep going through the rest of the selection.
      }
    }
    await loadMods()
    onToast?.(enabled ? t('toast.enabledCountMods', { count: selectedMods.length }) : t('toast.disabledCountMods', { count: selectedMods.length }), 'success')
  }

  async function bulkDeleteMods() {
    if (bulkBusy) return
    bulkBusy = true
    try {
      await bulkDeleteModsInner()
    } finally {
      bulkBusy = false
    }
  }

  async function bulkDeleteModsInner() {
    const files = selectedMods.map((m) => m.file_name)
    for (const fileName of files) {
      try {
        await invoke('delete_java_mod', { instanceId: activeInstanceId, fileName })
      } catch {
        // Best-effort — keep going through the rest of the selection.
      }
    }
    selectedModFiles = new Set()
    modSelectMode = false
    await loadMods()
    onToast?.(t('toast.deletedCountMods', { count: files.length }), 'success')
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

  /** Live status text pushed by the backend while `install_loader` runs a
   * Forge/NeoForge installer's processors (`loader-install-progress` event) -
   * shown via `InlineProgress` since a real processor run can take a while
   * (JRE download + many small jars) with no single percentage to report. */
  let loaderInstallStatus = $state('')

  async function installLoaderForActive(loader: 'fabric' | 'quilt' | 'forge' | 'neoforge') {
    if (!activeVersion || !activeInstanceId || javaUtilityBusy) return
    javaUtilityBusy = true
    installingLoader = loader
    loaderInstallStatus = loader === 'forge' || loader === 'neoforge' ? 'Starting install...' : ''
    error = ''
    try {
      await invoke('install_loader', {
        instanceId: activeInstanceId,
        loader,
        version: null,
      })
    } catch (e) {
      error = String(e)
    } finally {
      javaUtilityBusy = false
      installingLoader = null
      loaderInstallStatus = ''
    }
  }

  async function uninstallLoaderForActive() {
    if (!activeInstanceId) return
    javaUtilityBusy = true
    error = ''
    try {
      await invoke('uninstall_loader', { instanceId: activeInstanceId })
    } catch (e) {
      error = String(e)
    } finally {
      javaUtilityBusy = false
    }
  }

  $effect(() => {
    let unlisten: (() => void) | undefined
    let disposed = false
    ;(async () => {
      const { listen } = await import('@tauri-apps/api/event')
      const fn = await listen<{ instanceId: string; message: string }>(
        'loader-install-progress',
        (evt) => {
          if (evt.payload.instanceId === activeInstanceId) {
            loaderInstallStatus = evt.payload.message
          }
        },
      )
      // Effect torn down while the listener was still registering: drop it now
      // instead of leaking a handler that outlives this run.
      if (disposed) fn()
      else unlisten = fn
    })()
    return () => {
      disposed = true
      unlisten?.()
    }
  })

  /** Port of `OnOpenDatapacksTab` (Home.Datapacks.cs:17). */
  function onOpenDatapacksTab() {
    javaModsTab = 'datapacks'
    loadJavaWorldsForDatapacks()
    // The original refreshed javaWorldsList / javaDatapacks here; no
    // list_java_worlds or list_java_datapacks command exists yet.
  }

  loadInstances().then(() => {
    loadMods()
    loadScreenshotCount()
    loadPacks()
  })

  $effect(() => {
    activeInstanceId
    loadMods()
    loadScreenshotCount()
    loadPacks()
  })
</script>

{#snippet errorRow(message: string, retry?: () => void)}
  <div class="jm-error-row" role="alert">
    <i class="fa-solid fa-circle-exclamation"></i>
    <span class="jm-error-msg">{message}</span>
    {#if retry}
      <button type="button" class="btn-sm" onclick={retry} onmousedown={(e) => e.stopPropagation()}>
        <i class="fa-solid fa-rotate"></i> {t('common.retry')}
      </button>
    {/if}
  </div>
{/snippet}

<PanelHeader title={t('panel.modsAndAddons')} {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="btn-sm"
      onclick={() => onNavigate('modpacks')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-cubes"></i> Modpacks
    </button>
    <ViewModeToggle mode={javaModsViewMode} onChange={setJavaModsViewMode} />
  {/snippet}
</PanelHeader>

<SubTabs strip="packs-addons" active="addons" edition="java" onNavigate={(k, s) => onNavigate?.(k, s)} />

<!--
  IA change: "Mods & Addons" is the single biggest panel in the app (7
  sub-sections), and a wrapping horizontal tab strip meant the active
  section's content started at a different vertical position depending on
  how many rows the tabs themselves wrapped to. Master-detail — a fixed-width
  vertical rail (same pattern as SettingsView's category rail) + a content
  pane that always starts at the same place — needs the panel's wide mode to
  have room for a rail without squeezing the content (see
  WIDE_DASHBOARD_VIEWS in App.svelte).
-->
<div class="settings-layout">
  <nav class="settings-nav-rail" aria-label={t('tt.modsAddonsSections')}>
    <button
      type="button"
      class="settings-nav-btn {javaModsTab === 'loaders' ? 'active' : ''}"
      onclick={() => (javaModsTab = 'loaders')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-screwdriver-wrench"></i> Loaders
    </button>
    <button
      type="button"
      class="settings-nav-btn {javaModsTab === 'mods' ? 'active' : ''}"
      onclick={() => (javaModsTab = 'mods')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-puzzle-piece"></i> Mods
      <span class="settings-nav-btn-count">{report.mods.length}</span>
    </button>
    <button
      type="button"
      class="settings-nav-btn {javaModsTab === 'assets' ? 'active' : ''}"
      onclick={() => (javaModsTab = 'assets')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-image"></i> Assets
    </button>
    <button
      type="button"
      class="settings-nav-btn {javaModsTab === 'datapacks' ? 'active' : ''}"
      onclick={() => onOpenDatapacksTab()}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-cubes-stacked"></i> Datapacks
    </button>
    <button
      type="button"
      class="settings-nav-btn {javaModsTab === 'tools' ? 'active' : ''}"
      onclick={() => (javaModsTab = 'tools')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-toolbox"></i> Tools
    </button>
    <button
      type="button"
      class="settings-nav-btn {javaModsTab === 'curseforge' ? 'active' : ''}"
      onclick={() => (javaModsTab = 'curseforge')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-fire"></i> CurseForge
    </button>
    <button
      type="button"
      class="settings-nav-btn {javaModsTab === 'modrinth' ? 'active' : ''}"
      onclick={() => (javaModsTab = 'modrinth')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-leaf"></i> Modrinth
    </button>
  </nav>

  <div class="settings-content">

{#if javaModsTab !== 'curseforge' && javaModsTab !== 'modrinth'}
  <div class="panel-body jm-pt6">
    {#if error}{@render errorRow(error, () => void loadMods())}{/if}

    {#if instances.length > 1}
      <select
        class="setting-select jm-select-full"
        bind:value={activeInstanceId}
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
          <span>{t('empty.noVersionSelected')}</span>
          <small
            >Pick a Minecraft version from the Versions panel first, then come back to install a mod
            loader.</small
          >
        </div>
      {:else}
        <div
          class="version-row jm-active-row"
        >
          <div class="version-meta">
            <span class="jm-eyebrow"
              >{t('x.installingFor')}</span
            >
            <span class="jm-row-title"
              >{activeVersion}</span
            >
          </div>
          {#if activeInstance?.loader && activeInstance.loader !== 'vanilla'}
            <button
              class="btn-sm"
              disabled={javaUtilityBusy}
              onclick={uninstallLoaderForActive}
              onmousedown={(e) => e.stopPropagation()}
              title="Clear the configured loader (does not delete downloaded loader files)"
            >
              <i class="fa-solid fa-xmark"></i> Clear loader
            </button>
          {/if}
        </div>

        {#if loaderInstallStatus}
          <div class="jm-mb8">
            <small class="jm-dim">{loaderInstallStatus}</small>
            <InlineProgress progress={0} showSpinner={true} />
          </div>
        {/if}

        <div class="client-card-cols">
        <div class="client-card {activeVersion.includes('fabric') ? 'client-active' : ''}">
          <div class="client-card-header">
            <div
              class="client-card-icon jm-tile jm-tile--loader-fabric"
            >
              <i class="fa-solid fa-feather"></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name">Fabric</span>
              <span class="client-card-sub">
                {#if activeVersion.includes('fabric')}
                  <span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Active</span>
                {:else}
                  <span class="jm-dim">{t('x.lightweightModLoader')}</span>
                {/if}
              </span>
            </div>
            <div class="client-card-actions">
              <IconBtn
                icon={installingLoader === 'fabric' ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-download'}
                title="Install Fabric"
                disabled={javaUtilityBusy}
                onClick={() => installLoaderForActive('fabric')}
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
              class="client-card-icon jm-tile jm-tile--loader-quilt"
            >
              <i class="fa-solid fa-layer-group"></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name">Quilt</span>
              <span class="client-card-sub">
                {#if activeVersion.includes('quilt')}
                  <span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Active</span>
                {:else}
                  <span class="jm-dim">Fabric-compatible fork</span>
                {/if}
              </span>
            </div>
            <div class="client-card-actions">
              <IconBtn
                icon={installingLoader === 'quilt' ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-download'}
                title="Install Quilt"
                disabled={javaUtilityBusy}
                onClick={() => installLoaderForActive('quilt')}
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
              class="client-card-icon jm-tile jm-tile--loader-forge"
            >
              <i class="fa-solid fa-hammer"></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name">Forge</span>
              <span class="client-card-sub"><span class="jm-dim">{t('x.theClassicModLoader')}</span></span>
            </div>
            <div class="client-card-actions">
              <IconBtn
                icon={installingLoader === 'forge' ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-download'}
                title="Download Forge installer"
                disabled={javaUtilityBusy}
                onClick={() => installLoaderForActive('forge')}
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
              class="client-card-icon jm-tile jm-tile--loader-neoforge"
            >
              <i class="fa-solid fa-fire-flame-curved"></i>
            </div>
            <div class="client-card-meta">
              <span class="client-card-name">NeoForge</span>
              <span class="client-card-sub"
                ><span class="jm-dim">Modern Forge fork (1.20.2+)</span></span
              >
            </div>
            <div class="client-card-actions">
              <IconBtn
                icon={installingLoader === 'neoforge' ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-download'}
                title="Download NeoForge installer"
                disabled={javaUtilityBusy}
                onClick={() => installLoaderForActive('neoforge')}
              />
            </div>
          </div>
          <p class="client-card-desc">
            Auto-detects the latest NeoForge version for your active Minecraft version and downloads
            the installer.
          </p>
        </div>
        </div>
      {/if}
    {:else if javaModsTab === 'mods'}
      {#if report.mods.length > 0}
        <div class="version-row jm-toolbar">
          <button
            class="btn-sm btn-primary"
            disabled={checkingUpdates || !activeInstanceId}
            onclick={checkForModUpdates}
            onmousedown={(e) => e.stopPropagation()}
          >
            {#if checkingUpdates}<span class="spinner"></span>{:else}<i class="fa-solid fa-cloud-arrow-down"
              ></i>{/if} Check for updates
          </button>
          <button
            class="btn-sm"
            disabled={bisectBusy}
            data-tooltip={t('tt.toggleEveryModOffAtOnce')}
            onclick={toggleAllMods}
            onmousedown={(e) => e.stopPropagation()}
          >
            {#if bisectBusy}<span class="spinner"></span>{:else}<i
                class="fa-solid {anyModEnabled ? 'fa-toggle-off' : 'fa-toggle-on'}"
              ></i>{/if}
            {anyModEnabled ? 'Disable all' : 'Enable all'}
          </button>
          {#if instances.length > 1}
            <button
              class="btn-sm"
              onclick={() => (showDiff = !showDiff)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-code-compare"></i> Compare instance
            </button>
          {/if}
          <button
            class="btn-sm"
            onclick={exportModList}
            onmousedown={(e) => e.stopPropagation()}
            data-tooltip={t('tt.saveThisInstanceSModList')}
          >
            <i class="fa-solid fa-file-export"></i> Export list
          </button>
          <button
            class="btn-sm {modSelectMode ? 'active' : ''}"
            onclick={toggleModSelectMode}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-check-double"></i> {modSelectMode ? 'Cancel' : 'Select'}
          </button>
        </div>

        {#if modSelectMode}
          <div class="version-row jm-bulkbar">
            <span class="version-sub">{selectedModFiles.size} selected</span>
            <button class="btn-sm" onclick={() => bulkSetEnabled(false)} disabled={selectedModFiles.size === 0 || bulkBusy}>
              <i class="fa-solid fa-toggle-off"></i> Disable selected
            </button>
            <button class="btn-sm" onclick={() => bulkSetEnabled(true)} disabled={selectedModFiles.size === 0 || bulkBusy}>
              <i class="fa-solid fa-toggle-on"></i> Enable selected
            </button>
            <button class="btn-sm jm-danger" onclick={bulkDeleteMods} disabled={selectedModFiles.size === 0 || bulkBusy}>
              <i class="fa-solid fa-trash"></i> Delete selected
            </button>
          </div>
        {/if}

        {#if showDiff}
          <div class="mod-warnings jm-mb8">
            <div class="mod-warnings-head jm-split">
              <span><i class="fa-solid fa-code-compare"></i> Compare mods against another instance</span>
              <button
                class="panel-icon-btn"
                onclick={() => (showDiff = false)}
                onmousedown={(e) => e.stopPropagation()}
                aria-label={t('tt.close')}><i class="fa-solid fa-xmark"></i></button
              >
            </div>
            <div class="jm-diff-row">
              <select class="setting-select jm-grow" bind:value={diffOtherInstanceId}>
                <option value="">{t('x.chooseAnInstance')}</option>
                {#each instances.filter((i) => i.id !== activeInstanceId) as inst (inst.id)}
                  <option value={inst.id}>{inst.name}</option>
                {/each}
              </select>
              <button
                class="btn-sm"
                disabled={!diffOtherInstanceId || diffBusy}
                onclick={runDiff}
                onmousedown={(e) => e.stopPropagation()}
              >
                {#if diffBusy}<span class="spinner"></span>{:else}Diff{/if}
              </button>
            </div>
            {#if diffError}{@render errorRow(diffError, () => void runDiff())}{/if}
            {#if diffResult}
              <div class="mod-warning-item">
                <strong>Only in {activeInstance?.name ?? 'this instance'}</strong>
                ({diffResult.only_a.length}):
                {diffResult.only_a.map((m) => m.name).join(', ') || 'none'}
              </div>
              <div class="mod-warning-item">
                <strong>Only in {instances.find((i) => i.id === diffOtherInstanceId)?.name ?? 'the other instance'}</strong>
                ({diffResult.only_b.length}):
                {diffResult.only_b.map((m) => m.name).join(', ') || 'none'}
              </div>
              <div class="mod-warning-item">
                <strong>{t('x.differentVersionInBoth')}</strong> ({diffResult.different_version.length}):
                {diffResult.different_version.map((p) => p.a.name).join(', ') || 'none'}
              </div>
            {/if}
          </div>
        {/if}

        {#if updateCheckError}{@render errorRow(updateCheckError, () => void checkForModUpdates())}{/if}
        {#if modUpdates.length > 0}
          <div class="mod-warnings jm-mb8">
            <div class="mod-warnings-head jm-split">
              <span
                ><i class="fa-solid fa-cloud-arrow-down"></i>
                {modUpdates.length} update{modUpdates.length === 1 ? '' : 's'} available</span
              >
              <button
                class="btn-sm btn-primary"
                disabled={updatingAll || updatingFile !== null}
                onclick={applyAllModUpdates}
                onmousedown={(e) => e.stopPropagation()}
              >
                {#if updatingAll}<span class="spinner"></span>{:else}Update all{/if}
              </button>
            </div>
            {#each modUpdates as u (u.file_name)}
              <div class="mod-warning-item jm-row-flex">
                <span class="jm-grow">{u.name} → {u.available.name}</span>
                {#if u.available.changelog}
                  <button
                    class="panel-icon-btn"
                    aria-label={t('tt.viewChangelog')}
                    onclick={() => (changelogFor = changelogFor?.file_name === u.file_name ? null : u)}
                    onmousedown={(e) => e.stopPropagation()}
                    data-tooltip={t('tt.viewChangelog')}
                  >
                    <i class="fa-solid fa-scroll"></i>
                  </button>
                {/if}
                <button
                  class="btn-sm"
                  disabled={updatingFile === u.file_name || updatingAll}
                  onclick={() => applyModUpdate(u)}
                  onmousedown={(e) => e.stopPropagation()}
                >
                  {#if updatingFile === u.file_name}<span class="spinner"></span>{:else}Update{/if}
                </button>
              </div>
              {#if changelogFor?.file_name === u.file_name}
                <pre class="mod-warning-item jm-changelog">{u.available.changelog}</pre>
              {/if}
            {/each}
          </div>
        {/if}
      {/if}

      {#if lastInstallWarnings}
        <div class="mod-warnings jm-mb8">
          <div class="mod-warnings-head jm-split">
            <span
              ><i class="fa-solid fa-triangle-exclamation"></i> '{lastInstallWarnings.projectTitle}' needs
              these mods too</span
            >
            <button
              class="panel-icon-btn"
              onclick={() => (lastInstallWarnings = null)}
              onmousedown={(e) => e.stopPropagation()}
              aria-label={t('tt.dismiss')}><i class="fa-solid fa-xmark"></i></button
            >
          </div>
          {#each lastInstallWarnings.deps as d}
            <div class="mod-warning-item">{d.title} (not installed - not auto-installed)</div>
          {/each}
        </div>
      {/if}

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
      {#if modsLoading && report.mods.length === 0}
        <ListSkeleton rows={4} />
      {:else if report.mods.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-puzzle-piece"></i>
          <span>{t('empty.noLocalMods')}</span>
          <small>{t('empty.dropJarsIntoActiveInstanceMods')}</small>
          <button
            class="btn-sm jm-mt6"
            onclick={() => openJavaFolder('mods')}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-folder-open"></i> Open mods folder
          </button>
        </div>
      {:else}
        <div class="version-list {javaModsViewMode === 'grid' ? 'grid-view' : ''}">
        {#each report.mods as mod (mod.file_name)}
          <div class="version-row">
            {#if modSelectMode}
              <button
                type="button"
                class="btn-sm"
                onclick={() => toggleModSelected(mod.file_name)}
                onmousedown={(e) => e.stopPropagation()}
                aria-label={selectedModFiles.has(mod.file_name) ? `Deselect ${mod.name}` : `Select ${mod.name}`}
              >
                <i class="fa-solid {selectedModFiles.has(mod.file_name) ? 'fa-square-check' : 'fa-square'}"></i>
              </button>
            {/if}
            <div class="version-meta">
              <span class="version-name" class:jm-disabled={mod.disabled}>{mod.name}</span>
              <span class="version-sub"
                >{mod.disabled ? 'Disabled' : 'Enabled'}
                {mod.required_deps.length === 0 ? '' : `· Requires ${mod.required_deps.join(', ')}`}</span
              >
            </div>
            <button
              class="btn-sm"
              onclick={() => toggleJavaMod(mod)}
              onmousedown={(e) => e.stopPropagation()}
              aria-label={mod.disabled ? `Enable ${mod.name}` : `Disable ${mod.name}`}
            >
              <i class="fa-solid {mod.disabled ? 'fa-toggle-off' : 'fa-toggle-on'}"></i>
            </button>
            <button
              class="btn-sm"
              onclick={() => deleteJavaFile(mod)}
              onmousedown={(e) => e.stopPropagation()}
              aria-label={`Delete ${mod.name}`}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          </div>
        {/each}
        </div>
        <div class="jm-center-pad-sm">
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
      <div class="client-card-cols">
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon jm-tile jm-tile--green"
          >
            <i class="fa-solid fa-image"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{t('x.resourcePacks')}</span>
            <span class="client-card-sub jm-dim"
              >{resourcePacks.length} pack(s)</span
            >
          </div>
          <div class="client-card-actions">
            <IconBtn
              icon="fa-solid fa-upload"
              title="Import resource pack"
              onClick={() => addPack('resourcepack')}
              onMouseDown={(e) => e.stopPropagation()}
            />
            <IconBtn
              icon="fa-solid fa-folder-open"
              title="Open folder"
              onClick={() => openJavaFolder('resourcepacks')}
              onMouseDown={(e) => e.stopPropagation()}
            />
          </div>
        </div>
        {#if packsLoading && resourcePacks.length === 0}
          <ListSkeleton rows={2} />
        {:else if resourcePacks.length === 0}
          <div class="empty-state">
            <i class="fa-solid fa-image"></i>
            <span>No resource packs yet</span>
          </div>
        {/if}
        {#each resourcePacks as pack (pack.path)}
          <div class="version-row jm-indent">
            <div class="version-meta">
              <span class="version-name">{pack.name}</span>
              <span class="version-sub">{formatSize(pack.size_bytes)}</span>
            </div>
            <button
              class="btn-sm"
              onclick={() => deletePack('resourcepack', pack)}
              aria-label={`Delete ${pack.name}`}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          </div>
        {/each}
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon jm-tile jm-tile--orange"
          >
            <i class="fa-solid fa-wand-magic-sparkles"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{t('x.shaderPacks')}</span>
            <span class="client-card-sub jm-dim"
              >{shaderPacks.length} pack(s)</span
            >
          </div>
          <div class="client-card-actions">
            <IconBtn
              icon="fa-solid fa-upload"
              title="Import shader pack"
              onClick={() => addPack('shaderpack')}
              onMouseDown={(e) => e.stopPropagation()}
            />
            <IconBtn
              icon="fa-solid fa-folder-open"
              title="Open folder"
              onClick={() => openJavaFolder('shaderpacks')}
              onMouseDown={(e) => e.stopPropagation()}
            />
          </div>
        </div>
        {#if packsLoading && shaderPacks.length === 0}
          <ListSkeleton rows={2} />
        {:else if shaderPacks.length === 0}
          <div class="empty-state">
            <i class="fa-solid fa-wand-magic-sparkles"></i>
            <span>No shader packs yet</span>
          </div>
        {/if}
        {#each shaderPacks as pack (pack.path)}
          <div class="version-row jm-indent">
            <div class="version-meta">
              <span class="version-name">{pack.name}</span>
              <span class="version-sub">{formatSize(pack.size_bytes)}</span>
            </div>
            <button
              class="btn-sm"
              onclick={() => deletePack('shaderpack', pack)}
              aria-label={`Delete ${pack.name}`}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          </div>
        {/each}
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon jm-tile jm-tile--accent"
          >
            <i class="fa-solid fa-map"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{t('x.savesWorlds')}</span>
            <span class="client-card-sub jm-dim">{t('x.openYourWorldsFolder')}</span>
          </div>
          <div class="client-card-actions">
            <IconBtn
              icon="fa-solid fa-folder-open"
              title="Open folder"
              onClick={() => openJavaFolder('saves')}
              onMouseDown={(e) => e.stopPropagation()}
            />
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon jm-tile jm-tile--red"
          >
            <i class="fa-solid fa-camera"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{t('panel.screenshots')}</span>
            <span class="client-card-sub jm-dim"
              >{javaScreenshotsCount ?? '—'} screenshot(s)</span
            >
          </div>
          <div class="client-card-actions">
            <IconBtn
              icon="fa-solid fa-folder-open"
              title="Open folder"
              onClick={() => openJavaFolder('screenshots')}
              onMouseDown={(e) => e.stopPropagation()}
            />
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon jm-tile jm-tile--orange"
          >
            <i class="fa-solid fa-drafting-compass"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{t('x.schematics')}</span>
            <span
              class="client-card-sub jm-dim"
              data-tooltip={MISSING_FILE_LIST_SCHEMATICS}>— file(s)</span
            >
          </div>
          <div class="client-card-actions">
            <IconBtn
              icon="fa-solid fa-folder-open"
              title="Open folder"
              onClick={() => openJavaFolder('schematics')}
              onMouseDown={(e) => e.stopPropagation()}
            />
          </div>
        </div>
      </div>
      </div>
    {:else if javaModsTab === 'datapacks'}
      <label class="modal-input-label" for="datapacks-world">{t('label.world')}</label>
      <select
        id="datapacks-world"
        class="setting-select jm-select-full"
        bind:value={datapacksWorldName}
        onclick={(e) => e.stopPropagation()}
      >
        <option value="">Select a world…</option>
        {#each javaWorldsList as w (w.name)}
          <option value={w.name}>{w.name}</option>
        {/each}
      </select>

      {#if !datapacksWorldName}
        <div class="empty-state">
          <i class="fa-solid fa-cubes-stacked"></i>
          <span>{t('empty.pickWorldAbove')}</span>
          <small>{t('empty.datapacksAreInstalledPerWorldNot')}</small>
        </div>
      {:else if datapacksLoading && javaDatapacks.length === 0}
        <ListSkeleton rows={3} />
      {:else if javaDatapacks.length === 0}
        <div class="empty-state">
          <i class="fa-solid fa-cubes-stacked"></i>
          <span>No datapacks in "{datapacksWorldName}"</span>
          <button
            class="btn-sm jm-mt6"
            disabled={datapacksBusy}
            onclick={addJavaDatapack}
          >
            <i class="fa-solid fa-upload"></i> Add datapack (.zip)
          </button>
        </div>
      {:else}
        {#each javaDatapacks as pack (pack.path)}
          <div class="version-row">
            <div class="version-meta">
              <span class="version-name">{pack.name}</span>
              <span class="version-sub">{formatSize(pack.size_bytes)} · {pack.modified_at}</span>
            </div>
            <button
              class="btn-sm"
              disabled={datapacksBusy}
              onclick={() => deleteJavaDatapack(pack)}
              aria-label={`Delete ${pack.name}`}
            >
              <i class="fa-solid fa-trash"></i>
            </button>
          </div>
        {/each}
        <div class="jm-center-pad">
          <button class="btn-sm" disabled={datapacksBusy} onclick={addJavaDatapack}>
            <i class="fa-solid fa-upload"></i> Add datapack (.zip)
          </button>
          <button class="btn-sm" onclick={openJavaDatapacksFolder}>
            <i class="fa-solid fa-folder-open"></i> Open folder
          </button>
        </div>
      {/if}
    {:else if javaModsTab === 'tools'}
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon jm-tile jm-tile--accent"
          >
            <i class="fa-solid fa-box-archive"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{t('x.backupSaves')}</span>
            <span class="client-card-sub jm-dim"
              >{t('x.zipYourWorldsBeforeARiskyUpdate')}</span
            >
          </div>
          <div class="client-card-actions">
            <IconBtn disabled icon="fa-solid fa-download" title={MISSING_BACKUP} />
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon jm-tile jm-tile--green"
          >
            <i class="fa-solid fa-file-zipper"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{t('x.exportModpack')}</span>
            <span class="client-card-sub jm-dim"
              >Bundle mods + config into a shareable zip</span
            >
          </div>
          <div class="client-card-actions">
            <IconBtn disabled icon="fa-solid fa-file-export" title={MISSING_EXPORT} />
          </div>
        </div>
      </div>
      <div class="client-card">
        <div class="client-card-header">
          <div
            class="client-card-icon jm-tile jm-tile--orange"
          >
            <i class="fa-solid fa-clone"></i>
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{t('x.duplicateInstance')}</span>
            <span class="client-card-sub jm-dim"
              >Clone your active instance with all mods &amp; config</span
            >
          </div>
          <div class="client-card-actions">
            <IconBtn
              disabled={duplicateBusy || !activeInstanceId}
              icon="fa-solid fa-copy"
              title="Duplicate active instance"
              onClick={duplicateActiveInstance}
            />
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}

{#if javaModsTab === 'curseforge'}
  <div class="panel-body">
    <div class="versions-client-switcher switcher-wrap">
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
        placeholder={t('ph.searchCurseforgeJava')}
        bind:value={cfSearchQuery}
        onkeydown={(e) => {
          e.stopPropagation()
          if (e.key === 'Enter') void searchCurseForgeMods()
        }}
      />
    </div>

    {#if cfCategory !== 'all' && cfCategory !== 'mods'}
      <div class="empty-state">
        <i class="fa-solid fa-cube"></i>
        <span>{t('empty.modsOnlyNow')}</span>
        <small>{t('empty.curseforgeSearchIsWiredUpMods')}</small>
      </div>
    {:else if cfSearching}
      <ListSkeleton rows={5} />
    {:else if cfError}
      {@render errorRow(cfError, () => void searchCurseForgeMods())}
    {:else if cfResults.length === 0 && cfHasSearched}
      <div class="empty-state">
        <i class="fa-solid fa-box-open"></i>
        <span>{t('empty.noModsFound')}</span>
        <small>{t('empty.tryDifferentSearchTerm')}</small>
      </div>
    {:else if cfResults.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-magnifying-glass"></i>
        <span>{t('empty.browseCurseforge')}</span>
        <small>{t('empty.searchJavaModsAboveRequiresCurseforge')}</small>
      </div>
    {:else}
      <div class="version-list {javaModsViewMode === 'grid' ? 'grid-view' : ''}">
      {#each cfResults as a (a.id)}
        <div class="client-card jm-card-gap">
          <div class="client-card-header">
            <div class="client-card-icon jm-cover">
              {#if a.icon_url}
                <img
                  src={a.icon_url}
                  loading="lazy"
                  decoding="async"
                  class="jm-cover-img"
                  alt=""
                />
              {:else}
                <i class="fa-solid fa-cube jm-cover-icon"></i>
              {/if}
            </div>
            <div class="client-card-meta jm-minw0">
              <span
                class="client-card-name jm-ellipsis">{a.name}</span
              >
              <span class="client-card-sub jm-sub-row">
                <span class="jm-author">{a.author}</span>
                <span class="jm-dim">{formatDownloads(a.downloads)}</span>
              </span>
            </div>
            <div class="client-card-actions">
              {#if cfDownloadingId === a.id}
                <InlineProgress progress={cfDownloadProgress} />
              {:else}
                <IconBtn
                  icon="fa-solid fa-download"
                  title={activeInstanceId ? 'Install into active instance' : 'Select an instance first'}
                  disabled={!activeInstanceId}
                  onClick={() => installCurseForgeMod(a)}
                  onMouseDown={(e) => e.stopPropagation()}
                />
              {/if}
            </div>
          </div>
          <p
            class="client-card-desc jm-clamp2"
          >
            {a.summary}
          </p>
        </div>
      {/each}
      </div>

      <div class="jm-center-pad">
        <button class="btn-sm" onclick={() => searchCurseForgeMods(true)}>
          <i class="fa-solid fa-angles-down"></i> Load more
        </button>
      </div>
    {/if}
  </div>
{/if}

{#if javaModsTab === 'modrinth'}
  <div class="panel-body">
    <div class="versions-client-switcher switcher-wrap">
      {#each MR_CATEGORIES as cat (cat.key)}
        <button
          class="vcs-btn {mrCategory === cat.key ? 'active' : ''}"
          onclick={() => selectMrCategory(cat.key)}
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
        placeholder={t('ph.searchModrinth')}
        bind:value={mrSearchQuery}
        onkeydown={(e) => {
          e.stopPropagation()
          if (e.key === 'Enter') void searchModrinth()
        }}
      />
    </div>

    {#if mrSearching}
      <ListSkeleton rows={5} />
    {:else if mrError}
      {@render errorRow(mrError, () => void searchModrinth())}
    {:else if mrResults.length === 0 && mrHasSearched}
      <div class="empty-state">
        <i class="fa-solid fa-box-open"></i>
        <span>{t('empty.noResultsFound')}</span>
        <small>{t('empty.tryDifferentSearchTermOrCategory')}</small>
      </div>
    {:else if mrResults.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-leaf"></i>
        <span>{t('empty.browseModrinth')}</span>
        <small>{t('empty.searchModsModpacksResourcePacksOr')}</small>
      </div>
    {:else}
      <div class="version-list {javaModsViewMode === 'grid' ? 'grid-view' : ''}">
      {#each mrResults as p (p.id)}
        <div class="client-card jm-card-gap">
          <div class="client-card-header">
            <div class="client-card-icon jm-cover">
              {#if p.icon_url}
                <img
                  src={p.icon_url}
                  loading="lazy"
                  decoding="async"
                  class="jm-cover-img"
                  alt=""
                />
              {:else}
                <i class="fa-solid fa-leaf jm-cover-icon jm-modrinth"></i>
              {/if}
            </div>
            <div class="client-card-meta jm-minw0">
              <span
                class="client-card-name jm-ellipsis">{p.title}</span
              >
              <span class="client-card-sub jm-sub-row">
                <span class="jm-author">{p.author}</span>
                <span class="jm-dim">{formatDownloads(p.downloads)}</span>
                {#if p.category_name}
                  <span class="tag-uptodate jm-tag-sm"
                    >{p.category_name}</span
                  >
                {/if}
              </span>
            </div>
            <div class="client-card-actions jm-actions-tight">
              <IconBtn
                icon={isStarred(p.id) ? 'fa-solid fa-star' : 'fa-regular fa-star'}
                title={isStarred(p.id) ? 'Unstar' : 'Star'}
                onClick={() => toggleStar(p)}
                onMouseDown={(e) => e.stopPropagation()}
              />
              {#if mrDownloadingId === p.id}
                <InlineProgress progress={mrDownloadProgress} />
              {:else}
                <IconBtn
                  icon="fa-solid fa-download"
                  title={p.project_type === 'mod' ? 'Install into active instance' : `Installing ${p.project_type}s isn't wired up yet`}
                  disabled={p.project_type !== 'mod' || !activeInstanceId}
                  onClick={() => openDependencyModal(p)}
                  onMouseDown={(e) => e.stopPropagation()}
                />
              {/if}
            </div>
          </div>
          <p
            class="client-card-desc jm-clamp2"
          >
            {p.description}
          </p>
        </div>
      {/each}
      </div>

      {#if mrResults.length < mrTotalCount}
        <div class="jm-center-pad">
          <button class="btn-sm" onclick={() => searchModrinth(true)} onmousedown={(e) => e.stopPropagation()}>
            <i class="fa-solid fa-angles-down"></i> Load more ({mrResults.length} / {mrTotalCount})
          </button>
        </div>
      {/if}
    {/if}
  </div>
{/if}

  </div>
</div>

<Modal
  open={depModalOpen}
  title={depProject ? `Install '${depProject.title}'` : 'Install mod'}
  icon="fa-solid fa-download"
  onClose={() => (depModalOpen = false)}
>
  {#if depError}
    <p class="error-text">{depError}</p>
  {:else if !depPlan}
    <ListSkeleton rows={3} />
  {:else}
    {#if depPlan.conflicts.length > 0}
      <div class="empty-state jm-left">
        <span><i class="fa-solid fa-triangle-exclamation"></i> Conflicts</span>
        {#each depPlan.conflicts as c}
          <small class="jm-block">{c}</small>
        {/each}
      </div>
    {/if}
    <p class="modal-input-label">
      This will install {depPlan.files.length} file{depPlan.files.length === 1 ? '' : 's'}:
    </p>
    {#each depPlan.files as f (f.filename)}
      <div class="version-row">
        <div class="version-meta">
          <span class="version-name">{f.filename}</span>
          <span class="version-sub">{f.is_root ? 'requested mod' : 'dependency'} · {f.provider}</span>
        </div>
      </div>
    {/each}
    <div class="jm-right-pad">
      <button class="btn-sm btn-primary" disabled={depBusy} onclick={confirmDependencyInstall}>
        {#if depBusy}
          <Spinner size={14} borderWidth={2} />
        {:else}
          <i class="fa-solid fa-check"></i> Install {depPlan.files.length} file{depPlan.files.length === 1 ? '' : 's'}
        {/if}
      </button>
    </div>
  {/if}
</Modal>

<style>
  /* Spacing / layout helpers (replace former inline styles). */
  .jm-pt6 { padding-top: 6px; }
  .jm-mb8 { margin-bottom: 8px; }
  .jm-mt6 { margin-top: 6px; }
  .jm-mt10 { margin-top: 10px; }
  .jm-select-full { width: 100%; margin-bottom: 8px; }
  .jm-grow { flex: 1; min-width: 0; }
  .jm-split { display: flex; justify-content: space-between; align-items: center; }
  .jm-row-flex { display: flex; align-items: center; gap: 8px; }
  .jm-toolbar { margin-bottom: 8px; gap: 6px; flex-wrap: wrap; }
  .jm-bulkbar { gap: 8px; padding: 4px 0 8px; flex-wrap: wrap; }
  .jm-diff-row { display: flex; gap: 6px; padding: 6px 0; }
  .jm-changelog { white-space: pre-wrap; max-height: 180px; overflow: auto; }
  .jm-danger { color: var(--red); }
  .jm-disabled { opacity: 0.55; }
  .jm-center-pad { text-align: center; padding: 8px; }
  .jm-center-pad-sm { text-align: center; padding: 4px; }
  .jm-right-pad { text-align: right; padding-top: 12px; }
  .jm-left { text-align: left; align-items: flex-start; }
  .jm-block { display: block; }
  .jm-indent { padding-left: 8px; }
  .jm-active-row { border-left: 2px solid var(--accent); margin-bottom: 4px; }
  .jm-card-gap { margin-bottom: 6px; }
  .jm-minw0 { min-width: 0; }
  .jm-ellipsis { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .jm-sub-row { display: flex; gap: 8px; align-items: center; }
  .jm-clamp2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .jm-actions-tight { display: flex; gap: 4px; }
  .jm-tag-sm { font-size: 9px; padding: 1px 5px; }

  /* Typography: eyebrow / row title / caption. */
  .jm-eyebrow {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-dim);
  }
  .jm-row-title { font-size: 13px; font-weight: 600; color: var(--text); }
  .jm-dim { font-size: 11px; color: var(--text-dim); }
  .jm-author { font-size: 11px; color: var(--text-dim); }

  /* Icon tiles (loader cards + asset cards). */
  .jm-tile { padding: 8px; }
  .jm-tile--loader-fabric { background: var(--loader-fabric-bg); color: var(--loader-fabric); }
  .jm-tile--loader-quilt { background: var(--loader-quilt-bg); color: var(--loader-quilt); }
  .jm-tile--loader-forge { background: var(--loader-forge-bg); color: var(--loader-forge); }
  .jm-tile--loader-neoforge { background: var(--loader-neoforge-bg); color: var(--loader-neoforge); }
  .jm-tile--green { background: var(--green-bg); color: var(--green); }
  .jm-tile--orange { background: var(--orange-bg); color: var(--orange); }
  .jm-tile--accent { background: var(--accent-bg); color: var(--accent); }
  .jm-tile--red { background: var(--red-bg); color: var(--red); }

  /* Search-result cover art. */
  .jm-cover { padding: 3px; border-radius: 6px; overflow: hidden; }
  .jm-cover-img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 4px;
  }
  .jm-cover-icon { font-size: 18px; color: var(--accent); }
  .jm-cover-icon.jm-modrinth { color: var(--loader-modrinth); }

  /* Error row: one consistent treatment, with Retry where a retry exists. */
  .jm-error-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    margin-bottom: 8px;
    border-radius: var(--r-sm);
    background: var(--red-bg);
    color: var(--red);
    font-size: 12px;
  }
  .jm-error-row .jm-error-msg { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  .jm-error-row .btn-sm { color: var(--text); }
</style>
