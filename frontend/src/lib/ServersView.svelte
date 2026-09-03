<script lang="ts">
  /**
   * Literal port of the `servers` panel — Home.razor:3715-3828, with behaviour
   * from Pages/Home.Panels.cs:17-207 and Services/ServerPingService.cs.
   *
   * Transcribed element-for-element: same DOM order, class names, tooltips and
   * strings. The saved list is persisted through `servers.json`
   * (src-tauri/src/commands/servers.rs) rather than
   * `LauncherSettings.SavedServers`, but every field maps 1:1.
   *
   * Deliberate divergence, flagged rather than faked: the original's
   * "Launch & connect" action has no Tauri counterpart
   * (`launch_bedrock_instance` takes only `dllPath`, no server address), so
   * the play button renders visibly disabled with an explanatory tooltip.
   */
  import { invoke } from '@tauri-apps/api/core'
  import SubTabs from './components/SubTabs.svelte'
  import EditionTabs from './components/EditionTabs.svelte'

  let {
    onBack,
    onNavigate,
    onToast,
  }: {
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  /** Port of the `SavedServer` model. */
  type SavedServer = {
    id: string
    name: string
    address: string
    port: number
    icon: string
    icon_color: string
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
  let serverInputError = $state('')

  let savedServers = $state<SavedServer[]>([])

  /** Port of `_serverSuggestions` (Home.Panels.cs:24-32) — verbatim. */
  const _serverSuggestions: SavedServer[] = [
    { id: '', name: 'Hive', address: 'geo.hivebedrock.network', port: 19132, icon: 'fa-solid fa-hexagon-nodes', icon_color: '#f5a623' },
    { id: '', name: 'CubeCraft', address: 'play.cubecraft.net', port: 19132, icon: 'fa-solid fa-cube', icon_color: '#4a90e2' },
    { id: '', name: 'Mineplex', address: 'mco.mineplex.com', port: 19132, icon: 'fa-solid fa-tower-cell', icon_color: '#e94e4e' },
    { id: '', name: 'Lifeboat', address: 'play.lbsg.net', port: 19132, icon: 'fa-solid fa-life-ring', icon_color: '#00bcd4' },
    { id: '', name: 'Galaxite', address: 'play.galaxite.net', port: 19132, icon: 'fa-solid fa-star', icon_color: '#9b51e0' },
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
    serverInputError = ''
    serverModalOpen = true
  }

  /** Port of `OpenEditServerModal` (Home.Panels.cs:109-118). */
  function openEditServerModal(s: SavedServer) {
    editingServer = s
    serverInputName = s.name
    serverInputAddress = s.address
    serverInputPort = s.port
    serverInputError = ''
    serverModalOpen = true
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
      if (editingServer == null) {
        await invoke('add_saved_server', {
          name,
          address: serverInputAddress.trim(),
          port,
          icon: '',
          iconColor: '',
        })
      } else {
        await invoke('update_saved_server', {
          server: { ...editingServer, name, address: serverInputAddress.trim(), port },
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

  /** Port of `DeleteServer` (Home.Panels.cs:162-167) — immediate, no confirm. */
  async function deleteServer(s: SavedServer) {
    try {
      await invoke('remove_saved_server', { id: s.id })
    } catch (e) {
      onToast?.(String(e), 'error')
      return
    }
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

  /** Port of `GetServerIconClass` (Home.Panels.cs:194-195). */
  function getServerIconClass(s: SavedServer) {
    return !s.icon || !s.icon.trim() ? 'fa-solid fa-server' : s.icon
  }

  /** Port of `GetServerIconStyle` (Home.Panels.cs:197-201). */
  function getServerIconStyle(s: SavedServer) {
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
    onToast?.(`Copied ${text}`, 'info')
  }

  const unsavedSuggestions = $derived(_serverSuggestions.filter(isSuggestionUnsaved))

  openServers()
</script>

<div class="panel-header">
  <div class="panel-title-wrap">
    <span class="panel-title">Servers</span>
    <div class="panel-title-underline"></div>
  </div>
  <div class="panel-header-actions">
    <button
      class="panel-icon-btn versions-sort-btn"
      onclick={openAddServerModal}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Add a new server"
    >
      <i class="fa-solid fa-plus"></i>
      <span>Add</span>
    </button>
    <button
      class="panel-back-btn"
      onclick={onBack}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Back"
    >
      <i class="fa-solid fa-chevron-down"></i>
    </button>
  </div>
</div>

<SubTabs strip="worlds-servers" active="servers" onNavigate={(k) => onNavigate?.(k)} />

<div class="panel-body">
  {#if savedServers.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-server"></i>
      <span>No saved servers yet</span>
      <small>Add a Minecraft Bedrock server to quick-launch into it. The current client will be injected before connecting.</small>
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
    {#each savedServers as s (s.id)}
      {@const hasStatus = _serverStatus.has(serverKey(s))}
      {@const st = _serverStatus.get(serverKey(s))}
      <div class="server-row">
        <div class="server-icon" style={getServerIconStyle(s)}>
          <i class={getServerIconClass(s)}></i>
        </div>
        <div class="server-meta">
          <span class="server-name">{s.name}</span>
          <span class="server-sub">{s.address}:{s.port}</span>
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
        <div class="version-actions">
          <button
            class="icon-btn icon-btn-ghost"
            data-tooltip="Edit"
            onclick={() => openEditServerModal(s)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-pen"></i>
          </button>
          <button
            class="icon-btn icon-btn-ghost"
            data-tooltip="Delete"
            onclick={() => deleteServer(s)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-trash"></i>
          </button>
          <button
            class="copy-btn"
            data-tooltip="Copy address"
            onclick={() => copyServerAddress(s)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-copy"></i>
          </button>
          <!-- No Tauri command joins a server on launch yet, so this stays disabled. -->
          <button
            class="icon-btn"
            data-tooltip="Launch &amp; connect — not available yet (no launch-with-server command)"
            onmousedown={(e) => e.stopPropagation()}
            disabled
          >
            <i class="fa-solid fa-play"></i>
          </button>
        </div>
      </div>
    {/each}
  {/if}

  {#if unsavedSuggestions.length > 0}
    <span class="panel-section-label">Popular</span>
    {#each unsavedSuggestions as s (s.address + ':' + s.port)}
      {@const hasStatus = _serverStatus.has(serverKey(s))}
      {@const st = _serverStatus.get(serverKey(s))}
      <div class="server-row">
        <div class="server-icon" style={getServerIconStyle(s)}>
          <i class={getServerIconClass(s)}></i>
        </div>
        <div class="server-meta">
          <span class="server-name">{s.name}</span>
          <span class="server-sub">{s.address}:{s.port}</span>
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
        <div class="version-actions">
          <button
            class="copy-btn"
            data-tooltip="Copy address"
            onclick={() => copyServerAddress(s)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-copy"></i>
          </button>
          <button
            class="icon-btn"
            data-tooltip="Save"
            onclick={() => saveSuggestedServer(s)}
            onmousedown={(e) => e.stopPropagation()}
          >
            <i class="fa-solid fa-bookmark"></i>
          </button>
          <!-- No Tauri command joins a server on launch yet, so this stays disabled. -->
          <button
            class="icon-btn"
            data-tooltip="Launch &amp; connect — not available yet (no launch-with-server command)"
            onmousedown={(e) => e.stopPropagation()}
            disabled
          >
            <i class="fa-solid fa-play"></i>
          </button>
        </div>
      </div>
    {/each}
  {/if}
</div>

<EditionTabs edition="bedrock" active="worlds" onNavigate={(v) => onNavigate?.(v)} />

<!-- ── ADD/EDIT SERVER MODAL ── (Home.razor:4433-4477) -->
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
    >
      <div class="modal-header">
        <span class="modal-title">
          <i class="fa-solid fa-server" style="color:var(--accent);margin-right:8px;"></i>
          {editingServer == null ? 'Add server' : 'Edit server'}
        </span>
        <button class="modal-close-btn" onclick={closeServerModal}>
          <i class="fa-solid fa-xmark"></i>
        </button>
      </div>
      <div class="modal-body">
        <label class="modal-input-label" for="server-input-name">Display name</label>
        <input
          id="server-input-name"
          class="modal-input"
          placeholder="My Server"
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

        <label class="modal-input-label" style="margin-top:8px;" for="server-input-port">Port</label>
        <input
          id="server-input-port"
          class="modal-input"
          type="number"
          placeholder="19132"
          bind:value={serverInputPort}
          onkeydown={(e) => e.stopPropagation()}
        />

        {#if serverInputError}
          <span class="error-text" style="display:block;margin-top:6px;">{serverInputError}</span>
        {/if}

        <div class="modal-btn-row" style="margin-top:14px;">
          <button class="modal-btn modal-btn-cancel" onclick={closeServerModal}>Cancel</button>
          <button class="modal-btn modal-btn-confirm" onclick={saveServerModal}>
            <i class="fa-solid fa-floppy-disk"></i> Save
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
