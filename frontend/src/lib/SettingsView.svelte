<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open, save } from '@tauri-apps/plugin-dialog'
  import PanelHeader from './components/PanelHeader.svelte'
  import Toggle from './components/Toggle.svelte'
  import ThemeStudioView from './ThemeStudioView.svelte'
  import { t, LANGUAGES } from './i18n'
  import { loadFilterPref, saveFilterPref } from './filterMemory'
  import type { LauncherSettings, JavaAccount } from './settings'

  // 1:1 port of the `settings` panel — Home.razor lines 1013-1928 plus
  // Pages/Home.Settings.cs. Every label, hint and option string below is
  // transcribed from the original; the category strip is the panel's own
  // inline `.versions-client-switcher` (NOT the shared SubTabs). Inline styles
  // live in the scoped <style> block at the bottom; the shared row, label and
  // toggle classes still come from app.css.

  let {
    settings,
    onChange,
    onBack,
    onNavigate = () => {},
    initialCategory,
    onCheckForUpdates,
    checkingUpdates = false,
    updateAvailable = false,
    onUndoLastChange,
    canUndoSettings = false,
  }: {
    settings: LauncherSettings
    onChange: (s: LauncherSettings) => void
    onBack?: () => void
    onNavigate?: (view: string) => void
    initialCategory?: 'all' | 'injection' | 'java' | 'appearance' | 'account' | 'system'
    /** Drives the same check the top-bar pill polls with, then opens the
     *  shared `UpdateAvailableModal` — see App.svelte's `checkForUpdatesNow`. */
    onCheckForUpdates?: () => void
    checkingUpdates?: boolean
    updateAvailable?: boolean
    /** QOL: single-level undo, restoring the snapshot from right before the
     *  most recent settings save — see App.svelte's `undoLastSettingsChange`. */
    onUndoLastChange?: () => void
    canUndoSettings?: boolean
  } = $props()

  function update<K extends keyof LauncherSettings>(key: K, value: LauncherSettings[K]) {
    onChange({ ...settings, [key]: value })
  }

  async function toggleDiscordRpc() {
    const enabling = !settings.DiscordRichPresence
    try {
      await invoke(enabling ? 'start_discord_presence' : 'stop_discord_presence')
    } catch {
      // best-effort — still flip the persisted preference
    }
    update('DiscordRichPresence', enabling)
  }

  async function connectDiscord() {
    discordConnecting = true
    actionMessage = ''
    try {
      const profile = await invoke<{ username: string; avatar: string }>('login_discord')
      onChange({ ...settings, DiscordLoggedIn: true, DiscordUsername: profile.username, DiscordAvatar: profile.avatar })
      actionMessage = 'Signed in as ' + profile.username
    } catch (e) {
      actionMessage = 'Discord login failed: ' + String(e)
    } finally {
      discordConnecting = false
    }
  }

  async function disconnectDiscord() {
    discordConnecting = true
    actionMessage = ''
    try {
      await invoke('logout_discord')
      onChange({ ...settings, DiscordLoggedIn: false, DiscordUsername: '', DiscordAvatar: '' })
      actionMessage = 'Discord disconnected.'
    } catch (e) {
      actionMessage = 'Discord disconnect failed: ' + String(e)
    } finally {
      discordConnecting = false
    }
  }

  type SignInResult = {
    xbox_profile: {
      gamertag: string
      xuid: string
      gamer_picture_url: string
      gamerscore: string
      account_tier: string
      bio: string
    }
    minecraft_profile: { uuid: string; name: string; skin_url: string }
    minecraft_access_token: string
    minecraft_access_token_expiry_unix: number
    live_refresh_token: string | null
  }

  let xboxSigningIn = $state(false)

  /** Unix seconds → the RFC-3339-ish string `resolve_java_auth`
   * (`src-tauri/src/commands/launch.rs`) expects in `JavaAccessTokenExpiry`. */
  function unixToIso(secs: number): string {
    return new Date(secs * 1000).toISOString().replace(/\.\d+Z$/, 'Z')
  }

  /** Port of `PersistAccount`: add-or-update the signed-in account in
   * `JavaAccounts` (keyed by Minecraft UUID, falling back to the Xbox XUID
   * when Minecraft auth didn't resolve a profile), and make it active. */
  function applySignIn(result: SignInResult) {
    const xbox = result.xbox_profile
    const mc = result.minecraft_profile
    const id = mc.uuid || xbox.xuid
    const expiry = unixToIso(result.minecraft_access_token_expiry_unix)

    const accounts = [...settings.JavaAccounts]
    const idx = accounts.findIndex((a) => a.Id.toLowerCase() === id.toLowerCase())
    const account: JavaAccount = {
      Id: id,
      Username: mc.name,
      Uuid: mc.uuid,
      Gamertag: xbox.gamertag,
      Xuid: xbox.xuid,
      GamerPictureUrl: xbox.gamer_picture_url,
      MinecraftAccessToken: result.minecraft_access_token,
      MinecraftAccessTokenExpiry: expiry,
      MinecraftSkinUrl: mc.skin_url,
      RefreshToken: result.live_refresh_token || accounts[idx]?.RefreshToken || '',
      LastUsedAt: new Date().toISOString(),
    }
    if (idx >= 0) accounts[idx] = account
    else accounts.push(account)

    onChange({
      ...settings,
      XboxGamertag: xbox.gamertag,
      XboxXuid: xbox.xuid,
      XboxGamerPictureUrl: xbox.gamer_picture_url,
      XboxGamerscore: xbox.gamerscore,
      XboxAccountTier: xbox.account_tier,
      XboxBio: xbox.bio,
      XboxLiveRefreshToken: result.live_refresh_token ?? settings.XboxLiveRefreshToken,
      JavaUsername: mc.name,
      JavaUuid: mc.uuid,
      JavaAccessToken: result.minecraft_access_token,
      JavaAccessTokenExpiry: expiry,
      JavaSkinUrl: mc.skin_url,
      JavaOfflineMode: false,
      JavaAccounts: accounts,
      ActiveJavaAccountId: id,
    })
  }

  /** Adds a new account (or refreshes the current one) via the interactive
   * Microsoft sign-in popup. Port of `XboxAddAccount`/plain `XboxSignIn`. */
  async function connectXbox() {
    xboxSigningIn = true
    actionMessage = ''
    try {
      const result = await invoke<SignInResult>('sign_in')
      applySignIn(result)
      actionMessage = `Signed in as ${result.xbox_profile.gamertag}`
    } catch (e) {
      actionMessage = 'Xbox sign-in failed: ' + String(e)
    } finally {
      xboxSigningIn = false
    }
  }

  /** Port of `XboxSignOut`: clears the active profile/tokens but leaves
   * other saved accounts in `JavaAccounts` untouched. */
  function disconnectXbox() {
    onChange({
      ...settings,
      XboxGamertag: '',
      XboxXuid: '',
      XboxGamerPictureUrl: '',
      XboxGamerscore: '',
      XboxAccountTier: '',
      XboxBio: '',
      JavaUsername: '',
      JavaUuid: '',
      JavaAccessToken: '',
      JavaAccessTokenExpiry: '',
      ActiveJavaAccountId: '',
      JavaOfflineMode: true,
    })
    actionMessage = 'Xbox account disconnected.'
  }

  /** Port of `SwitchJavaAccount`: applies a saved account's tokens without
   * re-authenticating over the network. */
  function switchJavaAccount(account: JavaAccount) {
    onChange({
      ...settings,
      ActiveJavaAccountId: account.Id,
      XboxLiveRefreshToken: account.RefreshToken,
      XboxGamertag: account.Gamertag,
      XboxXuid: account.Xuid,
      XboxGamerPictureUrl: account.GamerPictureUrl,
      JavaUsername: account.Username,
      JavaUuid: account.Uuid,
      JavaAccessToken: account.MinecraftAccessToken,
      JavaAccessTokenExpiry: account.MinecraftAccessTokenExpiry,
      JavaSkinUrl: account.MinecraftSkinUrl,
      JavaOfflineMode: false,
    })
    actionMessage = `Switched to ${account.Username}`
  }

  /** Port of `RemoveJavaAccount`: drops the saved account, signing out if
   * it was the active one. */
  function removeJavaAccount(account: JavaAccount) {
    const accounts = settings.JavaAccounts.filter((a) => a.Id !== account.Id)
    if (settings.ActiveJavaAccountId === account.Id) {
      onChange({
        ...settings,
        JavaAccounts: accounts,
        ActiveJavaAccountId: '',
        XboxGamertag: '',
        XboxXuid: '',
        XboxGamerPictureUrl: '',
        XboxGamerscore: '',
        XboxAccountTier: '',
        XboxBio: '',
        JavaUsername: '',
        JavaUuid: '',
        JavaAccessToken: '',
        JavaAccessTokenExpiry: '',
        JavaOfflineMode: true,
      })
    } else {
      onChange({ ...settings, JavaAccounts: accounts })
    }
    actionMessage = 'Account removed.'
  }

  const isBedrock = $derived(settings.Edition === 'bedrock')
  const isJava = $derived(!isBedrock)

  // QOL: remembers the last-open Settings category across restarts when the
  // caller didn't ask for a specific one (`initialCategory` unset).
  let category = $state<'all' | 'injection' | 'java' | 'appearance' | 'account' | 'system'>(
    initialCategory ?? (loadFilterPref('settings.category', 'all') as typeof category),
  )
  let filter = $state('')
  /** Collapsed by default; a non-empty search always opens it so matches show. */
  let javaAdvancedOpen = $state(false)
  const javaAdvancedShown = $derived(javaAdvancedOpen || q() !== '')

  /** Port of `Home.razor.cs`'s `_accentSwatches` — verbatim hex values. */
  const accentSwatches = ['#7289da', '#5865f2', '#3ba55d', '#faa61a', '#f04747', '#eb459e', '#00d8ff', '#ff7043', '#a855f7', '#22d3ee', '#84cc16', '#f59e0b']

  function q() {
    return filter.trim().toLowerCase()
  }

  /** Port of `CategoryMatches`. */
  function categoryVisible(section: string): boolean {
    if (isJava && section === 'injection') return false
    if (isBedrock && section === 'java') return false
    if (q()) return true
    return category === 'all' || category === section
  }

  /** Port of `SH()` — true when the row carries `hidden-by-search`. */
  function hidden(label: string, keywords: string): boolean {
    const query = q()
    if (!query) return false
    return !label.toLowerCase().includes(query) && !keywords.toLowerCase().includes(query)
  }

  /** Port of `SectionMatch()`. */
  function sectionVisible(...items: [string, string][]): boolean {
    const query = q()
    if (!query) return true
    return items.some(([l, k]) => l.toLowerCase().includes(query) || k.toLowerCase().includes(query))
  }

  /** Verbatim port of `_settingDefs` (Home.Settings.cs:73-124). */
  const settingDefs: [string, string, string][] = [
    ['injection', 'Active client', 'active client injection vanilla'],
    ['injection', 'Injection delay', 'injection delay wait'],
    ['injection', 'Auto-inject', 'auto inject automatic'],
    ['injection', 'Close after launch', 'close minimize launch'],

    ['java', 'Maximum RAM', 'ram memory java heap'],
    ['java', 'Minecraft folder', 'minecraft folder dot directory'],
    ['java', 'Java runtime', 'java runtime jre jdk javaw'],
    ['java', 'Show snapshots', 'snapshots experimental'],
    ['java', 'Show beta and alpha', 'beta alpha historical old'],
    ['java', 'Close after launch', 'close minimize launch java'],
    ['java', 'Offline mode', 'offline mode cracked no account username'],
    ['java', 'Custom JVM args', 'jvm args java options garbage collector gc'],
    ['java', 'Open Minecraft folder', 'minecraft folder dot directory open'],
    ['java', 'Latest crash report', 'crash report log latest open view java'],

    ['appearance', 'Accent colour', 'accent color theme'],
    ['appearance', 'Theme preset', 'theme dark legacy midnight slate ocean forest sunset light'],
    ['appearance', 'Panel side', 'panel side left right sidebar position'],
    ['appearance', 'Compact mode', 'compact dense small'],
    ['appearance', 'Animations', 'animations smooth motion'],
    ['appearance', 'Accessibility features', 'accessibility a11y reduced motion focus outline screen reader'],
    ['appearance', 'Animation speed', 'animation speed motion fast slow multiplier'],
    ['appearance', 'Background blur', 'blur background'],
    ['appearance', 'Remember window size', 'window size remember'],
    ['appearance', 'Custom background wallpaper', 'background wallpaper image custom'],
    ['appearance', 'Background fit', 'background fit cover contain tile center'],
    ['appearance', 'UI scale', 'ui scale zoom size interface'],
    ['appearance', 'Font', 'font family typeface typography'],

    ['account', 'Display name', 'username name display account'],
    ['account', 'Profile display', 'profile display footer xbox discord switch which'],
    ['account', 'Discord Rich Presence', 'discord rpc social'],
    ['account', 'Discord account', 'discord login account'],
    ['account', 'Xbox profile', 'xbox gamertag account microsoft'],

    ['system', 'Show recently launched', 'recent history'],
    ['system', 'Show launch console', 'console log launch window java bedrock output stdout'],
    ['system', 'Collapse instance groups', 'library instances folders collapse expand sections'],
    ['system', 'Minimize to tray', 'tray minimize system'],
    ['system', 'Clear recent history', 'clear history recent'],
    ['system', 'Check for updates on startup', 'update auto check startup'],
    ['system', 'Check for updates now', 'update manual check'],
    ['system', 'Open launcher folder', 'launcher folder downloads location'],
    ['system', 'Open Minecraft folder', 'minecraft com mojang folder'],
    ['system', 'CurseForge API key', 'curseforge api key addons'],
    ['system', 'Copy settings summary', 'copy diagnostic summary support troubleshooting clipboard'],
    ['system', 'Export settings', 'export backup settings save'],
    ['system', 'Import settings', 'import restore settings load'],
    ['system', 'Reset settings', 'reset default factory settings'],
    ['system', 'Keyboard shortcuts', 'shortcuts keyboard help'],
    ['system', 'Credits', 'credits about launcher'],
  ]

  /** Port of `VisibleSettingsCount()`. */
  const visibleCount = $derived(
    settingDefs.filter(([cat, label, keywords]) => {
      if (isJava && cat === 'injection') return false
      if (isBedrock && cat === 'java') return false
      const query = q()
      if (!query && category !== 'all' && category !== cat) return false
      if (query && !label.toLowerCase().includes(query) && !keywords.toLowerCase().includes(query)) return false
      return true
    }).length,
  )

  /** Count of settings that belong to a category, ignoring the current
   *  filter/search — used for the nav rail's count badges. */
  function categoryCount(cat: string): number {
    return settingDefs.filter(([c]) => c === cat).length
  }

  /** Total settings applicable to the current edition, ignoring category and
   *  filter — the "All" nav button's count badge. */
  const visibleAllCount = $derived(
    settingDefs.filter(([cat]) => !(isJava && cat === 'injection') && !(isBedrock && cat === 'java')).length,
  )

  /** QOL: how many settings match the current filter text, shown next to
   *  the search box — same match rule as `hidden()`/`sectionVisible()`
   *  (label or keywords contains the query), so it's consistent with what
   *  actually gets shown/hidden rather than a separate approximation. */
  const filterMatchCount = $derived.by(() => {
    const query = q()
    if (!query) return -1
    return settingDefs.filter(
      ([cat, label, keywords]) =>
        !(isJava && cat === 'injection') &&
        !(isBedrock && cat === 'java') &&
        (label.toLowerCase().includes(query) || keywords.toLowerCase().includes(query)),
    ).length
  })

  function setCategory(c: typeof category) {
    category = c
    saveFilterPref('settings.category', c)
  }
  function clearFilter() {
    filter = ''
  }

  let exporting = $state(false)
  let importing = $state(false)
  let resetting = $state(false)
  let resetConfirmOpen = $state(false)
  let deletingAllData = $state(false)
  let deleteAllConfirmOpen = $state(false)
  let lastUpdateCheckLabel = $derived(
    updateAvailable
      ? 'Update available'
      : settings.LastUpdateCheck
        ? 'Last checked ' + formatRelativeTime(settings.LastUpdateCheck)
        : 'Never checked'
  )
  let actionMessage = $state('')
  let discordConnecting = $state(false)

  // QOL: compact startup-diagnostics summary instead of a whole separate
  // panel — disk space, last update check, and last crash are each already
  // available via existing commands (System tab, `check_launcher_update`,
  // `latest_java_crash_report`); this just surfaces the three together in
  // one glance for a bug report instead of hunting through three places.
  type Diagnostics = { diskName: string; diskFreeLabel: string; lastCrash: string | null }
  let diagnostics = $state<Diagnostics | null>(null)
  let diagnosticsFailed = $state(false)

  function formatGb(bytes: number): string {
    return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`
  }

  async function loadDiagnostics() {
    try {
      const root = await invoke<string>('get_launcher_root')
      const driveLetter = root[0]?.toUpperCase() ?? 'C'
      const snapshot = await invoke<{ disks: { name: string; free_bytes: number }[] }>('get_system_snapshot')
      const disk = snapshot.disks.find((d) => d.name === driveLetter) ?? snapshot.disks[0]
      let lastCrash: string | null = null
      if (settings.JavaMinecraftDir) {
        const crash = await invoke<{ name: string } | null>('latest_java_crash_report', {
          minecraftDir: settings.JavaMinecraftDir,
        }).catch(() => null)
        lastCrash = crash?.name ?? null
      }
      diagnostics = {
        diskName: disk ? `${disk.name}:` : '?',
        diskFreeLabel: disk ? formatGb(disk.free_bytes) : 'unknown',
        lastCrash,
      }
      diagnosticsFailed = false
    } catch {
      diagnostics = null
      diagnosticsFailed = true
    }
  }
  loadDiagnostics()

  /** Port of `FormatRelativeTime` (Home.razor.cs:677-690). */
  function formatRelativeTime(isoTimestamp: string): string {
    try {
      const dt = new Date(isoTimestamp)
      if (isNaN(dt.getTime())) return ''
      const diffMs = Date.now() - dt.getTime()
      const minutes = diffMs / 60000
      const hours = minutes / 60
      const days = hours / 24
      if (minutes < 2) return 'just now'
      if (hours < 1) return `${Math.floor(minutes)}m ago`
      if (days < 1) return `${Math.floor(hours)}h ago`
      if (days < 7) return `${Math.floor(days)}d ago`
      return dt.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
    } catch {
      return ''
    }
  }

  async function openFolder(path: string) {
    try {
      await invoke('open_folder', { path })
    } catch (e) {
      actionMessage = String(e)
    }
  }

  type CrashReportInfo = { name: string; path: string; modified_at: string }
  let latestCrash = $state<CrashReportInfo | null>(null)

  const latestCrashLabel = $derived(
    latestCrash
      ? `${latestCrash.name} · ${formatRelativeTime(new Date(Number(latestCrash.modified_at) * 1000).toISOString())}`
      : 'No crash reports yet',
  )

  $effect(() => {
    const dir = settings.JavaMinecraftDir
    if (!dir) {
      latestCrash = null
      return
    }
    invoke<CrashReportInfo | null>('latest_java_crash_report', { minecraftDir: dir })
      .then((info) => (latestCrash = info))
      .catch(() => (latestCrash = null))
  })

  function openLatestCrash() {
    if (!latestCrash) return
    invoke('open_path', { path: latestCrash.path }).catch((e) => (actionMessage = String(e)))
  }

  async function openLauncherFolder() {
    try {
      const root = await invoke<string>('get_launcher_root')
      await openFolder(root)
    } catch (e) {
      actionMessage = String(e)
    }
  }

  async function openMinecraftDataFolder() {
    try {
      const path = await invoke<string>('resolve_com_mojang_folder')
      await openFolder(path)
    } catch (e) {
      actionMessage = String(e)
    }
  }

  /** One-click plain-text diagnostic snapshot for support/bug reports —
   * distinct from `exportSettings`'s full JSON backup: this is meant to be
   * pasted straight into a Discord message or GitHub issue, not restored. */
  async function copySettingsSummary() {
    let launcherVersion = 'unknown'
    try {
      launcherVersion = await invoke<string>('launcher_version')
    } catch {
      // best-effort — still produce a summary without it
    }
    const lines = [
      `Glacier Launcher ${launcherVersion}`,
      `Edition: ${settings.Edition}`,
      `Selected client: ${settings.SelectedClient}`,
      `Java: max ${settings.JavaMaxRamMb} MB / min ${settings.JavaMinRamMb} MB, runtime: ${settings.JavaRuntimePath || 'auto'}`,
      `Discord Rich Presence: ${settings.DiscordRichPresence ? 'on' : 'off'}`,
      `Show launch console: ${settings.ShowLaunchConsole ? 'on' : 'off'}`,
      `Prevent game suspend: ${settings.PreventGameSuspend ? 'on' : 'off'}`,
      diagnostics
        ? `Disk: ${diagnostics.diskFreeLabel} free on ${diagnostics.diskName}${diagnostics.lastCrash ? ` · Last crash: ${diagnostics.lastCrash}` : ''}`
        : 'Disk: unknown',
      `Last update check: ${settings.LastUpdateCheck ? formatRelativeTime(settings.LastUpdateCheck) : 'never'}`,
    ]
    const summary = lines.join('\n')
    try {
      await navigator.clipboard.writeText(summary)
      actionMessage = 'Settings summary copied to clipboard.'
    } catch (e) {
      actionMessage = 'Could not copy to clipboard: ' + String(e)
    }
  }

  async function exportSettings() {
    const target = await save({
      title: 'Export Glacier Launcher settings',
      defaultPath: 'glacier-settings.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!target) return
    exporting = true
    actionMessage = ''
    try {
      const path = await invoke<string>('export_settings', { path: target })
      actionMessage = `Settings exported. (${path})`
    } catch (e) {
      actionMessage = 'Export failed: ' + String(e)
    } finally {
      exporting = false
    }
  }

  async function importSettings() {
    const source = await open({
      title: 'Import Glacier Launcher settings',
      multiple: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!source || typeof source !== 'string') return
    importing = true
    actionMessage = ''
    try {
      const imported = await invoke<LauncherSettings>('import_latest_settings', { path: source })
      onChange(imported)
      actionMessage = 'Settings imported.'
    } catch (e) {
      actionMessage = 'Import failed: ' + String(e)
    } finally {
      importing = false
    }
  }

  // QOL: config profiles — named settings snapshots, separate from the
  // single rolling manual/auto "export" file.
  type ConfigProfile = { name: string; path: string; modified_at: string }
  let profiles = $state<ConfigProfile[]>([])
  let newProfileName = $state('')

  async function loadProfiles() {
    try {
      profiles = await invoke<ConfigProfile[]>('list_config_profiles')
    } catch {
      profiles = []
    }
  }
  loadProfiles()

  async function saveProfile() {
    const name = newProfileName.trim()
    if (!name) return
    try {
      await invoke('save_config_profile', { name })
      newProfileName = ''
      actionMessage = `Profile "${name}" saved.`
      await loadProfiles()
    } catch (e) {
      actionMessage = String(e)
    }
  }

  async function loadProfile(p: ConfigProfile) {
    try {
      const loaded = await invoke<LauncherSettings>('load_config_profile', { path: p.path })
      onChange(loaded)
      actionMessage = `Profile "${p.name}" loaded.`
    } catch (e) {
      actionMessage = String(e)
    }
  }

  async function deleteProfile(p: ConfigProfile) {
    try {
      await invoke('delete_config_profile', { path: p.path })
      await loadProfiles()
    } catch (e) {
      actionMessage = String(e)
    }
  }

  async function doReset() {
    resetting = true
    try {
      const fresh = await invoke<LauncherSettings>('reset_settings')
      onChange(fresh)
      actionMessage = 'Settings reset.'
    } catch (e) {
      actionMessage = String(e)
    } finally {
      resetting = false
      resetConfirmOpen = false
    }
  }

  /**
   * Wipes the entire `%USERPROFILE%\Glacier Launcher` data folder via the
   * backend's `delete_all_data` (best-effort - keeps deleting past any
   * individual locked file rather than aborting) and relaunches into a
   * clean install. The relaunch means a success response never actually
   * reaches this function - only a partial-failure `Err` does, which is
   * surfaced so the user knows a few files were skipped rather than
   * assuming everything is gone.
   */
  async function doDeleteAllData() {
    deletingAllData = true
    try {
      await invoke('delete_all_data')
    } catch (e) {
      actionMessage = String(e)
    } finally {
      deletingAllData = false
      deleteAllConfirmOpen = false
    }
  }

  /// "Check for updates now" - actually checking, downloading, and
  /// installing all now live in one place (App.svelte's `checkForUpdatesNow`
  /// + `UpdateAvailableModal`), reached through `onCheckForUpdates`. This
  /// used to be its own separate inline check/download/install flow with
  /// its own copy of all three `invoke` calls, which could silently drift
  /// from the top-bar pill's version of the same flow.
  function manualUpdateCheck() {
    onChange({ ...settings, LastUpdateCheck: new Date().toISOString() })
    onCheckForUpdates?.()
  }

  function clearRecentHistory() {
    update('RecentlyLaunched', [])
  }

  // Appearance changes go through the ported interop.js helpers, exactly as
  // the original called them from OnThemeChanged/SetAccent/etc.
  function setAccent(color: string) {
    update('AccentColor', color)
    window.setAccentColor?.(color)
  }

  /** Drives every slider's filled-track gradient (`--pct` custom property,
   *  see `input[type=range]` in app.css) from its own value/min/max. */
  function pct(value: number, min: number, max: number): string {
    return `${max > min ? Math.round(((value - min) / (max - min)) * 100) : 0}%`
  }

  function setThemePreset(val: string) {
    update('ThemePreset', val)
    window.setTheme?.(val)
  }

  function setUiScale(v: number) {
    const clamped = Math.min(150, Math.max(75, v))
    update('UiScalePct', clamped)
    window.setUiScale?.(clamped)
  }

  function setBlur(v: number) {
    update('BlurIntensity', v)
    window.setBlurIntensity?.(v)
  }

  function setAnimSpeed(v: number) {
    const clamped = Math.min(2, Math.max(0.25, v))
    update('AnimationSpeed', clamped)
    window.setAnimationSpeed?.(clamped)
  }

  function setFont(family: string) {
    update('FontFamily', family.trim())
    window.setFont?.(family.trim())
  }

  function setBackgroundFit(val: string) {
    const v = val === 'contain' || val === 'tile' || val === 'center' ? val : 'cover'
    update('BackgroundFit', v)
    window.setBackgroundFit?.(v, 1.0)
  }

  function setCompactMode(on: boolean) {
    update('CompactMode', on)
    window.setCompactMode?.(on)
  }

  function setPanelSide(side: 'left' | 'right') {
    update('PanelSide', side)
    window.setPanelSide?.(side)
  }

  function setAnimationsEnabled(on: boolean) {
    update('AnimationsEnabled', on)
    window.setAnimationsEnabled?.(on)
  }

  function setAccessibilityMode(on: boolean) {
    update('AccessibilityMode', on)
    window.setAccessibilityMode?.(on)
  }

  // Feature 37: rebindable keyboard shortcuts. `KeyboardShortcuts` is a JSON
  // string ({} by default) mapping action -> normalized combo, mirroring the
  // format `interop.js`'s `comboFromEvent`/`setKeyboardShortcuts` use.
  const REBINDABLE_SHORTCUTS: { action: string; label: string; defaultCombo: string }[] = [
    { action: 'search', label: 'Open search', defaultCombo: 'ctrl+k' },
    { action: 'launch', label: 'Launch game', defaultCombo: 'ctrl+l' },
    { action: 'fullscreen', label: 'Toggle fullscreen', defaultCombo: 'f11' },
  ]

  let capturingShortcut = $state<string | null>(null)

  function parsedShortcuts(): Record<string, string> {
    try {
      return JSON.parse(settings.KeyboardShortcuts || '{}') ?? {}
    } catch {
      return {}
    }
  }

  function shortcutFor(action: string, defaultCombo: string): string {
    return parsedShortcuts()[action] || defaultCombo
  }

  function comboLabel(combo: string): string {
    return combo
      .split('+')
      .map((p) => (p.length === 1 ? p.toUpperCase() : p[0].toUpperCase() + p.slice(1)))
      .join('+')
  }

  function startCaptureShortcut(action: string) {
    capturingShortcut = action
  }

  function captureShortcutKeydown(e: KeyboardEvent, action: string) {
    e.preventDefault()
    e.stopPropagation()
    if (e.key === 'Escape') {
      capturingShortcut = null
      return
    }
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return
    const parts: string[] = []
    if (e.ctrlKey) parts.push('ctrl')
    if (e.altKey) parts.push('alt')
    if (e.shiftKey) parts.push('shift')
    parts.push(e.key.length === 1 ? e.key.toLowerCase() : e.key)
    const combo = parts.join('+')
    const next = { ...parsedShortcuts(), [action]: combo }
    update('KeyboardShortcuts', JSON.stringify(next))
    window.setKeyboardShortcuts?.(JSON.stringify(next))
    capturingShortcut = null
  }

  function resetShortcut(action: string) {
    const next = { ...parsedShortcuts() }
    delete next[action]
    update('KeyboardShortcuts', JSON.stringify(next))
    window.setKeyboardShortcuts?.(JSON.stringify(next))
  }

  // QOL: global (system-wide, works even minimized/unfocused) quick-launch
  // hotkey — a separate mechanism from the in-app-only shortcuts above
  // (those are plain JS keydown listeners, dead the moment the window
  // isn't focused). Registered backend-side via `set_quick_launch_hotkey`
  // (tauri-plugin-global-shortcut), which also persists it to settings.
  let capturingQuickLaunchHotkey = $state(false)
  let quickLaunchHotkeyError = $state('')

  function quickLaunchHotkeyLabel(): string {
    if (!settings.QuickLaunchHotkey) return 'Not set'
    return settings.QuickLaunchHotkey.replace('CommandOrControl', 'Ctrl')
  }

  async function captureQuickLaunchHotkey(e: KeyboardEvent) {
    e.preventDefault()
    e.stopPropagation()
    if (e.key === 'Escape') {
      capturingQuickLaunchHotkey = false
      return
    }
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return
    const parts: string[] = []
    if (e.ctrlKey || e.metaKey) parts.push('CommandOrControl')
    if (e.altKey) parts.push('Alt')
    if (e.shiftKey) parts.push('Shift')
    // Tauri accelerators want e.g. "L", "F9", "Space" — single letters
    // uppercased, named keys passed through as-is.
    const key = e.key.length === 1 ? e.key.toUpperCase() : e.key
    parts.push(key)
    if (parts.length < 2) {
      // A bare key with no modifier would hijack that key system-wide for
      // every other app — refuse rather than silently registering it.
      quickLaunchHotkeyError = 'Include at least one modifier (Ctrl/Alt/Shift).'
      return
    }
    const combo = parts.join('+')
    capturingQuickLaunchHotkey = false
    quickLaunchHotkeyError = ''
    try {
      await invoke('set_quick_launch_hotkey', { combo })
      update('QuickLaunchHotkey', combo)
    } catch (e) {
      quickLaunchHotkeyError = String(e)
    }
  }

  async function clearQuickLaunchHotkey() {
    try {
      await invoke('set_quick_launch_hotkey', { combo: '' })
      update('QuickLaunchHotkey', '')
      quickLaunchHotkeyError = ''
    } catch (e) {
      quickLaunchHotkeyError = String(e)
    }
  }

  function resetWallpaper() {
    update('CustomBackgroundPath', '')
    window.setCustomBackground?.('')
  }

  async function chooseWallpaper() {
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif'] }],
      })
      if (typeof picked !== 'string') return
      update('CustomBackgroundPath', picked)
      window.setCustomBackground?.(picked)
    } catch (e) {
      actionMessage = String(e)
    }
  }

  function fileName(path: string): string {
    return path.replace(/\\/g, '/').split('/').pop() ?? path
  }

  const MINECRAFT_DIR_DISABLED = 'Unavailable: set a custom Minecraft folder above, or add a backend command that resolves it'
</script>

<PanelHeader title={t('panel.settings', settings.Language)} {onBack} />

<!--
  Restructured from a horizontal pill switcher (which only ever showed one
  category name at a time and pushed the actual settings down a whole extra
  row) into a persistent vertical rail, like every other multi-category
  settings surface (VS Code, Discord, Windows Settings): every category is
  visible and one click away at all times, and the content pane gets the
  full remaining height to itself instead of fighting the switcher for it.
-->
<div class="settings-layout">
  <nav class="settings-nav-rail" aria-label={t('tt.settingsCategories', settings.Language)}>
    <button
      type="button"
      class="settings-nav-btn {category === 'all' ? 'active' : ''}"
      onclick={() => setCategory('all')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-layer-group"></i> All
      <span class="settings-nav-btn-count">{visibleAllCount}</span>
    </button>
    {#if isBedrock}
      <button
        type="button"
        class="settings-nav-btn {category === 'injection' ? 'active' : ''}"
        onclick={() => setCategory('injection')}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-syringe"></i> Injection
        <span class="settings-nav-btn-count">{categoryCount('injection')}</span>
      </button>
    {:else}
      <button
        type="button"
        class="settings-nav-btn {category === 'java' ? 'active' : ''}"
        onclick={() => setCategory('java')}
        onmousedown={(e) => e.stopPropagation()}
      >
        <img class="chip-icon nav-logo" src="images/mc-java-logo.svg" alt=""> Java
        <span class="settings-nav-btn-count">{categoryCount('java')}</span>
      </button>
    {/if}
    <button
      type="button"
      class="settings-nav-btn {category === 'appearance' ? 'active' : ''}"
      onclick={() => setCategory('appearance')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-palette"></i> Looks
      <span class="settings-nav-btn-count">{categoryCount('appearance')}</span>
    </button>
    <button
      type="button"
      class="settings-nav-btn {category === 'account' ? 'active' : ''}"
      onclick={() => setCategory('account')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-user"></i> Account
      <span class="settings-nav-btn-count">{categoryCount('account')}</span>
    </button>
    <button
      type="button"
      class="settings-nav-btn {category === 'system' ? 'active' : ''}"
      onclick={() => setCategory('system')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-gear"></i> System
      <span class="settings-nav-btn-count">{categoryCount('system')}</span>
    </button>
  </nav>

  <div class="settings-content">
    <div class="panel-search-wrap settings-search-wrap">
      <i class="fa-solid fa-magnifying-glass"></i>
      <input
        class="panel-search-input"
        placeholder={t('ph.filterSettings', settings.Language)}
        bind:value={filter}
        onkeydown={(e) => e.stopPropagation()}
      />
      {#if filter}
        <span class="panel-search-count">{filterMatchCount} match{filterMatchCount === 1 ? '' : 'es'}</span>
        <button class="panel-search-clear" title="Clear filter" aria-label="Clear filter" onclick={clearFilter} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-xmark"></i>
        </button>
      {/if}
    </div>

    <div class="panel-body settings-body">
      {#if visibleCount === 0}
    <div class="empty-state">
      <i class="fa-solid fa-folder-open"></i>
      <span>{t('empty.noMatchingSettings', settings.Language)}</span>
      <small>
        {#if filter.trim()}
          Nothing matches "{filter}".
        {:else}
          This category is empty.
        {/if}
      </small>
      {#if filter.trim()}
        <button class="vcs-btn" onclick={clearFilter} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-xmark"></i> Clear filter
        </button>
      {/if}
    </div>
  {/if}

  {#if categoryVisible('injection')}
    <div class="settings-section">
      {#if sectionVisible(['Active client', 'active client injection vanilla'], ['Injection delay', 'injection delay wait'], ['Auto-inject', 'auto inject automatic'], ['Close after launch', 'close minimize launch'])}
        <span class="panel-section-label">{t('settings.section.injection', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('Active client', 'active client injection vanilla') ? 'hidden-by-search' : ''}">
        <span class="setting-label">{t('x.activeClient', settings.Language)}</span>
        <select
          class="setting-select"
          value={settings.SelectedClient}
          onchange={(e) => update('SelectedClient', (e.currentTarget as HTMLSelectElement).value)}
          onclick={(e) => e.stopPropagation()}
        >
          <option value="Latite Client">Latite Client</option>
          <option value="Flarial Client">Flarial Client</option>
          <option value="OderSo Client">OderSo Client</option>
          <option value="LeviLamina">LeviLamina</option>
          {#if settings.CustomDllPath || settings.DefaultCustomClientId}
            <option value="Custom DLL">{t('x.customDll', settings.Language)}</option>
          {/if}
          <option value="Vanilla">Vanilla (no client)</option>
        </select>
      </div>

      <div class="setting-row {hidden('Injection delay', 'injection delay wait') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.injectionDelay', settings.Language)}</span>
          <span class="setting-hint">
            How long to wait after game launches before injecting
            {#if settings.SelectedClient === 'Vanilla'}<span class="setting-muted"> · ignored in Vanilla</span>{/if}
          </span>
        </div>
        <div class="setting-control">
          <input
            type="range"
            min="500"
            max="15000"
            step="500"
            value={settings.InjectionDelayMs}
            style="--pct:{pct(settings.InjectionDelayMs, 500, 15000)}"
            oninput={(e) => update('InjectionDelayMs', Number((e.currentTarget as HTMLInputElement).value))}
            disabled={settings.SelectedClient === 'Vanilla'}
          />
          <span class="slider-value">
            {settings.InjectionDelayMs / 1000}s
          </span>
        </div>
      </div>

      <div
        class="setting-row clickable {hidden('Auto-inject', 'auto inject automatic') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('AutoInject', !settings.AutoInject)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('AutoInject', !settings.AutoInject) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.autoInject', settings.Language)}</span>
          <span class="setting-hint">{t('x.injectAutomaticallyOnceTheGameProcessIsD', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.AutoInject ? 'on' : ''}"
          role="switch"
          aria-checked={settings.AutoInject}
          aria-label={t('tt.autoInject', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('AutoInject', !settings.AutoInject) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('AutoInject', !settings.AutoInject) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Close after launch', 'close minimize launch') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('CloseAfterLaunch', !settings.CloseAfterLaunch)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('CloseAfterLaunch', !settings.CloseAfterLaunch) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.closeAfterLaunch', settings.Language)}</span>
          <span class="setting-hint">{t('x.minimiseTheLauncherOnceInjectionSucceeds', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.CloseAfterLaunch ? 'on' : ''}"
          role="switch"
          aria-checked={settings.CloseAfterLaunch}
          aria-label={t('tt.closeAfterLaunch', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('CloseAfterLaunch', !settings.CloseAfterLaunch) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('CloseAfterLaunch', !settings.CloseAfterLaunch) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Verify injection', 'verify injection module confirm') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('VerifyInjection', !settings.VerifyInjection)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('VerifyInjection', !settings.VerifyInjection) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.verifyInjection', settings.Language)}</span>
          <span class="setting-hint">Confirm the DLL really appears in Minecraft's module list instead of trusting LoadLibrary alone</span>
        </div>
        <div
          class="toggle {settings.VerifyInjection ? 'on' : ''}"
          role="switch"
          aria-checked={settings.VerifyInjection}
          aria-label={t('tt.verifyInjection', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('VerifyInjection', !settings.VerifyInjection) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('VerifyInjection', !settings.VerifyInjection) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Prevent game suspend', 'prevent suspend alt tab background lifecycle') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('PreventGameSuspend', !settings.PreventGameSuspend)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('PreventGameSuspend', !settings.PreventGameSuspend) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.preventGameSuspend', settings.Language)}</span>
          <span class="setting-hint">Stop Windows throttling Bedrock when it isn't the focused window (needs AppLifecycleOptOut)</span>
        </div>
        <div
          class="toggle {settings.PreventGameSuspend ? 'on' : ''}"
          role="switch"
          aria-checked={settings.PreventGameSuspend}
          aria-label={t('tt.preventGameSuspend', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('PreventGameSuspend', !settings.PreventGameSuspend) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('PreventGameSuspend', !settings.PreventGameSuspend) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Accelerate loading', 'accelerate loading runtimebroker faster startup') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.accelerateLoading', settings.Language)}</span>
          <span class="setting-hint">
            Stop RuntimeBroker this long after launch to shorten Bedrock's loading screen
            {#if settings.AccelerateLoadingMs === 0}<span class="setting-muted"> · off</span>{/if}
          </span>
        </div>
        <div class="setting-control">
          <input
            type="range"
            min="0"
            max="20000"
            step="1000"
            value={settings.AccelerateLoadingMs}
            style="--pct:{pct(settings.AccelerateLoadingMs, 0, 20000)}"
            oninput={(e) => update('AccelerateLoadingMs', Math.min(20000, Math.max(0, Number((e.currentTarget as HTMLInputElement).value))))}
          />
          <span class="slider-value">
            {settings.AccelerateLoadingMs === 0 ? 'Off' : `${settings.AccelerateLoadingMs / 1000}s`}
          </span>
        </div>
      </div>
    </div>
  {/if}

  {#if categoryVisible('java')}
    <div class="settings-section">
      {#if sectionVisible(['Maximum RAM', 'ram memory java heap'], ['Minecraft folder', 'minecraft folder dot directory'], ['Java runtime', 'java runtime jre jdk javaw'])}
        <span class="panel-section-label">{t('settings.section.javaEdition', settings.Language)}</span>
      {/if}

      {#if !q()}<span class="settings-group-label">Memory</span>{/if}
      <div class="setting-row {hidden('Maximum RAM', 'ram memory java heap') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.maximumRam', settings.Language)}</span>
          <span class="setting-hint">Heap size cap for the Minecraft JVM (-Xmx)</span>
        </div>
        <div class="setting-control">
          <input
            type="range"
            min="1024"
            max="16384"
            step="512"
            value={settings.JavaMaxRamMb}
            style="--pct:{pct(settings.JavaMaxRamMb, 1024, 16384)}"
            oninput={(e) => update('JavaMaxRamMb', Number((e.currentTarget as HTMLInputElement).value))}
          />
          <span class="slider-value">
            {settings.JavaMaxRamMb} MB
          </span>
        </div>
      </div>

      <div class="setting-row {hidden('Minimum RAM', 'ram memory java heap xms') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.minimumRam', settings.Language)}</span>
          <span class="setting-hint">Initial heap size for the Minecraft JVM (-Xms)</span>
        </div>
        <div class="setting-control">
          <input
            type="range"
            min="256"
            max={settings.JavaMaxRamMb}
            step="256"
            value={settings.JavaMinRamMb}
            style="--pct:{pct(settings.JavaMinRamMb, 256, settings.JavaMaxRamMb)}"
            oninput={(e) => update('JavaMinRamMb', Number((e.currentTarget as HTMLInputElement).value))}
          />
          <span class="slider-value">
            {settings.JavaMinRamMb} MB
          </span>
        </div>
      </div>

      {#if !q()}<span class="settings-group-label">Game folder</span>{/if}
      <div
        class="setting-row clickable {hidden('Game files location','minecraft folder glacier location dot directory store') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaUseDotMinecraft', !settings.JavaUseDotMinecraft)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaUseDotMinecraft', !settings.JavaUseDotMinecraft) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.storeGameFilesInMinecraft', settings.Language)}</span>
          <span class="setting-hint">On: use %APPDATA%\.minecraft. Off: keep versions & saves in the Glacier Launcher folder.</span>
        </div>
        <div class="toggle {settings.JavaUseDotMinecraft ? 'on' : ''}"></div>
      </div>

      <div class="setting-row {hidden('Open Minecraft folder', 'minecraft folder dot directory open') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.openMinecraftFolder', settings.Language)}</span>
          <span class="setting-hint truncate" title={settings.JavaMinecraftDir || '%APPDATA%\\.minecraft'}>{settings.JavaMinecraftDir || '%APPDATA%\\.minecraft'}</span>
        </div>
        <button
          class="btn-sm"
          onclick={() => openFolder(settings.JavaMinecraftDir)}
          onmousedown={(e) => e.stopPropagation()}
          disabled={!settings.JavaMinecraftDir}
          data-tooltip={settings.JavaMinecraftDir ? undefined : MINECRAFT_DIR_DISABLED}
        >
          <i class="fa-solid fa-folder-open"></i> Open
        </button>
      </div>

      {#if !q()}<span class="settings-group-label">Launch</span>{/if}
      <div
        class="setting-row clickable {hidden('Close after launch', 'close minimize launch java')? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('CloseAfterLaunch', !settings.CloseAfterLaunch)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('CloseAfterLaunch', !settings.CloseAfterLaunch) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.closeAfterLaunch', settings.Language)}</span>
          <span class="setting-hint">{t('x.minimiseTheLauncherOnceTheGameStarts', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.CloseAfterLaunch ? 'on' : ''}"
          role="switch"
          aria-checked={settings.CloseAfterLaunch}
          aria-label={t('tt.closeAfterLaunch', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('CloseAfterLaunch', !settings.CloseAfterLaunch) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('CloseAfterLaunch', !settings.CloseAfterLaunch) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Offline mode', 'offline mode cracked no account username') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaOfflineMode', !settings.JavaOfflineMode)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaOfflineMode', !settings.JavaOfflineMode) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.offlineMode', settings.Language)}</span>
          <span class="setting-hint">Launch without signing in — online-mode servers will reject it</span>
        </div>
        <div
          class="toggle {settings.JavaOfflineMode ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaOfflineMode}
          aria-label={t('tt.offlineMode', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('JavaOfflineMode', !settings.JavaOfflineMode) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('JavaOfflineMode', !settings.JavaOfflineMode) } }}
        ></div>
      </div>

      {#if settings.JavaOfflineMode}
        <div class="setting-row {hidden('Offline username', 'offline username name player') ? 'hidden-by-search' : ''}">
          <div class="setting-meta">
            <span class="setting-label">{t('x.offlineUsername', settings.Language)}</span>
            <span class="setting-hint">{t('x.theNameUsedForOfflineLaunches', settings.Language)}</span>
          </div>
          <input
            class="setting-input setting-input-md"
            type="text"
            maxlength="16"
            placeholder={t('ph.player', settings.Language)}
            value={settings.JavaOfflineUsername}
            onchange={(e) => update('JavaOfflineUsername', (e.currentTarget as HTMLInputElement).value.trim() || 'Player')}
            onclick={(e) => e.stopPropagation()}
          />
        </div>
      {/if}

      <div
        class="setting-row clickable {hidden('Backup saves before launch', 'world backup saves zip') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaBackupSavesBeforeLaunch', !settings.JavaBackupSavesBeforeLaunch)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaBackupSavesBeforeLaunch', !settings.JavaBackupSavesBeforeLaunch) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.backupSavesBeforeLaunch', settings.Language)}</span>
          <span class="setting-hint">{t('x.createsAZipOfTheActiveInstanceSavesFolde', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.JavaBackupSavesBeforeLaunch ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaBackupSavesBeforeLaunch}
          aria-label={t('tt.backupSavesBeforeLaunch', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('JavaBackupSavesBeforeLaunch', !settings.JavaBackupSavesBeforeLaunch) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('JavaBackupSavesBeforeLaunch', !settings.JavaBackupSavesBeforeLaunch) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Server auto-connect', 'server port multiplayer quick play') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.serverAutoConnect', settings.Language)}</span>
          <span class="setting-hint">{t('x.optionalServerAndPortLaunchInjection', settings.Language)}</span>
        </div>
        <div class="setting-control-tight">
          <input
            class="setting-input setting-input-md"
            type="text"
            placeholder="server address"
            value={settings.JavaServerAddress}
            onchange={(e) => update('JavaServerAddress', (e.currentTarget as HTMLInputElement).value)}
            onkeydown={(e) => e.stopPropagation()}
          />
          <input
            class="setting-input setting-input-sm"
            type="number"
            value={settings.JavaServerPort}
            onchange={(e) => update('JavaServerPort', Number((e.currentTarget as HTMLInputElement).value))}
            onkeydown={(e) => e.stopPropagation()}
          />
        </div>
      </div>

      {#if !q()}<span class="settings-group-label">Display</span>{/if}
      <div
        class="setting-row clickable {hidden('Fullscreen', 'window fullscreen resolution') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaFullscreen', !settings.JavaFullscreen)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaFullscreen', !settings.JavaFullscreen) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('label.fullscreen', settings.Language)}</span>
          <span class="setting-hint">Adds --fullscreen to Java launches</span>
        </div>
        <div
          class="toggle {settings.JavaFullscreen ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaFullscreen}
          aria-label={t('tt.fullscreen', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('JavaFullscreen', !settings.JavaFullscreen) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('JavaFullscreen', !settings.JavaFullscreen) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Custom resolution', 'window fullscreen resolution width height') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaUseCustomResolution', !settings.JavaUseCustomResolution)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaUseCustomResolution', !settings.JavaUseCustomResolution) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.customResolution', settings.Language)}</span>
          <span class="setting-hint">Starts windowed Minecraft at the selected size</span>
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div class="setting-control-tight" onclick={(e) => e.stopPropagation()}>
          <input
            class="setting-input setting-input-sm"
            type="number"
            value={settings.JavaWindowWidth}
            onchange={(e) => update('JavaWindowWidth', Number((e.currentTarget as HTMLInputElement).value))}
            onkeydown={(e) => e.stopPropagation()}
          />
          <input
            class="setting-input setting-input-sm"
            type="number"
            value={settings.JavaWindowHeight}
            onchange={(e) => update('JavaWindowHeight', Number((e.currentTarget as HTMLInputElement).value))}
            onkeydown={(e) => e.stopPropagation()}
          />
          <div
            class="toggle {settings.JavaUseCustomResolution ? 'on' : ''}"
            role="switch"
            aria-checked={settings.JavaUseCustomResolution}
            aria-label={t('tt.customResolution', settings.Language)}
            tabindex="0"
            onclick={() => update('JavaUseCustomResolution', !settings.JavaUseCustomResolution)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaUseCustomResolution', !settings.JavaUseCustomResolution) }}
          ></div>
        </div>
      </div>

      {#if !q()}
        <button class="settings-advanced-toggle" type="button" aria-expanded={javaAdvancedShown} onclick={() => (javaAdvancedOpen = !javaAdvancedOpen)}>
          <i class="fa-solid fa-chevron-{javaAdvancedShown ? 'down' : 'right'}"></i> Advanced
        </button>
      {/if}
      {#if javaAdvancedShown}
      <div class="setting-row {hidden('Minecraft folder', 'minecraft folder dot directory') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Custom Minecraft folder</span>
          <span class="setting-hint">{t('x.overridesTheToggleAboveLeaveEmptyToUseIt', settings.Language)}</span>
        </div>
        <input
          class="setting-input"
          type="text"
          placeholder="%APPDATA%\.minecraft"
          value={settings.JavaMinecraftDir}
          onchange={(e) => update('JavaMinecraftDir', (e.currentTarget as HTMLInputElement).value)}
          onkeydown={(e) => e.stopPropagation()}
        />
      </div>

      <div class="setting-row {hidden('Java runtime', 'java runtime jre jdk javaw') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Java runtime (javaw.exe)</span>
          <span class="setting-hint">Auto-detected from runtime/JAVA_HOME/PATH/registry when blank</span>
        </div>
        <input
          class="setting-input"
          type="text"
          placeholder="C:\Program Files\Java\jdk-21\bin\javaw.exe"
          value={settings.JavaRuntimePath}
          onchange={(e) => update('JavaRuntimePath', (e.currentTarget as HTMLInputElement).value)}
          onkeydown={(e) => e.stopPropagation()}
        />
      </div>

      <div
        class="setting-row clickable {hidden('Show snapshots', 'snapshots experimental') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaShowSnapshots', !settings.JavaShowSnapshots)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaShowSnapshots', !settings.JavaShowSnapshots) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.showSnapshots', settings.Language)}</span>
          <span class="setting-hint">{t('x.includeExperimentalBuildsInTheVersionsLi', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.JavaShowSnapshots ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaShowSnapshots}
          aria-label={t('tt.showSnapshots', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('JavaShowSnapshots', !settings.JavaShowSnapshots) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('JavaShowSnapshots', !settings.JavaShowSnapshots) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Show beta and alpha', 'beta alpha historical old') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaShowHistorical', !settings.JavaShowHistorical)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaShowHistorical', !settings.JavaShowHistorical) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Show beta & alpha</span>
          <span class="setting-hint">{t('x.includeTheOriginalPreReleaseBuilds', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.JavaShowHistorical ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaShowHistorical}
          aria-label={t('tt.showBetaAlpha', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('JavaShowHistorical', !settings.JavaShowHistorical) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('JavaShowHistorical', !settings.JavaShowHistorical) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Custom JVM args', 'jvm args java options garbage collector gc') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.customJvmArgs', settings.Language)}</span>
          <span class="setting-hint">Appended after -Xmx (e.g. -XX:+UseG1GC -Dfile.encoding=UTF-8)</span>
        </div>
        <input
          class="setting-input"
          type="text"
          placeholder="(none)"
          value={settings.JavaCustomJvmArgs}
          onchange={(e) => update('JavaCustomJvmArgs', (e.currentTarget as HTMLInputElement).value)}
          onkeydown={(e) => e.stopPropagation()}
        />
      </div>

      <div class="setting-row {hidden('Latest crash report','crash report log latest open view java') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.latestCrashReport', settings.Language)}</span>
          <span class="setting-hint truncate" title={latestCrashLabel}>{latestCrashLabel}</span>
        </div>
        <button class="btn-sm" onclick={openLatestCrash} onmousedown={(e) => e.stopPropagation()} disabled={!latestCrash}>
          <i class="fa-solid fa-bug"></i> Open
        </button>
      </div>
      {/if}
    </div>
  {/if}

  {#if categoryVisible('appearance')}
    <div class="settings-section">
      <!--
        Redesign: the whole Looks tab, not just the Theme Studio piece of it.
        "Appearance" used to be 13 flat setting-rows followed by the theme
        editor at the very bottom — you'd scroll past everything before ever
        seeing what your look actually does. The live preview now leads the
        whole tab (it's the first thing rendered), with the theme picker and
        its editor right under it; Layout & Behavior and Quick Look — the
        settings a theme doesn't/does override, respectively — follow below
        as the detail content, same relationship as everywhere else in this
        overhaul (viewer/selector on top, the thing it's driving under it).
      -->
      <span class="panel-section-label first-label">{t('settings.section.customThemes', settings.Language)}</span>
      <p class="setting-hint setting-intro">
        Build a full custom look — every color, radius, blur, wallpaper and font in one saved theme that overrides Quick Look below while it's active.
      </p>
      <ThemeStudioView {settings} onChange={onChange} onToast={(m) => (actionMessage = m)} embedded />

      {#if sectionVisible(['Panel side', 'panel side left right sidebar position'], ['Compact mode', 'compact dense small'], ['Animations', 'animations smooth motion'], ['Accessibility features', 'accessibility a11y reduced motion focus outline screen reader'], ['Remember window size', 'window size remember'], ['UI scale', 'ui scale zoom size interface'])}
        <span class="panel-section-label">{t('settings.section.layoutAndBehavior', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('Panel side', 'panel side left right sidebar position') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.panelSide', settings.Language)}</span>
          <span class="setting-hint">{t('x.whichEdgeOfTheWindowPanelsSlideInFrom', settings.Language)}</span>
        </div>
        <div class="seg-control" onclick={(e) => e.stopPropagation()}>
          <button
            type="button"
            class="seg-btn {settings.PanelSide === 'right' ? 'active' : ''}"
            onclick={() => setPanelSide('right')}
          >
            Right
          </button>
          <button
            type="button"
            class="seg-btn {settings.PanelSide === 'left' ? 'active' : ''}"
            onclick={() => setPanelSide('left')}
          >
            Left
          </button>
        </div>
      </div>

      <div
        class="setting-row clickable {hidden('Compact mode', 'compact dense small') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => setCompactMode(!settings.CompactMode)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') setCompactMode(!settings.CompactMode) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.compactMode', settings.Language)}</span>
          <span class="setting-hint">Tighter spacing — fits more on screen</span>
        </div>
        <Toggle on={settings.CompactMode} onChanged={(v) => setCompactMode(v)} />
      </div>

      <div
        class="setting-row clickable {hidden('Animations', 'animations smooth motion') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => setAnimationsEnabled(!settings.AnimationsEnabled)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') setAnimationsEnabled(!settings.AnimationsEnabled) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.animations', settings.Language)}</span>
          <span class="setting-hint">{t('x.disableForLowEndDevicesOrReducedMotion', settings.Language)}</span>
        </div>
        <Toggle on={settings.AnimationsEnabled} onChanged={(v) => setAnimationsEnabled(v)} />
      </div>

      {#if settings.AnimationsEnabled}
        <div class="setting-row {hidden('Animation speed', 'animation speed motion fast slow multiplier') ? 'hidden-by-search' : ''}">
          <div class="setting-meta">
            <span class="setting-label">{t('x.animationSpeed', settings.Language)}</span>
            <span class="setting-hint">0.25× slow-mo to 2× snappy — 1× is default</span>
          </div>
          <div class="setting-control">
            <input
              type="range"
              min="0.25"
              max="2"
              step="0.25"
              value={settings.AnimationSpeed}
              style="--pct:{pct(settings.AnimationSpeed, 0.25, 2)}"
              oninput={(e) => setAnimSpeed(Number((e.currentTarget as HTMLInputElement).value))}
            />
            <span class="slider-value">
              {settings.AnimationSpeed}×
            </span>
          </div>
        </div>
      {/if}

      <div
        class="setting-row clickable {hidden('Accessibility features', 'accessibility a11y reduced motion focus outline screen reader') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => setAccessibilityMode(!settings.AccessibilityMode)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') setAccessibilityMode(!settings.AccessibilityMode) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.accessibilityFeatures', settings.Language)}</span>
          <span class="setting-hint">{t('x.reducedMotionStrongerFocusOutlinesLarger', settings.Language)}</span>
        </div>
        <Toggle on={settings.AccessibilityMode} onChanged={(v) => setAccessibilityMode(v)} />
      </div>

      <div
        class="setting-row clickable {hidden('Remember window size', 'window size remember') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('RememberWindowSize', !settings.RememberWindowSize)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('RememberWindowSize', !settings.RememberWindowSize) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.rememberWindowSize', settings.Language)}</span>
          <span class="setting-hint">{t('x.restoreLastUsedSizeOnNextLaunch', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.RememberWindowSize ? 'on' : ''}"
          role="switch"
          aria-checked={settings.RememberWindowSize}
          aria-label={t('tt.rememberWindowSize', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('RememberWindowSize', !settings.RememberWindowSize) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('RememberWindowSize', !settings.RememberWindowSize) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('UI scale', 'ui scale zoom size interface') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.uiScale', settings.Language)}</span>
          <span class="setting-hint">On top of the automatic window fit — 100% is default</span>
        </div>
        <div class="setting-control">
          <input
            type="range"
            min="75"
            max="150"
            step="5"
            value={settings.UiScalePct}
            style="--pct:{pct(settings.UiScalePct, 75, 150)}"
            oninput={(e) => setUiScale(Number((e.currentTarget as HTMLInputElement).value))}
          />
          <span class="slider-value">
            {settings.UiScalePct}%
          </span>
        </div>
      </div>

      {#if sectionVisible(['Accent colour', 'accent color theme'], ['Theme preset', 'theme dark midnight slate ocean forest sunset light'], ['Background blur', 'blur background'], ['Custom background wallpaper', 'background wallpaper image custom'], ['Background fit', 'background fit cover contain tile center'], ['Font', 'font family typeface typography'])}
        <span class="panel-section-label">{t('settings.section.quickLook', settings.Language)}</span>
      {/if}

      {#if settings.ActiveThemeId}
        <div class="cache-notice">
          <i class="fa-solid fa-circle-info"></i>
          <span>A custom theme is active, so it overrides the colors/blur/font/wallpaper below — edit it in Custom Themes instead.</span>
        </div>
      {/if}

      <div class="setting-row {hidden('Accent colour', 'accent color theme') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.accentColour', settings.Language)}</span>
          <span class="setting-hint">{t('x.usedForButtonsHighlightsAndGlows', settings.Language)}</span>
        </div>
        <div class="color-swatches">
          {#each accentSwatches as c (c)}
            <div
              class="color-swatch {settings.AccentColor === c ? 'active' : ''}"
              style="--swatch:{c};"
              data-tooltip={c}
              role="button"
              tabindex="0"
              aria-label={`Accent color ${c}`}
              aria-pressed={settings.AccentColor === c}
              onclick={() => setAccent(c)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setAccent(c) } }}
              onmousedown={(e) => e.stopPropagation()}
            ></div>
          {/each}
        </div>
      </div>

      <div class="setting-row {hidden('Theme preset', 'theme dark midnight slate ocean forest sunset light') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.themePreset', settings.Language)}</span>
          <span class="setting-hint">Background tone — affects panels and overlays</span>
        </div>
        <select
          class="setting-select"
          value={settings.ThemePreset}
          onchange={(e) => setThemePreset((e.currentTarget as HTMLSelectElement).value)}
          onclick={(e) => e.stopPropagation()}
        >
          <option value="dark">{t('x.dark', settings.Language)}</option>
          <option value="legacy">Legacy</option>
          <option value="darker">{t('x.darker', settings.Language)}</option>
          <option value="midnight">{t('x.midnight', settings.Language)}</option>
          <option value="slate">{t('x.slate', settings.Language)}</option>
          <option value="ocean">{t('x.ocean', settings.Language)}</option>
          <option value="forest">{t('x.forest', settings.Language)}</option>
          <option value="sunset">{t('x.sunset', settings.Language)}</option>
          <option value="light">{t('x.light', settings.Language)}</option>
        </select>
      </div>

      <div class="setting-row {hidden('Background blur', 'blur background') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.backgroundBlur', settings.Language)}</span>
          <span class="setting-hint">{t('x.backdropBlurIntensityForPanels', settings.Language)}</span>
        </div>
        <div class="setting-control">
          <input
            type="range"
            min="0"
            max="30"
            step="2"
            value={settings.BlurIntensity}
            style="--pct:{pct(settings.BlurIntensity, 0, 30)}"
            oninput={(e) => setBlur(Number((e.currentTarget as HTMLInputElement).value))}
          />
          <span class="slider-value">
            {settings.BlurIntensity}px
          </span>
        </div>
      </div>

      <div class="setting-row {hidden('Tooltip delay', 'tooltip hover delay speed') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.tooltipDelay', settings.Language)}</span>
          <span class="setting-hint">{t('x.howLongToHoverBeforeATooltipAppears', settings.Language)}</span>
        </div>
        <select
          class="setting-select"
          value={settings.TooltipDelayMs}
          onchange={(e) => { update('TooltipDelayMs', Number(e.currentTarget.value)); window.setTooltipDelay?.(Number(e.currentTarget.value)) }}
        >
          <option value={0}>{t('x.instant', settings.Language)}</option>
          <option value={150}>{t('x.short', settings.Language)}</option>
          <option value={350}>{t('x.normal', settings.Language)}</option>
          <option value={700}>{t('x.long', settings.Language)}</option>
        </select>
      </div>

      <div class="setting-row {hidden('Custom background wallpaper', 'background wallpaper image custom') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.backgroundWallpaper', settings.Language)}</span>
          <span class="setting-hint">
            {#if settings.CustomBackgroundPath}
              Custom — {fileName(settings.CustomBackgroundPath)}
            {:else}
              Default background
            {/if}
          </span>
        </div>
        <div class="setting-control-tight">
          {#if settings.CustomBackgroundPath}
            <button class="btn-sm" onclick={resetWallpaper} onmousedown={(e) => e.stopPropagation()} data-tooltip={t('tt.resetDefault', settings.Language)}>
              <i class="fa-solid fa-rotate-left"></i> Default
            </button>
          {/if}
          <button class="btn-sm" onclick={chooseWallpaper} onmousedown={(e) => e.stopPropagation()}>
            <i class="fa-solid fa-image"></i> Choose
          </button>
        </div>
      </div>

      <div class="setting-row {hidden('Background fit', 'background fit cover contain tile center') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.backgroundFit', settings.Language)}</span>
          <span class="setting-hint">{t('x.howTheWallpaperFillsTheWindow', settings.Language)}</span>
        </div>
        <div class="seg-control" onclick={(e) => e.stopPropagation()}>
          {#each [['cover', 'Fill'], ['contain', 'Fit'], ['tile', 'Tile'], ['center', 'Center']] as [v, label] (v)}
            <button
              type="button"
              class="seg-btn {settings.BackgroundFit === v ? 'active' : ''}"
              onclick={() => setBackgroundFit(v)}
            >
              {label}
            </button>
          {/each}
        </div>
      </div>

      <div class="setting-row {hidden('Font', 'font family typeface typography') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.font', settings.Language)}</span>
          <span class="setting-hint">Any installed font — empty uses Segoe UI</span>
        </div>
        <input
          class="setting-input setting-input-md"
          list="font-suggestions"
          placeholder="Segoe UI"
          value={settings.FontFamily}
          onchange={(e) => setFont((e.currentTarget as HTMLInputElement).value)}
          onclick={(e) => e.stopPropagation()}
        />
        <datalist id="font-suggestions">
          <option value="Segoe UI"></option><option value="Inter"></option><option value="Roboto"></option>
          <option value="Arial"></option><option value="Verdana"></option><option value="Tahoma"></option>
          <option value="Trebuchet MS"></option><option value="Georgia"></option><option value="Cascadia Code"></option>
          <option value="Consolas"></option><option value="JetBrains Mono"></option><option value="Comic Sans MS"></option>
        </datalist>
      </div>

    </div>
  {/if}

  {#if categoryVisible('account')}
    <div class="settings-section">
      {#if sectionVisible(['Display name', 'username name display account'], ['Profile display', 'profile display footer xbox discord switch which'])}
        <span class="panel-section-label">{t('settings.section.account', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('Display name', 'username name display account') ? 'hidden-by-search' : ''}">
        <span class="setting-label">{t('label.displayName', settings.Language)}</span>
        <input
          class="setting-input setting-input-xs"
          value={settings.Username}
          placeholder={t('ph.player', settings.Language)}
          onchange={(e) => update('Username', (e.currentTarget as HTMLInputElement).value)}
          onclick={(e) => e.stopPropagation()}
        />
      </div>

      <div class="setting-row {hidden('Profile display', 'profile display footer xbox discord switch which') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.profileDisplay', settings.Language)}</span>
          <span class="setting-hint">Which account fills the bottom-left avatar & name.</span>
        </div>
        <div class="seg-control" onclick={(e) => e.stopPropagation()}>
          {#each [['auto', 'Auto'], ['xbox', 'Xbox'], ['discord', 'Discord']] as [v, label] (v)}
            <button
              type="button"
              class="seg-btn {settings.ProfileDisplayMode === v ? 'active' : ''}"
              data-tooltip={v === 'auto' ? 'Prefer Xbox when both are signed in' : undefined}
              onclick={() => update('ProfileDisplayMode', v)}
            >
              {label}
            </button>
          {/each}
        </div>
      </div>

      <div class="setting-row {hidden('Language', 'language locale translation') ? 'hidden-by-search' : ''}">
        <span class="setting-label">{t('settings.language', settings.Language)}</span>
        <select
          class="setting-select"
          value={settings.Language}
          onchange={(e) => {
            const code = (e.currentTarget as HTMLSelectElement).value
            update('Language', LANGUAGES.some((l) => l.code === code) ? code : 'en')
          }}
          onclick={(e) => e.stopPropagation()}
        >
          {#each LANGUAGES as lang (lang.code)}
            <option value={lang.code}>{lang.label}</option>
          {/each}
        </select>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Discord Rich Presence', 'discord rpc social'], ['Discord account', 'discord login account'])}
        <span class="panel-section-label">{t('settings.section.social', settings.Language)}</span>
      {/if}

      <div
        class="setting-row clickable {hidden('Discord Rich Presence', 'discord rpc social') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={toggleDiscordRpc}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggleDiscordRpc() }}
      >
        <span class="setting-label">Discord Rich Presence</span>
        <div
          class="toggle {settings.DiscordRichPresence ? 'on' : ''}"
          role="switch"
          aria-checked={settings.DiscordRichPresence}
          aria-label={t('tt.discordRichPresence', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); toggleDiscordRpc() }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); toggleDiscordRpc() } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Discord account', 'discord login account') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Discord account</span>
          <span class="setting-hint">
            {#if settings.DiscordLoggedIn}<span class="status-ok"><i class="fa-solid fa-circle-check"></i> Connected as {settings.DiscordUsername}</span>{:else}<span class="status-muted"><i class="fa-regular fa-circle"></i> Not connected</span>{/if}
          </span>
        </div>
        <button
          class="btn-sm"
          disabled={discordConnecting}
          onclick={() => (settings.DiscordLoggedIn ? disconnectDiscord() : connectDiscord())}
        >
          <i class="fa-brands fa-discord"></i>
          {#if discordConnecting}
            Working...
          {:else}
            {settings.DiscordLoggedIn ? 'Manage' : 'Connect'}
          {/if}
        </button>
      </div>

      <div class="setting-row {hidden('Xbox profile', 'xbox gamertag account microsoft') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Xbox profile</span>
          <span class="setting-hint">
            {#if settings.XboxGamertag}<span class="status-ok"><i class="fa-solid fa-circle-check"></i> Signed in as {settings.XboxGamertag}</span>{:else}<span class="status-muted"><i class="fa-regular fa-circle"></i> Not connected</span>{/if}
          </span>
        </div>
        <button
          class="btn-sm"
          disabled={xboxSigningIn}
          onclick={() => (settings.XboxGamertag ? disconnectXbox() : connectXbox())}
        >
          <i class="fa-solid fa-gamepad xbox-icon"></i>
          {#if xboxSigningIn}
            Working...
          {:else}
            {settings.XboxGamertag ? 'Disconnect' : 'Sign in'}
          {/if}
        </button>
      </div>

      {#if settings.JavaAccounts.length > 0}
        <div class="setting-row setting-stack">
          <div class="setting-split">
            <span class="setting-label">{t('x.savedAccounts', settings.Language)}</span>
            <button class="btn-sm" disabled={xboxSigningIn} onclick={connectXbox}>
              <i class="fa-solid fa-plus"></i> Add account
            </button>
          </div>
          {#each settings.JavaAccounts as account (account.Id)}
            {@const isActive = account.Id === settings.ActiveJavaAccountId}
            <div class="instance-row {isActive ? 'active' : ''}">
              <button class="instance-name-btn" onclick={() => switchJavaAccount(account)} disabled={isActive}>
                {#if isActive}<i class="fa-solid fa-circle-check"></i>{:else}<i class="fa-regular fa-circle"></i>{/if}
                <span class="truncate" title={account.Username || account.Gamertag || account.Id}>{account.Username || account.Gamertag || account.Id}</span>
              </button>
              <button
                class="instance-icon-btn"
                onclick={() => removeJavaAccount(account)}
                data-tooltip={t('tt.remove', settings.Language)} aria-label={t('tt.removeAccount', settings.Language)}
              ><i class="fa-solid fa-trash"></i></button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if categoryVisible('system')}
    <div class="settings-section">
      {#if sectionVisible(['Show recently launched', 'recent history'], ['Minimize to tray', 'tray minimize system'], ['Clear recent history', 'clear history recent'])}
        <span class="panel-section-label">{t('settings.section.qualityOfLife', settings.Language)}</span>
      {/if}

      {#if !q()}<span class="settings-group-label">Launcher &amp; tray</span>{/if}
      <div
        class="setting-row clickable {hidden('Show recently launched','recent history') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('ShowRecentlyLaunched', !settings.ShowRecentlyLaunched)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('ShowRecentlyLaunched', !settings.ShowRecentlyLaunched) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.showRecentlyLaunched', settings.Language)}</span>
          <span class="setting-hint">{t('x.quickAccessChipsOnTheHomeScreen', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.ShowRecentlyLaunched ? 'on' : ''}"
          role="switch"
          aria-checked={settings.ShowRecentlyLaunched}
          aria-label={t('tt.showRecentlyLaunched', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('ShowRecentlyLaunched', !settings.ShowRecentlyLaunched) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('ShowRecentlyLaunched', !settings.ShowRecentlyLaunched) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Show launch console', 'console log launch window java bedrock output stdout') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('ShowLaunchConsole', !settings.ShowLaunchConsole)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('ShowLaunchConsole', !settings.ShowLaunchConsole) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.showLaunchConsole', settings.Language)}</span>
          <span class="setting-hint">Pops a window with live game output (Java) or launcher events (Bedrock) on every launch</span>
        </div>
        <div
          class="toggle {settings.ShowLaunchConsole ? 'on' : ''}"
          role="switch"
          aria-checked={settings.ShowLaunchConsole}
          aria-label={t('tt.showLaunchConsole', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('ShowLaunchConsole', !settings.ShowLaunchConsole) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('ShowLaunchConsole', !settings.ShowLaunchConsole) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Collapse instance groups', 'library instances folders collapse expand sections') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('CollapseInstanceGroups', !settings.CollapseInstanceGroups)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('CollapseInstanceGroups', !settings.CollapseInstanceGroups) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.collapseInstanceGroups', settings.Language)}</span>
          <span class="setting-hint">Start each folder section collapsed when you open Instances (pinned instances always stay visible)</span>
        </div>
        <div
          class="toggle {settings.CollapseInstanceGroups ? 'on' : ''}"
          role="switch"
          aria-checked={settings.CollapseInstanceGroups}
          aria-label={t('tt.collapseInstanceGroups', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('CollapseInstanceGroups', !settings.CollapseInstanceGroups) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('CollapseInstanceGroups', !settings.CollapseInstanceGroups) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Minimize to tray', 'tray minimize system') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('MinimizeToTray', !settings.MinimizeToTray)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('MinimizeToTray', !settings.MinimizeToTray) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.minimizeToTray', settings.Language)}</span>
          <span class="setting-hint">{t('x.minimizeButtonSendsToSystemTray', settings.Language)}</span>
        </div>
        <div
          class="toggle {settings.MinimizeToTray ? 'on' : ''}"
          role="switch"
          aria-checked={settings.MinimizeToTray}
          aria-label={t('tt.minimizeTray', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('MinimizeToTray', !settings.MinimizeToTray) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('MinimizeToTray', !settings.MinimizeToTray) } }}
        ></div>
      </div>

      {#if !q()}<span class="settings-group-label">Downloads &amp; alerts</span>{/if}
      <div class="setting-row {hidden('Concurrent downloads','download concurrency parallel limit') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.concurrentDownloads', settings.Language)}</span>
          <span class="setting-hint">Max downloads running at once — takes effect next launch</span>
        </div>
        <select
          class="setting-select"
          value={settings.MaxConcurrentDownloads}
          onchange={(e) => update('MaxConcurrentDownloads', Number(e.currentTarget.value))}
        >
          <option value={1}>1</option>
          <option value={2}>2</option>
          <option value={4}>4</option>
          <option value={8}>8</option>
        </select>
      </div>

      <div
        class="setting-row clickable {hidden('Desktop notifications', 'desktop system os notification toast') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('DesktopNotificationsEnabled', !settings.DesktopNotificationsEnabled)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('DesktopNotificationsEnabled', !settings.DesktopNotificationsEnabled) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.desktopNotifications', settings.Language)}</span>
          <span class="setting-hint">System notifications for things like finished downloads while the window isn't focused</span>
        </div>
        <div
          class="toggle {settings.DesktopNotificationsEnabled ? 'on' : ''}"
          role="switch"
          aria-checked={settings.DesktopNotificationsEnabled}
          aria-label={t('tt.desktopNotifications', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('DesktopNotificationsEnabled', !settings.DesktopNotificationsEnabled) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('DesktopNotificationsEnabled', !settings.DesktopNotificationsEnabled) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Mute notification categories', 'mute notifications toast category downloads updates social') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.muteNotificationCategories', settings.Language)}</span>
          <span class="setting-hint">Stop these from being recorded in the notification bell / toast history</span>
        </div>
        <div class="checkbox-group">
          {#each ['downloads', 'updates', 'social'] as cat (cat)}
            <label class="checkbox-item">
              <input
                type="checkbox"
                checked={settings.MutedNotificationCategories.includes(cat)}
                onchange={(e) => {
                  const next = e.currentTarget.checked
                    ? [...settings.MutedNotificationCategories, cat]
                    : settings.MutedNotificationCategories.filter((c) => c !== cat)
                  update('MutedNotificationCategories', next)
                }}
              />
              {cat[0].toUpperCase() + cat.slice(1)}
            </label>
          {/each}
        </div>
      </div>

      <div class="setting-row {hidden('Auto-clear finished downloads', 'downloads cleanup auto clear') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.autoClearFinishedDownloads', settings.Language)}</span>
          <span class="setting-hint">Removes completed/failed entries from the Downloads panel automatically</span>
        </div>
        <select
          class="setting-select"
          value={settings.AutoClearDownloadsAfterDays}
          onchange={(e) => update('AutoClearDownloadsAfterDays', Number(e.currentTarget.value))}
        >
          <option value={0}>{t('x.never', settings.Language)}</option>
          <option value={1}>{t('x.after1Day', settings.Language)}</option>
          <option value={7}>{t('x.after7Days', settings.Language)}</option>
          <option value={30}>{t('x.after30Days', settings.Language)}</option>
        </select>
      </div>

      <div class="setting-row {hidden('Clear recent history', 'clear history recent') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.clearRecentHistory', settings.Language)}</span>
          <span class="setting-hint">{settings.RecentlyLaunched.length} item(s)</span>
        </div>
        <button class="btn-sm" onclick={clearRecentHistory} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-trash"></i> Clear
        </button>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Check for updates on startup', 'update auto check startup'], ['Check for updates now', 'update manual check'])}
        <span class="panel-section-label">{t('settings.section.updates', settings.Language)}</span>
      {/if}

      <div
        class="setting-row clickable {hidden('Check for updates on startup', 'update auto check startup') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('CheckUpdatesOnStartup', !settings.CheckUpdatesOnStartup)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('CheckUpdatesOnStartup', !settings.CheckUpdatesOnStartup) }}
      >
        <div class="setting-meta">
          <span class="setting-label">{t('x.checkForUpdatesOnStartup', settings.Language)}</span>
          <span class="setting-hint">Silently checks launcher + client updates in the background</span>
        </div>
        <div
          class="toggle {settings.CheckUpdatesOnStartup ? 'on' : ''}"
          role="switch"
          aria-checked={settings.CheckUpdatesOnStartup}
          aria-label={t('tt.checkUpdatesStartup', settings.Language)}
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('CheckUpdatesOnStartup', !settings.CheckUpdatesOnStartup) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('CheckUpdatesOnStartup', !settings.CheckUpdatesOnStartup) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Check for updates now', 'update manual check') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.checkForUpdatesNow', settings.Language)}</span>
          <span class="setting-hint">{checkingUpdates ? 'Checking...' : lastUpdateCheckLabel}</span>
        </div>
        <button
          class="btn-sm"
          onclick={manualUpdateCheck}
          onmousedown={(e) => e.stopPropagation()}
          disabled={checkingUpdates}
        >
          {#if checkingUpdates}
            <span class="spinner spinner-sm"></span>
          {:else}
            <i class="fa-solid fa-rotate"></i>
          {/if}
          Check
        </button>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Open launcher folder', 'launcher folder downloads location'], ['Open Minecraft folder', 'minecraft com mojang folder'])}
        <span class="panel-section-label">{t('settings.section.folders', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('Open launcher folder', 'launcher folder downloads location') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.launcherFolder', settings.Language)}</span>
          <span class="setting-hint">{t('x.settingsDownloadedClientsAndDllsLiveHere', settings.Language)}</span>
        </div>
        <button class="btn-sm" onclick={openLauncherFolder} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-folder-open"></i> Open
        </button>
      </div>

      <div class="setting-row {hidden('Open Minecraft folder', 'minecraft com mojang folder') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Minecraft data folder</span>
          <span class="setting-hint">com.mojang — resource packs, worlds, behaviour packs</span>
        </div>
        <button class="btn-sm" onclick={openMinecraftDataFolder} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-cubes"></i> Open
        </button>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['CurseForge API key', 'curseforge api key addons'])}
        <span class="panel-section-label">{t('settings.section.curseforge', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('CurseForge API key', 'curseforge api key addons') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">CurseForge API key</span>
          <span class="setting-hint">
            {#if settings.CurseForgeApiKey.trim()}
              <span class="key-status"><i class="fa-solid fa-circle-check"></i> Custom key set</span>
            {:else}
              Required for Addons tab — get one free from CurseForge
            {/if}
          </span>
        </div>
        <div class="setting-control-tight">
          <input
            class="setting-input setting-input-key"
            type="password"
            value={settings.CurseForgeApiKey}
            placeholder={t('ph.pasteApiKey', settings.Language)}
            onchange={(e) => update('CurseForgeApiKey', (e.currentTarget as HTMLInputElement).value)}
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
          />
          <a
            class="btn-sm"
            href="https://console.curseforge.com/"
            target="_blank"
            rel="noreferrer"
            data-tooltip={t('tt.getApiKey', settings.Language)}
            aria-label={t('tt.getApiKey', settings.Language)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-arrow-up-right-from-square"></i>
          </a>
        </div>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Gemini API key', 'gemini google ai crash explain assistant'])}
        <span class="panel-section-label">{t('settings.section.ai', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('Gemini API key', 'gemini google ai crash explain assistant') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.geminiApiKey', settings.Language)}</span>
          <span class="setting-hint">
            {#if settings.GeminiApiKey.trim()}
              <span class="key-status"><i class="fa-solid fa-circle-check"></i> Custom key set</span>
            {:else}
              Optional — enables "Explain with AI" on crash logs. Your own key, used directly from your machine.
            {/if}
          </span>
        </div>
        <div class="setting-control-tight">
          <input
            class="setting-input setting-input-key"
            type="password"
            value={settings.GeminiApiKey}
            placeholder={t('ph.pasteApiKey', settings.Language)}
            onchange={(e) => update('GeminiApiKey', (e.currentTarget as HTMLInputElement).value)}
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
          />
          <a
            class="btn-sm"
            href="https://aistudio.google.com/app/apikey"
            target="_blank"
            rel="noreferrer"
            data-tooltip={t('tt.getApiKey', settings.Language)}
            aria-label={t('tt.getApiKey', settings.Language)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-arrow-up-right-from-square"></i>
          </a>
        </div>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Diagnostics', 'diagnostics disk space crash startup'], ['Copy settings summary', 'copy diagnostic summary support troubleshooting clipboard'])}
        <span class="panel-section-label">{t('settings.section.diagnostics', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('Diagnostics', 'diagnostics disk space crash startup') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.launcherStatus', settings.Language)}</span>
          <span class="setting-hint">
            {#if diagnostics}
              {diagnostics.diskFreeLabel} free on {diagnostics.diskName} · Last update check: {settings.LastUpdateCheck ? formatRelativeTime(settings.LastUpdateCheck) : 'never'}
              {#if diagnostics.lastCrash}· Last crash: {diagnostics.lastCrash}{/if}
            {:else if diagnosticsFailed}
              <span class="status-error"><i class="fa-solid fa-circle-exclamation"></i> Could not load diagnostics</span>
            {:else}
              Loading…
            {/if}
          </span>
        </div>
        <button class="btn-sm" onclick={loadDiagnostics} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-rotate"></i> {t('common.refresh', settings.Language)}
        </button>
      </div>

      <div class="setting-row {hidden('Copy settings summary', 'copy diagnostic summary support troubleshooting clipboard') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.copySettingsSummary', settings.Language)}</span>
          <span class="setting-hint">Copies a short plain-text snapshot of your settings — handy to paste into a bug report</span>
        </div>
        <button class="btn-sm" onclick={copySettingsSummary} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-clipboard-list"></i> {t('common.copy', settings.Language)}
        </button>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Export settings', 'export backup settings save'], ['Import settings', 'import restore settings load'], ['Reset settings', 'reset default factory settings'])}
        <span class="panel-section-label">{t('settings.section.backup', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('Export settings', 'export backup settings save') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.exportSettings', settings.Language)}</span>
          <span class="setting-hint">{t('x.saveYourSettingsAsAJsonBackup', settings.Language)}</span>
        </div>
        <button class="btn-sm" onclick={exportSettings} onmousedown={(e) => e.stopPropagation()} disabled={exporting}>
          <i class="fa-solid fa-file-export"></i> {t('common.export', settings.Language)}
        </button>
      </div>

      <div class="setting-row {hidden('Import settings', 'import restore settings load') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.importSettings', settings.Language)}</span>
          <span class="setting-hint">{t('x.restoreFromAPreviouslyExportedJson', settings.Language)}</span>
        </div>
        <button class="btn-sm" onclick={importSettings} onmousedown={(e) => e.stopPropagation()} disabled={importing}>
          <i class="fa-solid fa-file-import"></i> {t('common.import', settings.Language)}
        </button>
      </div>

      <div class="setting-row {hidden('Config profiles', 'config profiles presets streaming setup save load switch') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.saveCurrentSettingsAsAProfile', settings.Language)}</span>
          <span class="setting-hint">Named presets you can switch between (e.g. "Streaming", "Solo play")</span>
        </div>
        <div class="setting-control-tight">
          <input
            class="modal-input setting-input-md"
            placeholder={t('ph.profileName', settings.Language)}
            bind:value={newProfileName}
            onkeydown={(e) => { e.stopPropagation(); if (e.key === 'Enter') saveProfile() }}
          />
          <button class="btn-sm" onclick={saveProfile} onmousedown={(e) => e.stopPropagation()} disabled={!newProfileName.trim()}>
            <i class="fa-solid fa-floppy-disk"></i> {t('common.save', settings.Language)}
          </button>
        </div>
      </div>

      {#if profiles.length > 0}
        <div class="setting-row setting-stack">
          {#each profiles as p (p.path)}
            <div class="setting-control">
              <span class="setting-label setting-label-grow truncate">{p.name}</span>
              <button class="btn-sm" onclick={() => loadProfile(p)} onmousedown={(e) => e.stopPropagation()}>
                <i class="fa-solid fa-arrow-rotate-right"></i> Load
              </button>
              <button class="btn-sm btn-danger" onclick={() => deleteProfile(p)} onmousedown={(e) => e.stopPropagation()} aria-label="Delete profile {p.name}" data-tooltip="Delete profile">
                <i class="fa-solid fa-trash"></i>
              </button>
            </div>
          {/each}
        </div>
      {/if}

      {#if canUndoSettings}
        <div class="setting-row {hidden('Undo last change', 'undo revert last settings change') ? 'hidden-by-search' : ''}">
          <div class="setting-meta">
            <span class="setting-label">{t('x.undoLastChange', settings.Language)}</span>
            <span class="setting-hint">{t('x.restoresSettingsToJustBeforeYourMostRece', settings.Language)}</span>
          </div>
          <button class="btn-sm" onclick={onUndoLastChange} onmousedown={(e) => e.stopPropagation()}>
            <i class="fa-solid fa-rotate-left"></i> Undo
          </button>
        </div>
      {/if}

      {#if sectionVisible(['Reset settings', 'reset default factory settings'], ['Delete all data', 'wipe factory reset uninstall data folder'])}
        <div class="danger-zone">
          <span class="danger-zone-label">Danger zone</span>
          <div class="setting-row {hidden('Reset settings', 'reset default factory settings') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.resetToDefaults', settings.Language)}</span>
          <span class="setting-hint">Clears all preferences — does not delete clients</span>
        </div>
        {#if resetConfirmOpen}
          <div class="setting-control-tight">
            <button
              class="btn-sm btn-danger"
              onclick={doReset}
              onmousedown={(e) => e.stopPropagation()}
              disabled={resetting}
            >
              <i class="fa-solid fa-check"></i> Confirm
            </button>
            <button class="btn-sm" onclick={() => (resetConfirmOpen = false)} onmousedown={(e) => e.stopPropagation()}>
              Cancel
            </button>
          </div>
        {:else}
          <button
            class="btn-sm btn-danger"
            onclick={() => (resetConfirmOpen = true)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-arrow-rotate-left"></i> {t('common.reset', settings.Language)}
          </button>
        {/if}
      </div>

      <div class="setting-row {hidden('Delete all data', 'wipe factory reset uninstall data folder') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.deleteAllData', settings.Language)}</span>
          <span class="setting-hint">Wipes the entire Glacier Launcher data folder — settings, instances, backups, clients, everything. Cannot be undone.</span>
        </div>
        {#if deleteAllConfirmOpen}
          <div class="setting-control-tight">
            <button
              class="btn-sm btn-danger"
              onclick={doDeleteAllData}
              onmousedown={(e) => e.stopPropagation()}
              disabled={deletingAllData}
            >
              {#if deletingAllData}<span class="spinner spinner-sm"></span>{:else}<i class="fa-solid fa-check"></i>{/if} Confirm delete everything
            </button>
            <button class="btn-sm" onclick={() => (deleteAllConfirmOpen = false)} onmousedown={(e) => e.stopPropagation()} disabled={deletingAllData}>
              Cancel
            </button>
          </div>
        {:else}
          <button
            class="btn-sm btn-danger"
            onclick={() => (deleteAllConfirmOpen = true)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-trash-can"></i> Delete everything
          </button>
        {/if}
      </div>

        </div>
      {/if}

      {#if actionMessage}
        <div class="setting-row"><span class="setting-hint">{actionMessage}</span></div>
      {/if}
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Keyboard shortcuts', 'shortcuts keyboard help'], ['Credits', 'credits about launcher'])}
        <span class="panel-section-label">{t('settings.section.about', settings.Language)}</span>
      {/if}

      <div class="setting-row {hidden('Keyboard shortcuts', 'shortcuts keyboard help') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.keyboardShortcuts', settings.Language)}</span>
          <span class="setting-hint">Ctrl+1/2/3/4/5 panels · Ctrl+, settings · Ctrl+Tab cycle · Esc close — the three below can be rebound</span>
        </div>
        <div class="kbs"><kbd>?</kbd></div>
      </div>

      {#each REBINDABLE_SHORTCUTS as { action, label, defaultCombo } (action)}
        <div class="setting-row {hidden(label, 'shortcut rebind keybind ' + action) ? 'hidden-by-search' : ''}">
          <div class="setting-meta">
            <span class="setting-label">{label}</span>
            <span class="setting-hint">
              {#if capturingShortcut === action}
                Press a key… (Esc to cancel)
              {:else}
                Click to rebind
              {/if}
            </span>
          </div>
          <div class="setting-control-tight">
            <button
              type="button"
              class="kbs-rebind-btn {capturingShortcut === action ? 'capturing' : ''}"
              onclick={() => startCaptureShortcut(action)}
              onkeydown={(e) => { if (capturingShortcut === action) captureShortcutKeydown(e, action) }}
              onmousedown={(e) => e.stopPropagation()}
            >
              <kbd>{capturingShortcut === action ? 'Press key…' : comboLabel(shortcutFor(action, defaultCombo))}</kbd>
            </button>
            {#if shortcutFor(action, defaultCombo) !== defaultCombo}
              <button
                type="button"
                class="btn-sm"
                onclick={() => resetShortcut(action)}
                onmousedown={(e) => e.stopPropagation()}
                data-tooltip={t('tt.resetDefault', settings.Language)}
                aria-label={t('tt.resetDefault', settings.Language)}
              >
                <i class="fa-solid fa-rotate-left"></i>
              </button>
            {/if}
          </div>
        </div>
      {/each}

      <div class="setting-row {hidden('Global quick-launch hotkey', 'global hotkey quick launch shortcut system-wide') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">{t('x.globalQuickLaunchHotkey', settings.Language)}</span>
          <span class="setting-hint">
            {#if capturingQuickLaunchHotkey}
              Press a combo with a modifier… (Esc to cancel)
            {:else if quickLaunchHotkeyError}
              <span class="error-text">{quickLaunchHotkeyError}</span>
            {:else}
              Works even when the launcher isn't focused/is minimized
            {/if}
          </span>
        </div>
        <div class="setting-control-tight">
          <button
            type="button"
            class="kbs-rebind-btn {capturingQuickLaunchHotkey ? 'capturing' : ''}"
            onclick={() => (capturingQuickLaunchHotkey = true)}
            onkeydown={(e) => { if (capturingQuickLaunchHotkey) captureQuickLaunchHotkey(e) }}
            onmousedown={(e) => e.stopPropagation()}
          >
            <kbd>{capturingQuickLaunchHotkey ? 'Press combo…' : quickLaunchHotkeyLabel()}</kbd>
          </button>
          {#if settings.QuickLaunchHotkey}
            <button
              type="button"
              class="btn-sm"
              onclick={clearQuickLaunchHotkey}
              onmousedown={(e) => e.stopPropagation()}
              data-tooltip={t('tt.disable', settings.Language)}
              aria-label={t('tt.disable', settings.Language)}
            >
              <i class="fa-solid fa-xmark"></i>
            </button>
          {/if}
        </div>
      </div>

      <div class="setting-row {hidden('Credits', 'credits about launcher') ? 'hidden-by-search' : ''}">
        <span class="setting-label">{t('nav.credits', settings.Language)}</span>
        <button class="btn-sm" onclick={() => onNavigate('credits')} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-heart"></i> View
        </button>
      </div>
    </div>
  {/if}
    </div>
  </div>
</div>

<style>
  /* Destructive actions use --red; the danger zone sits at the bottom of its
     section, divided from the regular rows above it. */
  .btn-danger {
    color: var(--red);
  }
  .btn-danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--red) 14%, transparent);
  }
  .danger-zone {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 6px;
    padding-top: 12px;
    border-top: 1px solid color-mix(in srgb, var(--red) 30%, transparent);
  }
  .danger-zone-label {
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--red);
  }
  /* Row rhythm: every setting row shares one minimum height and is divided
     from its neighbour by a hairline, so each section reads as one list. */
  .setting-row {
    min-height: 48px;
    box-sizing: border-box;
  }
  .setting-row + .setting-row {
    border-top: 1px solid color-mix(in srgb, var(--text) 7%, transparent);
  }
  .setting-row.setting-stack {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }
  /* Controls share one right-aligned column; text takes the remaining width. */
  .setting-control,
  .setting-control-tight {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    margin-left: auto;
  }
  .setting-control {
    gap: 8px;
  }
  .setting-control-tight {
    gap: 6px;
  }
  .setting-split {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
  }
  .setting-label-grow {
    flex: 1;
    min-width: 0;
  }
  .setting-muted {
    color: var(--text-dim);
    opacity: 0.7;
  }
  .setting-intro {
    margin: 0 0 6px;
  }
  .first-label {
    margin-top: 0;
  }
  /* Titled sub-group inside a section: a clear gap above, tight to its rows. */
  .settings-group-label {
    display: block;
    grid-column: 1 / -1;
    margin: 20px 0 6px;
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }
  .settings-advanced-toggle {
    display: flex;
    grid-column: 1 / -1;
    align-items: center;
    gap: 8px;
    width: 100%;
    margin: 20px 0 6px;
    padding: 6px 0;
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .settings-advanced-toggle:hover {
    color: var(--text);
  }
  .settings-advanced-toggle i {
    width: 10px;
    font-size: 0.7rem;
  }
  .slider-value {
    min-width: 40px;
    font-size: 11px;
    color: var(--text-dim);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .setting-input-xs {
    width: 130px;
  }
  .setting-input-sm {
    width: 70px;
  }
  .setting-input-md {
    width: 150px;
  }
  .setting-input-key {
    width: 160px;
    font-size: 11px;
  }
  .checkbox-group {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }
  .checkbox-item {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    cursor: pointer;
  }
  .spinner-sm {
    width: 11px;
    height: 11px;
    border-width: 1.5px;
  }
  .nav-logo {
    width: 18px;
    height: 18px;
    object-fit: contain;
  }
  .xbox-icon {
    color: var(--green);
  }
  .color-swatch {
    background: var(--swatch);
  }
  .truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  /* Status = colour + icon + word, on a tinted token fill. */
  .key-status,
  .status-ok,
  .status-muted,
  .status-error {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    border-radius: var(--r-sm);
    font-size: 11px;
  }
  .key-status,
  .status-ok {
    color: var(--accent-hover);
    background: var(--accent-bg);
  }
  .status-ok {
    color: var(--green);
    background: var(--green-bg);
  }
  .status-muted {
    color: var(--text-dim);
  }
  .status-error {
    color: var(--red);
    background: var(--red-bg);
  }
  .danger-zone .setting-row {
    border: 1px solid color-mix(in srgb, var(--red) 22%, transparent);
  }
</style>
