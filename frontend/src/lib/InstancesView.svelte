<script lang="ts">
  /**
   * 1:1 port of the `bedrockinstances` panel in `Pages/Home.razor` plus its
   * code-behind `Pages/Home.BedrockInstances.cs` (92 lines).
   *
   * Everything here is transcribed from the original, including the
   * `.empty-state` blurb's inline styles, the inline (in-row) rename and
   * delete confirmations, and the fact that the bottom `.panel-tabs` strip
   * highlights `mcversions` rather than this panel.
   *
   * No scoped <style> block: every class below lives in app.css, which is a
   * verbatim copy of the original stylesheet. Scoped rules would win on
   * specificity and silently override it.
   */
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import EditionTabs from './components/EditionTabs.svelte'
  import type { BedrockInstance } from './settings'

  let {
    onBack,
    onNavigate,
    onToast,
  }: {
    onBack?: () => void
    onNavigate?: (view: string) => void
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

  let instances = $state<BedrockInstance[]>([])
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
      const versions =
        await invoke<Omit<VanillaVersion, 'is_downloaded'>[]>('list_vanilla_bedrock_versions')
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
      const switched = await invoke<boolean>('switch_bedrock_instance', { id })
      await loadInstances()
      const name = instances.find((i) => i.id === id)?.name ?? ''
      if (switched) toast(`Switched to '${name}'.`, 'success')
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

<PanelHeader title="Instances" {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn"
      onclick={newBedrockInstance}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="New instance"
      aria-label="New instance"
    >
      <i class="fa-solid fa-plus"></i>
    </button>
  {/snippet}
</PanelHeader>

<SubTabs strip="versions-instances" active="instances" onNavigate={(k) => onNavigate?.(k)} />

<div class="panel-body">
  <div class="empty-state" style="padding:14px 16px 4px; text-align:left; align-items:flex-start;">
    <small
      >Each instance keeps its own worlds, packs, and local settings. Switching copies the active
      instance's data out and the new one's data in — give it a moment on large worlds. Optionally
      pin a downloaded game version per instance in <a
        href="javascript:void(0)"
        onclick={() => onNavigate?.('versions')}
        onmousedown={(e) => e.stopPropagation()}>MC Versions</a
      > — switching to that instance registers it too.</small
    >
  </div>
  {#each instances as inst (inst.id)}
    {@const isActive = inst.id === activeInstanceId}
    <div class="instance-row {isActive ? 'active' : ''}">
      {#if renamingBedrockInstanceId === inst.id}
        <input
          class="setting-input instance-rename"
          value={renameBedrockInstanceValue}
          oninput={(e) => (renameBedrockInstanceValue = e.currentTarget.value)}
          onkeydown={(e) => e.stopPropagation()}
        />
        <button
          type="button"
          class="vcs-btn"
          onclick={commitRenameBedrockInstance}
          onmousedown={(e) => e.stopPropagation()}
          aria-label="Save name"><i class="fa-solid fa-check"></i></button
        >
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
          <span>{inst.name}</span>
        </button>
        <select
          class="setting-select instance-version-select"
          value={inst.version_tag}
          onchange={(e) => onBedrockInstanceVersionChanged(inst.id, e.currentTarget.value)}
          onclick={(e) => e.stopPropagation()}
          data-tooltip="Game version to register when switching to this instance"
          aria-label="Game version to register when switching to this instance"
        >
          <option value="">Current version</option>
          {#each downloadedMcVersions as v (v.version)}
            <option value={v.version}>{displayName(v)}</option>
          {/each}
        </select>
        <button
          type="button"
          class="instance-icon-btn"
          onclick={() => beginRenameBedrockInstance(inst.id, inst.name)}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip="Rename"
          aria-label="Rename"><i class="fa-solid fa-pen"></i></button
        >
        <button
          type="button"
          class="instance-icon-btn"
          onclick={() => backupBedrockInstance(inst)}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip="Back up"
          aria-label="Back up"><i class="fa-solid fa-camera"></i></button
        >
        {#if confirmDeleteBedrockInstanceId === inst.id}
          <button
            type="button"
            class="instance-icon-btn instance-del"
            onclick={() => deleteBedrockInstance(inst.id)}
            onmousedown={(e) => e.stopPropagation()}
            data-tooltip="Confirm delete"
            aria-label="Confirm delete"><i class="fa-solid fa-check"></i></button
          >
          <button
            type="button"
            class="instance-icon-btn"
            onclick={cancelDeleteBedrockInstance}
            onmousedown={(e) => e.stopPropagation()}
            aria-label="Cancel delete"><i class="fa-solid fa-xmark"></i></button
          >
        {:else if !isActive}
          <button
            type="button"
            class="instance-icon-btn"
            onclick={() => confirmDeleteBedrockInstance(inst.id)}
            onmousedown={(e) => e.stopPropagation()}
            data-tooltip="Delete"
            aria-label="Delete"><i class="fa-solid fa-trash"></i></button
          >
        {/if}
      {/if}
    </div>
  {/each}
</div>

<EditionTabs edition="bedrock" active="mcversions" onNavigate={(v) => onNavigate?.(v)} />
