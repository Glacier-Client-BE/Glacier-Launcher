<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import type { LauncherSettings } from './lib/settings'
  import { setLanguage } from './lib/i18nStore.svelte'
  import PanelHeader from './lib/components/PanelHeader.svelte'
  // ThemeStudioView is no longer a standalone panel — it's embedded inside
  // SettingsView's Looks category — but SettingsView itself imports it.
  import Footer from './lib/Footer.svelte'
  import { formatChangelog } from './lib/format'
  import Modal from './lib/components/Modal.svelte'
  import LazyView from './lib/components/LazyView.svelte'
  // Panel views are code-split: each loads on first open (see LazyView).
  const views = {
    InstancesView: () => import('./lib/InstancesView.svelte'),
    JavaInstancesView: () => import('./lib/JavaInstancesView.svelte'),
    LevelDatEditorView: () => import('./lib/LevelDatEditorView.svelte'),
    BedrockWorldsView: () => import('./lib/BedrockWorldsView.svelte'),
    BedrockPacksView: () => import('./lib/BedrockPacksView.svelte'),
    BrowseAddonsView: () => import('./lib/BrowseAddonsView.svelte'),
    BedrockBackupsView: () => import('./lib/BedrockBackupsView.svelte'),
    BedrockScreenshotsView: () => import('./lib/BedrockScreenshotsView.svelte'),
    ClientsView: () => import('./lib/ClientsView.svelte'),
    SettingsView: () => import('./lib/SettingsView.svelte'),
    JavaModsView: () => import('./lib/JavaModsView.svelte'),
    ModpacksView: () => import('./lib/ModpacksView.svelte'),
    VersionsView: () => import('./lib/VersionsView.svelte'),
    TweaksView: () => import('./lib/TweaksView.svelte'),
    ServersView: () => import('./lib/ServersView.svelte'),
    LogsPanel: () => import('./lib/LogsPanel.svelte'),
    StatsPanel: () => import('./lib/StatsPanel.svelte'),
    LeviLaminaModsView: () => import('./lib/LeviLaminaModsView.svelte'),
    MCVersionsView: () => import('./lib/MCVersionsView.svelte'),
    ProfileView: () => import('./lib/ProfileView.svelte'),
    ImportLauncherView: () => import('./lib/ImportLauncherView.svelte'),
    JavaBackupsView: () => import('./lib/JavaBackupsView.svelte'),
    SkinLibraryView: () => import('./lib/SkinLibraryView.svelte'),
    JavaScreenshotsView: () => import('./lib/JavaScreenshotsView.svelte'),
    InstanceWizard: () => import('./lib/InstanceWizard.svelte'),
    InstanceDetailView: () => import('./lib/InstanceDetailView.svelte'),
    ModrinthBrowseV2: () => import('./lib/ModrinthBrowseV2.svelte'),
    SyncPacksView: () => import('./lib/SyncPacksView.svelte'),
    LauncherClientsView: () => import('./lib/LauncherClientsView.svelte'),
    BedrockVanillaVersionsView: () => import('./lib/BedrockVanillaVersionsView.svelte'),
    UtilitiesView: () => import('./lib/UtilitiesView.svelte'),
    ClientLibraryView: () => import('./lib/ClientLibraryView.svelte'),
    DownloadsView: () => import('./lib/DownloadsView.svelte'),
    NewsView: () => import('./lib/NewsView.svelte'),
    CreditsView: () => import('./lib/CreditsView.svelte'),
  } as const
  import { listen } from '@tauri-apps/api/event'
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
  import OnboardingModal from './lib/OnboardingModal.svelte'
  import UpdateAvailableModal from './lib/components/UpdateAvailableModal.svelte'
  import OnboardingChecklist from './lib/OnboardingChecklist.svelte'
  import { t } from './lib/i18n'
  import type { BedrockInstance } from './lib/settings'

  type View =
    | 'home'
    | 'clients'
    | 'instances'
    | 'java-instances'
    | 'bedrockworlds'
    | 'bedrockpacks'
    | 'bedrock-addons'
    | 'bedrockbackups'
    | 'bedrockscreenshots'
    | 'leveldat'
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
    | 'instance-wizard'
    | 'instance-detail'
    | 'modrinth-browse-v2'
    | 'sync-packs'
    | 'import-launcher'


  type JavaInstance = { id: string; directory: string; name?: string }

  let settings = $state<LauncherSettings | null>(null)
  let currentView = $state<View>('home')
  /// Navigation history stack for real "back to parent" behavior + a
  /// breadcrumb trail. Each entry is the view being navigated AWAY from
  /// (with a human label) at the moment `openPanel` opens a new view on top
  /// of it. Cleared whenever the user returns to Home, since Home is always
  /// the root of every chain.
  type NavEntry = { view: View; label: string }
  let navHistory = $state<NavEntry[]>([])
  let contentInitialTab = $state('worlds')
  /// World whose level.dat is open in the NBT editor - the original's
  /// `levelDatEditing` deep-link from a world row.
  let levelDatWorldDir = $state('')
  /// Sub-tab the Utilities panel should open on (the original's `_utilTab`).
  let utilInitialTab = $state('options')
  /// Category the Settings panel should open on — set before openPanel('settings')
  /// by deep links like the search palette's Theme Studio entry (now folded
  /// into the Looks category rather than its own panel).
  let settingsInitialCategory = $state<'all' | 'appearance'>('all')
  let status = $state('loading…')
  let activeBedrockInstance = $state<BedrockInstance | null>(null)
  let activeJavaInstance = $state<JavaInstance | null>(null)
  let isLaunching = $state(false)

  /**
   * Feature 27 — auto-fallback to vanilla after repeated injection failures.
   *
   * Purely a UI-level counter around the existing `launch_bedrock_instance`
   * call: it does not touch gl-injection or the launch command itself. After
   * 3 consecutive failed launches with the same non-Vanilla client, offer to
   * launch Vanilla instead rather than silently letting the user retry the
   * same broken injection forever. Any successful launch, or switching
   * clients, resets the count.
   */
  const injectionFailureCounts: Record<string, number> = {}
  let vanillaFallbackPrompt = $state<{ client: string } | null>(null)

  function recordLaunchOutcome(client: string, succeeded: boolean) {
    if (client === 'Vanilla' || succeeded) {
      delete injectionFailureCounts[client]
      if (succeeded) vanillaFallbackPrompt = null
      return
    }
    const count = (injectionFailureCounts[client] ?? 0) + 1
    injectionFailureCounts[client] = count
    if (count >= 3) {
      vanillaFallbackPrompt = { client }
    }
  }

  async function launchVanillaFallback() {
    const failedClient = vanillaFallbackPrompt?.client
    vanillaFallbackPrompt = null
    if (failedClient) delete injectionFailureCounts[failedClient]
    if (!settings) return
    await saveSettings({ ...settings, SelectedClient: 'Vanilla' })
    await handleLaunch()
  }

  function dismissVanillaFallbackPrompt() {
    vanillaFallbackPrompt = null
  }
  type Announcement = { id: string; title: string; message: string; kind: string; url: string }
  let announcement = $state<Announcement | null>(null)
  let dragDepth = 0
  let currentVersion = $state('')
  let launcherUpdateAvailable = $state(false)
  let isFullscreen = $state(false)
  /// Was auto-set per view (wide for dashboard-style panels, narrow for the
  /// rest) and reset on every navigation, so toggling it by hand never
  /// stuck — the very next panel switch silently overrode whatever the user
  /// picked. Now a single manually-controlled preference shared by every
  /// panel: expand or collapse once and it stays that way across all of
  /// them until toggled again.
  let panelExpanded = $state(true)

  const panelSide = $derived(settings?.PanelSide ?? 'right')

  function togglePanelExpand() {
    setPanelExpanded(!panelExpanded)
  }

  type NewsItem = { icon: string; title: string; subtitle: string; url: string }
  let newsItems = $state<NewsItem[]>([])

  const isBedrock = $derived(settings?.Edition !== 'java')

  // Client chip + live-running indicator (top-bar): mirrors CycleClient /
  // mcRunning from Home.Clients.cs / Home.razor. Bedrock is detected by
  // scanning for the UWP process (is_bedrock_game_running); Java is detected
  // by checking the PID of the most recently launched Java process
  // (is_java_game_running) since it isn't UWP-packaged.
  const CLIENT_ICONS: Record<string, string> = {
    'Custom DLL': 'fa-solid fa-file-code',
  }
  const CLIENT_IMAGES: Record<string, string> = {
    'Flarial Client': '/images/clients/flarial.svg',
    'OderSo Client': '/images/clients/oderso.svg',
    'Latite Client': '/images/clients/latite.svg',
    'LeviLamina Client': '/images/clients/levi-lamina.svg',
    Vanilla: '/images/mc-logo.svg',
  }
  const CYCLE_CLIENTS = ['Vanilla', 'Flarial Client', 'Latite Client', 'OderSo Client', 'LeviLamina Client' ]

  let mcRunning = $state(false)

  /// Port of `clientsHasBadge` (Home.razor.cs:363): the Clients button shows
  /// a notification dot while either bundled client is missing from disk.
  /// Toasts - port of `ToastItem` + `ShowToast` (Home.razor.cs:231, 635).
  /// The original holds each toast for 2800ms, flips it to `.closing` for a
  /// 240ms exit animation, then drops it.
  type ToastAction = { label: string; icon?: string; run: () => void }
  type ToastItem = { id: number; message: string; kind: string; icon: string; closing: boolean; action?: ToastAction }
  let toasts = $state<ToastItem[]>([])
  let toastSeq = 0

  /** Same category heuristic as `notificationCategory`, applied to a toast's
   *  own text before it exists as an `AppNotification` yet — used to honor
   *  `MutedNotificationCategories` before ever recording the toast, and by
   *  `notificationCategory` itself once it's a real list entry. */
  function textCategory(text: string): 'downloads' | 'updates' | 'social' | 'all' {
    const hay = text.toLowerCase()
    if (hay.includes('download')) return 'downloads'
    if (hay.includes('update') || hay.includes('launcher-update') || hay.includes('version')) return 'updates'
    if (hay.includes('discord') || hay.includes('xbox') || hay.includes('friend') || hay.includes('social')) return 'social'
    return 'all'
  }

  function showToast(message: string, kind: string = 'info', action?: ToastAction) {
    const icon =
      kind === 'success'
        ? 'fa-solid fa-circle-check'
        : kind === 'error'
          ? 'fa-solid fa-circle-xmark'
          : kind === 'warning'
            ? 'fa-solid fa-triangle-exclamation'
            : 'fa-solid fa-circle-info'
    const id = ++toastSeq
    toasts = [...toasts, { id, message, kind, icon, closing: false, action }]
    // A toast carrying a next-step button stays up longer so it can be clicked.
    setTimeout(() => {
      toasts = toasts.map((t) => (t.id === id ? { ...t, closing: true } : t))
      setTimeout(() => {
        toasts = toasts.filter((t) => t.id !== id)
      }, 240)
    }, action ? 6000 : 2800)

    // QOL "toast history": every toast is also recorded into the existing
    // notification bell/history, unless its inferred category is muted —
    // toasts previously vanished forever the moment their 2.8s popup ended,
    // with no way to see what you missed.
    if (!settings?.MutedNotificationCategories?.includes(textCategory(message))) {
      const title = kind === 'error' ? 'Error' : kind === 'success' ? 'Success' : kind === 'warning' ? 'Warning' : 'Info'
      invoke('add_notification', { title, message, kind }).then(() => loadNotifications()).catch(() => {})
    }
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
    if (document.hidden) return
    try {
      mcRunning = await invoke<boolean>(
        isBedrock ? 'is_bedrock_game_running' : 'is_java_game_running',
      )
    } catch {
      mcRunning = false
    }
  }

  let clientMenuOpen = $state(false)

  // Java edition's client picker: Vanilla plus the clients Glacier installs
  // itself as a standalone mod (Essential, Dawn, NoRisk) always; Horizon and
  // Lunar only when they are actually on this PC. The pick is a per-machine
  // preference.
  const JAVA_CLIENT_IMAGES: Record<string, string> = {
    Vanilla: '/images/mc-java-logo.svg',
    Horizon: '/images/clients/horizon.svg',
    Lunar: '/images/clients/lunar.svg',
    Essential: '/images/clients/essential.svg',
    Dawn: '/images/clients/dawn.svg',
    NoRisk: '/images/clients/norisk.svg',
  }
  const MOD_CLIENTS = ['Essential', 'Dawn', 'NoRisk']
  let lunarExe = $state<string | null>(null)
  let horizonInstalled = $state(false)
  let javaClient = $state(readJavaClient())
  const javaClients = $derived([
    'Vanilla',
    ...(horizonInstalled ? ['Horizon'] : []),
    ...MOD_CLIENTS,
    ...(lunarExe ? ['Lunar'] : []),
  ])
  const activeJavaClient = $derived(javaClients.includes(javaClient) ? javaClient : 'Vanilla')

  function readJavaClient(): string {
    try {
      return localStorage.getItem('glacier.javaClient') ?? 'Vanilla'
    } catch {
      return 'Vanilla'
    }
  }

  function selectJavaClient(name: string) {
    clientMenuOpen = false
    javaClient = name
    try {
      localStorage.setItem('glacier.javaClient', name)
    } catch {}
  }

  async function refreshJavaClientOptions() {
    const [lb, hz] = await Promise.allSettled([
      invoke<{ lunar_exe_path: string | null; badlion_exe_path: string | null }>('detect_lunar_badlion'),
      invoke<{ installed: boolean }>('horizon_status'),
    ])
    lunarExe = lb.status === 'fulfilled' ? (lb.value.lunar_exe_path ?? lb.value.badlion_exe_path) : null
    horizonInstalled = hz.status === 'fulfilled' && hz.value.installed
  }

  function toggleClientMenu() {
    clientMenuOpen = !clientMenuOpen
  }

  function selectClientFromMenu(name: string) {
    clientMenuOpen = false
    if (!settings || settings.SelectedClient === name) return
    saveSettings({ ...settings, SelectedClient: name })
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

  // Feature 41: a small client-side category filter for the notification
  // list. There's no structured category field on AppNotification yet, so
  // categorization is inferred from the icon/title the same way the bell
  // panel already picks an icon per notification.
  type NotifCategory = 'all' | 'downloads' | 'updates' | 'social'
  let notifFilter = $state<NotifCategory>('all')

  function notificationCategory(n: AppNotification): NotifCategory {
    return textCategory(`${n.Icon} ${n.Title} ${n.Message}`)
  }

  const filteredNotifications = $derived(
    notifFilter === 'all' ? notifications : notifications.filter((n) => notificationCategory(n) === notifFilter),
  )

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
    try {
      await invoke('remove_notification', { id })
      await loadNotifications()
    } catch (e) {
      showToast(`Could not remove notification: ${e}`, 'error')
    }
  }

  async function clearNotifications() {
    try {
      await invoke('clear_notifications')
      await loadNotifications()
    } catch (e) {
      showToast(`Could not clear notifications: ${e}`, 'error')
    }
  }

  const activeDownloadCount = $derived(downloads.filter((d) => d.status === 'Downloading').length)
  /// Port of the original's `bellBadgeCount` (Home.razor:523).
  const bellBadgeCount = $derived(unreadCount + activeDownloadCount)

  /** QOL: OS-level notification for long operations finishing while the
   *  window doesn't have focus - opt-in (`DesktopNotificationsEnabled`,
   *  default off) since it needs an OS permission prompt on first use and
   *  shouldn't surprise anyone who never asked for it. */
  async function sendDesktopNotification(title: string, body: string) {
    if (!settings?.DesktopNotificationsEnabled) return
    if (document.hasFocus()) return
    try {
      let granted = await isPermissionGranted()
      if (!granted) granted = (await requestPermission()) === 'granted'
      if (granted) sendNotification({ title, body })
    } catch {
      // Best-effort — never let a notification failure surface as an error.
    }
  }

  // Live download-queue subscription so a desktop notification can fire on
  // the "something was downloading, now nothing is" transition, without
  // polling — the bell panel otherwise only fetches downloads on demand
  // when opened.
  let prevActiveDownloads = 0
  listen<DownloadEntry[]>('downloads-changed', (e) => {
    downloads = e.payload
    const activeNow = e.payload.filter((d) => d.status === 'Downloading' || d.status === 'Queued').length
    if (prevActiveDownloads > 0 && activeNow === 0) {
      sendDesktopNotification('Downloads complete', 'All downloads have finished.')
    }
    prevActiveDownloads = activeNow
  })

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
  // import, Utilities sub-tabs, Xbox/Discord modals) are omitted rather than
  // rendered as dead rows - see PARITY.md. Theme Studio, Skin Library and the
  // Bedrock/Java Backups panels now have their own entries below. The DOM,
  // grouping, highlighting and key handling are faithful.
  type SearchResult = {
    icon: string
    label: string
    category: string
    sub?: string
    shortcut?: string
    run: () => void
    /// Entries the original ships but this rewrite hasn't wired up yet.
    /// Shown greyed-out with a tooltip instead of being silently absent,
    /// so Ctrl+K doesn't feel like it's missing things at random.
    disabled?: boolean
  }

  let searchOpen = $state(false)
  let searchQuery = $state('')
  let searchSelIdx = $state(0)
  let searchInputEl = $state<HTMLInputElement | null>(null)
  /// Redesign: the empty-query view used to dump all ~36 entries (every
  /// category at once) into a grid mislabeled "Quick Actions". Now the
  /// empty-query grid only shows the actual Quick Actions category, and a
  /// row of category pills lets you jump into any other category (or type
  /// to search across all of them, same as before).
  let searchCategoryFilter = $state<string | null>(null)

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
    { icon: 'fa-solid fa-gear', label: 'Open Settings', category: 'Quick Actions', sub: 'Injection, Appearance, Account', shortcut: 'Ctrl+,', run: () => { closeSearch(); openPanel('settings') } },
    { icon: 'fa-solid fa-puzzle-piece', label: 'Manage Clients', category: 'Quick Actions', sub: 'Flarial, Latite, OderSo, Custom DLL', shortcut: 'Ctrl+2', run: () => { closeSearch(); openPanel('clients') } },
    { icon: 'fa-solid fa-server', label: 'Servers', category: 'Quick Actions', sub: 'Quick-launch into saved servers', shortcut: 'Ctrl+5', run: () => { closeSearch(); openPanel('servers') } },
    { icon: 'fa-solid fa-clock-rotate-left', label: 'Browse Versions', category: 'Quick Actions', sub: 'Download and manage client versions', run: () => { closeSearch(); openPanel('versions') } },
    { icon: 'fa-solid fa-box-archive', label: 'MC Versions', category: 'Quick Actions', sub: 'Switch Minecraft Bedrock versions', run: () => { closeSearch(); openPanel('mc-versions') } },
    { icon: 'fa-solid fa-screwdriver-wrench', label: 'Utilities', category: 'Quick Actions', sub: 'Game options, shaders, client library, system info', run: () => { closeSearch(); openPanel('utilities') } },
    { icon: 'fa-solid fa-file-code', label: 'Client Library', category: 'Bedrock', sub: 'Your own DLL / EXE clients with auto-update', run: () => { closeSearch(); openPanel('clientlibrary') } },
    { icon: 'fa-solid fa-boxes-stacked', label: 'Packs', category: 'Bedrock', sub: 'Resource, behavior and skin packs', run: () => { closeSearch(); openPanel('bedrock-addons') } },
    { icon: 'fa-solid fa-globe', label: 'Worlds', category: 'Bedrock', sub: 'Manage Bedrock worlds', run: () => { closeSearch(); openPanel('bedrockworlds') } },
    { icon: 'fa-solid fa-layer-group', label: 'LeviLamina Mods', category: 'Bedrock', sub: 'Manage LeviLamina mods', run: () => { closeSearch(); openPanel('levilamina-mods') } },
    { icon: 'fa-solid fa-wand-magic-sparkles', label: 'Tweaks', category: 'Bedrock', sub: 'Vanilla and Bedrock Tweaks packs', run: () => { closeSearch(); openPanel('tweaks') } },
    { icon: 'fa-solid fa-box-archive', label: 'Vanilla Bedrock Versions', category: 'Bedrock', sub: 'Install and switch vanilla Bedrock game versions', run: () => { closeSearch(); openPanel('bedrock-versions') } },
    { icon: 'fa-solid fa-clock-rotate-left', label: 'Bedrock Backups', category: 'Bedrock', sub: 'Back up and restore Bedrock worlds', run: () => { closeSearch(); openPanel('bedrockbackups') } },
    { icon: 'fa-solid fa-camera', label: 'Bedrock Photos', category: 'Bedrock', sub: 'Bedrock screenshots gallery', run: () => { closeSearch(); openPanel('bedrockscreenshots') } },
    { icon: '/images/mc-java-logo.svg', label: 'Java Versions', category: 'Java', sub: 'Install or launch Java Edition', run: () => { closeSearch(); openPanel('versions') } },
    { icon: 'fa-solid fa-cubes', label: 'Browse Modpacks', category: 'Java', sub: 'Install CurseForge / Modrinth modpacks into an instance', run: () => { closeSearch(); openPanel('modpacks') } },
    { icon: 'fa-solid fa-puzzle-piece', label: 'Mods', category: 'Java', sub: 'Manage installed mods', run: () => { closeSearch(); openPanel('java-mods') } },
    { icon: 'fa-solid fa-file-lines', label: 'Logs and Crashes', category: 'Java', sub: 'View logs, detect crashes, share to mclo.gs', run: () => { closeSearch(); openPanel('logs') } },
    { icon: 'fa-solid fa-clock-rotate-left', label: 'Java Backups', category: 'Java', sub: 'Back up and restore Java worlds', run: () => { closeSearch(); openPanel('java-backups') } },
    { icon: 'fa-solid fa-chart-simple', label: 'Statistics', category: 'Navigation', sub: 'Playtime, sessions and launch history', run: () => { closeSearch(); openPanel('stats') } },
    { icon: 'fa-solid fa-cubes', label: 'Instances', category: 'Navigation', sub: 'Manage game instances', run: () => { closeSearch(); openPanel(isBedrock ? 'instances' : 'java-instances') } },
    { icon: 'fa-solid fa-rocket', label: 'Launchers', category: 'Launchers', sub: 'Lunar, Badlion and other launchers', run: () => { closeSearch(); openPanel('launchers') } },
    { icon: 'fa-solid fa-user', label: 'Profile', category: 'Account', sub: 'Your account and skin', run: () => { closeSearch(); openPanel('profile') } },
    { icon: 'fa-solid fa-shirt', label: 'Skin Library', category: 'Account', sub: 'Manage and apply your saved skins', run: () => { closeSearch(); openPanel('skinlibrary') } },
    { icon: 'fa-solid fa-images', label: 'Photos', category: 'Java', sub: 'Java screenshots gallery', run: () => { closeSearch(); openPanel('java-screenshots') } },
    { icon: 'fa-solid fa-download', label: 'Downloads', category: 'Quick Actions', sub: 'Active and recent downloads', run: () => { closeSearch(); openPanel('downloads') } },
    { icon: 'fa-solid fa-palette', label: 'Theme Studio', category: 'Settings', sub: 'Customize the launcher appearance', run: () => { closeSearch(); settingsInitialCategory = 'appearance'; openPanel('settings') } },
    { icon: 'fa-solid fa-file-export', label: 'Export Settings', category: 'Settings', sub: 'Save settings as JSON', run: () => { closeSearch(); invoke('export_settings').catch(() => {}) } },
    { icon: 'fa-solid fa-expand', label: 'Toggle Fullscreen', category: 'Quick Actions', sub: 'F11 fullscreen mode', shortcut: 'F11', run: () => { closeSearch(); toggleFullscreen() } },
    { icon: '/images/clients/flarial.svg', label: 'Select Flarial', category: 'Clients', sub: 'Switch active client to Flarial', run: () => { closeSearch(); selectClient('Flarial Client') } },
    { icon: '/images/clients/latite.svg', label: 'Select Latite', category: 'Clients', sub: 'Switch active client to Latite', run: () => { closeSearch(); selectClient('Latite Client') } },
    { icon: '/images/clients/oderso.svg', label: 'Select OderSo', category: 'Clients', sub: 'Switch active client to OderSo', run: () => { closeSearch(); selectClient('OderSo Client') } },
    { icon: 'fa-solid fa-cube', label: 'Select Vanilla', category: 'Clients', sub: 'Launch Minecraft without a client', run: () => { closeSearch(); selectClient('Vanilla') } },
    { icon: 'fa-solid fa-newspaper', label: 'News and Updates', category: 'About', sub: 'Changelogs and announcements', run: () => { closeSearch(); openPanel('news') } },
    { icon: 'fa-solid fa-heart', label: 'Credits', category: 'About', sub: 'View launcher credits', run: () => { closeSearch(); openPanel('credits') } },

    // Not wired up yet in this rewrite (see scope note above / PARITY.md) -
    // shown disabled instead of omitted, so search doesn't feel incomplete
    // for no visible reason.
    { icon: 'fa-solid fa-folder-open', label: 'Open Game Folder', category: 'Quick Actions', sub: 'Not available yet', run: () => {}, disabled: true },
    { icon: 'fa-solid fa-file-import', label: 'Import Settings', category: 'Settings', sub: 'Not available yet', run: () => {}, disabled: true },
    { icon: 'fa-brands fa-xbox', label: 'Xbox Account', category: 'Account', sub: 'Not available yet', run: () => {}, disabled: true },
    { icon: 'fa-brands fa-discord', label: 'Discord Account', category: 'Account', sub: 'Not available yet', run: () => {}, disabled: true },
  ])

  /// The category pill row's options, in first-seen order.
  const searchCategories = $derived.by(() => {
    const seen: string[] = []
    for (const r of searchIndex) if (!seen.includes(r.category)) seen.push(r.category)
    return seen
  })

  const quickActions = $derived(searchIndex.filter((r) => r.category === 'Quick Actions'))

  /// QOL: Home-screen quick-access row — pin/unpin any search-index entry
  /// (keyed by its label, which is a fixed string per entry) so its own
  /// panel button is one click away without opening search first.
  const pinnedPanels = $derived(
    (settings?.PinnedPanels ?? [])
      .map((label) => searchIndex.find((r) => r.label === label))
      .filter((r): r is SearchResult => !!r),
  )
  function isPinned(r: SearchResult): boolean {
    return settings?.PinnedPanels?.includes(r.label) ?? false
  }
  function togglePinned(r: SearchResult) {
    if (!settings) return
    const next = isPinned(r)
      ? settings.PinnedPanels.filter((l) => l !== r.label)
      : [...settings.PinnedPanels, r.label]
    saveSettings({ ...settings, PinnedPanels: next })
  }

  /// True once there's a reason to show the grouped/filtered list instead of
  /// the empty-query Quick Actions grid: a typed query, or a category pill.
  const showingSearchList = $derived(searchQuery.trim().length > 0 || !!searchCategoryFilter)

  /// A query filters across label, sub and category; a category pill narrows
  /// to that category first. Either (or both) together.
  const searchMatches = $derived.by(() => {
    let list = searchCategoryFilter
      ? searchIndex.filter((r) => r.category === searchCategoryFilter)
      : searchIndex
    const q = searchQuery.trim().toLowerCase()
    if (q) {
      list = list.filter(
        (r) =>
          r.label.toLowerCase().includes(q) ||
          (r.sub ?? '').toLowerCase().includes(q) ||
          r.category.toLowerCase().includes(q),
      )
    }
    return list
  })

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
    showingSearchList ? searchGroups.flatMap((g) => g.results) : quickActions,
  )

  function selectSearchCategory(c: string | null) {
    searchCategoryFilter = searchCategoryFilter === c ? null : c
    searchSelIdx = 0
  }

  /// "More" (home screen) opens a browsable directory of every panel,
  /// grouped by category — distinct from the command palette (Ctrl+K),
  /// which is search-first and only shows something once you type or pick
  /// a pill. This is the opposite: everything visible at once, nothing to
  /// type, for people who don't already know what screens exist to search
  /// for. Reuses `searchIndex` — the same data, browsed instead of searched.
  /// Tabbed by category (one at a time), matching News' changelog tabs.
  let moreOpen = $state(false)
  let moreCategory = $state('')

  const moreItems = $derived(searchIndex.filter((r) => r.category === moreCategory))

  function openMore() {
    moreOpen = true
    moreCategory = searchCategories[0] ?? ''
  }

  function closeMore() {
    moreOpen = false
  }

  function openSearch() {
    searchOpen = true
    searchQuery = ''
    searchCategoryFilter = null
    searchSelIdx = 0
    queueMicrotask(() => searchInputEl?.focus())
  }

  function closeSearch() {
    searchOpen = false
    searchQuery = ''
    searchCategoryFilter = null
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
    dragDepth = 0
    window.setDropActive?.(false)
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
    if (ext === 'jar') {
      // Feature 19: local mod jar drag-and-drop. There's no per-card
      // Instances grid for Java instances (unlike Bedrock's), so this drops
      // into the currently active Java instance's mods folder rather than a
      // specific card — the closest equivalent target in this UI.
      if (!activeJavaInstance) {
        status = 'No active Java instance to install the mod into.'
        showToast('No active Java instance to install the mod into.', 'error')
        return
      }
      try {
        const fileName = await invoke<string>('install_local_mod_jar', {
          instanceId: activeJavaInstance.id,
          sourcePath: path,
        })
        status = `Installed "${fileName}" into ${activeJavaInstance.name ?? 'the active instance'}.`
        showToast(`Installed "${fileName}".`, 'success')
      } catch (e) {
        status = `Import failed: ${e}`
        showToast(`Import failed: ${e}`, 'error')
      }
      return
    }
    // Java modpack imports (.mrpack/.zip) aren't wired to a backend
    // handler yet — surface that honestly instead of silently discarding
    // the drop.
    status = `Importing .${ext || '?'} files isn't wired up yet.`
  }

  getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type === 'enter' || event.payload.type === 'over') {
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

  // ------------------------------------------- quick-switch (top-bar, Java)

  /** Feature 4: quick-switch instance dropdown in the top bar, for jumping
   *  between Java instances without opening the full Instances/Profile
   *  panel. Reuses `list_java_instances` + the same
   *  `JavaActiveInstanceId`/`saveSettings` mechanism `ProfileView`'s
   *  `switchInstance` already uses. */
  let quickSwitchInstances = $state<JavaInstance[]>([])

  async function loadQuickSwitchInstances() {
    if (!settings || settings.Edition === 'bedrock') {
      quickSwitchInstances = []
      return
    }
    try {
      quickSwitchInstances = await invoke<JavaInstance[]>('list_java_instances')
    } catch {
      quickSwitchInstances = []
    }
  }

  async function quickSwitchInstance(id: string) {
    if (!settings || id === settings.JavaActiveInstanceId) return
    await saveSettings({ ...settings, JavaActiveInstanceId: id })
    await loadActiveJavaInstance()
    await loadPrelaunchWarnings()
  }

  // --------------------------------------------------- pre-launch warnings

  /** Feature 1: non-blocking pre-launch health check (Java version match,
   *  configured heap vs available RAM, free disk space). Never blocks
   *  Launch — just an advisory banner above the launch button. */
  let prelaunchWarnings = $state<{ level: string; message: string }[]>([])

  async function loadPrelaunchWarnings() {
    if (!settings || isBedrock) {
      prelaunchWarnings = []
      return
    }
    try {
      prelaunchWarnings = await invoke('java_prelaunch_check', { id: null })
    } catch {
      prelaunchWarnings = []
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
      await loadQuickSwitchInstances()
      await loadPrelaunchWarnings()
      await loadNews()
      await loadAnnouncement()
      await pollMcRunning()
      setInterval(pollMcRunning, 5000)
    } catch (e) {
      status = `error: ${e}`
    }
  }

  // QOL: single-level "undo last settings change" — the snapshot right
  // before the most recent save, so one click restores it. Deliberately not
  // reactive state: it's pure bookkeeping nothing renders directly from,
  // and making it `$state` would mean every settings save (i.e. every
  // Settings-panel interaction) triggers an extra reactive write for no
  // visible effect.
  let lastSettingsSnapshot: LauncherSettings | null = null
  let canUndoSettings = $state(false)

  async function saveSettings(next: LauncherSettings) {
    if (settings) lastSettingsSnapshot = settings
    canUndoSettings = lastSettingsSnapshot !== null
    settings = next
    applyTheme()
    try {
      await invoke('set_settings', { settings: next })
    } catch (e) {
      status = `error: ${e}`
      showToast(`Could not save settings: ${e}`, 'error')
    }
  }

  function undoLastSettingsChange() {
    if (!lastSettingsSnapshot) return
    const restore = lastSettingsSnapshot
    lastSettingsSnapshot = null
    canUndoSettings = false
    saveSettings(restore)
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
    window.setKeyboardShortcuts?.(settings.KeyboardShortcuts)
    window.setTooltipDelay?.(settings.TooltipDelayMs)
    void applyActiveCustomTheme(settings.ActiveThemeId)
  }

  // A saved custom theme overrides the quick-look preset. Without this the
  // theme only showed while Theme Studio was open and vanished on restart.
  let appliedThemeId = ''
  async function applyActiveCustomTheme(id: string | undefined) {
    if (!id) {
      if (appliedThemeId) {
        appliedThemeId = ''
        window.clearThemeVars?.()
        window.setFont?.(settings?.FontFamily ?? '')
        window.setCustomCss?.('')
      }
      return
    }
    try {
      const themes = await invoke<any[]>('list_themes')
      const theme = themes.find((t) => t.Id === id)
      if (!theme || settings?.ActiveThemeId !== id) return
      const vars = await invoke<Record<string, string>>('theme_css_vars', { theme })
      window.setTheme?.(theme.BasePreset)
      window.setThemeVars?.(vars)
      window.setFont?.(theme.FontFamily)
      window.setBlurIntensity?.(theme.Blur)
      window.setAnimationSpeed?.(theme.AnimationSpeed)
      window.setCustomCss?.(theme.CustomCss)
      window.setBackgroundFit?.(theme.BackgroundFit, theme.BackgroundOpacity)
      if (theme.BackgroundImage) window.setCustomBackground?.(theme.BackgroundImage)
      appliedThemeId = id
    } catch {
      // A broken theme file must not brick the UI; the preset stays applied.
    }
  }

  function setEdition(ed: string) {
    if (!settings || settings.Edition === ed) return
    saveSettings({ ...settings, Edition: ed })
    currentView = 'home'
    void loadActiveJavaInstance()
    void loadQuickSwitchInstances()
    void loadPrelaunchWarnings()
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
    packs: 'bedrockpacks',
    mcversions: 'bedrock-versions',
    worlds: 'bedrockworlds',
    servers: 'servers',
    utilities: 'utilities',
    credits: 'credits',
    clientlib: 'clientlibrary',
    stats: 'stats',
    skinlibrary: 'skinlibrary',
    logs: 'logs',
    tweaks: 'tweaks',
    launchers: 'launchers',
    mods: 'java-mods',
    profile: 'profile',
    levilaminamods: 'levilamina-mods',
    modpacks: 'modpacks',
    levimods: 'levilamina-mods',
    themestudio: 'settings',
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
    // `addons` ("Browse") in the `packs-addons` strip is CurseForge browsing.
    // Bedrock has a dedicated browser (BrowseAddonsView); Java's CurseForge/
    // Modrinth browsing already lives inline as tabs on JavaModsView itself,
    // so the Java side of this key just lands back there.
    addons: { bedrock: 'bedrock-addons', java: 'java-mods' },
    // `versions` means the Java version browser on the Java edition
    // (JavaTabs:34 and the javaclients panel's Versions button, both
    // OpenJavaVersions) and the Latite/OderSo client-DLL picker on Bedrock
    // (reached from the Clients panel). The Bedrock *game* version browser is
    // `mcversions`, and the versions-instances strip's own `versions` key is
    // handled before this map because it means that one instead.
    versions: { bedrock: 'versions', java: 'mc-versions' },
    // The versions-instances strip's `instances` tab — Bedrock already had
    // its own InstancesView; Java's equivalent used to be a local tab
    // buried inside Profile with no cross-nav at all.
    instances: { bedrock: 'instances', java: 'java-instances' },
  }

  /// Human-readable labels for breadcrumbs and navHistory entries. Kept
  /// separate from the i18n `t()` labels used on the Home buttons since
  /// several views (e.g. leveldat) have no Home button at all.
  const VIEW_LABELS: Record<View, string> = {
    home: 'Home',
    clients: 'Clients',
    instances: 'Instances',
    'java-instances': 'Instances',
    bedrockworlds: 'Worlds',
    bedrockpacks: 'Packs',
    'bedrock-addons': 'Browse Addons',
    bedrockbackups: 'Backups',
    bedrockscreenshots: 'Screenshots',
    leveldat: 'Level Editor',
    settings: 'Settings',
    'java-mods': 'Mods',
    modpacks: 'Modpacks',
    versions: 'Versions',
    tweaks: 'Tweaks',
    servers: 'Servers',
    logs: 'Logs',
    stats: 'Statistics',
    'levilamina-mods': 'LeviLamina Mods',
    'mc-versions': 'MC Versions',
    launchers: 'Launchers',
    'bedrock-versions': 'Versions & Instances',
    profile: 'Profile',
    'java-screenshots': 'Screenshots',
    'java-backups': 'Backups',
    skinlibrary: 'Skin Library',
    utilities: 'Utilities',
    clientlibrary: 'Client Library',
    downloads: 'Downloads',
    news: 'News',
    credits: 'Credits',
  }

  function navigateFromPanel(originalView: string, strip?: string) {
    if (strip === 'utilities' && UTILITIES_SUBTABS.includes(originalView)) {
      utilInitialTab = originalView
      openPanel('utilities')
      return
    }
    // The versions-instances strip's `versions` tab means "the game-version
    // browser for the current edition" (Bedrock's `OpenMcVersions`,
    // Home.MergedPanels.cs:36, or Java's own MC Versions panel) - not the
    // client-DLL picker the bare `versions` key otherwise means. Now that
    // both editions' Versions panels carry this strip (Java's links to its
    // own Instances the same way Bedrock's does), this has to branch on
    // edition instead of hardcoding the Bedrock panel.
    if (strip === 'versions-instances' && originalView === 'versions') {
      openPanel(isBedrock ? 'bedrock-versions' : 'mc-versions')
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

  // Applies `panelExpanded` to the CSS var every time it changes, including
  // the very first render — the effect (not the individual setters) is what
  // actually keeps the CSS in sync with the shared preference now.
  // Keeps the reactive i18n module (`i18n.svelte.ts`) in sync with the
  // persisted setting — every `t()` call anywhere in the app re-renders
  // once this fires, since `setLanguage` writes a shared `$state`.
  $effect(() => {
    setLanguage(settings?.Language ?? 'en')
  })

  $effect(() => {
    if (panelExpanded) {
      document.documentElement.style.setProperty('--sidebar-w-user', '860px')
    } else {
      document.documentElement.style.removeProperty('--sidebar-w-user')
    }
  })

  function setPanelExpanded(expanded: boolean) {
    panelExpanded = expanded
  }

  // QOL "copy panel link": a `glacier://open/<view>` URL for whatever
  // panel is currently open — pasted anywhere (a chat, a note, a shortcut)
  // and clicked, it relaunches/refocuses the launcher straight into that
  // panel. `VIEW_LABELS` is a `Record<View, string>`, so its keys are
  // exactly every valid `View` — reused here as the validation set rather
  // than hand-maintaining a second list that could drift out of sync.
  function copyText(text: string, okMessage: string) {
    window.copyToClipboard?.(text).then((ok) =>
      showToast(ok ? okMessage : 'Could not copy to clipboard.', ok ? 'success' : 'error'),
    )
  }

  function copyCurrentPanelLink() {
    copyText(`glacier://open/${currentView}`, 'Panel link copied to clipboard.')
  }

  function handleDeepLink(url: string) {
    let target: string
    try {
      target = new URL(url).pathname.replace(/^\/+/, '') || new URL(url).hostname
    } catch {
      return
    }
    if ((Object.keys(VIEW_LABELS) as string[]).includes(target)) {
      openPanel(target as View)
    }
  }

  function openPanel(v: View, tab?: string) {
    navigate(() => {
      if (tab) contentInitialTab = tab
      // Track the chain so `goBack` can return to the actual parent panel
      // instead of always falling back to Home. Opening straight from Home
      // starts a fresh chain; re-opening the view already on screen (e.g. a
      // tab switch that reuses the same View key) is not a navigation.
      if (currentView === 'home') {
        navHistory = []
      } else if (currentView !== v) {
        navHistory = [...navHistory, { view: currentView, label: VIEW_LABELS[currentView] ?? currentView }]
      }
      currentView = v
    })
  }

  function closePanel() {
    navHistory = []
    refreshJavaClientOptions()
    navigate(() => {
      currentView = 'home'
    })
  }

  /// Pops the last entry off `navHistory` and returns to it, so a panel's
  /// back button lands on the panel that actually opened it (e.g.
  /// Instances -> MC Versions -> back goes to Instances) instead of always
  /// jumping to Home. Falls back to `closePanel` when the stack is empty,
  /// which is also what a single-level Home -> Panel back button should do.
  function goBack() {
    refreshClientsBadge()
    if (navHistory.length === 0) {
      closePanel()
      return
    }
    const prev = navHistory[navHistory.length - 1]
    navHistory = navHistory.slice(0, -1)
    navigate(() => {
      currentView = prev.view
    })
  }

  /// QOL: jump back several levels of `navHistory` at once via a breadcrumb
  /// segment, instead of clicking Back repeatedly. `index` is the clicked
  /// segment's position in `breadcrumbTrail` (0 = Home, then one per
  /// `navHistory` entry) — the current/last segment is never clickable
  /// (PanelHeader excludes it), so this only ever runs for an actual
  /// ancestor.
  function jumpToBreadcrumb(index: number) {
    if (index === 0) {
      closePanel()
      return
    }
    const target = navHistory[index - 1]
    if (!target) return
    navHistory = navHistory.slice(0, index - 1)
    navigate(() => {
      currentView = target.view
    })
  }

  /// Full "Home / Parent / Current" trail for the current view, handed to
  /// PanelHeader's optional `breadcrumb` prop. Panels that don't pass it
  /// render exactly as before.
  const breadcrumbTrail = $derived([
    VIEW_LABELS.home,
    ...navHistory.map((h) => h.label),
    VIEW_LABELS[currentView] ?? currentView,
  ])

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
        recordLaunchOutcome(settings.SelectedClient, true)
      } else if (activeJavaClient === 'Horizon') {
        await invoke('horizon_launch')
        showToast('Horizon is starting…', 'success')
      } else if (activeJavaClient === 'Lunar' && lunarExe) {
        await invoke('launch_client_exe', { exePath: lunarExe })
        showToast('Opening Lunar Client…', 'success')
      } else if (MOD_CLIENTS.includes(activeJavaClient)) {
        // These clients are standalone mods: put the right jar in the active
        // instance's mods folder, then launch that instance as usual.
        const inst = activeJavaInstance
        const loader = inst?.loader
        if (!inst || !loader || loader === 'vanilla') {
          throw new Error(`${activeJavaClient} needs a modded instance (Fabric${activeJavaClient === 'Dawn' ? '' : ', Forge or NeoForge'}). Create one in Versions → Instances first.`)
        }
        if (await invoke<boolean>('ensure_client_mod_installed', {
          instanceId: inst.id,
          client: activeJavaClient,
          gameVersion: inst.version_id,
          loader,
        })) {
          showToast(`Added ${activeJavaClient} to ${inst.name}`, 'success')
        }
        await invoke('launch_java_instance', { instanceId: inst.id })
      } else {
        await invoke('launch_java_instance', { instanceId: null })
      }
    } catch (e) {
      console.error('Launch failed:', e)
      status = `Launch failed: ${String(e)}`
      showToast(`Launch failed: ${String(e)}`, 'error')
      if (isBedrock && settings) recordLaunchOutcome(settings.SelectedClient, false)
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
      if (settings) recordLaunchOutcome(settings.SelectedClient, true)
    } catch (e) {
      isLaunching = false
      status = String(e)
      showToast(String(e), 'error')
      if (settings) recordLaunchOutcome(settings.SelectedClient, false)
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
  ///
  /// Previously this only ran once at startup, so a launcher session left
  /// open for hours (very normal for something that sits in the tray while
  /// you play) would never learn a new release had shipped until the next
  /// full restart. It now re-checks on an interval and only toasts the
  /// *first* time a given tag is seen, so it doesn't nag on every poll.
  ///
  /// The pill used to route to the News panel's changelog tab — a dead end
  /// with no way to actually start the update from there. It now opens
  /// `UpdateAvailableModal` directly with the full release info already in
  /// hand, so clicking "Update" goes straight to a changelog + one-click
  /// download-and-install instead of a scavenger hunt through Settings.
  type UpdateInfo = { tag: string; download_url: string; changelog: string; asset_size: number }
  let availableUpdateInfo = $state<UpdateInfo | null>(null)
  let updateModalOpen = $state(false)
  let lastAnnouncedUpdateTag = ''
  async function loadVersionInfo() {
    try {
      currentVersion = await invoke<string>('launcher_version')
    } catch {
      currentVersion = ''
    }
    try {
      const update = await invoke<UpdateInfo | null>('check_launcher_update')
      launcherUpdateAvailable = update !== null
      availableUpdateInfo = update
      if (update && update.tag !== lastAnnouncedUpdateTag) {
        lastAnnouncedUpdateTag = update.tag
        showToast(`Glacier Launcher ${update.tag} is available — click Update to install.`, 'info')
        sendDesktopNotification('Update available', `Glacier Launcher ${update.tag} is ready to install.`)
      }
    } catch {
      launcherUpdateAvailable = false
    }
  }

  // QOL "What's new": pops the changelog for the version that was just
  // installed, once, the first time the launcher runs after an update.
  // Gated on `OnboardingCompleted` so this can never fire alongside/before
  // the first-run onboarding modal, and on `LastSeenChangelogVersion`
  // already being non-empty so upgrading an *existing* install (which has
  // never set this field before) silently seeds the baseline instead of
  // popping up for a version the user already has installed and running.
  let whatsNewOpen = $state(false)
  let whatsNewChangelog = $state('')
  const normalizeTag = (t: string) => t.replace(/^v/i, '')
  async function maybeShowWhatsNew() {
    if (!settings || !currentVersion || !settings.OnboardingCompleted) return
    if (settings.LastSeenChangelogVersion === currentVersion) return
    if (!settings.LastSeenChangelogVersion) {
      saveSettings({ ...settings, LastSeenChangelogVersion: currentVersion })
      return
    }
    try {
      const releases = await invoke<{ tag: string; changelog: string }[]>('list_launcher_releases', { max: 20 })
      const match = releases.find((r) => normalizeTag(r.tag) === normalizeTag(currentVersion))
      if (match?.changelog) {
        whatsNewChangelog = match.changelog
        whatsNewOpen = true
      }
    } catch {
      // Non-fatal — just skip the popup this run.
    }
    saveSettings({ ...settings, LastSeenChangelogVersion: currentVersion })
  }

  Promise.all([loadSettings(), loadVersionInfo()]).then(maybeShowWhatsNew)
  // Re-check every 30 minutes while the launcher is open, instead of only
  // ever once at startup.
  setInterval(loadVersionInfo, 30 * 60 * 1000)

  // Warm the most-used panel chunks after first paint so opening them is
  // instant, without paying for the rest during startup.
  setTimeout(() => {
    for (const k of ['ClientsView', 'InstancesView', 'JavaInstancesView', 'BedrockWorldsView', 'SettingsView'] as const) {
      void views[k]().catch(() => {})
    }
  }, 1500)

  // QOL: auto-clear-finished-downloads sweep. A no-op server-side when the
  // setting is 0/disabled; run at startup and hourly rather than adding a
  // backend timer, so it only ever runs while the launcher is open.
  // Deferred 4s so these housekeeping calls never compete with first paint.
  setTimeout(() => {
    invoke('sweep_old_downloads').catch(() => {})
    invoke('auto_export_settings_if_due').catch(() => {})
  }, 4000)
  setInterval(() => invoke('sweep_old_downloads').catch(() => {}), 60 * 60 * 1000)

  // QOL: rolling settings auto-backup, separate from the manual "Export
  // settings" button — the command itself no-ops until 24h have passed.
  setInterval(() => invoke('auto_export_settings_if_due').catch(() => {}), 60 * 60 * 1000)

  /// Settings' "Check for updates now" button drives the exact same check
  /// as the background poll, then opens the same modal regardless of the
  /// result — `UpdateAvailableModal` renders an "up to date" state when
  /// `availableUpdateInfo` comes back null, so this is one update UI
  /// everywhere instead of Settings having its own separate inline
  /// download/install flow that could drift from the pill's.
  let checkingUpdatesManual = $state(false)
  async function checkForUpdatesNow() {
    checkingUpdatesManual = true
    try {
      await loadVersionInfo()
    } finally {
      checkingUpdatesManual = false
      updateModalOpen = true
    }
  }
  loadNotifications()
  refreshClientsBadge()
  refreshJavaClientOptions()

  window.addEventListener('glacier:kbshortcut', (e: Event) => {
    const name = (e as CustomEvent<string>).detail
    if (name === 'escape') {
      if (searchOpen) closeSearch()
      else if (moreOpen) closeMore()
      else if (isLaunching) forceCloseLaunch()
      else goBack()
    }
    else if (name === 'settings') openPanel('settings')
    else if (name === 'clients' && isBedrock) openPanel('clients')
    else if (name === 'launch') handleLaunch()
    else if (name === 'fullscreen') toggleFullscreen()
    else if (name === 'search') openSearch()
  })

  // --- Phase 4: crash-analysis modal ---------------------------------------
  // When a launched Java instance's process exits non-zero, checks its
  // newest crash report with the existing `log_looks_like_crash`/
  // `latest_java_crash_report` commands, and if it looks like a crash, runs
  // it through `analyze_crash`'s local pattern matcher and shows this modal.
  // Scoped to the currently-active instance (the common single-instance
  // launch case) — a bare version launch (`launch_java_version`, id shaped
  // like "version:<id>") has no per-instance directory to look a crash
  // report up in, so it's not covered here.
  type CrashAnalysis = { category: string; message: string; suggested_fix: string }
  let crashModalOpen = $state(false)
  let crashAnalysis = $state<CrashAnalysis | null>(null)
  let crashLogPath = $state('')
  let crashSharing = $state(false)
  let crashShareUrl = $state('')

  async function maybeShowCrashModal(instanceId: string) {
    if (!activeJavaInstance || activeJavaInstance.id !== instanceId) return
    try {
      const report = await invoke<{ path: string } | null>('latest_java_crash_report', {
        minecraftDir: activeJavaInstance.directory,
      })
      if (!report) return
      const text = await invoke<string>('read_log', { path: report.path })
      const looksLikeCrash = await invoke<boolean>('log_looks_like_crash', { text })
      if (!looksLikeCrash) return
      crashLogPath = report.path
      crashShareUrl = ''
      crashAnalysis = await invoke<CrashAnalysis>('analyze_crash', { logPath: report.path })
      crashModalOpen = true
    } catch (e) {
      console.error('Crash analysis failed:', e)
    }
  }

  async function shareCrashLog() {
    if (!crashLogPath) return
    crashSharing = true
    try {
      const content = await invoke<string>('read_log', { path: crashLogPath })
      crashShareUrl = await invoke<string>('share_log', { content })
    } catch (e) {
      console.error('Share crash log failed:', e)
    } finally {
      crashSharing = false
    }
  }

  async function openCrashLogFolder() {
    if (!crashLogPath) return
    // There's no per-file "reveal in Explorer" command in this app — just the
    // generic `open_folder(path)` (see commands::system::open_folder), so we
    // open the crash log's parent directory instead.
    const parentDir = crashLogPath.replace(/[\\/][^\\/]*$/, '')
    if (!parentDir || parentDir === crashLogPath) return
    try {
      await invoke('open_folder', { path: parentDir })
    } catch (e) {
      console.error('Open crash report folder failed:', e)
    }
  }

  function closeCrashModal() {
    crashModalOpen = false
    crashAnalysis = null
    crashLogPath = ''
    crashShareUrl = ''
  }

  listen<{ instance_id: string; status: string; exit_code?: number }>('java-process-changed', (evt) => {
    const { instance_id, status, exit_code } = evt.payload
    if (status === 'exited' && typeof exit_code === 'number' && exit_code !== 0) {
      maybeShowCrashModal(instance_id)
    }
  })

  // QOL: confirm before quitting while a download is still in flight — the
  // Rust `CloseRequested` handler already prevented the close and is
  // waiting on `force_quit`; this is just the dialog.
  let confirmQuitDownloadsOpen = $state(false)
  // QOL: global (system-wide) quick-launch hotkey — the Rust side already
  // brought the window to front on trigger (see lib.rs's global-shortcut
  // handler); this just runs the same launch path the button uses.
  listen('quick-launch-hotkey', () => {
    if (!isLaunching) handleLaunch()
  })

  // QOL "copy panel link": a `glacier://` URL opened while this instance is
  // already running arrives as this event (see lib.rs's single-instance and
  // deep-link setup); one cold-started with the app arrives instead as
  // `take_pending_deep_link`, checked once here since no listener existed
  // yet at the moment Rust would have emitted it.
  listen<string>('deep-link-open', (e) => handleDeepLink(e.payload))
  invoke<string | null>('take_pending_deep_link').then((url) => {
    if (url) handleDeepLink(url)
  }).catch(() => {})

  listen('confirm-quit-downloads', () => {
    confirmQuitDownloadsOpen = true
  })
</script>

<svelte:window
  onkeydown={(e) => {
    // Escape dismisses the top bar's dropdowns (outside click is the backdrop).
    if (e.key !== 'Escape') return
    if (clientMenuOpen) clientMenuOpen = false
    if (notifOpen) notifOpen = false
  }}
/>

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

  {#snippet directoryCard(r: SearchResult, sel: boolean, onHover: (() => void) | undefined, query: string)}
    <div
      class="client-card {sel ? 'client-active' : ''} {r.disabled ? 'disabled' : ''}"
      role="button"
      tabindex="0"
      data-tooltip={r.disabled ? 'Not available yet' : undefined}
      onclick={() => { if (!r.disabled) r.run() }}
      onkeydown={(e) => { if (!r.disabled && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); r.run() } }}
      onmouseenter={onHover}
      style="cursor:{r.disabled ? 'not-allowed' : 'pointer'};"
    >
      <div class="client-card-header">
        <div class="client-card-icon" style="background:var(--accent-bg); color:var(--accent);">
          {#if r.icon.startsWith('/images/')}
            <img src={r.icon} alt="" style="width:16px;height:16px;object-fit:contain;" />
          {:else}
            <i class={r.icon}></i>
          {/if}
        </div>
        <div class="client-card-meta">
          <span class="client-card-name">{@html highlight(r.label, query)}</span>
          {#if r.sub}<span class="client-card-sub">{@html highlight(r.sub, query)}</span>{/if}
        </div>
        {#if !r.disabled}
          <button
            type="button"
            class="panel-icon-btn {isPinned(r) ? 'active' : ''}"
            data-tooltip={isPinned(r) ? 'Unpin from Home' : 'Pin to Home'}
            aria-label={isPinned(r) ? 'Unpin from Home' : 'Pin to Home'}
            onclick={(e) => { e.stopPropagation(); togglePinned(r) }}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="{isPinned(r) ? 'fa-solid' : 'fa-regular'} fa-star"></i>
          </button>
          <i class="fa-solid fa-arrow-right news-ext" style="opacity:0.5;"></i>
        {/if}
      </div>
    </div>
  {/snippet}

  {#if searchOpen}
    <div class="panel-overlay directory-panel-overlay" role="presentation">
      <PanelHeader title={t('panel.search', settings?.Language)} onBack={closeSearch}>
        {#snippet extraActions()}
          {#if searchQuery}
            <span class="search-result-count">{searchFlat.length} result{searchFlat.length === 1 ? '' : 's'}</span>
          {/if}
        {/snippet}
      </PanelHeader>

      <div class="panel-search-wrap">
        <i class="fa-solid fa-magnifying-glass"></i>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          bind:this={searchInputEl}
          class="panel-search-input"
          placeholder="Search clients, versions, settings..."
          bind:value={searchQuery}
          oninput={() => (searchSelIdx = 0)}
          onkeydown={onSearchKeydown}
        />
      </div>

      <div class="mcv-filter-bar">
        <div class="mcv-channel-tabs">
          <button type="button" class="mcv-channel-tab {!searchCategoryFilter ? 'active' : ''}" onclick={() => selectSearchCategory(null)}>All</button>
          {#each searchCategories as c (c)}
            <button type="button" class="mcv-channel-tab {searchCategoryFilter === c ? 'active' : ''}" onclick={() => selectSearchCategory(c)}>{c}</button>
          {/each}
        </div>
      </div>

      <div class="panel-body">
        {#if showingSearchList}
          {#if searchMatches.length === 0}
            <div class="empty-state">
              <i class="fa-solid fa-magnifying-glass"></i>
              <span>No results for "{searchQuery}"</span>
            </div>
          {:else}
            {#each searchGroups as grp (grp.category)}
              <span class="panel-section-label">{grp.category}</span>
              <div class="news-card-col">
                {#each grp.results as r (r.label)}
                  {@const fi = searchFlat.indexOf(r)}
                  {@render directoryCard(r, searchSelIdx === fi, () => (searchSelIdx = fi), searchQuery)}
                {/each}
              </div>
            {/each}
          {/if}
        {:else}
          <span class="panel-section-label">Quick Actions</span>
          <div class="news-card-col">
            {#each quickActions as r, idx (r.label)}
              {@render directoryCard(r, searchSelIdx === idx, () => (searchSelIdx = idx), '')}
            {/each}
          </div>
        {/if}
      </div>

      <div class="search-footer">
        <span class="search-footer-hint"><kbd>&#8593;</kbd><kbd>&#8595;</kbd> Navigate</span>
        <span class="search-footer-hint"><kbd>&#8629;</kbd> Select</span>
        <span class="search-footer-hint"><kbd>ESC</kbd> Close</span>
      </div>
    </div>
  {/if}

  {#if moreOpen}
    <div class="panel-overlay directory-panel-overlay" role="presentation">
      <PanelHeader title={t('panel.everyPanel', settings?.Language)} onBack={closeMore} />

      <div class="mcv-filter-bar">
        <div class="mcv-channel-tabs">
          {#each searchCategories as c (c)}
            <button type="button" class="mcv-channel-tab {moreCategory === c ? 'active' : ''}" onclick={() => (moreCategory = c)}>{c}</button>
          {/each}
        </div>
      </div>

      <div class="panel-body">
        <div class="news-card-col">
          {#each moreItems as r (r.label)}
            {@render directoryCard(r, false, undefined, '')}
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <div class="top-bar">
    <div class="branding">
      <img class="logo-icon" src="/images/icon.svg" alt="" />
      <div class="branding-text">
        <span class="branding-title">Glacier Launcher</span>
        <span class="branding-sub">Glacier Productions</span>
      </div>
      {#if launcherUpdateAvailable}
        <button
          class="update-pill"
          onclick={() => (updateModalOpen = true)}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip={t('tt.launcherUpdateAvailableClickUpdate', settings?.Language)}
        >
          <i class="fa-solid fa-arrow-up"></i>
          <span>Update</span>
        </button>
      {:else}
        <button
          class="update-pill"
          onclick={() => openPanel('news')}
          oncontextmenu={(e) => { e.preventDefault(); copyText(currentVersion, 'Version copied to clipboard.') }}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip={t('tt.newsUpdatesRightClickCopyVersion', settings?.Language)}
        >
          <span>v{currentVersion}</span>
        </button>
      {/if}
    </div>
    <div class="top-bar-spacer"></div>

    {#if settings}
      <div class="edition-switcher" role="presentation" onmousedown={(e) => e.stopPropagation()} data-tooltip={t('tt.switchMinecraftEdition', settings?.Language)}>
        <button class="edition-tab" class:active={isBedrock} onclick={() => setEdition('bedrock')}>
          <img class="chip-icon" src="images/mc-logo.svg" alt="" style="width:14px;height:14px;object-fit:contain;"><span>Bedrock</span>
        </button>
        <button class="edition-tab" class:active={!isBedrock} onclick={() => setEdition('java')}>
          <img class="chip-icon" src="images/mc-java-logo.svg" alt="" style="width:18px;height:18px;object-fit:contain;"><span>Java</span>
        </button>
      </div>

      {#if isBedrock}
        <div class="topbar-client-chip {clientMenuOpen ? 'open' : ''}" style="position:relative;">
          <button
            class="topbar-client-chip-btn"
            style="all:unset; display:flex; align-items:center; gap:inherit; cursor:pointer;"
            onclick={toggleClientMenu}
            onmousedown={(e) => e.stopPropagation()}
            aria-haspopup="listbox"
            aria-expanded={clientMenuOpen}
            data-tooltip={clientMenuOpen ? undefined : `Select client (${settings.SelectedClient})`}
          >
            <div class="chip-dot" class:live={mcRunning}></div>
            {#if CLIENT_IMAGES[settings.SelectedClient]}
              <img class="chip-icon" src={CLIENT_IMAGES[settings.SelectedClient]} alt="" style="width:14px;height:14px;object-fit:contain;" />
            {:else}
              <i class="chip-icon {CLIENT_ICONS[settings.SelectedClient] ?? ''}"></i>
            {/if}
            <span>{settings.SelectedClient}</span>
            <i class="fa-solid fa-chevron-down chip-arrow"></i>
          </button>
          {#if clientMenuOpen}
            <div class="tw-dropdown-menu" role="listbox" tabindex="-1" onmousedown={(e) => e.stopPropagation()}>
              {#each CYCLE_CLIENTS as name (name)}
                <button
                  class="tw-dropdown-item {settings.SelectedClient === name ? 'active' : ''}"
                  role="option"
                  aria-selected={settings.SelectedClient === name}
                  onclick={() => selectClientFromMenu(name)}
                >
                  {#if CLIENT_IMAGES[name]}
                    <img src={CLIENT_IMAGES[name]} alt="" style="width:14px;height:14px;object-fit:contain;" />
                  {:else}
                    <i class="fa-solid {CLIENT_ICONS[name] ?? ''}"></i>
                  {/if}
                  <span style="flex:1;">{name}</span>
                  {#if settings.SelectedClient === name}
                    <i class="fa-solid fa-check"></i>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if !isBedrock && javaClients.length > 1}
        <div class="topbar-client-chip {clientMenuOpen ? 'open' : ''}" style="position:relative;">
          <button
            class="topbar-client-chip-btn"
            style="all:unset; display:flex; align-items:center; gap:inherit; cursor:pointer;"
            onclick={toggleClientMenu}
            onmousedown={(e) => e.stopPropagation()}
            aria-haspopup="listbox"
            aria-expanded={clientMenuOpen}
            data-tooltip={clientMenuOpen ? undefined : `Select client (${activeJavaClient})`}
          >
            <div class="chip-dot" class:live={mcRunning}></div>
            <img class="chip-icon" src={JAVA_CLIENT_IMAGES[activeJavaClient]} alt="" style="width:14px;height:14px;object-fit:contain;border-radius:3px;" />
            <span>{activeJavaClient}</span>
            <i class="fa-solid fa-chevron-down chip-arrow"></i>
          </button>
          {#if clientMenuOpen}
            <div class="tw-dropdown-menu" role="listbox" tabindex="-1" onmousedown={(e) => e.stopPropagation()}>
              {#each javaClients as name (name)}
                <button
                  class="tw-dropdown-item {activeJavaClient === name ? 'active' : ''}"
                  role="option"
                  aria-selected={activeJavaClient === name}
                  onclick={() => selectJavaClient(name)}
                >
                  <img src={JAVA_CLIENT_IMAGES[name]} alt="" style="width:14px;height:14px;object-fit:contain;border-radius:3px;" />
                  <span style="flex:1;">{name}</span>
                  {#if activeJavaClient === name}
                    <i class="fa-solid fa-check"></i>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if clientMenuOpen}
        <div class="notif-backdrop" role="presentation" onclick={() => (clientMenuOpen = false)}></div>
      {/if}

      {#if !isBedrock && quickSwitchInstances.length > 1}
        <select
          class="setting-select"
          style="max-width:150px;"
          value={activeJavaInstance?.id ?? ''}
          onchange={(e) => quickSwitchInstance(e.currentTarget.value)}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip={t('tt.quickSwitchJavaInstance', settings?.Language)}
          aria-label={t('tt.quickSwitchJavaInstance', settings?.Language)}
        >
          {#each quickSwitchInstances as inst (inst.id)}
            <option value={inst.id}>{inst.name ?? inst.id}</option>
          {/each}
        </select>
      {/if}

      {#if mcRunning}
        <div class="mc-running-pill" data-tooltip={t('tt.minecraftIsCurrentlyRunning', settings?.Language)}>
          <i class="fa-solid fa-circle"></i>
          <span>MC</span>
        </div>
      {/if}
    {/if}

    {#if currentView !== 'home'}
      <button
        class="search-trigger"
        onclick={copyCurrentPanelLink}
        onmousedown={(e) => e.stopPropagation()}
        data-tooltip={t('tt.copyLinkThisPanel', settings?.Language)}
        aria-label={t('tt.copyPanelLink', settings?.Language)}
      >
        <i class="fa-solid fa-link"></i>
      </button>
    {/if}

    <button
      class="search-trigger"
      onclick={openSearch}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.searchCtrlK', settings?.Language)}
      aria-label={t('tt.search', settings?.Language)}
    >
      <i class="fa-solid fa-magnifying-glass"></i>
    </button>

    {#if settings}
      {#if notifOpen}
        <div class="notif-backdrop" role="presentation" onclick={() => (notifOpen = false)}></div>
      {/if}
      <div class="notif-bell-wrap" role="presentation" onmousedown={(e) => e.stopPropagation()}>
        <button class="notif-bell-btn" onclick={toggleNotifPanel} data-tooltip={t('tt.notificationsDownloads', settings?.Language)}>
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
            {#if notifications.length > 0}
              <div class="notif-filter-tabs" role="tablist" aria-label={t('tt.filterNotifications', settings?.Language)}>
                {#each [['all', 'All'], ['downloads', 'Downloads'], ['updates', 'Updates'], ['social', 'Social']] as [value, label] (value)}
                  <button
                    type="button"
                    role="tab"
                    aria-selected={notifFilter === value}
                    class="notif-filter-tab {notifFilter === value ? 'active' : ''}"
                    onclick={() => (notifFilter = value as typeof notifFilter)}
                  >{label}</button>
                {/each}
              </div>
            {/if}
            <div class="notif-panel-list">
              {#if notifications.length === 0}
                <div class="notif-empty">
                  <i class="fa-solid fa-bell-slash"></i>
                  <span>No notifications</span>
                </div>
              {:else if filteredNotifications.length === 0}
                <div class="notif-empty">
                  <i class="fa-solid fa-bell-slash"></i>
                  <span>No notifications in this category</span>
                </div>
              {:else}
                {#each filteredNotifications as n (n.Id)}
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
                      aria-label={t('tt.dismissNotification', settings?.Language)}
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
      <button class="wnd-btn minimize-btn" data-tooltip={t('tt.minimize', settings?.Language)} aria-label={t('tt.minimize', settings?.Language)} onmousedown={(e) => e.stopPropagation()} onclick={() => window.minimizeWindow?.()}>
        <i class="fa-solid fa-minus"></i>
      </button>
      <button class="wnd-btn maximize-btn" data-tooltip={t('tt.maximize', settings?.Language)} aria-label={t('tt.maximize', settings?.Language)} onmousedown={(e) => e.stopPropagation()} onclick={() => window.maximizeWindow?.()}>
        <i class="fa-solid fa-square"></i>
      </button>
      <button class="wnd-btn close-btn" data-tooltip={t('tt.close', settings?.Language)} aria-label={t('tt.close', settings?.Language)} onmousedown={(e) => e.stopPropagation()} onclick={() => window.closeWindow?.()}>
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
        <button class="announcement-banner-close" onclick={dismissAnnouncement} data-tooltip={t('tt.dismiss', settings?.Language)} aria-label={t('tt.dismiss', settings?.Language)}>
          <i class="fa-solid fa-xmark"></i>
        </button>
      {/if}
    </div>
  {/if}

  {#if !isBedrock && prelaunchWarnings.length > 0 && currentView === 'home'}
    <div class="announcement-banner" style="background:var(--warning-bg, #4a3a12); border-color:var(--warning-border, #a97d1a);">
      <i class="fa-solid fa-triangle-exclamation"></i>
      <div class="announcement-banner-text">
        {#each prelaunchWarnings as w, i (i)}
          <span class="announcement-banner-msg">{w.message}</span>
        {/each}
      </div>
      <button class="announcement-banner-close" onclick={() => (prelaunchWarnings = [])} data-tooltip={t('tt.dismiss', settings?.Language)} aria-label={t('tt.dismiss', settings?.Language)}>
        <i class="fa-solid fa-xmark"></i>
      </button>
    </div>
  {/if}

  <div class="main-content" class:panel-open={currentView !== 'home'}>
    {#if settings}
      {#if currentView === 'home' && pinnedPanels.length > 0}
        <div class="pinned-panels-row">
          {#each pinnedPanels as r (r.label)}
            <button
              type="button"
              class="vcs-btn"
              onclick={r.run}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={r.label}
            >
              {#if r.icon.startsWith('/images/')}
                <img src={r.icon} alt="" style="width:13px;height:13px;object-fit:contain;" />
              {:else}
                <i class={r.icon}></i>
              {/if}
              {r.label}
            </button>
          {/each}
        </div>
      {/if}
      <div class="btn-row">
        <button
          class="action-btn launch-btn btn-wide btn-blue {isLaunching ? 'launching' : ''}"
          onclick={handleLaunch}
          disabled={isLaunching}
          data-tooltip={t('tt.launchCtrlL', settings?.Language)}
        >
          {#if isLaunching}
            <span class="spinner"></span><span>Launching...</span>
          {:else}
            <i class="fa-solid fa-play"></i><span class="btn-label">{t('nav.launch', settings?.Language)}</span>
          {/if}
        </button>

        {#if isLaunching}
          <button class="action-btn btn-sq force-close-btn" onclick={forceCloseLaunch} data-tooltip={t('tt.forceCloseLaunchEsc', settings?.Language)} aria-label={t('tt.forceCloseLaunch', settings?.Language)}>
            <i class="fa-solid fa-stop"></i>
          </button>
        {/if}

        {#if isBedrock}
          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('clients')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.clientsCtrl1', settings?.Language)}
            >
              <i class="fa-solid fa-puzzle-piece"></i><span class="btn-label">{t('nav.clients', settings?.Language)}</span>
            </button>
            {#if clientsHasBadge}<span class="notification-badge"></span>{/if}
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('bedrock-addons')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.packsCtrl2', settings?.Language)}
            >
              <i class="fa-solid fa-boxes-stacked"></i><span class="btn-label">Packs</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('bedrockworlds')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.worldsServersCtrl3', settings?.Language)}
            >
              <i class="fa-solid fa-globe"></i><span class="btn-label">Worlds</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('bedrock-versions')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.versionsInstancesCtrl4', settings?.Language)}
            >
              <i class="fa-solid fa-box-archive"></i><span class="btn-label">{t('nav.mcversions', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('tweaks')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.bedrockTweaks', settings?.Language)}
            >
              <i class="fa-solid fa-wand-magic-sparkles"></i><span class="btn-label">Tweaks</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('utilities')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.utilitiesGameOptionsShadersClientsSystem', settings?.Language)}
            >
              <i class="fa-solid fa-screwdriver-wrench"></i><span class="btn-label">Utilities</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('settings')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.settingsCtrl', settings?.Language)}
            >
              <i class="fa-solid fa-gear"></i><span class="btn-label">{t('nav.settings', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={openMore}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.everyPanelServersLevilaminaBackupsPhotos', settings?.Language)}
            >
              <i class="fa-solid fa-ellipsis"></i><span class="btn-label">More</span>
            </button>
          </div>
        {:else}
          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('launchers')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.launchersCtrl1', settings?.Language)}
            >
              <i class="fa-solid fa-rocket"></i><span class="btn-label">{t('nav.launchers', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('java-mods')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.modsCtrl2', settings?.Language)}
            >
              <i class="fa-solid fa-puzzle-piece"></i><span class="btn-label">{t('nav.mods', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('mc-versions')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.versionsCtrl3', settings?.Language)}
            >
              <i class="fa-solid fa-box-archive"></i><span class="btn-label">{t('nav.versions', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('tweaks')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.vanillaTweaks', settings?.Language)}
            >
              <i class="fa-solid fa-wand-magic-sparkles"></i><span class="btn-label">Tweaks</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('profile')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.profile', settings?.Language)}
            >
              <i class="fa-solid fa-user"></i><span class="btn-label">{t('nav.profile', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('java-screenshots')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.screenshots', settings?.Language)}
            >
              <i class="fa-solid fa-images"></i><span class="btn-label">{t('nav.photos', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={() => openPanel('settings')}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.settingsCtrl', settings?.Language)}
            >
              <i class="fa-solid fa-gear"></i><span class="btn-label">{t('nav.settings', settings?.Language)}</span>
            </button>
          </div>

          <div class="btn-badge-wrap">
            <button
              class="action-btn btn-sq hover-grow"
              onclick={openMore}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.everyPanelModpacksInstancesBackupsLogs', settings?.Language)}
            >
              <i class="fa-solid fa-ellipsis"></i><span class="btn-label">More</span>
            </button>
          </div>
        {/if}
      </div>

      {#if currentView === 'home'}
        <p class="status-msg visible">{status === 'ready' ? '' : status}</p>

        {#if settings.OnboardingCompleted && !settings.OnboardingChecklistDismissed}
          <OnboardingChecklist {settings} onChange={saveSettings} onNavigate={(v) => openPanel(v as View)} />
        {/if}

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
    <!-- The footer chip always shows the installed *Bedrock* build
         (`installed_bedrock_version`) regardless of which edition is
         currently active elsewhere in the app, so it must always open the
         Bedrock versions panel — it was hardcoded to the Java 'mc-versions'
         panel, so clicking a Bedrock version chip opened the Java version
         list instead. -->
    <Footer {settings} onChange={saveSettings} onOpenMcVersions={() => openPanel('bedrock-versions')} />
  {/if}

  <div class="toast-container">
    {#each toasts as toast (toast.id)}
      <div class="toast {toast.kind} {toast.closing ? 'closing' : ''} {toast.action ? 'has-action' : ''}">
        <i class={toast.icon}></i>
        {toast.message}
        {#if toast.action}
          <button
            type="button"
            class="toast-action"
            onclick={() => { toast.action?.run(); toasts = toasts.filter((x) => x.id !== toast.id) }}
          >
            {#if toast.action.icon}<i class={toast.action.icon}></i>{/if}
            {toast.action.label}
          </button>
        {/if}
      </div>
    {/each}
  </div>

  {#if vanillaFallbackPrompt}
    <div class="vanilla-fallback-prompt" role="alertdialog" aria-label={t('tt.repeatedInjectionFailures', settings?.Language)}>
      <i class="fa-solid fa-triangle-exclamation"></i>
      <span>{vanillaFallbackPrompt.client} failed to inject 3 times in a row.</span>
      <button class="btn-sm" onclick={launchVanillaFallback} onmousedown={(e) => e.stopPropagation()}>
        Launch Vanilla instead
      </button>
      <button class="icon-btn icon-btn-ghost" data-tooltip={t('tt.dismiss', settings?.Language)} aria-label={t('tt.dismiss', settings?.Language)} onclick={dismissVanillaFallbackPrompt} onmousedown={(e) => e.stopPropagation()}>
        <i class="fa-solid fa-xmark"></i>
      </button>
    </div>
  {/if}

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
    <button
      class="panel-expand-btn"
      onclick={togglePanelExpand}
      onmousedown={(e) => e.stopPropagation()}
      title={panelExpanded ? 'Collapse panel' : 'Expand panel'}
      aria-label={panelExpanded ? 'Collapse panel' : 'Expand panel'}
    >
      {#if panelSide === 'left'}
        <i class="fa-solid {panelExpanded ? 'fa-chevron-left' : 'fa-chevron-right'}"></i>
      {:else}
        <i class="fa-solid {panelExpanded ? 'fa-chevron-right' : 'fa-chevron-left'}"></i>
      {/if}
    </button>
    <div class="panel-overlay">
      {#if currentView === 'clients'}
        <LazyView loader={views.ClientsView}
          {settings}
          onChange={saveSettings}
          onBack={goBack}
          onOpenVersions={() => openPanel('versions')}
          onNavigate={navigateFromPanel}
          breadcrumb={breadcrumbTrail}
          onBreadcrumbJump={jumpToBreadcrumb}
        />
      {:else if currentView === 'instances'}
        <LazyView loader={views.InstancesView}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={(m, k) => showToast(m, k ?? 'info')}
        />
      {:else if currentView === 'java-instances'}
        <LazyView loader={views.JavaInstancesView}
          {settings}
          onChange={saveSettings}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={(m, k) => showToast(m, k ?? 'info')}
        />
      {:else if currentView === 'bedrockworlds'}
        <LazyView loader={views.BedrockWorldsView}
          instance={activeBedrockInstance}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onEditLevelDat={(dir) => { levelDatWorldDir = dir; openPanel('leveldat') }}
          onToast={showToast}
        />
      {:else if currentView === 'bedrockpacks'}
        <LazyView loader={views.BedrockPacksView}
          instance={activeBedrockInstance}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'bedrock-addons'}
        <LazyView loader={views.BrowseAddonsView}
          {settings}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'bedrockbackups'}
        <LazyView loader={views.BedrockBackupsView}
          instance={activeBedrockInstance}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'bedrockscreenshots'}
        <LazyView loader={views.BedrockScreenshotsView}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'leveldat'}
        <LazyView loader={views.LevelDatEditorView} worldDir={levelDatWorldDir} onBack={goBack} />
      {:else if currentView === 'settings'}
        <LazyView loader={views.SettingsView}
          {settings}
          onChange={saveSettings}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          initialCategory={settingsInitialCategory}
          onCheckForUpdates={checkForUpdatesNow}
          checkingUpdates={checkingUpdatesManual}
          updateAvailable={launcherUpdateAvailable}
          onUndoLastChange={undoLastSettingsChange}
          {canUndoSettings}
        />
      {:else if currentView === 'java-mods'}
        <LazyView loader={views.JavaModsView} {settings} onBack={goBack} onNavigate={navigateFromPanel} onToast={showToast} />
      {:else if currentView === 'modpacks'}
        <LazyView loader={views.ModpacksView} {settings} onBack={goBack} />
      {:else if currentView === 'versions'}
        <LazyView loader={views.VersionsView}
          {settings}
          onBack={goBack}
          onSaveSettings={saveSettings}
          onSelectClient={(name) => settings && saveSettings({ ...settings, SelectedClient: name })}
          onLaunchDll={launchWithDll}
          onToast={showToast}
          breadcrumb={breadcrumbTrail}
          onBreadcrumbJump={jumpToBreadcrumb}
        />
      {:else if currentView === 'tweaks'}
        <LazyView loader={views.TweaksView}
          edition={isBedrock ? 'Bedrock' : 'Java'}
          minecraftDir={(isBedrock ? activeBedrockInstance?.directory : activeJavaInstance?.directory) ?? ''}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'servers'}
        <LazyView loader={views.ServersView}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={(m, k) => showToast(m, k ?? 'info')}
        />
      {:else if currentView === 'logs'}
        <LazyView loader={views.LogsPanel}
          instanceDir={(isBedrock ? activeBedrockInstance?.directory : activeJavaInstance?.directory) ?? ''}
          instanceId={isBedrock ? undefined : activeJavaInstance?.id}
          geminiApiKey={settings?.GeminiApiKey ?? ''}
          onBack={goBack}
        />
      {:else if currentView === 'stats'}
        <LazyView loader={views.StatsPanel} onBack={goBack} />
      {:else if currentView === 'skinlibrary'}
        <LazyView loader={views.SkinLibraryView}
          {settings}
          onBack={goBack}
          onSaveSettings={saveSettings}
          onToast={showToast}
        />
      {:else if currentView === 'java-backups'}
        <LazyView loader={views.JavaBackupsView}
          instanceName={activeJavaInstance?.name ?? ''}
          onBack={goBack}
          onToast={showToast}
        />
      {:else if currentView === 'levilamina-mods'}
        <LazyView loader={views.LeviLaminaModsView} onBack={goBack} onToast={showToast} />
      {:else if currentView === 'mc-versions'}
        <LazyView loader={views.MCVersionsView}
          {settings}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onSaveSettings={saveSettings}
          onToast={showToast}
        />
      {:else if currentView === 'launchers'}
        <LazyView loader={views.LauncherClientsView}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={showToast}
          activeClient={activeJavaClient}
          onSelectClient={selectJavaClient}
        />
      {:else if currentView === 'bedrock-versions'}
        <LazyView loader={views.BedrockVanillaVersionsView}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={(m, k) => showToast(m, k ?? 'info')}
        />
      {:else if currentView === 'profile'}
        <LazyView loader={views.ProfileView}
          {settings}
          {mcRunning}
          onChange={saveSettings}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'import-launcher'}
        <LazyView loader={views.ImportLauncherView}
          onBack={goBack}
          onToast={showToast}
          onImported={() => {}}
        />
      {:else if currentView === 'java-screenshots'}
        <LazyView loader={views.JavaScreenshotsView} onBack={goBack} onNavigate={navigateFromPanel} />
      {:else if currentView === 'instance-wizard'}
        <LazyView loader={views.InstanceWizard}
          onDone={(instance) => {
            goBack()
            activeJavaInstance = instance
            showToast(`Created "${instance.name}".`, 'success', {
              label: 'Play',
              icon: 'fa-solid fa-play',
              run: () => void handleLaunch(),
            })
          }}
          onCancel={goBack}
          onToast={showToast}
        />
      {:else if currentView === 'instance-detail'}
        {#if activeJavaInstance}
          <LazyView loader={views.InstanceDetailView}
            instance={activeJavaInstance}
            onBack={goBack}
            onToast={showToast}
            onLaunch={(id) => { void id; goBack() }}
          />
        {:else}
          <div class="panel-body"><p>No instance selected.</p></div>
        {/if}
      {:else if currentView === 'modrinth-browse-v2'}
        <LazyView loader={views.ModrinthBrowseV2} instanceId={activeJavaInstance?.id} onBack={goBack} onToast={showToast} />
      {:else if currentView === 'sync-packs'}
        <LazyView loader={views.SyncPacksView} onBack={goBack} onToast={showToast} />
      {:else if currentView === 'utilities'}
        <LazyView loader={views.UtilitiesView}
          instanceDir={activeBedrockInstance?.directory ?? ''}
          initialTab={utilInitialTab}
          onBack={goBack}
          onNavigate={navigateFromPanel}
          onToast={showToast}
        />
      {:else if currentView === 'clientlibrary'}
        <LazyView loader={views.ClientLibraryView}
          instanceDir={activeBedrockInstance?.directory ?? ''}
          onBack={goBack}
          onNavigate={navigateFromPanel}
        />
      {:else if currentView === 'downloads'}
        <LazyView loader={views.DownloadsView} onBack={goBack} onOpenSettings={() => openPanel('settings')} onOpenCredits={() => openPanel('credits')} />
      {:else if currentView === 'news'}
        <LazyView loader={views.NewsView} onBack={goBack} onOpenSettings={() => openPanel('settings')} onOpenCredits={() => openPanel('credits')} />
      {:else if currentView === 'credits'}
        <LazyView loader={views.CreditsView}
          edition={isBedrock ? 'bedrock' : 'java'}
          onBack={goBack}
          onNavigate={navigateFromPanel}
        />
      {/if}

    </div>
  {/if}

  <Modal
    open={crashModalOpen && !!crashAnalysis}
    title={crashAnalysis ? `Game crashed — ${crashAnalysis.category}` : 'Game crashed'}
    icon="fa-solid fa-skull-crossbones"
    iconColor="var(--red)"
    onClose={closeCrashModal}
  >
    {#if crashAnalysis}
      <div class="crash-modal-body">
        <p class="crash-modal-message">{crashAnalysis.message}</p>
        <p class="crash-modal-fix"><i class="fa-solid fa-wrench"></i> {crashAnalysis.suggested_fix}</p>
        {#if crashShareUrl}
          <div class="logs-share-url">
            <i class="fa-solid fa-link"></i>
            <a href={crashShareUrl} target="_blank" rel="noreferrer">{crashShareUrl}</a>
          </div>
        {/if}
      </div>
      <div class="crash-modal-actions">
        <button type="button" class="btn-sm" onclick={openCrashLogFolder}>
          <i class="fa-solid fa-folder-open"></i> Open log folder
        </button>
        <button type="button" class="btn-sm" disabled={crashSharing} onclick={shareCrashLog}>
          {#if crashSharing}
            <i class="fa-solid fa-spinner fa-spin"></i>
          {:else}
            <i class="fa-solid fa-share-nodes"></i>
          {/if}
          Share to mclo.gs
        </button>
        <button type="button" class="btn-sm" onclick={closeCrashModal}>Dismiss</button>
      </div>
    {/if}
  </Modal>

  <UpdateAvailableModal
    open={updateModalOpen}
    update={availableUpdateInfo}
    {currentVersion}
    onClose={() => (updateModalOpen = false)}
    onToast={showToast}
  />

  <Modal
    open={confirmQuitDownloadsOpen}
    title="Downloads in progress"
    icon="fa-solid fa-triangle-exclamation"
    iconColor="var(--orange)"
    onClose={() => (confirmQuitDownloadsOpen = false)}
  >
    <p class="setting-hint">Quitting now will cancel any downloads still in progress. Quit anyway?</p>
    <div class="modal-footer-actions" style="display:flex; justify-content:flex-end; gap:10px; margin-top:16px;">
      <button class="modal-btn modal-btn-cancel" onclick={() => (confirmQuitDownloadsOpen = false)}>Keep waiting</button>
      <button class="onboarding-continue-btn" onclick={() => invoke('force_quit').catch((e) => showToast(`Quit failed: ${e}`, 'error'))}>Quit anyway</button>
    </div>
  </Modal>

  <Modal
    open={whatsNewOpen}
    title="What's new in v{currentVersion}"
    icon="fa-solid fa-sparkles"
    onClose={() => (whatsNewOpen = false)}
  >
    <div class="update-modal-changelog">{@html formatChangelog(whatsNewChangelog)}</div>
    <div class="modal-footer-actions" style="display:flex; justify-content:flex-end; margin-top:16px;">
      <button class="onboarding-continue-btn" onclick={() => (whatsNewOpen = false)}>Got it</button>
    </div>
  </Modal>
</div>
