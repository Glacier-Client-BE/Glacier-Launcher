<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import type { LauncherSettings } from './lib/settings'
  import InstancesView from './lib/InstancesView.svelte'
  import ThemeStudioView from './lib/ThemeStudioView.svelte'
  import LevelDatEditorView from './lib/LevelDatEditorView.svelte'
  import BedrockWorldsView from './lib/BedrockWorldsView.svelte'
  import BedrockPacksView from './lib/BedrockPacksView.svelte'
  import BedrockBackupsView from './lib/BedrockBackupsView.svelte'
  import BedrockScreenshotsView from './lib/BedrockScreenshotsView.svelte'
  import ClientsView from './lib/ClientsView.svelte'
  import SettingsView from './lib/SettingsView.svelte'
  import JavaModsView from './lib/JavaModsView.svelte'
  import ModpacksView from './lib/ModpacksView.svelte'
  import VersionsView from './lib/VersionsView.svelte'
  import TweaksView from './lib/TweaksView.svelte'
  import ServersView from './lib/ServersView.svelte'
  import LogsPanel from './lib/LogsPanel.svelte'
  import StatsPanel from './lib/StatsPanel.svelte'
  import LeviLaminaModsView from './lib/LeviLaminaModsView.svelte'
  import MCVersionsView from './lib/MCVersionsView.svelte'
  import ProfileView from './lib/ProfileView.svelte'
  import JavaBackupsView from './lib/JavaBackupsView.svelte'
  import SkinLibraryView from './lib/SkinLibraryView.svelte'
  import JavaScreenshotsView from './lib/JavaScreenshotsView.svelte'
  import LauncherClientsView from './lib/LauncherClientsView.svelte'
  import BedrockVanillaVersionsView from './lib/BedrockVanillaVersionsView.svelte'
  import UtilitiesView from './lib/UtilitiesView.svelte'
  import ClientLibraryView from './lib/ClientLibraryView.svelte'
  import DownloadsView from './lib/DownloadsView.svelte'
  import NewsView from './lib/NewsView.svelte'
  import CreditsView from './lib/CreditsView.svelte'
  import Footer from './lib/Footer.svelte'
  import OnboardingModal from './lib/OnboardingModal.svelte'
  import { t } from './lib/i18n'
  import type { BedrockInstance } from './lib/settings'

  type View =
    | 'home'
    | 'clients'
    | 'instances'
    | 'bedrockworlds'
    | 'bedrockpacks'
    | 'bedrockbackups'
    | 'bedrockscreenshots'
    | 'leveldat'
    | 'themestudio'
    | 'settings'
    | 'java-mods'
    | 'modpacks'
    | 'versions'
    | 'tweaks'
    | 'servers'
    | 'logs'
    | 'stats'
    | 'levilamina-mods'
    | 'mc-versions'
    | 'launchers'
    | 'bedrock-versions'
    | 'profile'
    | 'java-screenshots'
    | 'java-backups'
    | 'skinlibrary'
    | 'utilities'
    | 'clientlibrary'
    | 'downloads'
    | 'news'
    | 'credits'


  type JavaInstance = { id: string; directory: string }

  let settings = $state<LauncherSettings | null>(null)
  let currentView = $state<View>('home')
  let contentInitialTab = $state('worlds')
  /// World whose level.dat is open in the NBT editor - the original's
  /// `levelDatEditing` deep-link from a world row.
  let levelDatWorldDir = $state('')
  /// Sub-tab the Utilities panel should open on (the original's `_utilTab`).
  let utilInitialTab = $state('options')
  let status = $state('loading…')
  let activeBedrockInstance = $state<BedrockInstance | null>(null)
  let activeJavaInstance = $state<JavaInstance | null>(null)
  let isLaunching = $state(false)
  type Announcement = { id: string; title: string; message: string; kind: string; url: string }
  let announcement = $state<Announcement | null>(null)
  let dragDepth = 0
  let currentVersion = $state('')
  let launcherUpdateAvailable = $state(false)
  let isFullscreen = $state(false)

  type NewsItem = { icon: string; title: string; subtitle: string; url: string }
  let newsItems = $state<NewsItem[]>([])

  const isBedrock = $derived(settings?.Edition !== 'java')

  // Client chip + live-running indicator (top-bar): mirrors CycleClient /
  // mcRunning from Home.Clients.cs / Home.razor — only Bedrock has a
  // process-detection command wired (is_bedrock_game_running); Java has no
  // equivalent yet, so the running pill only lights up on Bedrock.
  const CLIENT_ICONS: Record<string, string> = {
    'Custom DLL': 'fa-solid fa-file-code',
    Vanilla: 'fa-solid fa-cube',
    'LeviLamina Client': 'fa-solid fa-layer-group',
  }
  const CLIENT_IMAGES: Record<string, string> = {
    'Flarial Client': '/images/clients/flarial.svg',
    'OderSo Client': '/images/clients/oderso.png',
    'Latite Client': '/images/clients/latite.png',
  }
  const CYCLE_CLIENTS = ['Latite Client', 'Flarial Client', 'OderSo Client', 'LeviLamina Client', 'Vanilla']

  let mcRunning = $state(false)

  /// Port of `clientsHasBadge` (Home.razor.cs:363): the Clients button shows
  /// a notification dot while either bundled client is missing from disk.
  /// Toasts - port of `ToastItem` + `ShowToast` (Home.razor.cs:231, 635).
  /// The original holds each toast for 2800ms, flips it to `.closing` for a
  /// 240ms exit animation, then drops it.
  type ToastItem = { id: number; message: string; kind: string; icon: string; closing: boolean }
  let toasts = $state<ToastItem[]>([])
  let toastSeq = 0

  function showToast(message: string, kind: string = 'info') {
    const icon =
      kind === 'success'
        ? 'fa-solid fa-circle-check'
        : kind === 'error'
          ? 'fa-solid fa-circle-xmark'
          : kind === 'warning'
            ? 'fa-solid fa-triangle-exclamation'
            : 'fa-solid fa-circle-info'
    const id = ++toastSeq
    toasts = [...toasts, { id, message, kind, icon, closing: false }]
    setTimeout(() => {
      toasts = toasts.map((t) => (t.id === id ? { ...t, closing: true } : t))
      setTimeout(() => {
        toasts = toasts.filter((t) => t.id !== id)
      }, 240)
    }, 2800)
  }

  let clientsHasBadge = $state(false)

  async function refreshClientsBadge() {
    try {
      const [flarial, oderso] = await Promise.all([
        invoke<boolean>('flarial_client_installed'),
        invoke<boolean>('oderso_client_installed'),
      ])
      clientsHasBadge = !flarial || !oderso
    } catch {
      clientsHasBadge = false
    }
  }

  async function pollMcRunning() {
    if (!isBedrock) {
      mcRunning = false
      return
    }
    try {
      mcRunning = await invoke<boolean>('is_bedrock_game_running')
    } catch {
      mcRunning = false
    }
  }

  function cycleClient() {
    if (!settings) return
    const idx = CYCLE_CLIENTS.indexOf(settings.SelectedClient)
    const next = CYCLE_CLIENTS[(idx + 1) % CYCLE_CLIENTS.length]
    saveSettings({ ...settings, SelectedClient: next })
  }

  // Notification bell: the original shows BOTH the download queue and the
  // persisted AppNotification feed, and badges the bell with
  // `Notify.UnreadCount + activeDownloadCount` (Home.razor:520-524).
  type DownloadEntry = { id: string; label: string; status: string; progress: number }
  type AppNotification = {
    Id: string
    Title: string
    Message: string
    Kind: string
    Icon: string
    TimeIso: string
    Read: boolean
    Url: string
  }
  let notifOpen = $state(false)
  let downloads = $state<DownloadEntry[]>([])
  let notifications = $state<AppNotification[]>([])
  let unreadCount = $state(0)

  async function loadDownloads() {
    try {
      downloads = await invoke<DownloadEntry[]>('list_downloads')
    } catch {
      downloads = []
    }
  }

  async function loadNotifications() {
    try {
      notifications = await invoke<AppNotification[]>('list_notifications')
      unreadCount = await invoke<number>('notification_unread_count')
    } catch {
      notifications = []
      unreadCount = 0
    }
  }

  function toggleNotifPanel() {
    notifOpen = !notifOpen
    if (notifOpen) {
      loadDownloads()
      loadNotifications()
    }
  }

  async function removeNotification(id: string) {
    await invoke('remove_notification', { id })
    await loadNotifications()
  }

  async function clearNotifications() {
    await invoke('clear_notifications')
    await loadNotifications()
  }

  const activeDownloadCount = $derived(downloads.filter((d) => d.status === 'Downloading').length)
  /// Port of the original's `bellBadgeCount` (Home.razor:523).
  const bellBadgeCount = $derived(unreadCount + activeDownloadCount)

  /// Port of `FormatRelativeTime` used by each notification row.
  function formatRelativeTime(iso: string): string {
    const then = Date.parse(iso)
    if (Number.isNaN(then)) return ''
    const mins = Math.floor((Date.now() - then) / 60000)
    if (mins < 1) return 'just now'
    if (mins < 60) return `${mins}m ago`
    const hours = Math.floor(mins / 60)
    if (hours < 24) return `${hours}h ago`
    return `${Math.floor(hours / 24)}d ago`
  }

  // Search - transcribed from the original's full-screen overlay
  // (Home.razor:141-215) and Home.Search.cs. The previous rewrite replaced
  // this with an invented inline-widening widget plus its own scoped CSS;
  // the real `.search-overlay`/`.search-modal`/`.search-result-*` classes
  // already exist in app.css and were going unused.
  //
  // Scope note: the original ships 42 quick actions. Entries whose target
  // does not exist in this rewrite yet (folder-open commands, settings
  // import, Utilities sub-tabs, Xbox/Discord modals, Theme Studio, Skin
  // Library, Backups) are omitted rather than rendered as dead rows - see
  // PARITY.md. The DOM, grouping, highlighting and key handling are faithful.
  type SearchResult = {
    icon: string
    label: string
    category: string
    sub?: string
    shortcut?: string
    run: () => void
  }

  let searchOpen = $state(false)
  let searchQuery = $state('')
  let searchSelIdx = $state(0)
  let searchInputEl = $state<HTMLInputElement | null>(null)

  /// Port of `RecentsForCurrentEdition`: Java entries are prefixed "Java "
  /// and only shown on the Java edition; Bedrock shows everything else.
  const editionRecents = $derived(
    (settings?.RecentlyLaunched ?? []).filter((r) =>
      isBedrock ? !r.startsWith('Java ') : r.startsWith('Java '),
    ),
  )

  /// Port of `RecentDisplayLabel`: strips the "Java " routing prefix.
  function recentDisplayLabel(raw: string): string {
    return raw.startsWith('Java ') ? raw.slice(5) : raw
  }

  function selectClient(name: string) {
    if (!settings) return
    saveSettings({ ...settings, SelectedClient: name })
  }

  const searchIndex = $derived<SearchResult[]>([
    { icon: 'fa-solid fa-play', label: 'Launch Game', category: 'Quick Actions', sub: 'with ' + (settings?.SelectedClient ?? ''), shortcut: 'Ctrl+L', run: () => { closeSearch(); handleLaunch() } },
    { icon: 'fa-solid fa-gear', label: 'Open Settings', category: 'Quick Actions', sub: 'Injection, Appearance, Account', shortcut: 'Ctrl+1', run: () => { closeSearch(); openPanel('settings') } },
    { icon: 'fa-solid fa-puzzle-piece', label: 'Manage Clients', category: 'Quick Actions', sub: 'Flarial, Latite, OderSo, Custom DLL', shortcut: 'Ctrl+2', run: () => { closeSearch(); openPanel('clients') } },
    { icon: 'fa-solid fa-server', label: 'Servers', category: 'Quick Actions', sub: 'Quick-launch into saved servers', shortcut: 'Ctrl+5', run: () => { closeSearch(); openPanel('servers') } },
    { icon: 'fa-solid fa-clock-rotate-left', label: 'Browse Versions', category: 'Quick Actions', sub: 'Download and manage client versions', run: () => { closeSearch(); openPanel('versions') } },
    { icon: 'fa-solid fa-box-archive', label: 'MC Versions', category: 'Quick Actions', sub: 'Switch Minecraft Bedrock versions', run: () => { closeSearch(); openPanel('mc-versions') } },
    { icon: 'fa-solid fa-screwdriver-wrench', label: 'Utilities', category: 'Quick Actions', sub: 'Game options, shaders, client library, system info', run: () => { closeSearch(); openPanel('utilities') } },
    { icon: 'fa-solid fa-file-code', label: 'Client Library', category: 'Bedrock', sub: 'Your own DLL / EXE clients with auto-update', run: () => { closeSearch(); openPanel('clientlibrary') } },
    { icon: 'fa-solid fa-boxes-stacked', label: 'Packs', category: 'Bedrock', sub: 'Resource, behavior and skin packs', run: () => { closeSearch(); openPanel('bedrockpacks') } },
    { icon: 'fa-solid fa-globe', label: 'Worlds', category: 'Bedrock', sub: 'Manage Bedrock worlds', run: () => { closeSearch(); openPanel('bedrockworlds') } },
    { icon: 'fa-solid fa-layer-group', label: 'LeviLamina Mods', category: 'Bedrock', sub: 'Manage LeviLamina mods', run: () => { closeSearch(); openPanel('levilamina-mods') } },
    { icon: 'fa-solid fa-wand-magic-sparkles', label: 'Tweaks', category: 'Bedrock', sub: 'Vanilla and Bedrock Tweaks packs', run: () => { closeSearch(); openPanel('tweaks') } },
    { icon: 'fa-brands fa-java', label: 'Java Versions', category: 'Java', sub: 'Install or launch Java Edition', run: () => { closeSearch(); openPanel('versions') } },
    { icon: 'fa-solid fa-cubes', label: 'Browse Modpacks', category: 'Java', sub: 'Install CurseForge / Modrinth modpacks into an instance', run: () => { closeSearch(); openPanel('modpacks') } },
    { icon: 'fa-solid fa-puzzle-piece', label: 'Mods', category: 'Java', sub: 'Manage installed mods', run: () => { closeSearch(); openPanel('java-mods') } },
    { icon: 'fa-solid fa-file-lines', label: 'Logs and Crashes', category: 'Java', sub: 'View logs, detect crashes, share to mclo.gs', run: () => { closeSearch(); openPanel('logs') } },
    { icon: 'fa-solid fa-chart-simple', label: 'Statistics', category: 'Navigation', sub: 'Playtime, sessions and launch history', run: () => { closeSearch(); openPanel('stats') } },
    { icon: 'fa-solid fa-cubes', label: 'Instances', category: 'Navigation', sub: 'Manage game instances', run: () => { closeSearch(); openPanel('instances') } },
    { icon: 'fa-solid fa-rocket', label: 'Launchers', category: 'Launchers', sub: 'Lunar, Badlion and other launchers', run: () => { closeSearch(); openPanel('launchers') } },
    { icon: 'fa-solid fa-user', label: 'Profile', category: 'Account', sub: 'Your account and skin', run: () => { closeSearch(); openPanel('profile') } },
    { icon: 'fa-solid fa-images', label: 'Photos', category: 'Java', sub: 'Java screenshots gallery', run: () => { closeSearch(); openPanel('java-screenshots') } },
    { icon: 'fa-solid fa-download', label: 'Downloads', category: 'Quick Actions', sub: 'Active and recent downloads', run: () => { closeSearch(); openPanel('downloads') } },
    { icon: 'fa-solid fa-file-export', label: 'Export Settings', category: 'Settings', sub: 'Save settings as JSON', run: () => { closeSearch(); invoke('export_settings').catch(() => {}) } },
    { icon: 'fa-solid fa-expand', label: 'Toggle Fullscreen', category: 'Quick Actions', sub: 'F11 fullscreen mode', shortcut: 'F11', run: () => { closeSearch(); toggleFullscreen() } },
    { icon: '/images/clients/flarial.svg', label: 'Select Flarial', category: 'Clients', sub: 'Switch active client to Flarial', run: () => { closeSearch(); selectClient('Flarial Client') } },
    { icon: '/images/clients/latite.png', label: 'Select Latite', category: 'Clients', sub: 'Switch active client to Latite', run: () => { closeSearch(); selectClient('Latite Client') } },
    { icon: '/images/clients/oderso.png', label: 'Select OderSo', category: 'Clients', sub: 'Switch active client to OderSo', run: () => { closeSearch(); selectClient('OderSo Client') } },
    { icon: 'fa-solid fa-cube', label: 'Select Vanilla', category: 'Clients', sub: 'Launch Minecraft without a client', run: () => { closeSearch(); selectClient('Vanilla') } },
    { icon: 'fa-solid fa-newspaper', label: 'News and Updates', category: 'About', sub: 'Changelogs and announcements', run: () => { closeSearch(); openPanel('news') } },
    { icon: 'fa-solid fa-heart', label: 'Credits', category: 'About', sub: 'View launcher credits', run: () => { closeSearch(); openPanel('credits') } },
  ])

  /// An empty query shows the default Quick Actions list; a query filters
  /// across label, sub and category, as the original does.
  const searchMatches = $derived(
    searchQuery.trim()
      ? searchIndex.filter((r) => {
          const q = searchQuery.trim().toLowerCase()
          return (
            r.label.toLowerCase().includes(q) ||
            (r.sub ?? '').toLowerCase().includes(q) ||
            r.category.toLowerCase().includes(q)
          )
        })
      : [],
  )

  /// Results grouped by category, preserving first-seen order - the
  /// original's `_searchGroups`.
  const searchGroups = $derived.by(() => {
    const groups: { category: string; results: SearchResult[] }[] = []
    for (const r of searchMatches) {
      let g = groups.find((x) => x.category === r.category)
      if (!g) {
        g = { category: r.category, results: [] }
        groups.push(g)
      }
      g.results.push(r)
    }
    return groups
  })

  /// Flattened in render order so arrow-key selection tracks what is shown.
  const searchFlat = $derived(
    searchQuery.trim() ? searchGroups.flatMap((g) => g.results) : searchIndex,
  )

  function openSearch() {
    searchOpen = true
    searchQuery = ''
    searchSelIdx = 0
    queueMicrotask(() => searchInputEl?.focus())
  }

  function closeSearch() {
    searchOpen = false
    searchQuery = ''
    searchSelIdx = 0
  }

  /// Port of `Highlight`: wraps the matched span in <mark>. The text is
  /// HTML-escaped first, so a query can never inject markup.
  function highlight(text: string, query: string): string {
    const escapeHtml = (v: string) =>
      v.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
    const safe = escapeHtml(text)
    const q = escapeHtml(query.trim())
    if (!q) return safe
    const idx = safe.toLowerCase().indexOf(q.toLowerCase())
    if (idx < 0) return safe
    return safe.slice(0, idx) + '<mark>' + safe.slice(idx, idx + q.length) + '</mark>' + safe.slice(idx + q.length)
  }

  function onSearchKeydown(e: KeyboardEvent) {
    e.stopPropagation()
    if (e.key === 'Escape') {
      closeSearch()
    } else if (e.key === 'ArrowDown') {
      e.preventDefault()
      if (searchFlat.length > 0) searchSelIdx = (searchSelIdx + 1) % searchFlat.length
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      if (searchFlat.length > 0)
        searchSelIdx = (searchSelIdx - 1 + searchFlat.length) % searchFlat.length
    } else if (e.key === 'Enter') {
      e.preventDefault()
      searchFlat[searchSelIdx]?.run()
    }
  }

  function onDragStart() {
    window.startDrag?.()
  }

  function onWindowDragEnter(e: DragEvent) {
    e.preventDefault()
    dragDepth++
    window.setDropActive?.(true)
  }

  function onWindowDragLeave(e: DragEvent) {
    e.preventDefault()
    dragDepth = Math.max(0, dragDepth - 1)
    if (dragDepth === 0) window.setDropActive?.(false)
  }

  function onWindowDragOver(e: DragEvent) {
    e.preventDefault()
  }

  function onWindowDrop(e: DragEvent) {
    // Browser DragEvent.dataTransfer never carries real filesystem paths in
    // a WebView2-backed webview — the actual import runs off Tauri's native
    // onDragDropEvent below, which delivers real paths. This handler only
    // exists to prevent the browser's default "navigate to file" action.
    e.preventDefault()
  }

  const BEDROCK_CONTENT_EXTENSIONS = ['mcworld', 'mctemplate', 'mcpack', 'mcaddon']

  function extensionOf(path: string): string {
    const dot = path.lastIndexOf('.')
    return dot === -1 ? '' : path.slice(dot + 1).toLowerCase()
  }

  async function importDroppedFile(path: string) {
    const ext = extensionOf(path)
    if (BEDROCK_CONTENT_EXTENSIONS.includes(ext)) {
      if (!activeBedrockInstance) {
        status = 'No active Bedrock instance to import into.'
        return
      }
      showToast('Importing\u2026', 'info')
      try {
        const outcome = await invoke<{ kind: string; name?: string; paths?: string[] }>(
          'import_bedrock_content',
          { sourcePath: path, instanceDir: activeBedrockInstance.directory },
        )
        status = outcome.kind === 'world' ? `Imported world "${outcome.name}"` : `Imported ${outcome.paths?.length ?? 0} pack(s)`
        // The original raises BOTH a transient toast and a persisted
        // notification for a successful import (Home.razor.cs:434-436).
        const fileName = path.split(/[\\/]/).pop() ?? path
        await invoke('add_notification', {
          title: `${outcome.kind} imported`,
          message: `'${fileName}' was added to Minecraft.`,
          kind: 'success',
        }).catch(() => {})
        await loadNotifications()
        showToast(`${outcome.kind} imported.`, 'success')
      } catch (e) {
        status = `Import failed: ${e}`
        showToast(`Import failed: ${e}`, 'error')
      }
      return
    }
    if (ext === 'dll') {
      if (!settings) {
        status = 'Settings not loaded yet.'
        return
      }
      try {
        await saveSettings({ ...settings, SelectedClient: 'Custom DLL', CustomDllPath: path })
        status = `Set custom DLL client to "${path}"`
      } catch (e) {
        status = `Import failed: ${e}`
      }
      return
    }
    // Java modpack imports (.mrpack/.zip/.jar) aren't wired to a backend
    // handler yet — surface that honestly instead of silently discarding
    // the drop.
    status = `Importing .${ext || '?'} files isn't wired up yet.`
  }

  getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type === 'over') {
      window.setDropActive?.(true)
    } else if (event.payload.type === 'drop') {
      window.setDropActive?.(false)
      for (const path of event.payload.paths) importDroppedFile(path)
    } else {
      window.setDropActive?.(false)
    }
  })

  async function loadActiveBedrockInstance() {
    if (!settings || settings.Edition !== 'bedrock') {
      activeBedrockInstance = null
      return
    }
    try {
      const instances = await invoke<BedrockInstance[]>('list_bedrock_instances')
      activeBedrockInstance =
        instances.find((i) => i.id === settings!.BedrockActiveInstanceId) ?? instances[0] ?? null
    } catch {
      activeBedrockInstance = null
    }
  }

  async function loadActiveJavaInstance() {
    if (!settings || settings.Edition === 'bedrock') {
      activeJavaInstance = null
      return
    }
    try {
      const instances = await invoke<JavaInstance[]>('list_java_instances')
      activeJavaInstance = instances.find((i) => i.id === settings!.JavaActiveInstanceId) ?? instances[0] ?? null
    } catch {
      activeJavaInstance = null
    }
  }

  async function loadNews() {
    try {
      const posts = await invoke<any[] | null>('get_news')
      newsItems = (posts ?? []).map((p) => ({
        icon: p.icon || 'fa-solid fa-newspaper',
        title: p.title ?? p.Title ?? '',
        subtitle: p.subtitle ?? p.Subtitle ?? '',
        url: p.url ?? p.Url ?? '',
      }))
    } catch {
      newsItems = []
    }
  }

  // Ported from Home.Announcement.cs. Fetches the dismissible top-of-window
  // banner and hides it once its id matches the persisted dismissal.
  async function loadAnnouncement() {
    try {
      announcement = await invoke<Announcement | null>('get_announcement')
    } catch {
      announcement = null
    }
  }

  const showAnnouncementBanner = $derived(
    !!announcement && announcement.id !== settings?.LastDismissedAnnouncementId,
  )

  async function dismissAnnouncement() {
    if (!announcement || !settings) return
    await saveSettings({ ...settings, LastDismissedAnnouncementId: announcement.id })
  }

  async function loadSettings() {
    try {
      settings = await invoke<LauncherSettings>('get_settings')
      status = 'ready'
      applyTheme()
      await loadActiveBedrockInstance()
      await loadActiveJavaInstance()
      await loadNews()
      await loadAnnouncement()
      await pollMcRunning()
      setInterval(pollMcRunning, 5000)
    } catch (e) {
      status = `error: ${e}`
    }
  }

  async function saveSettings(next: LauncherSettings) {
    settings = next
    applyTheme()
    try {
      await invoke('set_settings', { settings: next })
    } catch (e) {
      status = `error: ${e}`
    }
  }

  function applyTheme() {
    if (!settings) return
    // Delegates to the ported interop.js helpers so accent-derived tokens
    // (--accent-hover/--accent-glow/--accent-bg), blur, compact mode, and
    // animation speed are computed exactly like the original app instead of
    // being reimplemented here.
    window.applyStoredSettings?.(
      settings.AccentColor,
      settings.ThemePreset,
      settings.BlurIntensity,
      settings.CustomBackgroundPath,
      settings.CompactMode,
      settings.AnimationsEnabled,
      settings.AnimationSpeed,
    )
  }

  function setEdition(ed: string) {
    if (!settings || settings.Edition === ed) return
    saveSettings({ ...settings, Edition: ed })
    currentView = 'home'
  }

  /// Panel navigation runs inside a View Transition when the platform
  /// supports one, mirroring how the original wrapped `ApplyPendingNav`.
  /// `interop.js` falls back to applying the change directly when view
  /// transitions are unavailable or animations are disabled.
  function navigate(apply: () => void) {
    if (window.glacierViewTransition) window.glacierViewTransition(apply)
    else apply()
  }

  /// The ported panels emit the *original* app's view keys (the strings
  /// `BedrockTabs`/`JavaTabs`/`SubTabs` were built with). Map them onto this
  /// rewrite's `View` union rather than renaming them inside the panels,
  /// which would break their fidelity to the source.
  const ORIGINAL_VIEW_MAP: Record<string, View> = {
    settings: 'settings',
    clients: 'clients',
    addons: 'bedrockpacks',
    packs: 'bedrockpacks',
    mcversions: 'bedrock-versions',
    worlds: 'bedrockworlds',
    servers: 'servers',
    utilities: 'utilities',
    credits: 'credits',
    instances: 'instances',
    clientlib: 'clientlibrary',
    stats: 'stats',
    skinlibrary: 'skinlibrary',
    logs: 'logs',
    tweaks: 'tweaks',
    launchers: 'launchers',
    mods: 'java-mods',
    profile: 'profile',
    levilaminamods: 'levilamina-mods',
    themestudio: 'themestudio',
    modpacks: 'modpacks',
    levimods: 'levilamina-mods',
  }

  /**
   * Keys the Utilities sub-tab strip owns internally (`SwitchUtilTab` in
   * Home.MergedPanels.cs:46-49). They are only Utilities tabs when they come
   * FROM that strip: `clients` from `BedrockTabs` is the Clients panel.
   */
  const UTILITIES_SUBTABS = ['options', 'shaders', 'clients', 'system']

  /**
   * `photos` and `backups` mean different panels per edition:
   * `JavaTabs`' photos is `OpenJavaScreenshots` (Home.BigFeatures.cs:49) and
   * its backups is the World Backups panel, while the Utilities strip's are
   * the Bedrock gallery and Bedrock backups (Home.MergedPanels.cs:50-51).
   */
  const EDITION_VIEW_MAP: Record<string, { bedrock: View; java: View }> = {
    photos: { bedrock: 'bedrockscreenshots', java: 'java-screenshots' },
    backups: { bedrock: 'bedrockbackups', java: 'java-backups' },
    // `versions` means the Java version browser on the Java edition
    // (JavaTabs:34 and the javaclients panel's Versions button, both
    // OpenJavaVersions) and the Latite/OderSo client-DLL picker on Bedrock
    // (reached from the Clients panel). The Bedrock *game* version browser is
    // `mcversions`, and the versions-instances strip's own `versions` key is
    // handled before this map because it means that one instead.
    versions: { bedrock: 'versions', java: 'mc-versions' },
  }

  function navigateFromPanel(originalView: string, strip?: string) {
    if (strip === 'utilities' && UTILITIES_SUBTABS.includes(originalView)) {
      utilInitialTab = originalView
      openPanel('utilities')
      return
    }
    // The versions-instances strip is Bedrock-only and its `versions` tab is
    // `OpenMcVersions` (Home.MergedPanels.cs:36) - the Bedrock game-version
    // browser, not the client-DLL picker the bare key otherwise means.
    if (strip === 'versions-instances' && originalView === 'versions') {
      openPanel('bedrock-versions')
      return
    }
    const perEdition = EDITION_VIEW_MAP[originalView]
    if (perEdition) {
      // The Utilities strip is Bedrock-only, so a `photos`/`backups` from it
      // is always the Bedrock panel regardless of the current edition.
      const useBedrock = strip === 'utilities' || isBedrock
      openPanel(useBedrock ? perEdition.bedrock : perEdition.java)
      return
    }
    const target = ORIGINAL_VIEW_MAP[originalView]
    if (target) openPanel(target)
  }

  function openPanel(v: View, tab?: string) {
    navigate(() => {
      if (tab) contentInitialTab = tab
      currentView = v
    })
  }

  function closePanel() {
    navigate(() => {
      currentView = 'home'
    })
  }

  /// Port of `OpenUrl` (Home.razor.cs:670). `window.open` is inert inside the
  /// WebView2 host, so this goes through the `open_url` command.
  function openUrl(url: string) {
    if (url) invoke('open_url', { url }).catch(() => {})
  }

  async function handleLaunch() {
    if (!settings || isLaunching) return
    isLaunching = true
    try {
      if (isBedrock) {
        let dllPath: string | null = null
        if (settings.SelectedClient === 'Flarial Client') {
          dllPath = await invoke<string>('flarial_client_path')
        } else if (settings.SelectedClient === 'OderSo Client') {
          dllPath = await invoke<string>('oderso_client_path')
        } else if (settings.SelectedClient === 'LeviLamina Client') {
          dllPath = await invoke<string>('levilamina_client_path')
        } else if (settings.SelectedClient === 'Custom DLL') {
          dllPath = settings.CustomDllPath || null
        }
        await invoke('launch_bedrock_instance', { dllPath })
      } else {
        await invoke('launch_java_instance', { instanceId: null })
      }
    } catch (e) {
      console.error('Launch failed:', e)
      status = `Launch failed: ${String(e)}`
      showToast(`Launch failed: ${String(e)}`, 'error')
    } finally {
      isLaunching = false
    }
  }

  /**
   * Tail of `HandleVersionLaunch` (Home.Clients.cs:322) and
   * `HandleOderSoEntryLaunch` (:397): both go home, show a launching status,
   * inject the DLL the panel picked, and report. The panels resolve the path;
   * the shell owns the launch, exactly as in the original where these lived on
   * `Home` rather than in the panel markup.
   */
  async function launchWithDll(dllPath: string, label: string) {
    closePanel()
    if (isLaunching) return
    isLaunching = true
    status = `Launching ${label}…`
    try {
      await invoke('launch_bedrock_instance', { dllPath })
      isLaunching = false
      status = 'Injected! Have fun.'
      showToast('Injection successful!', 'success')
    } catch (e) {
      isLaunching = false
      status = String(e)
      showToast(String(e), 'error')
    }
  }

  async function forceCloseLaunch() {
    isLaunching = false
    try {
      await invoke('cancel_launch')
    } catch (e) {
      console.error('Cancel launch failed:', e)
    }
  }

  /// Window controls' fullscreen button + the F11 shortcut. `interop.js`
  /// owns the actual Tauri call and broadcasts the resulting state, so the
  /// button label stays correct even when F11 drives the change.
  function toggleFullscreen() {
    window.toggleFullscreen?.()
  }

  window.addEventListener('glacier:fullscreen-changed', (e: Event) => {
    isFullscreen = (e as CustomEvent<boolean>).detail
  })

  /// Real launcher version + update availability, replacing a hardcoded
  /// 'v0.1.0' and the missing `launcherUpdateAvailable` branch of the
  /// update pill.
  async function loadVersionInfo() {
    try {
      currentVersion = await invoke<string>('launcher_version')
    } catch {
      currentVersion = ''
    }
    try {
      launcherUpdateAvailable = (await invoke<unknown>('check_launcher_update')) !== null
    } catch {
      launcherUpdateAvailable = false
    }
  }

  loadSettings()
  loadVersionInfo()
  loadNotifications()
  refreshClientsBadge()

  window.addEventListener('glacier:kbshortcut', (e: Event) => {
    const name = (e as CustomEvent<string>).detail
    if (name === 'escape') { if (isLaunching) forceCloseLaunch(); else closePanel() }
    else if (name === 'settings') openPanel('settings')
    else if (name === 'clients' && isBedrock) openPanel('clients')
    else if (name === 'launch') handleLaunch()
    else if (name === 'fullscreen') toggleFullscreen()
    else if (name === 'search') openSearch()
  })
