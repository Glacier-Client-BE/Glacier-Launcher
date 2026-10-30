<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * Literal port of the `servers` panel — Home.razor:3715-3828, with behaviour
   * from Pages/Home.Panels.cs:17-207 and Services/ServerPingService.cs.
   *
   * Transcribed element-for-element: same DOM order, class names, tooltips and
   * strings. The saved list is persisted through `servers.json`
   * (src-tauri/src/commands/servers.rs) rather than
   * `LauncherSettings.SavedServers`, but every field maps 1:1.
   *
   * "Launch & connect" is wired to `launch_bedrock_server`
   * (src-tauri/src/commands/launch.rs), the port of `LaunchServerAsync`'s
   * `minecraft://?addExternalServer=` deep link: it launches Bedrock (with
   * whatever client DLL Settings → Selected client resolves to, same as the
   * home Launch button) and then fires the deep link to join `host:port`.
   */
  // TODO(feature 31 — friends list / online-friends-per-server): skipped.
  // crates/gl-auth has no friends-list scope/endpoint on its Xbox Live token
  // (only auth/profile/skin scopes); implementing this would require
  // requesting a new Xbox Live relationship-service scope and consent flow,
  // which is out of scope for this pass.
  import { invoke } from '@tauri-apps/api/core'
  import SubTabs from './components/SubTabs.svelte'
  import IconBtn from './components/IconBtn.svelte'
  import type { LauncherSettings } from './settings'

  let {
    onBack,
    onNavigate,
    onToast,
  }: {
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  let launchingKey = $state<string | null>(null)

  /** Mirrors App.svelte's `handleLaunch` client-DLL resolution. */
  async function resolveDllPath(): Promise<string | null> {
    const settings = await invoke<LauncherSettings>('get_settings')
    if (settings.SelectedClient === 'Flarial Client') {
      return await invoke<string>('flarial_client_path')
    } else if (settings.SelectedClient === 'OderSo Client') {
      return await invoke<string>('oderso_client_path')
    } else if (settings.SelectedClient === 'LeviLamina Client') {
      return await invoke<string>('levilamina_client_path')
    } else if (settings.SelectedClient === 'Custom DLL') {
      return settings.CustomDllPath || null
    }
    return null
  }

  async function launchAndConnect(s: SavedServer) {
    const key = serverKey(s)
    if (launchingKey) return
    launchingKey = key
    try {
      const dllPath = await resolveDllPath()
      await invoke('launch_bedrock_server', {
        dllPath,
        serverName: s.name,
        host: s.address,
        port: s.port,
      })
      onToast?.(t('toast.launchingIntoName', { name: s.name }), 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      launchingKey = null
    }
  }

  /** Port of the `SavedServer` model. */
  type SavedServer = {
    id: string
    name: string
    address: string
    port: number
    icon: string
    icon_color: string
    favorite: boolean
    tags: string[]
  }

  /** LAN "open to world" broadcast, discovered via UDP multicast scan. */
  type LanGame = {
    motd: string
    port: number
    host: string
  }

  /** Port of `ServerStatus` (Services/ServerPingService.cs:11). */
  type ServerStatus = {
    online: boolean
    players_online: number
    players_max: number
    motd: string
    version: string
    latency_ms: number
  }

  // ── Port of Home.Panels.cs:17-22 (modal state). ─────────────────────────
  let serverModalOpen = $state(false)
  let editingServer = $state<SavedServer | null>(null)
  let serverInputName = $state('')
  let serverInputAddress = $state('')
  let serverInputPort = $state(19132)
  let serverInputTags = $state('')
  let serverInputError = $state('')

  let savedServers = $state<SavedServer[]>([])
  let _confirmDeleteServerId = $state('')
  let refreshing = $state(false)

  // ── LAN world browser (feature 34) ──────────────────────────────────────
  let lanScanning = $state(false)
  let lanGames = $state<LanGame[]>([])
  let lanError = $state('')

  async function scanLanGames() {
    lanScanning = true
    lanError = ''
    try {
      lanGames = await invoke<LanGame[]>('scan_lan_games', { scanMs: 5000 })
    } catch (e) {
      lanGames = []
      lanError = String(e)
    } finally {
      lanScanning = false
    }
  }

  /** Port of `_serverSuggestions` (Home.Panels.cs:24-32) — verbatim, except
   *  icons now point at the real server-brand images shipped in
   *  /images/servers/ instead of generic Font Awesome glyphs. */
  const _serverSuggestions: SavedServer[] = [
    { id: '', name: 'Hive', address: 'geo.hivebedrock.network', port: 19132, icon: '/images/servers/hive.png', icon_color: '#f5a623', favorite: false, tags: [] },
    { id: '', name: 'CubeCraft', address: 'play.cubecraft.net', port: 19132, icon: '/images/servers/cubecraft.png', icon_color: '#4a90e2', favorite: false, tags: [] },
    { id: '', name: 'Mineplex', address: 'mco.mineplex.com', port: 19132, icon: '/images/servers/mineplex.png', icon_color: '#e94e4e', favorite: false, tags: [] },
    { id: '', name: 'Lifeboat', address: 'play.lbsg.net', port: 19132, icon: '/images/servers/lifeboat.png', icon_color: '#00bcd4', favorite: false, tags: [] },
    { id: '', name: 'Galaxite', address: 'play.galaxite.net', port: 19132, icon: '/images/servers/galaxite.png', icon_color: '#9b51e0', favorite: false, tags: [] },
    { id: '', name: 'NetherGames', address: 'play.nethergames.org', port: 19132, icon: '/images/servers/nethergames.png', icon_color: '#2ecc71', favorite: false, tags: [] },
    { id: '', name: 'Zeqa', address: 'zeqa.net', port: 19132, icon: '/images/servers/zeqa.png', icon_color: '#c94ee0', favorite: false, tags: [] },
    { id: '', name: 'Mineville', address: 'play.mineville.club', port: 19132, icon: '/images/servers/mineville.png', icon_color: '#3ddc97', favorite: false, tags: [] },
    { id: '', name: 'Ectary', address: 'play.ectary.net', port: 19132, icon: '/images/servers/ectary.png', icon_color: '#6c7ae0', favorite: false, tags: [] },
  ]

  // Live status keyed by "address:port". A present-but-null value means a ping
  // is in flight; an absent key means we haven't started one yet.
  // (Comment and semantics carried over from Home.Panels.cs:33-35.)
  let _serverStatus = $state<Map<string, ServerStatus | null>>(new Map())

  /** Port of `ServerKey` (Home.Panels.cs:45). */
  function serverKey(s: SavedServer) {
    return `${s.address}:${s.port}`
  }

  /** `:N0` group separators, as in `RenderServerStatus`. */
  function n0(value: number) {
    return (value ?? 0).toLocaleString('en-US')
  }

  /** Port of `OpenServers` (Home.Panels.cs:36-44) — load, then ping. */
  async function openServers() {
    try {
      savedServers = await invoke<SavedServer[]>('list_saved_servers')
    } catch {
      savedServers = []
    }
    await pingAllServers()
  }

  /** Manual on-demand re-ping of every listed server (feature 33). */
  async function refreshServerStatuses() {
    if (refreshing) return
    refreshing = true
    try {
      await pingAllServers()
    } finally {
      refreshing = false
    }
  }

  /** Port of `PingAllServersAsync` (Home.Panels.cs:71-96). */
  async function pingAllServers() {
    const targets: SavedServer[] = []
    const seen = new Set<string>()
    for (const s of [...savedServers, ..._serverSuggestions]) {
      const key = serverKey(s)
      if (seen.has(key)) continue
      seen.add(key)
      targets.push(s)
    }

    const pending = new Map(_serverStatus)
    for (const s of targets) pending.set(serverKey(s), null) // mark "pinging"
    _serverStatus = pending

    await Promise.all(
      targets.map(async (s) => {
        try {
          const status = await invoke<ServerStatus>('ping_server', { host: s.address, port: s.port })
          const next = new Map(_serverStatus)
          next.set(serverKey(s), status)
          _serverStatus = next
        } catch {
          /* every failure path resolves to Offline server-side */
        }
      }),
    )
  }

  /** Port of `OpenAddServerModal` (Home.Panels.cs:98-107). */
  function openAddServerModal() {
    editingServer = null
    serverInputName = ''
    serverInputAddress = ''
    serverInputPort = 19132
    serverInputTags = ''
    serverInputError = ''
    serverModalOpen = true
  }

  /** Port of `OpenEditServerModal` (Home.Panels.cs:109-118). */
  function openEditServerModal(s: SavedServer) {
    editingServer = s
    serverInputName = s.name
    serverInputAddress = s.address
    serverInputPort = s.port
    serverInputTags = (s.tags ?? []).join(', ')
    serverInputError = ''
    serverModalOpen = true
  }

  /** Star toggle — flips `favorite` and persists via `update_saved_server`. */
  async function toggleFavorite(s: SavedServer) {
    try {
      await invoke('update_saved_server', { server: { ...s, favorite: !s.favorite } })
    } catch (e) {
      onToast?.(String(e), 'error')
      return
    }
    await openServers()
  }

  function parseTags(input: string): string[] {
    return input
      .split(',')
      .map((t) => t.trim())
      .filter((t) => t.length > 0)
  }

  /** Port of `CloseServerModal` (Home.Panels.cs:120-125). */
  function closeServerModal() {
    serverModalOpen = false
    editingServer = null
  }

  /** Port of `SaveServerModal` (Home.Panels.cs:127-160). */
  async function saveServerModal() {
    if (!serverInputAddress || !serverInputAddress.trim()) {
      serverInputError = 'Address is required.'
      return
    }
    const port = Number(serverInputPort)
    if (!Number.isFinite(port) || port <= 0 || port > 65535) {
      serverInputError = 'Port must be between 1 and 65535.'
      return
    }

    const name = !serverInputName || !serverInputName.trim() ? serverInputAddress : serverInputName.trim()
    const wasAdding = editingServer == null

    try {
      const tags = parseTags(serverInputTags)
      if (editingServer == null) {
        await invoke('add_saved_server', {
          name,
          address: serverInputAddress.trim(),
          port,
          icon: '',
          iconColor: '',
          favorite: false,
          tags,
        })
      } else {
        await invoke('update_saved_server', {
          server: { ...editingServer, name, address: serverInputAddress.trim(), port, tags },
        })
      }
    } catch (e) {
      serverInputError = String(e)
      return
    }

    closeServerModal()
    await openServers()
    onToast?.(wasAdding ? 'Server saved' : 'Server updated', 'success')
  }

  /** "More" menu per saved-server card — Edit/Copy are secondary to
   *  Launch/Favorite/Delete, same overflow-avoidance pattern used on the
   *  Instances cards. */
  let openServerMenuId = $state('')
  function toggleServerMenu(id: string) {
    openServerMenuId = openServerMenuId === id ? '' : id
  }
  function closeServerMenu() {
    openServerMenuId = ''
  }

  /** Inline two-step confirm, matching BedrockBackupsView/InstancesView. */
  function confirmDeleteServer(id: string) {
    _confirmDeleteServerId = id
  }
  function cancelDeleteServer() {
    _confirmDeleteServerId = ''
  }

  /** Port of `DeleteServer` (Home.Panels.cs:162-167). */
  async function deleteServer(s: SavedServer) {
    try {
      await invoke('remove_saved_server', { id: s.id })
    } catch (e) {
      onToast?.(String(e), 'error')
      return
    }
    _confirmDeleteServerId = ''
    await openServers()
  }

  /** Port of `SaveSuggestedServer` (Home.Panels.cs:169-188). */
  async function saveSuggestedServer(s: SavedServer) {
    if (
      savedServers.some(
        (x) => x.address.toLowerCase() === s.address.toLowerCase() && x.port === s.port,
      )
    ) {
      onToast?.(s.name + ' is already saved', 'info')
      return
    }
    try {
      await invoke('add_saved_server', {
        name: s.name,
        address: s.address,
        port: s.port,
        icon: s.icon,
        iconColor: s.icon_color,
        favorite: false,
        tags: [],
      })
    } catch (e) {
      onToast?.(String(e), 'error')
      return
    }
    onToast?.(s.name + ' saved', 'success')
    await openServers()
  }

  /** Port of `IsSuggestionUnsaved` (Home.Panels.cs:190-192). */
  function isSuggestionUnsaved(s: SavedServer) {
    return !savedServers.some(
      (x) => x.address.toLowerCase() === s.address.toLowerCase() && x.port === s.port,
    )
  }

  /** True when `s.icon` is a real brand image rather than a Font Awesome class. */
  function isServerIconImage(s: SavedServer) {
    return !!s.icon && s.icon.startsWith('/images/')
  }

  /** Port of `GetServerIconClass` (Home.Panels.cs:194-195). */
  function getServerIconClass(s: SavedServer) {
    return !s.icon || !s.icon.trim() ? 'fa-solid fa-server' : s.icon
  }

  /** Port of `GetServerIconStyle` (Home.Panels.cs:197-201). Image icons render on
   *  a neutral surface rather than tinted behind a glyph. */
  function getServerIconStyle(s: SavedServer) {
    if (isServerIconImage(s)) return 'background:var(--bg-item);'
    const color = !s.icon_color || !s.icon_color.trim() ? 'var(--accent)' : s.icon_color
    return `background:${color};`
  }

  /** Port of `CopyServerAddress` (Home.Panels.cs:203-207). */
  async function copyServerAddress(s: SavedServer) {
    const text = `${s.address}:${s.port}`
    try {
      await navigator.clipboard?.writeText(text)
    } catch {
      /* ignore */
    }
    onToast?.(t('toast.copiedText', { text }), 'info')
  }

  const unsavedSuggestions = $derived(_serverSuggestions.filter(isSuggestionUnsaved))

  /** Favorites float to the top of the saved list. */
  const sortedSavedServers = $derived(
    [...savedServers].sort((a, b) => Number(b.favorite) - Number(a.favorite)),
  )

  // QOL: drag-to-reorder. Persists the new order to disk via
  // `reorder_saved_servers`; the favorite-sort above is stable, so this
  // reorders within whichever favorite/non-favorite bucket a server is in.
  let dragServerId = $state<string | null>(null)
  function handleServerDragStart(id: string) {
    dragServerId = id
  }
  function handleServerDragOver(e: DragEvent) {
    e.preventDefault()
  }
  async function handleServerDrop(targetId: string) {
    if (!dragServerId || dragServerId === targetId) return
    const next = [...savedServers]
    const from = next.findIndex((s) => s.id === dragServerId)
    const to = next.findIndex((s) => s.id === targetId)
    if (from < 0 || to < 0) return
    const [moved] = next.splice(from, 1)
    next.splice(to, 0, moved)
    savedServers = next
    dragServerId = null
    try {
      await invoke('reorder_saved_servers', { orderedIds: next.map((s) => s.id) })
    } catch (e) {
      onToast?.(String(e), 'error')
    }
  }

  openServers()
</script>

<div class="panel-header has-back">
  <button
    type="button"
    class="panel-back-btn"
    onclick={onBack}
    onmousedown={(e) => e.stopPropagation()}
    data-tooltip={t('tt.back')}
    aria-label={t('tt.back')}
  >
    <i class="fa-solid fa-arrow-left"></i>
  </button>
  <div class="panel-title-wrap">
    <span class="panel-title">{t('nav.servers')}</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-spacer"></div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn versions-sort-btn"
      onclick={refreshServerStatuses}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.refreshServerStatus')}
      disabled={refreshing}
    >
      <i class={`fa-solid ${refreshing ? 'fa-spinner fa-spin' : 'fa-rotate'}`}></i>
      <span>{t('common.refresh')}</span>
    </button>
    <button
      class="panel-icon-btn versions-sort-btn"
      onclick={openAddServerModal}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.addNewServer')}
    >
      <i class="fa-solid fa-plus"></i>
      <span>{t('x.add')}</span>
    </button>
  </div>
</div>

<SubTabs strip="worlds-servers" active="servers" onNavigate={(k) => onNavigate?.(k)} />

<div class="panel-body">
  {#if savedServers.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-server"></i>
      <span>{t('empty.noSavedServersYet')}</span>
      <small>{t('empty.addMinecraftBedrockServerQuickLaunch')}</small>
      <button
        class="btn-sm"
        style="margin-top:8px;"
        onclick={openAddServerModal}
        onmousedown={(e) => e.stopPropagation()}
      >
        <i class="fa-solid fa-plus"></i> Add server
      </button>
    </div>
  {:else}
    <div class="client-card-cols">
    {#each sortedSavedServers as s (s.id)}
      {@const hasStatus = _serverStatus.has(serverKey(s))}
      {@const st = _serverStatus.get(serverKey(s))}
      <div
        class="client-card {dragServerId === s.id ? 'dragging' : ''}"
        draggable="true"
        ondragstart={() => handleServerDragStart(s.id)}
        ondragover={handleServerDragOver}
        ondrop={() => handleServerDrop(s.id)}
        ondragend={() => (dragServerId = null)}
      >
        <div class="client-card-header">
          <div class="client-card-icon" style={getServerIconStyle(s)}>
            {#if isServerIconImage(s)}
              <img src={s.icon} alt="" loading="lazy" decoding="async" style="width:20px;height:20px;object-fit:contain;border-radius:4px;" />
            {:else}
              <i class={getServerIconClass(s)}></i>
            {/if}
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{s.name}</span>
            <span class="client-card-sub">{s.address}:{s.port}</span>
          </div>
          <div class="client-card-actions">
            <button
              class="copy-btn"
              data-tooltip={s.favorite ? 'Unfavorite' : 'Favorite'}
              aria-label={s.favorite ? 'Unfavorite' : 'Favorite'}
              aria-pressed={s.favorite}
              onclick={() => toggleFavorite(s)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class={s.favorite ? 'fa-solid fa-star fav-on' : 'fa-regular fa-star'}></i>
            </button>
            {#if _confirmDeleteServerId === s.id}
              <IconBtn
                ghost
                icon="fa-solid fa-check"
                title="Confirm delete"
                onClick={() => deleteServer(s)}
                onMouseDown={(e) => e.stopPropagation()}
              />
              <IconBtn
                ghost
                icon="fa-solid fa-xmark"
                title="Cancel"
                onClick={cancelDeleteServer}
                onMouseDown={(e) => e.stopPropagation()}
              />
            {:else}
              <IconBtn
                ghost
                icon="fa-solid fa-trash"
                title="Delete"
                onClick={() => confirmDeleteServer(s.id)}
                onMouseDown={(e) => e.stopPropagation()}
              />
            {/if}
            <IconBtn
              icon={`fa-solid ${launchingKey === serverKey(s) ? 'fa-spinner fa-spin' : 'fa-play'}`}
              title="Launch & connect"
              disabled={launchingKey !== null}
              onClick={() => launchAndConnect(s)}
              onMouseDown={(e) => e.stopPropagation()}
            />
            <div style="position:relative;" onmousedown={(e) => e.stopPropagation()}>
              <IconBtn ghost icon="fa-solid fa-ellipsis-vertical" title="More" onClick={() => toggleServerMenu(s.id)} />
              {#if openServerMenuId === s.id}
                <div class="tw-dropdown-menu">
                  <button type="button" class="tw-dropdown-item" onclick={() => { closeServerMenu(); openEditServerModal(s) }}>
                    <span><i class="fa-solid fa-pen"></i> Edit</span>
                  </button>
                  <button type="button" class="tw-dropdown-item" onclick={() => { closeServerMenu(); copyServerAddress(s) }}>
                    <span><i class="fa-solid fa-copy"></i> Copy address</span>
                  </button>
                </div>
              {/if}
            </div>
          </div>
        </div>
        {#if hasStatus || (s.tags && s.tags.length > 0)}
          <div style="display:flex; align-items:center; justify-content:space-between; gap:8px;">
            {#if hasStatus}
              {#if st == null}
                <span class="server-ping checking">Pinging…</span>
              {:else}
                <span class="server-ping {st.online ? 'online' : 'offline'}">
                  <span class="ping-dot"></span>{st.online
                    ? `${n0(st.players_online)}/${n0(st.players_max)} · ${st.latency_ms} ms`
                    : 'Offline'}
                </span>
              {/if}
            {/if}
            {#if s.tags && s.tags.length > 0}
              <div class="server-tags">
                {#each s.tags as tag}
                  <span class="server-tag-chip">{tag}</span>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
    </div>
  {/if}

  {#if unsavedSuggestions.length > 0}
    <span class="panel-section-label">{t('x.popular')}</span>
    <div class="client-card-cols">
    {#each unsavedSuggestions as s (s.address + ':' + s.port)}
      {@const hasStatus = _serverStatus.has(serverKey(s))}
      {@const st = _serverStatus.get(serverKey(s))}
      <div class="client-card">
        <div class="client-card-header">
          <div class="client-card-icon" style={getServerIconStyle(s)}>
            {#if isServerIconImage(s)}
              <img src={s.icon} alt="" loading="lazy" decoding="async" style="width:20px;height:20px;object-fit:contain;border-radius:4px;" />
            {:else}
              <i class={getServerIconClass(s)}></i>
            {/if}
          </div>
          <div class="client-card-meta">
            <span class="client-card-name">{s.name}</span>
            <span class="client-card-sub">{s.address}:{s.port}</span>
          </div>
          <div class="client-card-actions">
            <button
              class="copy-btn"
              data-tooltip={t('tt.copyAddress')}
              aria-label={t('tt.copyAddress')}
              onclick={() => copyServerAddress(s)}
              onmousedown={(e) => e.stopPropagation()}
            >
              <i class="fa-solid fa-copy"></i>
            </button>
            <IconBtn
              icon="fa-solid fa-bookmark"
              title="Save"
              onClick={() => saveSuggestedServer(s)}
              onMouseDown={(e) => e.stopPropagation()}
            />
            <IconBtn
              icon={`fa-solid ${launchingKey === serverKey(s) ? 'fa-spinner fa-spin' : 'fa-play'}`}
              title="Launch & connect"
              disabled={launchingKey !== null}
              onClick={() => launchAndConnect(s)}
              onMouseDown={(e) => e.stopPropagation()}
            />
          </div>
        </div>
        {#if hasStatus}
          {#if st == null}
            <span class="server-ping checking">Pinging…</span>
          {:else}
            <span class="server-ping {st.online ? 'online' : 'offline'}">
              <span class="ping-dot"></span>{st.online
                ? `${n0(st.players_online)}/${n0(st.players_max)} · ${st.latency_ms} ms`
                : 'Offline'}
            </span>
          {/if}
        {/if}
      </div>
    {/each}
    </div>
  {/if}

  <!-- ── LAN GAMES (feature 34) ── new section, no original counterpart. -->
  <div class="panel-section-label-row" style="display:flex;align-items:center;justify-content:space-between;margin-top:10px;">
    <span class="panel-section-label">{t('x.lanGames')}</span>
    <button
      class="btn-sm"
      onclick={scanLanGames}
      onmousedown={(e) => e.stopPropagation()}
      disabled={lanScanning}
      data-tooltip={t('tt.scanLocalNetworkOpenLanWorlds')}
    >
      <i class={`fa-solid ${lanScanning ? 'fa-spinner fa-spin' : 'fa-magnifying-glass'}`}></i>
      {lanScanning ? 'Scanning…' : 'Scan'}
    </button>
  </div>
  {#if lanError}
    <span class="error-text" style="display:block;margin:4px 0;">{lanError}</span>
  {:else if !lanScanning && lanGames.length === 0}
    <small style="color:var(--text-muted);display:block;margin:4px 0 8px;">
      No LAN worlds found yet. Open a world to LAN on this network, then Scan.
    </small>
  {/if}
  <div class="client-card-cols">
  {#each lanGames as g (g.host + ':' + g.port)}
    <div class="client-card">
      <div class="client-card-header">
        <div class="client-card-icon" style="background:var(--bg-item);">
          <i class="fa-solid fa-house-signal"></i>
        </div>
        <div class="client-card-meta">
          <span class="client-card-name">{g.motd || 'LAN World'}</span>
          <span class="client-card-sub">{g.host}:{g.port}</span>
        </div>
        <div class="client-card-actions">
          <button
            class="copy-btn"
            data-tooltip={t('tt.copyAddress')}
            aria-label={t('tt.copyAddress')}
            onclick={() => copyServerAddress({ id: '', name: g.motd, address: g.host, port: g.port, icon: '', icon_color: '', favorite: false, tags: [] })}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-copy"></i>
          </button>
        </div>
      </div>
    </div>
  {/each}
  </div>
</div>

<!-- ── ADD/EDIT SERVER MODAL ── (Home.razor:4433-4477) -->
<svelte:window onkeydown={(e) => { if (e.key === 'Escape' && serverModalOpen) closeServerModal() }} />

{#if serverModalOpen}
  <div
    class="modal-overlay"
    role="presentation"
    onclick={closeServerModal}
  >
    <div
      class="modal-box"
      role="presentation"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => { if (e.key === 'Enter' && e.target instanceof HTMLInputElement) { e.preventDefault(); void saveServerModal() } }}
    >
      <div class="modal-header">
        <span class="modal-title">
          <i class="fa-solid fa-server" style="color:var(--accent);margin-right:8px;"></i>
          {editingServer == null ? 'Add server' : 'Edit server'}
        </span>
        <button class="modal-close-btn" onclick={closeServerModal} aria-label={t('tt.close')}>
          <i class="fa-solid fa-xmark"></i>
        </button>
      </div>
      <div class="modal-body">
        <label class="modal-input-label" for="server-input-name">{t('label.displayName')}</label>
        <input
          id="server-input-name"
          class="modal-input"
          placeholder={t('ph.myServer')}
          bind:value={serverInputName}
          onkeydown={(e) => e.stopPropagation()}
        />

        <label class="modal-input-label" style="margin-top:8px;" for="server-input-address">Address (IP or hostname)</label>
        <input
          id="server-input-address"
          class="modal-input"
          placeholder="play.example.com"
          bind:value={serverInputAddress}
          onkeydown={(e) => e.stopPropagation()}
        />

        <label class="modal-input-label" style="margin-top:8px;" for="server-input-port">{t('label.port')}</label>
        <input
          id="server-input-port"
          class="modal-input"
          type="number"
          placeholder="19132"
          bind:value={serverInputPort}
          onkeydown={(e) => e.stopPropagation()}
        />

        <label class="modal-input-label" style="margin-top:8px;" for="server-input-tags">Tags (comma-separated)</label>
        <input
          id="server-input-tags"
          class="modal-input"
          placeholder="pvp, survival, friends"
          bind:value={serverInputTags}
          onkeydown={(e) => e.stopPropagation()}
        />

        {#if serverInputError}
          <span class="error-text" style="display:block;margin-top:6px;">{serverInputError}</span>
        {/if}

        <div class="modal-btn-row" style="margin-top:14px;">
          <button class="modal-btn modal-btn-cancel" onclick={closeServerModal}>{t('common.cancel')}</button>
          <button class="modal-btn modal-btn-confirm" onclick={saveServerModal}>
            <i class="fa-solid fa-floppy-disk"></i> {t('common.save')}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .fav-on { color: var(--orange); }
</style>
