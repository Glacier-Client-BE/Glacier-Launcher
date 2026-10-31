<script lang="ts">
  import { t } from './i18nStore.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { onDestroy } from 'svelte'
  import type { LauncherSettings } from './settings'

  let {
    settings,
    onChange,
    onOpenMcVersions,
  }: {
    settings: LauncherSettings
    onChange: (next: LauncherSettings) => void
    onOpenMcVersions?: () => void
  } = $props()

  const CLIENT_ICONS: Record<string, string> = {
    'Custom DLL': 'fa-solid fa-file-code',
  }

  const CLIENT_IMAGES: Record<string, string> = {
    'Flarial Client': '/images/clients/flarial.svg',
    'OderSo Client': '/images/clients/oderso.svg',
    'Latite Client': '/images/clients/latite.svg',
    'LeviLamina Client': '/images/clients/levi-lamina.svg',
    'LeviLamina': '/images/clients/levi-lamina.svg',
    Vanilla: '/images/mc-logo.svg',
  }

  /// Installed Bedrock build for the `.footer-mc-version` chip.
  type InstalledBedrockVersion = { installed: boolean; display_version: string; is_preview: boolean }
  let mcVersion = $state<InstalledBedrockVersion>({
    installed: false,
    display_version: '',
    is_preview: false,
  })

  async function loadMcVersion() {
    try {
      mcVersion = await invoke<InstalledBedrockVersion>('installed_bedrock_version')
    } catch {
      mcVersion = { installed: false, display_version: '', is_preview: false }
    }
  }
  loadMcVersion()

  /// Port of `GetFooterVersionLabel` (Home.razor.cs).
  function footerVersionLabel(): string {
    const client = settings?.SelectedClient
    if (client === 'Flarial Client') return 'Flarial Client'
    if (client === 'OderSo Client') return 'OderSo Client'
    if (client === 'LeviLamina Client') return 'LeviLamina Client'
    if (client === 'Vanilla') return 'Vanilla'
    if (client === 'Custom DLL') {
      const path = settings?.CustomDllPath ?? ''
      if (!path) return 'Custom DLL'
      const base = path.split(/[\\/]/).pop() ?? path
      return base.replace(/\.[^.]+$/, '')
    }
    return settings?.LastUsedVersion ? settings.LastUsedVersion : 'Latite Client'
  }

  const clientLabel = $derived(footerVersionLabel())
  const clientImage = $derived(CLIENT_IMAGES[clientLabel])
  const clientIconClass = $derived(CLIENT_ICONS[clientLabel])

  const xboxSignedIn = $derived(!!settings.XboxGamertag)
  const discordSignedIn = $derived(settings.DiscordLoggedIn)

  const effectiveProfile = $derived.by(() => {
    switch (settings.ProfileDisplayMode) {
      case 'xbox':
        return xboxSignedIn ? 'xbox' : 'none'
      case 'discord':
        return discordSignedIn ? 'discord' : 'none'
      default:
        if (xboxSignedIn) return 'xbox'
        if (discordSignedIn) return 'discord'
        return 'none'
    }
  })

  const footerName = $derived(
    effectiveProfile === 'xbox'
      ? settings.XboxGamertag
      : effectiveProfile === 'discord'
        ? settings.DiscordUsername
        : settings.Username || 'Not signed in',
  )
  const footerHandle = $derived(
    effectiveProfile === 'xbox' ? 'Xbox Live' : effectiveProfile === 'discord' ? 'Discord' : settings.UserHandle,
  )
  const footerAvatarClass = $derived(
    effectiveProfile === 'xbox' ? 'xbox-connected' : effectiveProfile === 'discord' ? 'discord-connected' : '',
  )

  const avatarSrc = $derived.by(() => {
    // Xbox/Discord take priority over the Minecraft skin head on both
    // editions - a Java install can still have Xbox connected (used for
    // Bedrock features / cross-play), and previously this branch short-
    // circuited straight to the skin head for any Java install with a
    // JavaUuid, so a connected Xbox account's gamerpic never showed there.
    if (effectiveProfile === 'xbox' && settings.XboxGamerPictureUrl) return settings.XboxGamerPictureUrl
    if (effectiveProfile === 'discord' && settings.DiscordAvatar) return settings.DiscordAvatar
    if (settings.Edition === 'java' && settings.JavaUuid) {
      const ticks = settings.SkinChangedTicks
      const uuid = settings.JavaUuid.replace(/-/g, '')
      return `https://mc-heads.net/avatar/${uuid}${ticks > 0 ? `?v=${ticks}` : ''}`
    }
    return '/favicon.svg'
  })

  const switchTooltip = $derived(
    settings.ProfileDisplayMode === 'xbox'
      ? 'Showing Xbox — click to switch to Discord'
      : settings.ProfileDisplayMode === 'discord'
        ? 'Showing Discord — click to switch to Auto'
        : 'Auto (prefers Xbox) — click to switch to Xbox',
  )

  function cycleProfileDisplay() {
    const next =
      settings.ProfileDisplayMode === 'auto'
        ? 'xbox'
        : settings.ProfileDisplayMode === 'xbox'
          ? 'discord'
          : 'auto'
    onChange({ ...settings, ProfileDisplayMode: next })
  }

  async function toggleDiscordRpc() {
    const enabling = !settings.DiscordRichPresence
    try {
      await invoke(enabling ? 'start_discord_presence' : 'stop_discord_presence')
    } catch {
      // best-effort — still flip the persisted preference
    }
    onChange({ ...settings, DiscordRichPresence: enabling })
  }

  // --- Phase 4: running Java instances badge -------------------------------
  // Polls `list_running_instances` (cheap — a `Vec` clone off a `Mutex`) on a
  // slow interval and additionally refreshes on `java-process-changed`, so
  // the badge updates immediately on launch/exit without needing a tight
  // poll loop (this targets a Celeron N5100 — see project perf notes).
  type RunningProcess = { instance_id: string; pid: number; started_at: number }
  let runningInstances = $state<RunningProcess[]>([])
  let showRunningDropdown = $state(false)
  let killingId = $state('')

  async function refreshRunning() {
    try {
      runningInstances = await invoke<RunningProcess[]>('list_running_instances')
    } catch {
      // best-effort
    }
  }

  function displayName(instanceId: string): string {
    return instanceId.startsWith('version:') ? instanceId.slice('version:'.length) : instanceId
  }

  async function stopInstance(instanceId: string) {
    killingId = instanceId
    try {
      await invoke('kill_instance', { instanceId })
    } catch {
      // best-effort — refreshRunning() below reconciles regardless
    } finally {
      killingId = ''
      await refreshRunning()
    }
  }

  let unlistenProcessChanged: (() => void) | undefined
  let runningPollHandle: ReturnType<typeof setInterval> | undefined
  let footerDestroyed = false

  refreshRunning()
  runningPollHandle = setInterval(refreshRunning, 10000)
  listen('java-process-changed', () => refreshRunning()).then((un) => {
    // If the footer was torn down before the listener resolved, release it
    // immediately instead of leaking it.
    if (footerDestroyed) un()
    else unlistenProcessChanged = un
  })

  onDestroy(() => {
    footerDestroyed = true
    if (runningPollHandle) clearInterval(runningPollHandle)
    unlistenProcessChanged?.()
  })