</script>

<div
  class="window"
  role="presentation"
  ondragenter={onWindowDragEnter}
  ondragleave={onWindowDragLeave}
  ondragover={onWindowDragOver}
  ondrop={onWindowDrop}
>
  <div class="window-bg" style="background-image: url('/images/bg.jpg');"></div>
  <div class="drag-zone" role="presentation" onmousedown={onDragStart}></div>

  <div class="drop-zone-overlay">
    <i class="fa-solid fa-file-arrow-down"></i>
    <span>Drop a file to import</span>
    <small>DLL clients, .mrpack/.zip/.jar modpacks &amp; mods, or .mcpack/.mcworld/.mctemplate/.mcaddon Bedrock packs</small>
  </div>

  {#if settings && !settings.OnboardingCompleted}
    <OnboardingModal {settings} onChange={saveSettings} />
  {/if}

  {#if searchOpen}
    <div class="search-overlay" role="presentation" onclick={closeSearch}>
      <div class="search-modal" role="presentation" onclick={(e) => e.stopPropagation()}>
        <div class="search-modal-input-row">
          <i class="fa-solid fa-magnifying-glass"></i>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            bind:this={searchInputEl}
            class="search-modal-input"
            placeholder="Search clients, versions, settings..."
            bind:value={searchQuery}
            oninput={() => (searchSelIdx = 0)}
            onkeydown={onSearchKeydown}
          />
          <span class="search-modal-esc">ESC</span>
        </div>
        <div class="search-results">
          {#if searchMatches.length === 0}
            {#if !searchQuery}
              <div class="search-category">Quick Actions</div>
              {#each searchIndex as r, idx (r.label)}
                <div
                  class="search-result-item {searchSelIdx === idx ? 'sel' : ''}"
                  role="presentation"
                  onclick={r.run}
                  onmouseenter={() => (searchSelIdx = idx)}
                >
                  <div class="search-result-icon">
                    {#if r.icon.startsWith('/images/')}
                      <img src={r.icon} alt="" style="width:16px;height:16px;object-fit:contain;" />
                    {:else}
                      <i class={r.icon}></i>
                    {/if}
                  </div>
                  <div class="search-result-text">
                    <span class="search-result-label">{@html highlight(r.label, searchQuery)}</span>
                    {#if r.sub}
                      <span class="search-result-sub">{@html highlight(r.sub, searchQuery)}</span>
                    {/if}
                  </div>
                  {#if r.shortcut}
                    <div class="kbs"><kbd>{r.shortcut}</kbd></div>
                  {/if}
                  <span class="search-result-enter">&#8629;</span>
                </div>
              {/each}
            {:else}
              <div class="search-no-results">No results for "<strong>{searchQuery}</strong>"</div>
            {/if}
          {:else}
            {#each searchGroups as grp (grp.category)}
              <div class="search-category">{grp.category}</div>
              {#each grp.results as r (r.label)}
                {@const fi = searchFlat.indexOf(r)}
                <div
                  class="search-result-item {searchSelIdx === fi ? 'sel' : ''}"
                  role="presentation"
                  onclick={r.run}
                  onmouseenter={() => (searchSelIdx = fi)}
                >
                  <div class="search-result-icon">
                    {#if r.icon.startsWith('/images/')}
                      <img src={r.icon} alt="" style="width:16px;height:16px;object-fit:contain;" />
                    {:else}
                      <i class={r.icon}></i>
                    {/if}
                  </div>
                  <div class="search-result-text">
                    <span class="search-result-label">{@html highlight(r.label, searchQuery)}</span>
                    {#if r.sub}
                      <span class="search-result-sub">{@html highlight(r.sub, searchQuery)}</span>
                    {/if}
                  </div>
                  <span class="search-result-enter">&#8629;</span>
                </div>
              {/each}
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <div class="top-bar">
    <div class="branding">
      <img class="logo-icon" src="/images/icon.png" alt="" />
      <div class="branding-text">
        <span class="branding-title">Glacier Launcher</span>
        <span class="branding-sub">Glacier Productions</span>
      </div>
      {#if launcherUpdateAvailable}
        <button
          class="update-pill"
          onclick={() => openPanel('news')}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip="Launcher update available — click to update"
        >
          <i class="fa-solid fa-arrow-up"></i>
          <span>Update</span>
        </button>
      {:else}
        <button class="update-pill">
          <span>v{currentVersion}</span>
        </button>
      {/if}
    </div>

    <div class="top-bar-spacer"></div>

    {#if settings}
      <div class="edition-switcher" role="presentation" onmousedown={(e) => e.stopPropagation()} data-tooltip="Switch Minecraft edition">
        <button class="edition-tab" class:active={isBedrock} onclick={() => setEdition('bedrock')}>
          <i class="fa-solid fa-cube"></i><span>Bedrock</span>
        </button>
        <button class="edition-tab" class:active={!isBedrock} onclick={() => setEdition('java')}>
          <i class="fa-brands fa-java"></i><span>Java</span>
        </button>
      </div>

      {#if isBedrock}
        <div
          class="topbar-client-chip"
          role="button"
          tabindex="0"
          onclick={cycleClient}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') cycleClient() }}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip="Click to cycle client ({settings.SelectedClient})"
        >
          <div class="chip-dot" class:live={mcRunning}></div>
          {#if CLIENT_IMAGES[settings.SelectedClient]}
            <img class="chip-icon" src={CLIENT_IMAGES[settings.SelectedClient]} alt="" style="width:14px;height:14px;object-fit:contain;" />
          {:else}
            <i class="chip-icon {CLIENT_ICONS[settings.SelectedClient] ?? ''}"></i>
          {/if}
          <span>{settings.SelectedClient}</span>
          <i class="fa-solid fa-chevron-right chip-arrow"></i>
        </div>
      {/if}

      {#if mcRunning}
        <div class="mc-running-pill" data-tooltip="Minecraft is currently running">
          <i class="fa-solid fa-circle"></i>
          <span>MC</span>
        </div>
      {/if}
    {/if}

    <button
      class="search-trigger"
      onclick={openSearch}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Search (Ctrl+K)"
      aria-label="Search"
    >
      <i class="fa-solid fa-magnifying-glass"></i>
    </button>

    {#if settings}
      {#if notifOpen}
        <div class="notif-backdrop" role="presentation" onclick={() => (notifOpen = false)}></div>
      {/if}
      <div class="notif-bell-wrap" role="presentation" onmousedown={(e) => e.stopPropagation()}>
        <button class="notif-bell-btn" onclick={toggleNotifPanel} data-tooltip="Notifications & downloads">
          <i class="fa-solid fa-bell"></i>
          {#if bellBadgeCount > 0}
            <span class="notif-bell-badge">{bellBadgeCount > 9 ? '9+' : bellBadgeCount}</span>
          {/if}
        </button>
        {#if notifOpen}
          <div class="notif-panel">
            {#if downloads.length > 0}
              <div class="notif-panel-header">
                <span>Downloads</span>
                <button class="notif-clear-btn" onclick={() => openPanel('downloads')}>View all</button>
              </div>
              <div class="notif-panel-list" style="max-height:160px;">
                {#each downloads.slice(0, 5) as d (d.id)}
                  <div class="notif-item info">
                    <i class="fa-solid fa-download"></i>
                    <div class="notif-item-body">
                      <div class="notif-item-title" style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">{d.label}</div>
                      <div class="notif-item-msg">
                        {d.status}{d.status === 'Downloading' ? ` · ${Math.round(d.progress * 100)}%` : ''}
                      </div>
                      {#if d.status === 'Downloading'}
                        <div class="progress-bar-wrap" style="margin-top:4px;">
                          <div class="progress-bar-fill" style="width:{Math.round(d.progress * 100)}%"></div>
                        </div>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}

            <div class="notif-panel-header">
              <span>Notifications</span>
              {#if notifications.length > 0}
                <button class="notif-clear-btn" onclick={clearNotifications}>Clear all</button>
              {/if}
            </div>
            <div class="notif-panel-list">
              {#if notifications.length === 0}
                <div class="notif-empty">
                  <i class="fa-regular fa-bell-slash"></i>
                  <span>No notifications</span>
                </div>
              {:else}
                {#each notifications as n (n.Id)}
                  <div class="notif-item {n.Kind}">
                    <i class={n.Icon}></i>
                    <div class="notif-item-body">
                      <div class="notif-item-title">{n.Title}</div>
                      <div class="notif-item-msg">{n.Message}</div>
                      <div class="notif-item-time">{formatRelativeTime(n.TimeIso)}</div>
                    </div>
                    <button
                      class="notif-item-remove"
                      onclick={(e) => { e.stopPropagation(); removeNotification(n.Id) }}
                      aria-label="Dismiss notification"
                    >
                      <i class="fa-solid fa-xmark"></i>
                    </button>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <div class="window-controls">
      <button class="wnd-btn minimize-btn" data-tooltip="Minimize" aria-label="Minimize" onmousedown={(e) => e.stopPropagation()} onclick={() => window.minimizeWindow?.()}>
        <i class="fa-solid fa-minus"></i>
      </button>
      <button
        class="wnd-btn fullscreen-btn"
        data-tooltip={isFullscreen ? 'Exit fullscreen' : 'Fullscreen (F11)'}
        aria-label={isFullscreen ? 'Exit fullscreen' : 'Fullscreen'}
        onmousedown={(e) => e.stopPropagation()}
        onclick={toggleFullscreen}
      >
        <i class="fa-solid {isFullscreen ? 'fa-compress' : 'fa-expand'}"></i>
      </button>
      <button class="wnd-btn maximize-btn" data-tooltip="Maximize" aria-label="Maximize" onmousedown={(e) => e.stopPropagation()} onclick={() => window.maximizeWindow?.()}>
        <i class="fa-solid fa-square"></i>
      </button>
      <button class="wnd-btn close-btn" data-tooltip="Close" aria-label="Close" onmousedown={(e) => e.stopPropagation()} onclick={() => window.closeWindow?.()}>
        <i class="fa-solid fa-xmark"></i>
      </button>
    </div>
  </div>

  {#if showAnnouncementBanner && announcement}
    <div class="announcement-banner {announcement.kind}">
      <i class={announcement.kind === 'maintenance' ? 'fa-solid fa-triangle-exclamation' : 'fa-solid fa-bullhorn'}></i>
      <div class="announcement-banner-text">
        <span class="announcement-banner-title">{announcement.title}</span>
        <span class="announcement-banner-msg">{announcement.message}</span>
      </div>
      {#if announcement.url}
        <button class="btn-sm" onclick={() => openUrl(announcement?.url ?? '')}>Learn more</button>
      {/if}
      {#if announcement.kind !== 'maintenance'}
        <button class="announcement-banner-close" onclick={dismissAnnouncement} data-tooltip="Dismiss">
          <i class="fa-solid fa-xmark"></i>
        </button>
      {/if}
    </div>
  {/if}

  <div class="main-content" class:panel-open={currentView !== 'home'}>
    {#if settings}
      <div class="btn-row">
        <button
          class="action-btn hover-grow launch-btn {isLaunching ? 'btn-wide btn-blue launching' : 'btn-sq'}"
          onclick={handleLaunch}
          disabled={isLaunching}
          data-tooltip="Launch (Ctrl+L)"
        >
          {#if isLaunching}
            <span class="spinner"></span><span>Launching...</span>
          {:else}
            <i class="fa-solid fa-play"></i><span class="btn-label">{t('nav.launch', settings?.Language)}</span>
          {/if}
        </button>

        {#if isLaunching}
          <button class="action-btn btn-sq force-close-btn" onclick={forceCloseLaunch} data-tooltip="Force close launch (Esc)" aria-label="Force close launch">
            <i class="fa-solid fa-stop"></i>
          </button>
        {/if}

        <div class="btn-badge-wrap">
          <button
            class="action-btn btn-sq hover-grow"
            onclick={() => openPanel('settings')}
            onmousedown={(e) => e.stopPropagation()}
            data-tooltip="Settings (Ctrl+,)"
          >
            <i class="fa-solid fa-gear"></i><span class="btn-label">{t('nav.settings', settings?.Language)}</span>
          </button>
        </div>

        {#if isBedrock}
          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('clients')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Clients (Ctrl+1)"
            >
              <i class="fa-solid fa-puzzle-piece"></i><span class="btn-label">{t('nav.clients', settings?.Language)}</span>
            </button>
            {#if clientsHasBadge}<span class="notification-badge"></span>{/if}
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('bedrockpacks')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Packs (Ctrl+2)"
            >
              <i class="fa-solid fa-boxes-stacked"></i><span class="btn-label">Packs</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('bedrockworlds')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Worlds & Servers (Ctrl+3)"
            >
              <i class="fa-solid fa-globe"></i><span class="btn-label">Worlds</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('bedrock-versions')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Versions & Instances (Ctrl+4)"
            >
              <i class="fa-solid fa-box-archive"></i><span class="btn-label">{t('nav.mcversions', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('tweaks')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Bedrock Tweaks"
            >
              <i class="fa-solid fa-wand-magic-sparkles"></i><span class="btn-label">Tweaks</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('utilities')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Utilities — game options, shaders, clients, system"
            >
              <i class="fa-solid fa-screwdriver-wrench"></i><span class="btn-label">Utilities</span>
            </button>
          </div>
        {:else}
          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('launchers')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Launchers (Ctrl+1)"
            >
              <i class="fa-solid fa-rocket"></i><span class="btn-label">{t('nav.launchers', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('java-mods')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Mods (Ctrl+2)"
            >
              <i class="fa-solid fa-puzzle-piece"></i><span class="btn-label">{t('nav.mods', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('mc-versions')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Versions (Ctrl+3)"
            >
              <i class="fa-solid fa-box-archive"></i><span class="btn-label">{t('nav.versions', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('tweaks')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Vanilla Tweaks"
            >
              <i class="fa-solid fa-wand-magic-sparkles"></i><span class="btn-label">Tweaks</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('profile')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Profile"
            >
              <i class="fa-solid fa-user"></i><span class="btn-label">{t('nav.profile', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('java-screenshots')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip="Screenshots"
            >
              <i class="fa-solid fa-images"></i><span class="btn-label">{t('nav.photos', settings?.Language)}</span>
            </button>
          </div>
        {/if}
      </div>

      {#if currentView === 'home'}
        <p class="status-msg visible">{status === 'ready' ? '' : status}</p>

        {#if settings.ShowRecentlyLaunched && editionRecents.length > 0}
          <div class="recently-launched">
            <span class="recently-label">Recent:</span>
            {#each editionRecents.slice(0, 3) as item (item)}
              <button class="recent-item" onmousedown={(e) => e.stopPropagation()}>
                <i class="fa-solid fa-clock-rotate-left"></i>{recentDisplayLabel(item)}
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    {:else}
      <p class="status-msg visible">{status}</p>
    {/if}
  </div>

  {#if settings}
    <Footer {settings} onChange={saveSettings} onOpenMcVersions={() => openPanel('mc-versions')} />
  {/if}

  <div class="toast-container">
    {#each toasts as toast (toast.id)}
      <div class="toast {toast.kind} {toast.closing ? 'closing' : ''}">
        <i class={toast.icon}></i>
        {toast.message}
      </div>
    {/each}
  </div>

  <div class="news-strip">
    <div class="news-track">
      {#each newsItems.length ? [...newsItems, ...newsItems] : [] as item}
        <span
          class="news-item"
          role="link"
          tabindex="0"
          onclick={() => openUrl(item.url)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault()
              openUrl(item.url)
            }
          }}
        >
          <i class={item.icon}></i>
          <span class="news-title">{item.title}</span>
          <span>{item.subtitle}</span>
        </span>
      {/each}
    </div>
  </div>

  {#if currentView !== 'home' && settings}
    <div class="panel-overlay">
      <div class="panel-handle"></div>
      {#if currentView === 'clients'}
        <ClientsView
          {settings}
          onChange={saveSettings}
          onBack={closePanel}
          onOpenVersions={() => openPanel('versions')}
          onNavigate={navigateFromPanel}
        />
      {:else if currentView === 'instances'}
        <InstancesView
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={(m, k) => showToast(m, k ?? 'info')}
        />
      {:else if currentView === 'bedrockworlds'}
        <BedrockWorldsView
          instance={activeBedrockInstance}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onEditLevelDat={(dir) => { levelDatWorldDir = dir; openPanel('leveldat') }}
          onToast={showToast}
        />
      {:else if currentView === 'bedrockpacks'}
        <BedrockPacksView
          instance={activeBedrockInstance}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'bedrockbackups'}
        <BedrockBackupsView
          instance={activeBedrockInstance}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'bedrockscreenshots'}
        <BedrockScreenshotsView
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'themestudio'}
        <ThemeStudioView
          {settings}
          onChange={saveSettings}
          onBack={() => openPanel('settings')}
          onToast={showToast}
        />
      {:else if currentView === 'leveldat'}
        <LevelDatEditorView worldDir={levelDatWorldDir} onBack={() => openPanel('bedrockworlds')} />
      {:else if currentView === 'settings'}
        <SettingsView {settings} onChange={saveSettings} onBack={closePanel} onNavigate={navigateFromPanel} />
      {:else if currentView === 'java-mods'}
        <JavaModsView {settings} onBack={closePanel} onNavigate={navigateFromPanel} />
      {:else if currentView === 'modpacks'}
        <ModpacksView {settings} onBack={closePanel} />
      {:else if currentView === 'versions'}
        <VersionsView
          {settings}
          onBack={() => openPanel('clients')}
          onSaveSettings={saveSettings}
          onSelectClient={(name) => settings && saveSettings({ ...settings, SelectedClient: name })}
          onLaunchDll={launchWithDll}
          onToast={showToast}
        />
      {:else if currentView === 'tweaks'}
        <TweaksView
          edition={isBedrock ? 'Bedrock' : 'Java'}
          minecraftDir={(isBedrock ? activeBedrockInstance?.directory : activeJavaInstance?.directory) ?? ''}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'servers'}
        <ServersView
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={(m, k) => showToast(m, k ?? 'info')}
        />
      {:else if currentView === 'logs'}
        <LogsPanel instanceDir={(isBedrock ? activeBedrockInstance?.directory : activeJavaInstance?.directory) ?? ''} onBack={closePanel} />
      {:else if currentView === 'stats'}
        <StatsPanel onBack={closePanel} />
      {:else if currentView === 'skinlibrary'}
        <SkinLibraryView
          {settings}
          onBack={closePanel}
          onSaveSettings={saveSettings}
          onToast={showToast}
        />
      {:else if currentView === 'java-backups'}
        <JavaBackupsView
          instanceName={activeJavaInstance?.name ?? ''}
          onBack={closePanel}
          onToast={showToast}
        />
      {:else if currentView === 'levilamina-mods'}
        <LeviLaminaModsView onBack={() => openPanel('clients')} onToast={showToast} />
      {:else if currentView === 'mc-versions'}
        <MCVersionsView
          {settings}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onSaveSettings={saveSettings}
          onToast={showToast}
        />
      {:else if currentView === 'launchers'}
        <LauncherClientsView
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'bedrock-versions'}
        <BedrockVanillaVersionsView
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={(m, k) => showToast(m, k ?? 'info')}
        />
      {:else if currentView === 'profile'}
        <ProfileView
          {settings}
          onChange={saveSettings}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'java-screenshots'}
        <JavaScreenshotsView onBack={closePanel} onNavigate={navigateFromPanel} />
      {:else if currentView === 'utilities'}
        <UtilitiesView
          instanceDir={activeBedrockInstance?.directory ?? ''}
          initialTab={utilInitialTab}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'clientlibrary'}
        <ClientLibraryView
          instanceDir={activeBedrockInstance?.directory ?? ''}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
        />
      {:else if currentView === 'downloads'}
        <DownloadsView onBack={closePanel} onOpenSettings={() => openPanel('settings')} onOpenCredits={() => openPanel('credits')} />
      {:else if currentView === 'news'}
        <NewsView onBack={closePanel} onOpenSettings={() => openPanel('settings')} onOpenCredits={() => openPanel('credits')} />
      {:else if currentView === 'credits'}
        <CreditsView
          edition={isBedrock ? 'bedrock' : 'java'}
          onBack={closePanel}
          onNavigate={navigateFromPanel}
        />
      {/if}

    </div>
  {/if}
</div>
