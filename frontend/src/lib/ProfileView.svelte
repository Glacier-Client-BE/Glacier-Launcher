<script lang="ts">
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
  import { open, save } from '@tauri-apps/plugin-dialog'
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import SkinViewer from './components/SkinViewer.svelte'
  import type { LauncherSettings, JavaInstance, JvmPreset } from './settings'

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

  /** Port of `OpenPlayerNameMc`. */
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
      onToast?.('Skin applied.', 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  async function resetSkin() {
    try {
      await invoke('reset_skin', {})
      onToast?.('Skin reset to the default.', 'success')
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

  let instances = $state<JavaInstance[]>([])
  let instanceMaxRam = $state(0)
  let renamingInstanceId = $state('')
  let renameInstanceValue = $state('')
  let confirmDeleteInstanceId = $state('')

  /** Port of `JavaInstances.ActiveInstance`. */
  const activeInstance = $derived(
    instances.find((i) => i.id === settings.JavaActiveInstanceId) ?? instances[0] ?? null,
  )

  async function reloadInstances() {
    try {
      instances = await invoke<JavaInstance[]>('list_java_instances')
      loadInstanceRam()
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  /** Port of `LoadInstanceRam`. */
  function loadInstanceRam() {
    instanceMaxRam = activeInstance?.max_ram_mb ?? 0
  }

  $effect(() => {
    void reloadInstances()
  })

  /** Port of `OnInstanceMaxRamChange`. */
  async function onInstanceMaxRamChange(e: Event) {
    let v = parseInt((e.currentTarget as HTMLInputElement).value, 10)
    if (Number.isNaN(v)) v = 0
    if (v !== 0) v = Math.min(16384, Math.max(512, v))
    instanceMaxRam = v
    if (activeInstance) {
      try {
        await invoke('set_java_instance_ram', { id: activeInstance.id, minMb: 0, maxMb: v })
      } catch (err) {
        toast(String(err), 'error')
        return
      }
    }
    toast(v > 0 ? `This instance: ${v} MB` : 'Using global memory', 'info')
    await reloadInstances()
  }

  /** Port of `SwitchInstance`. The rewrite persists the active instance on
   *  `LauncherSettings.JavaActiveInstanceId` (as App.svelte already reads it). */
  function switchInstance(id: string) {
    onChange({ ...settings, JavaActiveInstanceId: id })
    toast('Instance switched.', 'info')
  }

  // ------------------------------------------------ templates / JVM presets

  /** Built-in JVM-argument templates ("Vanilla", "Balanced", "Performance",
   *  "Low-End") — feature 2 (offered at creation) and feature 3 (per-instance
   *  preset dropdown). JVM flags only, no mod/API downloads. */
  let jvmPresets = $state<JvmPreset[]>([])
  let newInstanceTemplate = $state('vanilla')

  async function loadJvmPresets() {
    try {
      jvmPresets = await invoke<JvmPreset[]>('list_jvm_presets')
    } catch {
      jvmPresets = []
    }
  }
  loadJvmPresets()

  /** Port of `NewJavaInstance`, extended to apply the selected template. */
  async function newJavaInstance() {
    try {
      const instance = await invoke<JavaInstance>('create_java_instance_from_template', {
        name: 'New Instance',
        versionId: '',
        template: newInstanceTemplate || null,
      })
      await reloadInstances()
      onChange({ ...settings, JavaActiveInstanceId: instance.id })
      toast('Instance created.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  /** Feature 3: change an instance's JVM-args preset from its row dropdown. */
  async function onInstancePresetChange(id: string, preset: string) {
    try {
      await invoke('set_instance_jvm_preset', { id, preset })
      await reloadInstances()
      toast('JVM args preset applied.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  // ---------------------------------------------------------------- groups

  let editingGroupId = $state('')
  let editingGroupValue = $state('')

  function beginEditGroup(inst: JavaInstance) {
    editingGroupId = inst.id
    editingGroupValue = inst.group ?? ''
  }

  async function commitGroup() {
    const id = editingGroupId
    editingGroupId = ''
    try {
      await invoke('set_instance_group', { id, group: editingGroupValue.trim() || null })
      await reloadInstances()
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  /** Groups instances by `group` (falling back to "Ungrouped"), preserving
   *  each group's first-seen order. No drag-and-drop — just collapsible
   *  `<details>` sections. */
  const groupedInstances = $derived.by(() => {
    const groups = new Map<string, JavaInstance[]>()
    for (const inst of instances) {
      const key = inst.group?.trim() || 'Ungrouped'
      if (!groups.has(key)) groups.set(key, [])
      groups.get(key)!.push(inst)
    }
    return Array.from(groups.entries())
  })

  // ---------------------------------------------------------------- export

  let exportingId = $state('')

  /** Feature 6: export an instance's full directory tree as a zip via the
   *  same `@tauri-apps/plugin-dialog` picker used for import. `zip` is
   *  already a workspace dependency in `gl-java`, so this is a real zip, not
   *  a folder copy. */
  async function exportInstance(inst: JavaInstance) {
    if (exportingId) return
    const dest = await save({
      title: `Export '${inst.name}'`,
      defaultPath: `${inst.name}.zip`,
      filters: [{ name: 'Zip archive', extensions: ['zip'] }],
    })
    if (!dest) return
    exportingId = inst.id
    try {
      await invoke('export_java_instance', { id: inst.id, destPath: dest })
      toast('Instance exported.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      exportingId = ''
    }
  }

  /** Port of `BeginRenameInstance`. */
  function beginRenameInstance(id: string, current: string) {
    renamingInstanceId = id
    renameInstanceValue = current
  }

  /** Port of `OnRenameInstanceKey`. */
  function onRenameInstanceKey(e: KeyboardEvent) {
    if (e.key === 'Enter') void commitRenameInstance()
  }

  /** Port of `CommitRenameInstance`. */
  async function commitRenameInstance() {
    if (renamingInstanceId.trim() && renameInstanceValue.trim()) {
      try {
        await invoke('rename_java_instance', { id: renamingInstanceId, newName: renameInstanceValue })
      } catch (e) {
        toast(String(e), 'error')
      }
    }
    renamingInstanceId = ''
    toast('Instance renamed.', 'success')
    await reloadInstances()
  }

  /** Port of `ConfirmDeleteInstance` / `CancelDeleteInstance`. */
  function confirmDeleteInstance(id: string) {
    confirmDeleteInstanceId = id
  }
  function cancelDeleteInstance() {
    confirmDeleteInstanceId = ''
  }

  /** Port of `DeleteJavaInstance`. */
  async function deleteJavaInstance(id: string) {
    try {
      const ok = await invoke<boolean>('delete_java_instance', { id })
      if (ok) {
        await reloadInstances()
        toast('Instance deleted.', 'info')
      } else {
        toast("Can't delete the last instance.", 'error')
      }
    } catch (e) {
      toast(String(e), 'error')
    }
    confirmDeleteInstanceId = ''
  }

  /** Port of `CloneInstance` - copies the instance's metadata and full
   * directory tree (mods, saves, config, etc.) into a new "<name> Copy". */
  async function cloneJavaInstance(id: string) {
    try {
      await invoke<JavaInstance>('duplicate_java_instance', { id })
      await reloadInstances()
      toast('Instance cloned.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  let importBusy = $state(false)

  /**
   * Port of `ImportInstanceZip`'s non-`.mrpack` branch only
   * (`JavaInstanceService.ImportModpackAsync`): extracts a CurseForge/
   * Modrinth-`overrides/`-shaped or MultiMC-shaped zip directly.
   *
   * A `.mrpack` file needs a different path (`InstallMrpackFileAsync`):
   * parse `modrinth.index.json`, resolve and download each listed mod from
   * Modrinth's API, *then* extract `overrides/`. That resolver doesn't
   * exist in this build yet, so `.mrpack` is intentionally not offered here
   * — accepting it and only extracting overrides would silently produce an
   * instance with no mods, which is worse than not offering it at all.
   */
  async function importInstanceZip() {
    if (importBusy) return
    const path = await open({
      multiple: false,
      title: 'Import instance / modpack',
      filters: [{ name: 'Modpacks & instances', extensions: ['zip'] }],
    })
    if (typeof path !== 'string') return
    importBusy = true
    try {
      const instance = await invoke<JavaInstance>('import_java_instance_zip', { zipPath: path })
      onChange({ ...settings, JavaActiveInstanceId: instance.id })
      await reloadInstances()
      toast('Instance imported.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      importBusy = false
    }
  }

  /** Port of `ImportOfficialMinecraft`. */
  async function importOfficialMinecraft() {
    if (importBusy) return
    importBusy = true
    try {
      const instance = await invoke<JavaInstance>('import_official_minecraft')
      onChange({ ...settings, JavaActiveInstanceId: instance.id })
      await reloadInstances()
      toast('Imported .minecraft into a new instance.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      importBusy = false
    }
  }
</script>

<PanelHeader title="Profile" {onBack} />

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
            <span class="profile-online"><span class="profile-online-dot"></span>In game</span>
          {/if}
        </div>
        <div class="profile-tagline">
          <i class="fa-brands fa-microsoft"></i> Microsoft account · {isJava ? 'Java Edition' : 'Bedrock'}
        </div>

        <div class="pstat-list">
          <button class="pstat pstat-click" onclick={copyPlayerUuid} data-tooltip="Copy UUID">
            <i class="fa-solid fa-fingerprint"></i>
            <span class="pstat-k">UUID</span>
            <span class="pstat-v"
              >{playerCleanUuid.length >= 12 ? playerCleanUuid.slice(0, 12) + '…' : playerCleanUuid}</span
            >
          </button>
          <div class="pstat">
            <i class="fa-solid fa-stopwatch"></i>
            <span class="pstat-k">Playtime</span>
            <span class="pstat-v">{formatPlaytime(settings.TotalPlaytimeSeconds)}</span>
          </div>
          <div class="pstat">
            <i class="fa-solid fa-calendar-day"></i>
            <span class="pstat-k">Last played</span>
            <span class="pstat-v"
              >{!settings.LastPlayed ? '—' : formatRelativeTime(settings.LastPlayed)}</span
            >
          </div>
          <div class="pstat">
            <i class="fa-solid fa-cubes-stacked"></i>
            <span class="pstat-k">Instance</span>
            <span class="pstat-v">{activeInstance?.name ?? ''}</span>
          </div>
          <div class="pstat">
            <i class="fa-solid fa-microchip"></i>
            <span class="pstat-k">Memory</span>
            <span class="pstat-v"
              >{instanceMaxRam > 0 ? instanceMaxRam + ' MB' : settings.JavaMaxRamMb + ' MB'}</span
            >
          </div>
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
            ><i class="fa-solid fa-eraser"></i> Reset</button
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

    {#if capesLoading || capes.length > 0}
      <div class="capes-card">
        <div class="capes-head">
          <i class="fa-solid fa-feather"></i> Cape wardrobe{capes.length > 0 ? ` (${capes.length})` : ''}
        </div>
        {#if capesLoading}
          <div class="versions-loading"><span class="spinner"></span><span>Loading capes…</span></div>
        {:else}
          <div class="capes-grid">
            <button class="cape-tile {noCapeActive ? 'active' : ''}" title="No cape">
              <div class="cape-thumb cape-none"><i class="fa-solid fa-ban"></i></div>
              <span class="cape-label">None</span>
            </button>
            {#each capes as c (c.id)}
              <button class="cape-tile {c.active ? 'active' : ''}" title={c.alias}>
                <div class="cape-thumb" style="background-image:url('{c.url}')"></div>
                <span class="cape-label">{c.alias}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  {/if}

  <div class="instance-card">
    <div class="instance-card-head">
      <i class="fa-solid fa-microchip"></i> Memory · {activeInstance?.name ?? ''}
    </div>
    <div class="instance-ram-row">
      <input type="number" min="0" max="16384" step="512" value={instanceMaxRam} onchange={onInstanceMaxRamChange} />
      <span class="instance-ram-unit">MB max</span>
    </div>
    <small class="instance-hint">0 = use the global setting ({settings.JavaMaxRamMb} MB).</small>
  </div>

  <div class="instance-card">
    <div class="instance-card-head">
      <span><i class="fa-solid fa-cubes-stacked"></i> Instances</span>
      <div style="flex:1"></div>
      <select
        class="setting-select"
        bind:value={newInstanceTemplate}
        data-tooltip="JVM-args template applied to the new instance"
        aria-label="New instance template"
      >
        {#each jvmPresets as p (p.id)}
          <option value={p.id}>{p.label}</option>
        {/each}
      </select>
      <button class="btn-sm" onclick={newJavaInstance}><i class="fa-solid fa-plus"></i> New</button>
      <button
        class="btn-sm"
        onclick={() => onNavigate?.('instance-wizard')}
        data-tooltip="Guided setup: version, name, RAM"
        ><i class="fa-solid fa-wand-magic-sparkles"></i> Wizard</button
      >
      <button
        class="btn-sm"
        onclick={importInstanceZip}
        disabled={importBusy}
        data-tooltip="Import a .zip modpack/instance"
        ><i class="fa-solid fa-file-import"></i> Import</button
      >
      <button
        class="btn-sm"
        onclick={importOfficialMinecraft}
        disabled={importBusy}
        data-tooltip="Import your official .minecraft" aria-label="Import official .minecraft"
        ><i class="fa-brands fa-microsoft"></i></button
      >
      <button
        class="btn-sm"
        onclick={() => onNavigate?.('import-launcher')}
        data-tooltip="Import instances from MultiMC, Prism, CurseForge, ATLauncher, or Modrinth App"
        aria-label="Import from another launcher"
        ><i class="fa-solid fa-right-to-bracket"></i> From another launcher</button
      >
      <button
        class="btn-sm"
        onclick={() => onNavigate?.('sync-packs')}
        data-tooltip="Keep resourcepacks, options, or mod lists consistent across instances"
        ><i class="fa-solid fa-arrows-rotate"></i> Sync</button
      >
    </div>
    {#each groupedInstances as [groupName, groupInstances] (groupName)}
      <details class="instance-group" open>
        <summary class="instance-group-summary">{groupName} ({groupInstances.length})</summary>
        {#each groupInstances as inst (inst.id)}
          {@const isActive = inst.id === activeInstance?.id}
          <div class="instance-row {isActive ? 'active' : ''}">
            {#if renamingInstanceId === inst.id}
              <input
                class="setting-input instance-rename"
                value={renameInstanceValue}
                oninput={(e) => (renameInstanceValue = e.currentTarget.value)}
                onkeydown={onRenameInstanceKey}
              />
              <button class="vcs-btn" onclick={commitRenameInstance} aria-label="Confirm rename"><i class="fa-solid fa-check"></i></button>
            {:else if editingGroupId === inst.id}
              <span class="instance-hint" style="margin-right:6px;">{inst.name} · group:</span>
              <input
                class="setting-input instance-rename"
                placeholder="Ungrouped"
                value={editingGroupValue}
                oninput={(e) => (editingGroupValue = e.currentTarget.value)}
                onkeydown={(e) => { e.stopPropagation(); if (e.key === 'Enter') void commitGroup() }}
              />
              <button class="vcs-btn" onclick={commitGroup} aria-label="Confirm group"><i class="fa-solid fa-check"></i></button>
            {:else}
              <button class="instance-name-btn" onclick={() => switchInstance(inst.id)}>
                {#if isActive}<i class="fa-solid fa-circle-check"></i>{:else}<i class="fa-regular fa-circle"></i>{/if}
                <span>{inst.name}</span>
              </button>
              <select
                class="setting-select instance-version-select"
                value={inst.jvm_args_preset ?? ''}
                onchange={(e) => onInstancePresetChange(inst.id, e.currentTarget.value)}
                onclick={(e) => e.stopPropagation()}
                data-tooltip="JVM-args preset for this instance"
                aria-label="JVM-args preset for this instance"
              >
                <option value="" disabled>{inst.jvm_args_preset ? '' : 'Custom'}</option>
                {#each jvmPresets as p (p.id)}
                  <option value={p.id}>{p.label}</option>
                {/each}
              </select>
              <button
                class="instance-icon-btn"
                onclick={() => { switchInstance(inst.id); onNavigate?.('instance-detail') }}
                data-tooltip="Open details" aria-label="Open details"><i class="fa-solid fa-arrow-up-right-from-square"></i></button
              >
              <button
                class="instance-icon-btn"
                onclick={() => beginRenameInstance(inst.id, inst.name)}
                data-tooltip="Rename" aria-label="Rename"><i class="fa-solid fa-pen"></i></button
              >
              <button
                class="instance-icon-btn"
                onclick={() => beginEditGroup(inst)}
                data-tooltip="Set group/folder" aria-label="Set group"><i class="fa-solid fa-folder"></i></button
              >
              <button
                class="instance-icon-btn"
                onclick={() => cloneJavaInstance(inst.id)}
                data-tooltip="Clone" aria-label="Clone instance"
                ><i class="fa-solid fa-copy"></i></button
              >
              <button
                class="instance-icon-btn"
                onclick={() => exportInstance(inst)}
                disabled={exportingId === inst.id}
                data-tooltip="Export as .zip" aria-label="Export instance"
                ><i class="fa-solid fa-file-export"></i></button
              >
              {#if confirmDeleteInstanceId === inst.id}
                <button
                  class="instance-icon-btn instance-del"
                  onclick={() => deleteJavaInstance(inst.id)}
                  data-tooltip="Confirm delete" aria-label="Confirm delete"><i class="fa-solid fa-check"></i></button
                >
                <button class="instance-icon-btn" onclick={cancelDeleteInstance} aria-label="Cancel delete"><i class="fa-solid fa-xmark"></i></button>
              {:else}
                <button class="instance-icon-btn" onclick={() => confirmDeleteInstance(inst.id)} data-tooltip="Delete" aria-label="Delete"
                  ><i class="fa-solid fa-trash"></i></button
                >
              {/if}
            {/if}
          </div>
        {/each}
      </details>
    {/each}
  </div>
</div>
