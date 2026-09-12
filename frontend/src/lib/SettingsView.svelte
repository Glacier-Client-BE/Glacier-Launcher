<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open, save } from '@tauri-apps/plugin-dialog'
  import PanelHeader from './components/PanelHeader.svelte'
  import Toggle from './components/Toggle.svelte'
  import { t, LANGUAGES } from './i18n'
  import type { LauncherSettings, JavaAccount } from './settings'

  // 1:1 port of the `settings` panel — Home.razor lines 1013-1928 plus
  // Pages/Home.Settings.cs. Every label, hint and option string below is
  // transcribed from the original; the category strip is the panel's own
  // inline `.versions-client-switcher` (NOT the shared SubTabs). No scoped
  // <style> block: every class used here already exists in app.css, and
  // Svelte scoping would silently outrank it.

  let {
    settings,
    onChange,
    onBack,
    onNavigate = () => {},
  }: {
    settings: LauncherSettings
    onChange: (s: LauncherSettings) => void
    onBack?: () => void
    onNavigate?: (view: string) => void
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

  let category = $state<'all' | 'injection' | 'java' | 'appearance' | 'account' | 'system'>('all')
  let filter = $state('')

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
    ['appearance', 'Theme preset', 'theme dark midnight slate ocean forest sunset light'],
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
    ['appearance', 'Theme Studio', 'theme studio custom editor create colors design'],

    ['account', 'Display name', 'username name display account'],
    ['account', 'Profile display', 'profile display footer xbox discord switch which'],
    ['account', 'Discord Rich Presence', 'discord rpc social'],
    ['account', 'Discord account', 'discord login account'],
    ['account', 'Xbox profile', 'xbox gamertag account microsoft'],

    ['system', 'Show recently launched', 'recent history'],
    ['system', 'Show launch console', 'console log launch window java bedrock output stdout'],
    ['system', 'Minimize to tray', 'tray minimize system'],
    ['system', 'Clear recent history', 'clear history recent'],
    ['system', 'Check for updates on startup', 'update auto check startup'],
    ['system', 'Check for updates now', 'update manual check'],
    ['system', 'Open launcher folder', 'launcher folder downloads location'],
    ['system', 'Open Minecraft folder', 'minecraft com mojang folder'],
    ['system', 'CurseForge API key', 'curseforge api key addons'],
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

  function setCategory(c: typeof category) {
    category = c
  }
  function clearFilter() {
    filter = ''
  }

  let exporting = $state(false)
  let importing = $state(false)
  let resetting = $state(false)
  let resetConfirmOpen = $state(false)
  let checkingUpdatesManual = $state(false)
  let availableUpdate = $state<{ tag: string; download_url: string; changelog: string; asset_size: number } | null>(null)
  let downloadingUpdate = $state(false)
  let installingUpdate = $state(false)
  let lastUpdateCheckLabel = $derived(
    settings.LastUpdateCheck ? 'Last checked ' + formatRelativeTime(settings.LastUpdateCheck) : 'Never checked'
  )
  let actionMessage = $state('')
  let discordConnecting = $state(false)

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

  async function manualUpdateCheck() {
    checkingUpdatesManual = true
    try {
      const update = await invoke<{ tag: string; download_url: string; changelog: string; asset_size: number } | null>(
        'check_launcher_update',
      )
      availableUpdate = update
      const now = new Date().toISOString()
      onChange({ ...settings, LastUpdateCheck: now })
      lastUpdateCheckLabel = update ? `Update ${update.tag} available` : "You're up to date"
    } catch (e) {
      actionMessage = String(e)
    } finally {
      checkingUpdatesManual = false
    }
  }

  // Downloads the update installer through the same DownloadQueue every
  // other download in the app uses, then launches it and closes the
  // launcher so the installer can take over — mirrors
  // AutoUpdateService's download-then-relaunch flow (see gl-update::resume
  // for why "swap" is "run the NSIS installer" rather than a raw exe copy
  // in this Tauri build).
  async function downloadAndInstallUpdate() {
    if (!availableUpdate) return
    downloadingUpdate = true
    try {
      const installerPath = await invoke<string>('download_launcher_update', {
        downloadUrl: availableUpdate.download_url,
        tag: availableUpdate.tag,
      })
      downloadingUpdate = false
      installingUpdate = true
      // On success this exits the app; on failure the installer path is
      // preserved (gl_update's pending-update marker) so the next launch
      // retries automatically before showing any UI.
      await invoke('install_launcher_update', { installerPath })
    } catch (e) {
      actionMessage = String(e)
    } finally {
      downloadingUpdate = false
      installingUpdate = false
    }
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

<PanelHeader title="Settings" {onBack} />

<div class="versions-client-switcher switcher-wrap">
  <button
    class="vcs-btn {category === 'all' ? 'active' : ''}"
    onclick={() => setCategory('all')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-layer-group"></i> All
  </button>
  {#if isBedrock}
    <button
      class="vcs-btn {category === 'injection' ? 'active' : ''}"
      onclick={() => setCategory('injection')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-solid fa-syringe"></i> Inject
    </button>
  {:else}
    <button
      class="vcs-btn {category === 'java' ? 'active' : ''}"
      onclick={() => setCategory('java')}
      onmousedown={(e) => e.stopPropagation()}
    >
      <i class="fa-brands fa-java"></i> Java
    </button>
  {/if}
  <button
    class="vcs-btn {category === 'appearance' ? 'active' : ''}"
    onclick={() => setCategory('appearance')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-palette"></i> Looks
  </button>
  <button
    class="vcs-btn {category === 'account' ? 'active' : ''}"
    onclick={() => setCategory('account')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-user"></i> Account
  </button>
  <button
    class="vcs-btn {category === 'system' ? 'active' : ''}"
    onclick={() => setCategory('system')}
    onmousedown={(e) => e.stopPropagation()}
  >
    <i class="fa-solid fa-gear"></i> System
  </button>
</div>

<div class="panel-search-wrap">
  <i class="fa-solid fa-magnifying-glass"></i>
  <input
    class="panel-search-input"
    placeholder="Filter settings"
    bind:value={filter}
    onkeydown={(e) => e.stopPropagation()}
  />
  {#if filter}
    <button class="panel-search-clear" title="Clear filter" onclick={clearFilter} onmousedown={(e) => e.stopPropagation()}>
      <i class="fa-solid fa-xmark"></i>
    </button>
  {/if}
</div>

<div class="panel-body settings-body">
  {#if visibleCount === 0}
    <div class="empty-state">
      <i class="fa-solid fa-folder-open"></i>
      <span>No matching settings</span>
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
        <span class="panel-section-label">Injection</span>
      {/if}

      <div class="setting-row {hidden('Active client', 'active client injection vanilla') ? 'hidden-by-search' : ''}">
        <span class="setting-label">Active client</span>
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
            <option value="Custom DLL">Custom DLL</option>
          {/if}
          <option value="Vanilla">Vanilla (no client)</option>
        </select>
      </div>

      <div class="setting-row {hidden('Injection delay', 'injection delay wait') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Injection delay</span>
          <span class="setting-hint">
            How long to wait after game launches before injecting
            {#if settings.SelectedClient === 'Vanilla'}<span style="color:var(--text-dim); opacity:0.7;"> · ignored in Vanilla</span>{/if}
          </span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range"
            min="500"
            max="15000"
            step="500"
            value={settings.InjectionDelayMs}
            oninput={(e) => update('InjectionDelayMs', Number((e.currentTarget as HTMLInputElement).value))}
            disabled={settings.SelectedClient === 'Vanilla'}
          />
          <span style="font-size:11px; color:var(--text-dim); width:38px; text-align:right;">
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
          <span class="setting-label">Auto-inject</span>
          <span class="setting-hint">Inject automatically once the game process is detected</span>
        </div>
        <div
          class="toggle {settings.AutoInject ? 'on' : ''}"
          role="switch"
          aria-checked={settings.AutoInject}
          aria-label="Auto-inject"
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
          <span class="setting-label">Close after launch</span>
          <span class="setting-hint">Minimise the launcher once injection succeeds</span>
        </div>
        <div
          class="toggle {settings.CloseAfterLaunch ? 'on' : ''}"
          role="switch"
          aria-checked={settings.CloseAfterLaunch}
          aria-label="Close after launch"
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
          <span class="setting-label">Verify injection</span>
          <span class="setting-hint">Confirm the DLL really appears in Minecraft's module list instead of trusting LoadLibrary alone</span>
        </div>
        <div
          class="toggle {settings.VerifyInjection ? 'on' : ''}"
          role="switch"
          aria-checked={settings.VerifyInjection}
          aria-label="Verify injection"
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
          <span class="setting-label">Prevent game suspend</span>
          <span class="setting-hint">Stop Windows throttling Bedrock when it isn't the focused window (needs AppLifecycleOptOut)</span>
        </div>
        <div
          class="toggle {settings.PreventGameSuspend ? 'on' : ''}"
          role="switch"
          aria-checked={settings.PreventGameSuspend}
          aria-label="Prevent game suspend"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('PreventGameSuspend', !settings.PreventGameSuspend) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('PreventGameSuspend', !settings.PreventGameSuspend) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Accelerate loading', 'accelerate loading runtimebroker faster startup') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Accelerate loading</span>
          <span class="setting-hint">
            Stop RuntimeBroker this long after launch to shorten Bedrock's loading screen
            {#if settings.AccelerateLoadingMs === 0}<span style="color:var(--text-dim); opacity:0.7;"> · off</span>{/if}
          </span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range"
            min="0"
            max="20000"
            step="1000"
            value={settings.AccelerateLoadingMs}
            oninput={(e) => update('AccelerateLoadingMs', Math.min(20000, Math.max(0, Number((e.currentTarget as HTMLInputElement).value))))}
          />
          <span style="font-size:11px; color:var(--text-dim); width:38px; text-align:right;">
            {settings.AccelerateLoadingMs === 0 ? 'Off' : `${settings.AccelerateLoadingMs / 1000}s`}
          </span>
        </div>
      </div>
    </div>
  {/if}

  {#if categoryVisible('java')}
    <div class="settings-section">
      {#if sectionVisible(['Maximum RAM', 'ram memory java heap'], ['Minecraft folder', 'minecraft folder dot directory'], ['Java runtime', 'java runtime jre jdk javaw'])}
        <span class="panel-section-label">Java Edition</span>
      {/if}

      <div class="setting-row {hidden('Maximum RAM', 'ram memory java heap') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Maximum RAM</span>
          <span class="setting-hint">Heap size cap for the Minecraft JVM (-Xmx)</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range"
            min="1024"
            max="16384"
            step="512"
            value={settings.JavaMaxRamMb}
            oninput={(e) => update('JavaMaxRamMb', Number((e.currentTarget as HTMLInputElement).value))}
          />
          <span style="font-size:11px; color:var(--text-dim); width:50px; text-align:right;">
            {settings.JavaMaxRamMb} MB
          </span>
        </div>
      </div>

      <div class="setting-row {hidden('Minimum RAM', 'ram memory java heap xms') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Minimum RAM</span>
          <span class="setting-hint">Initial heap size for the Minecraft JVM (-Xms)</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range"
            min="256"
            max={settings.JavaMaxRamMb}
            step="256"
            value={settings.JavaMinRamMb}
            oninput={(e) => update('JavaMinRamMb', Number((e.currentTarget as HTMLInputElement).value))}
          />
          <span style="font-size:11px; color:var(--text-dim); width:50px; text-align:right;">
            {settings.JavaMinRamMb} MB
          </span>
        </div>
      </div>

      <div
        class="setting-row clickable {hidden('Game files location', 'minecraft folder glacier location dot directory store') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaUseDotMinecraft', !settings.JavaUseDotMinecraft)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaUseDotMinecraft', !settings.JavaUseDotMinecraft) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Store game files in .minecraft</span>
          <span class="setting-hint">On: use %APPDATA%\.minecraft. Off: keep versions & saves in the Glacier Launcher folder.</span>
        </div>
        <div class="toggle {settings.JavaUseDotMinecraft ? 'on' : ''}"></div>
      </div>

      <div class="setting-row {hidden('Minecraft folder', 'minecraft folder dot directory') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Custom Minecraft folder</span>
          <span class="setting-hint">Overrides the toggle above. Leave empty to use it.</span>
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
          <span class="setting-label">Show snapshots</span>
          <span class="setting-hint">Include experimental builds in the Versions list</span>
        </div>
        <div
          class="toggle {settings.JavaShowSnapshots ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaShowSnapshots}
          aria-label="Show snapshots"
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
          <span class="setting-hint">Include the original pre-release builds</span>
        </div>
        <div
          class="toggle {settings.JavaShowHistorical ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaShowHistorical}
          aria-label="Show beta & alpha"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('JavaShowHistorical', !settings.JavaShowHistorical) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('JavaShowHistorical', !settings.JavaShowHistorical) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Close after launch', 'close minimize launch java') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('CloseAfterLaunch', !settings.CloseAfterLaunch)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('CloseAfterLaunch', !settings.CloseAfterLaunch) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Close after launch</span>
          <span class="setting-hint">Minimise the launcher once the game starts</span>
        </div>
        <div
          class="toggle {settings.CloseAfterLaunch ? 'on' : ''}"
          role="switch"
          aria-checked={settings.CloseAfterLaunch}
          aria-label="Close after launch"
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
          <span class="setting-label">Offline mode</span>
          <span class="setting-hint">Launch without signing in — online-mode servers will reject it</span>
        </div>
        <div
          class="toggle {settings.JavaOfflineMode ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaOfflineMode}
          aria-label="Offline mode"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('JavaOfflineMode', !settings.JavaOfflineMode) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('JavaOfflineMode', !settings.JavaOfflineMode) } }}
        ></div>
      </div>

      {#if settings.JavaOfflineMode}
        <div class="setting-row {hidden('Offline username', 'offline username name player') ? 'hidden-by-search' : ''}">
          <div class="setting-meta">
            <span class="setting-label">Offline username</span>
            <span class="setting-hint">The name used for offline launches</span>
          </div>
          <input
            class="setting-input"
            style="width:150px;"
            type="text"
            maxlength="16"
            placeholder="Player"
            value={settings.JavaOfflineUsername}
            onchange={(e) => update('JavaOfflineUsername', (e.currentTarget as HTMLInputElement).value.trim() || 'Player')}
            onclick={(e) => e.stopPropagation()}
          />
        </div>
      {/if}

      <div class="setting-row {hidden('Custom JVM args', 'jvm args java options garbage collector gc') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Custom JVM args</span>
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

      <div
        class="setting-row clickable {hidden('Backup saves before launch', 'world backup saves zip') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaBackupSavesBeforeLaunch', !settings.JavaBackupSavesBeforeLaunch)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaBackupSavesBeforeLaunch', !settings.JavaBackupSavesBeforeLaunch) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Backup saves before launch</span>
          <span class="setting-hint">Creates a zip of the active instance saves folder</span>
        </div>
        <div
          class="toggle {settings.JavaBackupSavesBeforeLaunch ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaBackupSavesBeforeLaunch}
          aria-label="Backup saves before launch"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('JavaBackupSavesBeforeLaunch', !settings.JavaBackupSavesBeforeLaunch) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('JavaBackupSavesBeforeLaunch', !settings.JavaBackupSavesBeforeLaunch) } }}
        ></div>
      </div>

      <div
        class="setting-row clickable {hidden('Fullscreen', 'window fullscreen resolution') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('JavaFullscreen', !settings.JavaFullscreen)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaFullscreen', !settings.JavaFullscreen) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Fullscreen</span>
          <span class="setting-hint">Adds --fullscreen to Java launches</span>
        </div>
        <div
          class="toggle {settings.JavaFullscreen ? 'on' : ''}"
          role="switch"
          aria-checked={settings.JavaFullscreen}
          aria-label="Fullscreen"
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
          <span class="setting-label">Custom resolution</span>
          <span class="setting-hint">Starts windowed Minecraft at the selected size</span>
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div style="display:flex; align-items:center; gap:6px;" onclick={(e) => e.stopPropagation()}>
          <input
            class="setting-input"
            style="width:70px;"
            type="number"
            value={settings.JavaWindowWidth}
            onchange={(e) => update('JavaWindowWidth', Number((e.currentTarget as HTMLInputElement).value))}
            onkeydown={(e) => e.stopPropagation()}
          />
          <input
            class="setting-input"
            style="width:70px;"
            type="number"
            value={settings.JavaWindowHeight}
            onchange={(e) => update('JavaWindowHeight', Number((e.currentTarget as HTMLInputElement).value))}
            onkeydown={(e) => e.stopPropagation()}
          />
          <div
            class="toggle {settings.JavaUseCustomResolution ? 'on' : ''}"
            role="switch"
            aria-checked={settings.JavaUseCustomResolution}
            aria-label="Custom resolution"
            tabindex="0"
            onclick={() => update('JavaUseCustomResolution', !settings.JavaUseCustomResolution)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('JavaUseCustomResolution', !settings.JavaUseCustomResolution) }}
          ></div>
        </div>
      </div>

      <div class="setting-row {hidden('Server auto-connect', 'server port multiplayer quick play') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Server auto-connect</span>
          <span class="setting-hint">Optional --server and --port launch injection</span>
        </div>
        <div style="display:flex; align-items:center; gap:6px;">
          <input
            class="setting-input"
            style="width:150px;"
            type="text"
            placeholder="server address"
            value={settings.JavaServerAddress}
            onchange={(e) => update('JavaServerAddress', (e.currentTarget as HTMLInputElement).value)}
            onkeydown={(e) => e.stopPropagation()}
          />
          <input
            class="setting-input"
            style="width:70px;"
            type="number"
            value={settings.JavaServerPort}
            onchange={(e) => update('JavaServerPort', Number((e.currentTarget as HTMLInputElement).value))}
            onkeydown={(e) => e.stopPropagation()}
          />
        </div>
      </div>

      <div class="setting-row {hidden('Open Minecraft folder', 'minecraft folder dot directory open') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Open .minecraft folder</span>
          <span class="setting-hint">{settings.JavaMinecraftDir || '%APPDATA%\\.minecraft'}</span>
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

      <div class="setting-row {hidden('Latest crash report', 'crash report log latest open view java') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Latest crash report</span>
          <span class="setting-hint">{latestCrashLabel}</span>
        </div>
        <button class="btn-sm" onclick={openLatestCrash} onmousedown={(e) => e.stopPropagation()} disabled={!latestCrash}>
          <i class="fa-solid fa-bug"></i> Open
        </button>
      </div>
    </div>
  {/if}

  {#if categoryVisible('appearance')}
    <div class="settings-section">
      {#if sectionVisible(['Accent colour', 'accent color theme'], ['Theme preset', 'theme dark midnight slate ocean forest sunset light'], ['Panel side', 'panel side left right sidebar position'], ['Compact mode', 'compact dense small'], ['Animations', 'animations smooth motion'], ['Accessibility features', 'accessibility a11y reduced motion focus outline screen reader'], ['Background blur', 'blur background'], ['Remember window size', 'window size remember'], ['Custom background wallpaper', 'background wallpaper image custom'])}
        <span class="panel-section-label">Appearance</span>
      {/if}

      <div class="setting-row {hidden('Accent colour', 'accent color theme') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Accent colour</span>
          <span class="setting-hint">Used for buttons, highlights and glows</span>
        </div>
        <div class="color-swatches">
          {#each accentSwatches as c (c)}
            <div
              class="color-swatch {settings.AccentColor === c ? 'active' : ''}"
              style="background:{c};"
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
          <span class="setting-label">Theme preset</span>
          <span class="setting-hint">Background tone — affects panels and overlays</span>
        </div>
        <select
          class="setting-select"
          value={settings.ThemePreset}
          onchange={(e) => setThemePreset((e.currentTarget as HTMLSelectElement).value)}
          onclick={(e) => e.stopPropagation()}
        >
          <option value="dark">Dark</option>
          <option value="darker">Darker</option>
          <option value="midnight">Midnight</option>
          <option value="slate">Slate</option>
          <option value="ocean">Ocean</option>
          <option value="forest">Forest</option>
          <option value="sunset">Sunset</option>
          <option value="light">Light</option>
        </select>
      </div>

      <div class="setting-row {hidden('Panel side', 'panel side left right sidebar position') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Panel side</span>
          <span class="setting-hint">Which edge of the window panels slide in from</span>
        </div>
        <select
          class="setting-select"
          value={settings.PanelSide}
          onchange={(e) => setPanelSide((e.currentTarget as HTMLSelectElement).value as 'left' | 'right')}
          onclick={(e) => e.stopPropagation()}
        >
          <option value="right">Right</option>
          <option value="left">Left</option>
        </select>
      </div>

      <div
        class="setting-row clickable {hidden('Compact mode', 'compact dense small') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => setCompactMode(!settings.CompactMode)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') setCompactMode(!settings.CompactMode) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Compact mode</span>
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
          <span class="setting-label">Animations</span>
          <span class="setting-hint">Disable for low-end devices or reduced motion</span>
        </div>
        <Toggle on={settings.AnimationsEnabled} onChanged={(v) => setAnimationsEnabled(v)} />
      </div>

      <div
        class="setting-row clickable {hidden('Accessibility features', 'accessibility a11y reduced motion focus outline screen reader') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => setAccessibilityMode(!settings.AccessibilityMode)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') setAccessibilityMode(!settings.AccessibilityMode) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Accessibility features</span>
          <span class="setting-hint">Reduced motion, stronger focus outlines, larger click targets</span>
        </div>
        <Toggle on={settings.AccessibilityMode} onChanged={(v) => setAccessibilityMode(v)} />
      </div>

      {#if settings.AnimationsEnabled}
        <div class="setting-row {hidden('Animation speed', 'animation speed motion fast slow multiplier') ? 'hidden-by-search' : ''}">
          <div class="setting-meta">
            <span class="setting-label">Animation speed</span>
            <span class="setting-hint">0.25× slow-mo to 2× snappy — 1× is default</span>
          </div>
          <div style="display:flex; align-items:center; gap:8px;">
            <input
              type="range"
              min="0.25"
              max="2"
              step="0.25"
              value={settings.AnimationSpeed}
              oninput={(e) => setAnimSpeed(Number((e.currentTarget as HTMLInputElement).value))}
            />
            <span style="font-size:11px; color:var(--text-dim); width:32px; text-align:right;">
              {settings.AnimationSpeed}×
            </span>
          </div>
        </div>
      {/if}

      <div class="setting-row {hidden('Background blur', 'blur background') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Background blur</span>
          <span class="setting-hint">Backdrop blur intensity for panels</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range"
            min="0"
            max="30"
            step="2"
            value={settings.BlurIntensity}
            oninput={(e) => setBlur(Number((e.currentTarget as HTMLInputElement).value))}
          />
          <span style="font-size:11px; color:var(--text-dim); width:32px; text-align:right;">
            {settings.BlurIntensity}px
          </span>
        </div>
      </div>

      <div
        class="setting-row clickable {hidden('Remember window size', 'window size remember') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('RememberWindowSize', !settings.RememberWindowSize)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('RememberWindowSize', !settings.RememberWindowSize) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Remember window size</span>
          <span class="setting-hint">Restore last used size on next launch</span>
        </div>
        <div
          class="toggle {settings.RememberWindowSize ? 'on' : ''}"
          role="switch"
          aria-checked={settings.RememberWindowSize}
          aria-label="Remember window size"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('RememberWindowSize', !settings.RememberWindowSize) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('RememberWindowSize', !settings.RememberWindowSize) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Custom background wallpaper', 'background wallpaper image custom') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Background wallpaper</span>
          <span class="setting-hint">
            {#if settings.CustomBackgroundPath}
              Custom — {fileName(settings.CustomBackgroundPath)}
            {:else}
              Default background
            {/if}
          </span>
        </div>
        <div style="display:flex; align-items:center; gap:6px;">
          {#if settings.CustomBackgroundPath}
            <button class="btn-sm" onclick={resetWallpaper} onmousedown={(e) => e.stopPropagation()} data-tooltip="Reset to default">
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
          <span class="setting-label">Background fit</span>
          <span class="setting-hint">How the wallpaper fills the window</span>
        </div>
        <select
          class="setting-select"
          value={settings.BackgroundFit}
          onchange={(e) => setBackgroundFit((e.currentTarget as HTMLSelectElement).value)}
          onclick={(e) => e.stopPropagation()}
        >
          <option value="cover">Fill (cover)</option>
          <option value="contain">Fit (contain)</option>
          <option value="tile">Tile</option>
          <option value="center">Center</option>
        </select>
      </div>

      <div class="setting-row {hidden('UI scale', 'ui scale zoom size interface') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">UI scale</span>
          <span class="setting-hint">On top of the automatic window fit — 100% is default</span>
        </div>
        <div style="display:flex; align-items:center; gap:8px;">
          <input
            type="range"
            min="75"
            max="150"
            step="5"
            value={settings.UiScalePct}
            oninput={(e) => setUiScale(Number((e.currentTarget as HTMLInputElement).value))}
          />
          <span style="font-size:11px; color:var(--text-dim); width:36px; text-align:right;">
            {settings.UiScalePct}%
          </span>
        </div>
      </div>

      <div class="setting-row {hidden('Font', 'font family typeface typography') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Font</span>
          <span class="setting-hint">Any installed font — empty uses Segoe UI</span>
        </div>
        <input
          class="setting-input"
          style="width:150px;"
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

      <div
        class="setting-row clickable {hidden('Theme Studio', 'theme studio custom editor create colors design') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => onNavigate('themestudio')}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onNavigate('themestudio') }}
      >
        <div class="setting-meta">
          <span class="setting-label">Theme Studio</span>
          <span class="setting-hint">
            {settings.ActiveThemeId
              ? 'Custom theme active — click to edit or switch'
              : 'Build fully custom themes — every color, radius and font'}
          </span>
        </div>
        <button
          class="btn-sm"
          onclick={(e) => { e.stopPropagation(); onNavigate('themestudio') }}
          onmousedown={(e) => e.stopPropagation()}
        >
          <i class="fa-solid fa-palette"></i> Open
        </button>
      </div>
    </div>
  {/if}

  {#if categoryVisible('account')}
    <div class="settings-section">
      {#if sectionVisible(['Display name', 'username name display account'], ['Profile display', 'profile display footer xbox discord switch which'])}
        <span class="panel-section-label">Account</span>
      {/if}

      <div class="setting-row {hidden('Display name', 'username name display account') ? 'hidden-by-search' : ''}">
        <span class="setting-label">Display name</span>
        <input
          class="setting-input"
          style="width:130px;"
          value={settings.Username}
          placeholder="Player"
          onchange={(e) => update('Username', (e.currentTarget as HTMLInputElement).value)}
          onclick={(e) => e.stopPropagation()}
        />
      </div>

      <div class="setting-row {hidden('Profile display', 'profile display footer xbox discord switch which') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Profile display</span>
          <span class="setting-hint">Which account fills the bottom-left avatar & name.</span>
        </div>
        <select
          class="setting-select"
          value={settings.ProfileDisplayMode}
          onchange={(e) => {
            const v = (e.currentTarget as HTMLSelectElement).value
            update('ProfileDisplayMode', v === 'xbox' || v === 'discord' ? v : 'auto')
          }}
          onclick={(e) => e.stopPropagation()}
        >
          <option value="auto">Auto (prefer Xbox)</option>
          <option value="xbox">Xbox</option>
          <option value="discord">Discord</option>
        </select>
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
        <span class="panel-section-label">Social</span>
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
          aria-label="Discord Rich Presence"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); toggleDiscordRpc() }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); toggleDiscordRpc() } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Discord account', 'discord login account') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Discord account</span>
          <span class="setting-hint">
            {settings.DiscordLoggedIn ? 'Connected as ' + settings.DiscordUsername : 'Not connected'}
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
            {settings.XboxGamertag ? 'Signed in as ' + settings.XboxGamertag : 'Not connected'}
          </span>
        </div>
        <button
          class="btn-sm"
          disabled={xboxSigningIn}
          onclick={() => (settings.XboxGamertag ? disconnectXbox() : connectXbox())}
        >
          <i class="fa-solid fa-gamepad" style="color:#107c10;"></i>
          {#if xboxSigningIn}
            Working...
          {:else}
            {settings.XboxGamertag ? 'Disconnect' : 'Sign in'}
          {/if}
        </button>
      </div>

      {#if settings.JavaAccounts.length > 0}
        <div class="setting-row" style="flex-direction:column; align-items:stretch; gap:8px;">
          <div style="display:flex; align-items:center; justify-content:space-between;">
            <span class="setting-label">Saved accounts</span>
            <button class="btn-sm" disabled={xboxSigningIn} onclick={connectXbox}>
              <i class="fa-solid fa-plus"></i> Add account
            </button>
          </div>
          {#each settings.JavaAccounts as account (account.Id)}
            {@const isActive = account.Id === settings.ActiveJavaAccountId}
            <div class="instance-row {isActive ? 'active' : ''}">
              <button class="instance-name-btn" onclick={() => switchJavaAccount(account)} disabled={isActive}>
                {#if isActive}<i class="fa-solid fa-circle-check"></i>{:else}<i class="fa-regular fa-circle"></i>{/if}
                <span>{account.Username || account.Gamertag || account.Id}</span>
              </button>
              <button
                class="instance-icon-btn"
                onclick={() => removeJavaAccount(account)}
                data-tooltip="Remove" aria-label="Remove account"
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
        <span class="panel-section-label">Quality of Life</span>
      {/if}

      <div
        class="setting-row clickable {hidden('Show recently launched', 'recent history') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('ShowRecentlyLaunched', !settings.ShowRecentlyLaunched)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('ShowRecentlyLaunched', !settings.ShowRecentlyLaunched) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Show recently launched</span>
          <span class="setting-hint">Quick-access chips on the home screen</span>
        </div>
        <div
          class="toggle {settings.ShowRecentlyLaunched ? 'on' : ''}"
          role="switch"
          aria-checked={settings.ShowRecentlyLaunched}
          aria-label="Show recently launched"
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
          <span class="setting-label">Show launch console</span>
          <span class="setting-hint">Pops a window with live game output (Java) or launcher events (Bedrock) on every launch</span>
        </div>
        <div
          class="toggle {settings.ShowLaunchConsole ? 'on' : ''}"
          role="switch"
          aria-checked={settings.ShowLaunchConsole}
          aria-label="Show launch console"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('ShowLaunchConsole', !settings.ShowLaunchConsole) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('ShowLaunchConsole', !settings.ShowLaunchConsole) } }}
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
          <span class="setting-label">Minimize to tray</span>
          <span class="setting-hint">Minimize button sends to system tray</span>
        </div>
        <div
          class="toggle {settings.MinimizeToTray ? 'on' : ''}"
          role="switch"
          aria-checked={settings.MinimizeToTray}
          aria-label="Minimize to tray"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('MinimizeToTray', !settings.MinimizeToTray) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('MinimizeToTray', !settings.MinimizeToTray) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Clear recent history', 'clear history recent') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Clear recent history</span>
          <span class="setting-hint">{settings.RecentlyLaunched.length} item(s)</span>
        </div>
        <button class="btn-sm" onclick={clearRecentHistory} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-trash"></i> Clear
        </button>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Check for updates on startup', 'update auto check startup'], ['Check for updates now', 'update manual check'])}
        <span class="panel-section-label">Updates</span>
      {/if}

      <div
        class="setting-row clickable {hidden('Check for updates on startup', 'update auto check startup') ? 'hidden-by-search' : ''}"
        role="button"
        tabindex="0"
        onclick={() => update('CheckUpdatesOnStartup', !settings.CheckUpdatesOnStartup)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') update('CheckUpdatesOnStartup', !settings.CheckUpdatesOnStartup) }}
      >
        <div class="setting-meta">
          <span class="setting-label">Check for updates on startup</span>
          <span class="setting-hint">Silently checks launcher + client updates in the background</span>
        </div>
        <div
          class="toggle {settings.CheckUpdatesOnStartup ? 'on' : ''}"
          role="switch"
          aria-checked={settings.CheckUpdatesOnStartup}
          aria-label="Check for updates on startup"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); update('CheckUpdatesOnStartup', !settings.CheckUpdatesOnStartup) }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); update('CheckUpdatesOnStartup', !settings.CheckUpdatesOnStartup) } }}
        ></div>
      </div>

      <div class="setting-row {hidden('Check for updates now', 'update manual check') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Check for updates now</span>
          <span class="setting-hint">{checkingUpdatesManual ? 'Checking...' : lastUpdateCheckLabel}</span>
        </div>
        <button
          class="btn-sm"
          onclick={manualUpdateCheck}
          onmousedown={(e) => e.stopPropagation()}
          disabled={checkingUpdatesManual}
        >
          {#if checkingUpdatesManual}
            <span class="spinner" style="width:11px;height:11px;border-width:1.5px;"></span>
          {:else}
            <i class="fa-solid fa-rotate"></i>
          {/if}
          Check
        </button>
      </div>

      {#if availableUpdate}
        <div class="setting-row">
          <div class="setting-meta">
            <span class="setting-label">Update {availableUpdate.tag} available</span>
            <span class="setting-hint">
              {#if installingUpdate}
                Launching installer…
              {:else if downloadingUpdate}
                Downloading…
              {:else}
                Download and install now
              {/if}
            </span>
          </div>
          <button
            class="btn-sm"
            onclick={downloadAndInstallUpdate}
            onmousedown={(e) => e.stopPropagation()}
            disabled={downloadingUpdate || installingUpdate}
          >
            {#if downloadingUpdate || installingUpdate}
              <span class="spinner" style="width:11px;height:11px;border-width:1.5px;"></span>
            {:else}
              <i class="fa-solid fa-download"></i>
            {/if}
            Update
          </button>
        </div>
      {/if}
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Open launcher folder', 'launcher folder downloads location'], ['Open Minecraft folder', 'minecraft com mojang folder'])}
        <span class="panel-section-label">Folders</span>
      {/if}

      <div class="setting-row {hidden('Open launcher folder', 'launcher folder downloads location') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Launcher folder</span>
          <span class="setting-hint">Settings, downloaded clients and DLLs live here</span>
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
        <span class="panel-section-label">CurseForge</span>
      {/if}

      <div class="setting-row {hidden('CurseForge API key', 'curseforge api key addons') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">CurseForge API key</span>
          <span class="setting-hint">
            {#if settings.CurseForgeApiKey.trim()}
              <span style="color:var(--accent);"><i class="fa-solid fa-circle-check"></i> Custom key set</span>
            {:else}
              Required for Addons tab — get one free from CurseForge
            {/if}
          </span>
        </div>
        <div style="display:flex; align-items:center; gap:6px;">
          <input
            class="setting-input"
            style="width:160px; font-size:10px;"
            type="password"
            value={settings.CurseForgeApiKey}
            placeholder="Paste API key..."
            onchange={(e) => update('CurseForgeApiKey', (e.currentTarget as HTMLInputElement).value)}
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
          />
          <a
            class="btn-sm"
            href="https://console.curseforge.com/"
            target="_blank"
            rel="noreferrer"
            data-tooltip="Get API key"
            aria-label="Get API key"
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-arrow-up-right-from-square"></i>
          </a>
        </div>
      </div>
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Export settings', 'export backup settings save'], ['Import settings', 'import restore settings load'], ['Reset settings', 'reset default factory settings'])}
        <span class="panel-section-label">Backup</span>
      {/if}

      <div class="setting-row {hidden('Export settings', 'export backup settings save') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Export settings</span>
          <span class="setting-hint">Save your settings as a JSON backup</span>
        </div>
        <button class="btn-sm" onclick={exportSettings} onmousedown={(e) => e.stopPropagation()} disabled={exporting}>
          <i class="fa-solid fa-file-export"></i> Export
        </button>
      </div>

      <div class="setting-row {hidden('Import settings', 'import restore settings load') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Import settings</span>
          <span class="setting-hint">Restore from a previously exported JSON</span>
        </div>
        <button class="btn-sm" onclick={importSettings} onmousedown={(e) => e.stopPropagation()} disabled={importing}>
          <i class="fa-solid fa-file-import"></i> Import
        </button>
      </div>

      <div class="setting-row {hidden('Reset settings', 'reset default factory settings') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Reset to defaults</span>
          <span class="setting-hint">Clears all preferences — does not delete clients</span>
        </div>
        {#if resetConfirmOpen}
          <div style="display:flex; align-items:center; gap:6px;">
            <button
              class="btn-sm"
              style="color:var(--red);"
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
            class="btn-sm"
            style="color:var(--red);"
            onclick={() => (resetConfirmOpen = true)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-arrow-rotate-left"></i> Reset
          </button>
        {/if}
      </div>

      {#if actionMessage}
        <div class="setting-row"><span class="setting-hint">{actionMessage}</span></div>
      {/if}
    </div>

    <div class="settings-section">
      {#if sectionVisible(['Keyboard shortcuts', 'shortcuts keyboard help'], ['Credits', 'credits about launcher'])}
        <span class="panel-section-label">About</span>
      {/if}

      <div class="setting-row {hidden('Keyboard shortcuts', 'shortcuts keyboard help') ? 'hidden-by-search' : ''}">
        <div class="setting-meta">
          <span class="setting-label">Keyboard shortcuts</span>
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
          <div style="display:flex; align-items:center; gap:6px;">
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
                data-tooltip="Reset to default"
                aria-label="Reset to default"
              >
                <i class="fa-solid fa-rotate-left"></i>
              </button>
            {/if}
          </div>
        </div>
      {/each}

      <div class="setting-row {hidden('Credits', 'credits about launcher') ? 'hidden-by-search' : ''}">
        <span class="setting-label">Credits</span>
        <button class="btn-sm" onclick={() => onNavigate('credits')} onmousedown={(e) => e.stopPropagation()}>
          <i class="fa-solid fa-heart"></i> View
        </button>
      </div>
    </div>
  {/if}
</div>

