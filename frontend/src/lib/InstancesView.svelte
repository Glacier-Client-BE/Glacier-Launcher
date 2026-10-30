<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * 1:1 port of the `bedrockinstances` panel in `Pages/Home.razor` plus its
   * code-behind `Pages/Home.BedrockInstances.cs` (92 lines).
   *
   * Everything here is transcribed from the original, including the
   * `.empty-state` blurb's inline styles, the inline (in-row) rename and
   * delete confirmations, and the fact that the bottom `.panel-tabs` strip
   * highlights `mcversions` rather than this panel.
   *
   * Scoped <style> block at the bottom only adds spacing-token rhythm, the
   * hover/disabled states and truncation. Every other class still lives in
   * app.css, so the scoped rules only touch the properties listed there.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { invokeCached } from './invokeCache'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import IconBtn from './components/IconBtn.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import type { BedrockInstance } from './settings'

  let {
    onBack,
    onNavigate,
    onToast,
  }: {
    onBack?: () => void
    onNavigate?: (view: string, strip?: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  function toast(message: string, kind = 'info') {
    onToast?.(message, kind)
  }

  /** Port of `Models/VanillaVersion.cs`, for the per-instance version picker. */
  type VanillaVersion = {
    version: string
    update_id: string
    package_name: string
    channel: string
    size_bytes: number
    is_downloaded: boolean
  }

  // Legacy Bedrock versions are all "1.x.y.z"; the newer calendar-based
  // scheme (e.g. "26.1") uses a non-1 major, so those get a "v" prefix — a
  // transcription of `VanillaVersion.DisplayName`.
  function displayName(v: VanillaVersion): string {
    const baseName = v.version.startsWith('1.') ? v.version : `v${v.version}`
    return v.channel === 'Preview' ? `${baseName} Preview` : baseName
  }

  // TODO(UI polish feature 38 - drag-and-drop instance reordering): skipped —
  // no drag library (e.g. svelte-dnd-action) is a dependency in package.json,
  // and the spec says only attempt this if one is already present.
  let instances = $state<BedrockInstance[]>([])
  let instancesLoading = $state(true)
  let activeInstanceId = $state('')
  let mcVersionsList = $state<VanillaVersion[]>([])

  // `Home.BedrockInstances.cs` field-for-field.
  let isSwitchingBedrockInstance = $state(false)
  let renamingBedrockInstanceId = $state('')
  let renameBedrockInstanceValue = $state('')
  let confirmDeleteBedrockInstanceId = $state('')

  const downloadedMcVersions = $derived(mcVersionsList.filter((v) => v.is_downloaded))

  async function loadInstances() {
    try {
      instances = await invoke<BedrockInstance[]>('list_bedrock_instances')
      activeInstanceId = (await invoke<string | null>('active_bedrock_instance_id')) ?? ''
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      instancesLoading = false
    }
  }

  /**
   * `OpenBedrockInstances` ends with
   * `if (mcVersionsList.Count == 0) await LoadMcVersionsAsync();` — the
   * version picker below is populated from that same list.
   */
  async function loadMcVersions() {
    if (mcVersionsList.length > 0) return
    try {
      // Windows Update version list rarely changes; a 10-minute cache keeps
      // re-opening the instance picker instant.
      const versions = await invokeCached<Omit<VanillaVersion, 'is_downloaded'>[]>(
        'list_vanilla_bedrock_versions',
        undefined,
        10 * 60 * 1000,
      )
      mcVersionsList = await Promise.all(
        versions.map(async (v) => ({
          ...v,
          is_downloaded: await invoke<boolean>('vanilla_bedrock_version_installed', {
            version: v.version,
          }).catch(() => false),
        })),
      )
    } catch {
      // The picker degrades to just "Current version" when Windows Update is
      // unreachable; the original surfaces that error in the MC Versions panel.
      mcVersionsList = []
    }
  }

  /** Port of `SwitchBedrockInstance`. */
  async function switchBedrockInstance(id: string) {
    if (isSwitchingBedrockInstance) return
    if (id === activeInstanceId) return
    isSwitchingBedrockInstance = true
    try {
      // `switch_bedrock_instance` returns the original's warning string, not a
      // bool: empty means the data swap AND the game-version switch both
      // succeeded, non-empty means the data swap worked but the version didn't
      // (Home.BedrockInstances.cs:29-34).
      const warning = await invoke<string>('switch_bedrock_instance', { id })
      await loadInstances()
      const name = instances.find((i) => i.id === id)?.name ?? ''
      if (warning) toast(warning, 'warning')
      else toast(`Switched to '${name}'.`, 'success')
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      isSwitchingBedrockInstance = false
    }
  }

  /** Port of `NewBedrockInstance`. */
  async function newBedrockInstance() {
    try {
      const instance = await invoke<BedrockInstance>('create_bedrock_instance', {
        name: 'New Instance',
        versionTag: '',
      })
      await loadInstances()
      toast(
        `Created '${instance.name}'. Switch to it to start using its own worlds and packs.`,
        'info',
      )
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  /** Port of `OnBedrockInstanceVersionChanged`. */
  async function onBedrockInstanceVersionChanged(id: string, versionTag: string) {
    try {
      await invoke('set_bedrock_instance_version', { id, versionTag })
      await loadInstances()
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  /** Feature 40: per-instance accent color, shown as a small dot on the card. */
  async function onBedrockInstanceAccentChanged(id: string, color: string) {
    try {
      await invoke('set_bedrock_instance_accent_color', { id, color: color || null })
      await loadInstances()
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  function beginRenameBedrockInstance(id: string, current: string) {
    renamingBedrockInstanceId = id
    renameBedrockInstanceValue = current
  }

  /** Port of `CommitRenameBedrockInstance`. */
  async function commitRenameBedrockInstance() {
    if (renameBedrockInstanceValue.trim()) {
      try {
        await invoke('rename_bedrock_instance', {
          id: renamingBedrockInstanceId,
          newName: renameBedrockInstanceValue,
        })
        await loadInstances()
      } catch (e) {
        toast(String(e), 'error')
      }
    }
    renamingBedrockInstanceId = ''
  }

  /** Per-card "More" menu — Backup is secondary to Rename/Delete, same
   *  overflow-avoidance reasoning applied to JavaInstancesView's cards. */
  let openCardMenuId = $state('')

  function toggleCardMenu(id: string) {
    openCardMenuId = openCardMenuId === id ? '' : id
  }

  function closeCardMenu() {
    openCardMenuId = ''
  }

  function confirmDeleteBedrockInstance(id: string) {
    confirmDeleteBedrockInstanceId = id
  }

  function cancelDeleteBedrockInstance() {
    confirmDeleteBedrockInstanceId = ''
  }

  /** Port of `DeleteBedrockInstance`. */
  async function deleteBedrockInstance(id: string) {
    try {
      const ok = await invoke<boolean>('delete_bedrock_instance', { id })
      confirmDeleteBedrockInstanceId = ''
      await loadInstances()
      toast(
        ok
          ? 'Instance deleted.'
          : "Can't delete the active instance — switch to another one first.",
        ok ? 'success' : 'error',
      )
    } catch (e) {
      confirmDeleteBedrockInstanceId = ''
      toast(String(e), 'error')
    }
  }

  /** Port of `BackupBedrockInstance`. */
  async function backupBedrockInstance(instance: BedrockInstance) {
    try {
      toast(`Backing up '${instance.name}'…`, 'info')
      await invoke<string>('create_bedrock_backup', {
        instanceDir: instance.directory,
        label: instance.name,
      })
      toast('Instance backed up.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  loadInstances()
  loadMcVersions()
</script>

<PanelHeader title={t('panel.instances')} {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn"
      onclick={newBedrockInstance}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip={t('tt.newInstance')}
      aria-label={t('tt.newInstance')}
    >
      <i class="fa-solid fa-plus"></i>
    </button>
  {/snippet}
</PanelHeader>

<SubTabs strip="versions-instances" active="instances" onNavigate={(k, s) => onNavigate?.(k, s)} />

<div class="panel-body">
  <div class="empty-state align-left">
    <small>
      Each instance keeps its own worlds, packs, and local settings. Switching copies the active
      instance's data out and the new one's data in — give it a moment on large worlds. Optionally
      pin a downloaded game version per instance in <button
        type="button"
        class="link-button"
        onclick={() => onNavigate?.('versions')}
        onmousedown={(e) => e.stopPropagation()}>{t('nav.mcversions')}</button
      > — switching to that instance registers it too.
    </small>
  </div>
  <!--
    Redesign: every instance was one long row cramming a name button, a
    version select, a color input, and three icon buttons onto a single
    line — the exact "too many controls in one row" case the rest of this
    overhaul turned into cards elsewhere. Same `.client-card-cols` grid, one
    card per instance, with a clear header (switch target) and a body that
    actually has room for its two settings and three actions.
  -->
  {#if instancesLoading && instances.length === 0}
    <ListSkeleton rows={3} />
  {:else if instances.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-cubes"></i>
      <span>{t('empty.noInstancesYet')}</span>
      <small>{t('empty.createOneKeepSeparateWorldsPacks')}</small>
    </div>
  {/if}
  <div class="client-card-cols">
  {#each instances as inst (inst.id)}
    {@const isActive = inst.id === activeInstanceId}
    <div
      class="client-card instance-card {isActive ? 'client-active' : ''}"
      style={inst.accent_color ? `--instance-accent: ${inst.accent_color};` : ''}
    >
      <div class="client-card-header">
        <div
          class="client-card-icon"
          style={inst.accent_color
            ? `background:color-mix(in srgb, var(--instance-accent) 18%, transparent); color:var(--instance-accent);`
            : 'background:var(--accent-bg); color:var(--accent);'}
        >
          <i class="fa-solid fa-cubes"></i>
        </div>
        <div class="client-card-meta">
          {#if renamingBedrockInstanceId === inst.id}
            <input
              class="setting-input instance-rename"
              aria-label={t('tt.rename')}
              value={renameBedrockInstanceValue}
              oninput={(e) => (renameBedrockInstanceValue = e.currentTarget.value)}
              onkeydown={(e) => { e.stopPropagation(); if (e.key === 'Enter') commitRenameBedrockInstance() }}
            />
          {:else}
            <button
              type="button"
              class="instance-name-btn"
              onclick={() => switchBedrockInstance(inst.id)}
              disabled={isSwitchingBedrockInstance}
              onmousedown={(e) => e.stopPropagation()}
            >
              {#if isActive}<i class="fa-solid fa-circle-check"></i>{:else}<i
                  class="fa-regular fa-circle"
                ></i>{/if}
              <span class="client-card-name" title={inst.name}>{inst.name}</span>
            </button>
          {/if}
          <span class="client-card-sub">{isActive ? 'Active instance' : 'Click name to switch'}</span>
        </div>
        <div class="client-card-actions">
          {#if renamingBedrockInstanceId === inst.id}
            <IconBtn ghost icon="fa-solid fa-check" title="Save name" onClick={commitRenameBedrockInstance} onMouseDown={(e) => e.stopPropagation()} />
          {:else}
            {#if !isActive}
              <IconBtn
                ghost
                icon="fa-solid fa-right-left"
                title="Switch to this instance"
                disabled={isSwitchingBedrockInstance}
                onClick={() => switchBedrockInstance(inst.id)}
                onMouseDown={(e) => e.stopPropagation()}
              />
            {/if}
            <IconBtn ghost icon="fa-solid fa-pen" title="Rename" onClick={() => beginRenameBedrockInstance(inst.id, inst.name)} onMouseDown={(e) => e.stopPropagation()} />
            {#if confirmDeleteBedrockInstanceId === inst.id}
              <IconBtn ghost icon="fa-solid fa-check" title="Confirm delete" onClick={() => deleteBedrockInstance(inst.id)} onMouseDown={(e) => e.stopPropagation()} />
              <IconBtn ghost icon="fa-solid fa-xmark" title="Cancel" onClick={cancelDeleteBedrockInstance} onMouseDown={(e) => e.stopPropagation()} />
            {:else if !isActive}
              <IconBtn ghost icon="fa-solid fa-trash" title="Delete" onClick={() => confirmDeleteBedrockInstance(inst.id)} onMouseDown={(e) => e.stopPropagation()} />
            {/if}
            <div class="card-menu-anchor" role="presentation" onmousedown={(e) => e.stopPropagation()}>
              <IconBtn ghost icon="fa-solid fa-ellipsis-vertical" title="More" onClick={() => toggleCardMenu(inst.id)} />
              {#if openCardMenuId === inst.id}
                <div class="tw-dropdown-menu">
                  <button
                    type="button"
                    class="tw-dropdown-item"
                    onclick={() => { closeCardMenu(); backupBedrockInstance(inst) }}
                  >
                    <span><i class="fa-solid fa-camera"></i> Back up</span>
                  </button>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </div>

      <div class="instance-card-footer">
        <select
          class="setting-select instance-version-select"
          value={inst.version_tag}
          onchange={(e) => onBedrockInstanceVersionChanged(inst.id, e.currentTarget.value)}
          onclick={(e) => e.stopPropagation()}
          aria-label={t('tt.gameVersionRegisterWhenSwitchingThis')}
          data-tooltip={t('tt.gameVersionRegisteredWhenYouSwitch')}
        >
          <option value="">{t('x.currentVersion')}</option>
          {#each downloadedMcVersions as v (v.version)}
            <option value={v.version}>{displayName(v)}</option>
          {/each}
        </select>
        <input
          type="color"
          class="instance-accent-input"
          value={inst.accent_color || '#7289da'}
          onchange={(e) => onBedrockInstanceAccentChanged(inst.id, e.currentTarget.value)}
          onclick={(e) => e.stopPropagation()}
          aria-label={t('tt.instanceAccentColor')}
          data-tooltip={t('tt.accentColorShownAsThisCard')}
        />
      </div>
    </div>
  {/each}
  </div>
</div>

<style>
  .client-card-cols {
    gap: var(--panel-band);
  }
  .instance-card {
    padding: var(--card-pad-y) var(--card-pad-x);
  }
  .card-menu-anchor {
    position: relative;
  }
  .client-card-meta {
    min-width: 0;
  }
  .client-card-name {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .instance-card-footer {
    gap: var(--panel-band);
    padding-top: var(--row-pad-y);
    margin-top: 0;
  }
  .instance-name-btn {
    padding: 4px var(--row-pad-x) 4px 4px;
    border-radius: var(--r-sm);
    min-width: 0;
    transition: background-color var(--t);
  }
  .instance-name-btn:hover:not(:disabled) {
    background: var(--bg-item-hover);
  }
  .instance-name-btn:disabled,
  .instance-accent-input:disabled,
  .instance-version-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .instance-accent-input {
    transition: opacity var(--t);
  }
  .tw-dropdown-item:hover {
    background: var(--bg-item-hover);
  }
</style>