</script>

<div class="footer">
  <div class="footer-profile-area">
    {#if settings.Edition === 'java' && settings.JavaUuid && effectiveProfile === 'none'}
      <span class="avatar avatar-mchead" style="background-image:url('{avatarSrc}'), url('{avatarSrc}');"></span>
    {:else}
      <img class="avatar {footerAvatarClass}" src={avatarSrc} alt="" />
    {/if}

    <div class="user-info">
      <span class="username" title={footerName}>{footerName}</span>
      <span class="user-handle" title={footerHandle}>{footerHandle}</span>
    </div>

    <button
      class="profile-switch-btn"
      title={switchTooltip}
      aria-label={switchTooltip}
      onclick={(e) => {
        e.stopPropagation()
        cycleProfileDisplay()
      }}
    >
      <i class="fa-solid fa-arrows-rotate"></i>
    </button>

    <button
      class="profile-switch-btn"
      class:rpc-active={settings.DiscordRichPresence}
      aria-pressed={settings.DiscordRichPresence}
      title={settings.DiscordRichPresence ? 'Disable Discord RPC' : 'Enable Discord RPC'}
      aria-label={settings.DiscordRichPresence ? 'Disable Discord RPC' : 'Enable Discord RPC'}
      onclick={(e) => {
        e.stopPropagation()
        toggleDiscordRpc()
      }}
    >
      <i class="fa-brands fa-discord"></i>
    </button>

    {#if effectiveProfile === 'xbox' && xboxSignedIn}
      <div class="xbox-hover-card">
        <div class="xbox-hover-header">
          {#if settings.XboxGamerPictureUrl}
            <img class="xbox-hover-avatar" src={settings.XboxGamerPictureUrl} alt="" />
          {:else}
            <div class="xbox-hover-avatar xbox-hover-avatar-placeholder"><i class="fa-solid fa-user"></i></div>
          {/if}
          <div class="xbox-hover-identity">
            <span class="xbox-hover-gamertag">{settings.XboxGamertag}</span>
            {#if settings.XboxAccountTier}
              <span class="xbox-hover-tier-badge">{settings.XboxAccountTier}</span>
            {/if}
          </div>
        </div>
        {#if settings.XboxBio}
          <p class="xbox-hover-bio">{settings.XboxBio}</p>
        {/if}
        <div class="xbox-hover-stats">
          <div class="xbox-hover-stat">
            <i class="fa-solid fa-trophy xbox-icon-gold"></i>
            <span class="xbox-hover-stat-val">{settings.XboxGamerscore || '0'}</span>
            <span class="xbox-hover-stat-lbl">Gamerscore</span>
          </div>
          <div class="xbox-hover-stat">
            <i class="fa-solid fa-id-badge xbox-icon-green"></i>
            <span class="xbox-hover-stat-val xbox-hover-xuid">{settings.XboxXuid || '—'}</span>
            <span class="xbox-hover-stat-lbl">XUID</span>
          </div>
        </div>
        <div class="xbox-hover-footer">
          <i class="fa-solid fa-gamepad xbox-icon-green"></i>
          <span>Xbox Live</span>
          <span class="xbox-hover-dot">•</span>
          <span>{t('x.clickToManage')}</span>
        </div>
      </div>
    {/if}
  </div>

  <div class="footer-right">
    {#if runningInstances.length > 0}
      <div class="footer-running-badge-wrap">
        <button
          type="button"
          class="footer-client-label footer-running-badge"
          onclick={(e) => {
            e.stopPropagation()
            showRunningDropdown = !showRunningDropdown
          }}
          data-tooltip={t('tt.runningJavaInstances')}
        >
          <i class="fa-solid fa-circle-play"></i>
          <span>{runningInstances.length} running</span>
        </button>
        {#if showRunningDropdown}
          <div class="footer-running-dropdown" role="menu">
            {#each runningInstances as p (p.instance_id)}
              <div class="footer-running-row">
                <span class="footer-running-name" title={p.instance_id}>{displayName(p.instance_id)}</span>
                <span class="footer-running-pid">PID {p.pid}</span>
                <button
                  type="button"
                  class="btn-sm footer-running-stop"
                  disabled={killingId === p.instance_id}
                  onclick={(e) => {
                    e.stopPropagation()
                    stopInstance(p.instance_id)
                  }}
                >
                  {#if killingId === p.instance_id}
                    <i class="fa-solid fa-spinner fa-spin"></i>
                  {:else}
                    <i class="fa-solid fa-stop"></i>
                  {/if}
                  Stop
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
    {#if mcVersion.installed}
      <div
        class="footer-mc-version"
        role="button"
        tabindex="0"
        onclick={() => onOpenMcVersions?.()}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onOpenMcVersions?.() }}
        onmousedown={(e) => e.stopPropagation()}
        data-tooltip={mcVersion.is_preview
          ? 'Minecraft Preview — click to manage versions'
          : 'Minecraft Bedrock — click to manage versions'}
      >
        <img class="chip-img" src="images/mc-logo.svg" alt="" />
        <span class="footer-mc-text" title={`${mcVersion.is_preview ? 'Preview ' : ''}${mcVersion.display_version}`}>{mcVersion.is_preview ? 'Preview ' : ''}{mcVersion.display_version}</span>
      </div>
    {:else}
      <div
        class="footer-mc-version footer-mc-version-missing"
        role="button"
        tabindex="0"
        onclick={() => onOpenMcVersions?.()}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onOpenMcVersions?.() }}
        onmousedown={(e) => e.stopPropagation()}
        data-tooltip={t('tt.minecraftBedrockNotDetectedClickInstall')}
      >
        <img class="chip-img" src="images/mc-logo.svg" alt="" />
        <span class="footer-mc-text">{t('x.notInstalled')}</span>
      </div>
    {/if}
    <span class="footer-client-label">
      {#if clientImage}
        <img class="chip-img" src={clientImage} alt="" />
      {:else if clientIconClass}
        <i class="chip-icon {clientIconClass}"></i>
      {:else}
        <i class="fa-solid fa-clock"></i>
      {/if}
      <span class="footer-client-text" title={clientLabel}>{clientLabel}</span>
    </span>
  </div>
</div>

<style>
  /* Footer rhythm: outer gutter and band tokens, compact 32px row. */
  .footer {
    gap: var(--panel-band);
    padding: 0 var(--panel-gutter) var(--panel-gutter);
    align-items: center;
  }
  .footer-right {
    gap: var(--panel-band);
    min-width: 0;
  }
  .user-info {
    min-width: 0;
    flex: 1;
  }
  .username,
  .user-handle,
  .footer-mc-text,
  .footer-client-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 180px;
  }
  .footer-profile-area {
    padding: var(--row-pad-y) var(--row-pad-x) var(--row-pad-y) 6px;
    box-shadow: none;
    transition: background-color var(--t);
  }
  .footer-profile-area:hover {
    box-shadow: none;
    background: var(--bg-item-hover);
  }
  .profile-switch-btn {
    transition: background-color var(--t), color var(--t);
    margin-left: 0;
  }
  .profile-switch-btn:hover {
    background: var(--bg-item-hover);
    border-color: var(--border);
    color: var(--text);
  }
  .profile-switch-btn.rpc-active {
    color: var(--accent);
  }
  .profile-switch-btn:disabled,
  .footer-running-stop:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .footer-mc-version,
  .footer-client-label {
    padding: 4px var(--row-pad-x);
    transition: background-color var(--t), border-color var(--t);
  }
  .footer-mc-version:hover {
    background: var(--bg-item-hover);
    border-color: var(--border);
  }
  .footer-mc-version:focus-visible {
    outline: 2px solid var(--accent-hover);
    outline-offset: 2px;
  }
  .footer-running-row:hover {
    background: var(--bg-item-hover);
  }
  .avatar.xbox-connected {
    box-shadow: none;
  }
  .chip-img {
    width: 14px;
    height: 14px;
    object-fit: contain;
    flex-shrink: 0;
  }
  .xbox-icon-gold {
    color: var(--orange);
  }
  .xbox-icon-green {
    color: var(--green);
  }
</style>
