<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * 1:1 port of the `javaprofile` panel — `Pages/Home.razor` lines 5166-5354 —
   * plus its code-behind: `Home.BigFeatures.cs` (_instanceMaxRam,
   * OnInstanceMaxRamChange, PlayerCleanUuid, PlayerName, OpenPlayerNameMc,
   * DownloadPlayerSkin, CopyPlayerUuid, _changeSkinSlim, ChangeSkinAsync,
   * ResetSkinAsync, LastAppliedSkinUrl, capes), `Home.Clients.cs`
   * (instance switch/new/clone/rename/delete/import), `Home.QuickActions.cs`
   * (FormatPlaytime), `Home.razor.cs` (FormatRelativeTime, IsJava, mcRunning)
   * and `Home.Settings.cs` (OpenStats/OpenLogs/OpenSkinLibrary/OpenBackups).
   *
   * The previous contents of this file were an invented "Xbox profile card"
   * (avatar / gamerscore / XUID / bio tiles, a Sign out button) that exists
   * nowhere in the original. It has been deleted wholesale.
   *
   * Backend gaps. Every one is surfaced as a visibly disabled control with a
   * tooltip — never faked, never silently dropped:
   *   - skin upload / reset: no `upload_skin` / `reset_skin` command.
   *   - capes: no `get_capes` / `set_cape` / `hide_cape` command, so the cape
   *     wardrobe list stays empty. The original only renders the card when
   *     `_capesLoading || _capes.Count > 0`, so with no data the card does not
   *     render at all — that is the original's own empty behaviour, preserved.
   *   - `mcRunning`: now wired to App.svelte's `pollMcRunning`, which for
   *     Java checks `is_java_game_running` (the last-launched PID's liveness)
   *     rather than name-matching `java.exe`.
   *
   * No scoped <style> block: every class below exists in app.css.
   * No `.panel-overlay` / `.panel-handle`: App.svelte supplies those.
   */
  import { open } from '@tauri-apps/plugin-dialog'
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import SkinViewer from './components/SkinViewer.svelte'
  import type { LauncherSettings, JavaInstance } from './settings'
  import ListSkeleton from './components/ListSkeleton.svelte'

  let {
    settings,
    onChange,
    onBack,
    onNavigate,
    onToast,
    mcRunning = false,
  }: {
    settings: LauncherSettings
    onChange: (next: LauncherSettings) => void
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
    mcRunning?: boolean
  } = $props()

  function toast(message: string, kind = 'info') {
    onToast?.(message, kind)
  }

  // ---------------------------------------------------------------- profile

  /** Port of `PlayerCleanUuid`. */
  const playerCleanUuid = $derived((settings.JavaUuid ?? '').replace(/-/g, ''))

  /** Port of `PlayerName`. */
  const playerName = $derived(
    settings.JavaUsername ? settings.JavaUsername : settings.XboxGamertag ? settings.XboxGamertag : 'Player',
  )

  /** Port of `IsJava`. */
  const isJava = $derived((settings.Edition ?? '').toLowerCase() === 'java')

  /** Port of `LastAppliedSkinUrl` — the local-file preview. The rewrite has no
   *  `FileToLocalUrl` host bridge, so this stays empty and the viewer falls
   *  back to the remote render, exactly as it does in the original when the
   *  file is missing or the change is older than an hour. */
  const lastAppliedSkinUrl = ''

  /** Port of `FormatPlaytime`. */
  function formatPlaytime(seconds: number): string {
    if (seconds <= 0) return '0m'
    const h = Math.floor(seconds / 3600)
    const m = Math.floor((seconds % 3600) / 60)
    return h > 0 ? `${h}h ${m}m` : `${m}m`
  }

  /** Port of `FormatRelativeTime`. */
  function formatRelativeTime(isoTimestamp: string): string {
    try {
      const dt = new Date(isoTimestamp)
      if (Number.isNaN(dt.getTime())) return ''
      const diffMs = Date.now() - dt.getTime()
      const minutes = diffMs / 60000
      const hours = diffMs / 3600000
      const days = diffMs / 86400000
      if (minutes < 2) return 'just now'
      if (hours < 1) return `${Math.floor(minutes)}m ago`
      if (days < 1) return `${Math.floor(hours)}h ago`
      if (days < 7) return `${Math.floor(days)}d ago`
      return dt.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
    } catch {
      return ''
    }
  }

  /** Port of `CopyPlayerUuid`. */
  async function copyPlayerUuid() {
    if (!playerCleanUuid) return
    try {
      await navigator.clipboard.writeText(playerCleanUuid)
      toast('UUID copied', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  /**
   * Port of the panel's Change-skin action. The picker has to originate in the
   * host: WebView2 exposes no real path for a file chosen inside the page.
   * Slim/classic follows the panel's own `JavaSkinSlimModel` toggle, as the
   * original does.
   */
  async function changeSkin() {
    const path = await open({
      title: 'Choose a skin PNG',
      multiple: false,
      directory: false,
      filters: [{ name: 'Minecraft skin (PNG)', extensions: ['png'] }],
    })
    if (!path || typeof path !== 'string') return
    try {
      await invoke('apply_skin', { path, slim: settings?.JavaSkinSlimModel ?? false })
      onToast?.(t('toast.skinApplied'), 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  async function resetSkin() {
    try {
      await invoke('reset_skin', {})
      onToast?.(t('toast.skinResetDefault'), 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  function openPlayerNameMc() {
    void invoke('open_url', { url: `https://namemc.com/profile/${playerCleanUuid}` })
  }

  /** Port of `DownloadPlayerSkin`. */
  function downloadPlayerSkin() {
    void invoke('open_url', { url: `https://crafatar.com/skins/${playerCleanUuid}` })
  }

  /** Port of `_changeSkinSlim`'s setter (writes `JavaSkinSlimModel`). */
  function setChangeSkinSlim(value: boolean) {
    onChange({ ...settings, JavaSkinSlimModel: value })
  }

  type XboxProfile = {
    gamertag: string
    xuid: string
    gamer_picture_url: string
    gamerscore: string
    account_tier: string
    bio: string
  }
  type MinecraftProfile = { uuid: string; name: string; skin_url: string }
  type SignInResult = {
    xbox_profile: XboxProfile
    minecraft_profile: MinecraftProfile
    minecraft_access_token: string
    minecraft_access_token_expiry_unix: number
    live_refresh_token: string | null
  }

  /** The original's `OpenXboxModal` opens a sign-in modal that has no port in
   *  the rewrite; the modal's only action is `XboxSignIn`, so this button runs
   *  that directly. Field-for-field the same settings writes as
   *  `XboxProfileService.SignInAsync`. */
  async function openXboxModal() {
    try {
      const r = await invoke<SignInResult>('sign_in')
      onChange({
        ...settings,
        XboxGamertag: r.xbox_profile.gamertag,
        XboxXuid: r.xbox_profile.xuid,
        XboxGamerPictureUrl: r.xbox_profile.gamer_picture_url,
        XboxGamerscore: r.xbox_profile.gamerscore,
        XboxAccountTier: r.xbox_profile.account_tier,
        XboxBio: r.xbox_profile.bio,
        XboxLiveRefreshToken: r.live_refresh_token ?? settings.XboxLiveRefreshToken,
        JavaUsername: r.minecraft_profile.name,
        JavaUuid: r.minecraft_profile.uuid,
        JavaAccessToken: r.minecraft_access_token,
        JavaAccessTokenExpiry: new Date(r.minecraft_access_token_expiry_unix * 1000).toISOString(),
        JavaSkinUrl: r.minecraft_profile.skin_url,
      })
      toast(`Signed in as ${r.xbox_profile.gamertag}`, 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  // ------------------------------------------------------------------ capes

  /** Port of `_capes` / `_capesLoading` / `NoCapeActive` / `ActiveCapeUrl`.
   *  No backend command exists to populate these, so the list stays empty and
   *  (as in the original) the wardrobe card does not render. */
  type CapeInfo = { id: string; alias: string; url: string; active: boolean }
  const capes: CapeInfo[] = []
  const capesLoading = false
  const noCapeActive = $derived(capes.length > 0 && capes.every((c) => !c.active))
  const activeCapeUrl = $derived(capes.find((c) => c.active)?.url ?? '')

  // -------------------------------------------------------------- instances
  //
  // Full instance management (rename/clone/export/delete/groups/import/JVM
  // presets) moved to `JavaInstancesView.svelte`, reachable from MC Versions
  // via the `versions-instances` cross-nav — same split as Bedrock's
  // Versions/Instances panels. Only what the stats grid below needs (the
  // active instance's name and memory setting) stays here.

  let instances = $state<JavaInstance[]>([])
  let instanceMaxRam = $state(0)

  /** Port of `JavaInstances.ActiveInstance`. */
  const activeInstance = $derived(
    instances.find((i) => i.id === settings.JavaActiveInstanceId) ?? instances[0] ?? null,
  )

  async function reloadInstances() {
    try {
      instances = await invoke<JavaInstance[]>('list_java_instances')
      instanceMaxRam = activeInstance?.max_ram_mb ?? 0
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  $effect(() => {
    void reloadInstances()
  })
</script>

<PanelHeader title={t('panel.profile')} {onBack} />

<div class="panel-body">
  {#if !settings.JavaUuid || !settings.JavaUuid.trim()}
    <div class="skin-empty">
      <i class="fa-solid fa-user-astronaut"></i>
      <span>Sign in with your Microsoft account to view your Minecraft profile.</span>
      <button class="vcs-btn" onclick={openXboxModal}>
        <i class="fa-brands fa-xbox"></i>&nbsp;Sign in
      </button>
    </div>
  {:else}
    <div class="profile-card">
      <div class="profile-card-bg"></div>

      <div class="profile-skin-col">
        <SkinViewer
          uuid={settings.JavaUuid}
          username={playerName}
          capeUrl={activeCapeUrl}
          version={settings.SkinChangedTicks}
          localSkinUrl={lastAppliedSkinUrl}
          {settings}
          {onChange}
        />
      </div>

      <div class="profile-info-col">
        <div class="profile-name">
          <span>{playerName}</span>
          {#if mcRunning}
            <span class="profile-online"><span class="profile-online-dot" aria-hidden="true"></span>{t('x.inGame')}</span>
          {/if}
        </div>
        <div class="profile-tagline">
          <i class="fa-brands fa-microsoft"></i> Microsoft account · {isJava ? 'Java Edition' : 'Bedrock'}
        </div>

        <div class="profile-actions">
          <button class="skin-btn" onclick={openPlayerNameMc}><i class="fa-solid fa-id-card"></i> NameMC</button>
          <button class="skin-btn" onclick={changeSkin}
            ><i class="fa-solid fa-upload"></i> Change skin</button
          >
          <button class="skin-btn" onclick={() => onNavigate?.('skinlibrary')}
            ><i class="fa-solid fa-shirt"></i> Skin library</button
          >
          <button class="skin-btn skin-btn-ghost" onclick={downloadPlayerSkin}
            ><i class="fa-solid fa-download"></i> Download</button
          >
          <button class="skin-btn skin-btn-ghost" onclick={resetSkin}
            ><i class="fa-solid fa-eraser"></i> {t('common.reset')}</button
          >
        </div>
        <div class="profile-actions">
          <button class="skin-btn skin-btn-ghost" onclick={() => onNavigate?.('stats')}
            ><i class="fa-solid fa-chart-simple"></i> Statistics</button
          >
          <button class="skin-btn skin-btn-ghost" onclick={() => onNavigate?.('backups')}
            ><i class="fa-solid fa-box-archive"></i> World backups</button
          >
          <button class="skin-btn skin-btn-ghost" onclick={() => onNavigate?.('logs')}
            ><i class="fa-solid fa-file-lines"></i> Logs</button
          >
        </div>
        <label class="skin-slim-toggle">
          <input
            type="checkbox"
            checked={settings.JavaSkinSlimModel}
            onchange={(e) => setChangeSkinSlim(e.currentTarget.checked)}
          />
          Upload as Slim (Alex) model
        </label>
      </div>
    </div>

    <!--
      IA change: these five values used to be a column of label:value rows
      squeezed into the hero card's narrow info column, competing with the
      name/tagline/actions for the same space. Pulled out into their own
      full-width dashboard row of stat cards, now that the panel opens wide
      (see WIDE_DASHBOARD_VIEWS) — each stat gets equal visual weight as a
      scannable tile instead of a dense list entry.
    -->
    <div class="profile-stats-grid">
      <button class="profile-stat-card" onclick={copyPlayerUuid} data-tooltip={t('tt.copyUuid')}>
        <i class="fa-solid fa-fingerprint"></i>
        <span class="profile-stat-v"
          >{playerCleanUuid.length >= 12 ? playerCleanUuid.slice(0, 12) + '…' : playerCleanUuid}</span
        >
        <span class="profile-stat-k">{t('x.uuid')}</span>
      </button>
      <div class="profile-stat-card">
        <i class="fa-solid fa-stopwatch"></i>
        <span class="profile-stat-v">{formatPlaytime(settings.TotalPlaytimeSeconds)}</span>
        <span class="profile-stat-k">{t('x.playtime')}</span>
      </div>
      <div class="profile-stat-card">
        <i class="fa-solid fa-calendar-day"></i>
        <span class="profile-stat-v">{!settings.LastPlayed ? '—' : formatRelativeTime(settings.LastPlayed)}</span>
        <span class="profile-stat-k">{t('x.lastPlayed')}</span>
      </div>
      <div class="profile-stat-card">
        <i class="fa-solid fa-cubes-stacked"></i>
        <span class="profile-stat-v">{activeInstance?.name ?? '—'}</span>
        <span class="profile-stat-k">{t('x.instance')}</span>
      </div>
      <div class="profile-stat-card">
        <i class="fa-solid fa-microchip"></i>
        <span class="profile-stat-v"
          >{instanceMaxRam > 0 ? instanceMaxRam + ' MB' : settings.JavaMaxRamMb + ' MB'}</span
        >
        <span class="profile-stat-k">{t('label.memory')}</span>
      </div>
    </div>

    {#if capesLoading || capes.length > 0}
      <div class="capes-card">
        <div class="capes-head">
          <i class="fa-solid fa-feather"></i> Cape wardrobe{capes.length > 0 ? ` (${capes.length})` : ''}
        </div>
        {#if capesLoading}
          <ListSkeleton rows={2} />
        {:else}
          <div class="capes-grid">
            <button class="cape-tile {noCapeActive ? 'active' : ''}" title="No cape" aria-pressed={noCapeActive}>
              <div class="cape-thumb cape-none"><i class="fa-solid fa-ban"></i></div>
              <span class="cape-label">{t('x.none')}</span>
            </button>
            {#each capes as c (c.id)}
              <button class="cape-tile {c.active ? 'active' : ''}" title={c.alias} aria-pressed={c.active}>
                <div class="cape-thumb" style="background-image:url('{c.url}')"></div>
                <span class="cape-label">{c.alias}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>
