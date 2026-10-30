<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * Java's counterpart to `InstancesView.svelte` (Bedrock) — instance
   * management pulled out of a local tab inside `ProfileView.svelte` into
   * its own panel, reachable from `MCVersionsView.svelte` via the shared
   * `versions-instances` cross-nav strip, mirroring how Bedrock's Versions
   * and Instances panels link to each other. Every function here is
   * unchanged from its former home in ProfileView.
   */
  import { open, save } from '@tauri-apps/plugin-dialog'
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import SubTabs from './components/SubTabs.svelte'
  import IconBtn from './components/IconBtn.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import type { LauncherSettings, JavaInstance, JvmPreset } from './settings'

  let {
    settings,
    onChange,
    onBack,
    onNavigate,
    onToast,
  }: {
    settings: LauncherSettings
    onChange: (next: LauncherSettings) => void
    onBack?: () => void
    onNavigate?: (view: string) => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  function toast(message: string, kind = 'info') {
    onToast?.(message, kind)
  }

  let instances = $state<JavaInstance[]>([])
  let instancesLoading = $state(true)
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
    } finally {
      instancesLoading = false
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

  /** Built-in JVM-argument templates ("Vanilla", "Balanced", "Performance",
   *  "Low-End") — offered at creation and as a per-instance preset dropdown. */
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

  /** Change an instance's JVM-args preset from its card dropdown. */
  async function onInstancePresetChange(id: string, preset: string) {
    try {
      await invoke('set_instance_jvm_preset', { id, preset })
      await reloadInstances()
      toast('JVM args preset applied.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  /** Toolbar "More" menu — import-official/from-another-launcher/sync are
   *  rare next to New/Wizard/Import, so they're tucked behind a kebab
   *  instead of crowding a single row into overflow. */
  let toolbarMenuOpen = $state(false)

  function closeToolbarMenu() {
    toolbarMenuOpen = false
  }

  /** Per-card "More" menu — group/clone/export are secondary to
   *  switch/rename/delete, same reasoning as the toolbar menu above. */
  let openCardMenuId = $state('')

  function toggleCardMenu(id: string) {
    openCardMenuId = openCardMenuId === id ? '' : id
  }

  function closeCardMenu() {
    openCardMenuId = ''
  }

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
   *  each group's first-seen order. Pinned instances (any group) are pulled
   *  into their own always-first "Pinned" section instead, mirroring a
   *  favorites list — a pinned instance still keeps its real `group` value,
   *  it just also shows up here for quick access. No drag-and-drop — just
   *  collapsible `<details>` sections. */
  const groupedInstances = $derived.by(() => {
    const groups = new Map<string, JavaInstance[]>()
    for (const inst of instances) {
      if (inst.pinned) continue
      const key = inst.group?.trim() || 'Ungrouped'
      if (!groups.has(key)) groups.set(key, [])
      groups.get(key)!.push(inst)
    }
    return Array.from(groups.entries())
  })

  const pinnedInstances = $derived(instances.filter((i) => i.pinned))

  async function togglePinned(inst: JavaInstance) {
    try {
      await invoke('set_instance_pinned', { id: inst.id, pinned: !inst.pinned })
      await reloadInstances()
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  let exportingId = $state('')

  /** Export an instance's full directory tree as a zip via the same
   *  `@tauri-apps/plugin-dialog` picker used for import. */
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
    const id = renamingInstanceId
    const name = renameInstanceValue
    renamingInstanceId = ''
    if (!id.trim() || !name.trim()) return
    try {
      await invoke('rename_java_instance', { id, newName: name })
      toast('Instance renamed.', 'success')
    } catch (e) {
      toast(String(e), 'error')
    }
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

  /** Port of `CloneInstance` — copies the instance's metadata and full
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

<PanelHeader title={t('panel.instances')} {onBack} />

<SubTabs strip="versions-instances" active="instances" onNavigate={(k) => onNavigate?.(k)} />

<div class="panel-body">
  <div class="instance-card">
    <div class="instance-card-head">
      <i class="fa-solid fa-microchip"></i> Memory · {activeInstance?.name ?? ''}
    </div>
    <div class="instance-ram-row">
      <input type="number" min="0" max="16384" step="512" aria-label={t('x.mbMax')} value={instanceMaxRam} onchange={onInstanceMaxRamChange} />
      <span class="instance-ram-unit">{t('x.mbMax')}</span>
    </div>
    <small class="instance-hint">0 = use the global setting ({settings.JavaMaxRamMb} MB).</small>
  </div>

  <div class="instance-card">
    <div class="instance-card-head instance-toolbar">
      <span><i class="fa-solid fa-cubes-stacked"></i> Instances</span>
      <div class="toolbar-spacer"></div>
      <select
        class="setting-select"
        bind:value={newInstanceTemplate}
        data-tooltip={t('tt.jvmArgsTemplateAppliedNewInstance')}
        aria-label={t('tt.newInstanceTemplate')}
      >
        {#each jvmPresets as p (p.id)}
          <option value={p.id}>{p.label}</option>
        {/each}
      </select>
      <button class="btn-sm btn-primary" onclick={newJavaInstance}><i class="fa-solid fa-plus"></i> New</button>
      <button
        class="btn-sm"
        onclick={() => onNavigate?.('instance-wizard')}
        data-tooltip={t('tt.guidedSetupVersionNameRam')}
        ><i class="fa-solid fa-wand-magic-sparkles"></i> Wizard</button
      >
      <button
        class="btn-sm"
        onclick={importInstanceZip}
        disabled={importBusy}
        data-tooltip={t('tt.importZipModpackInstance')}
        ><i class={importBusy ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-file-import'}></i> {importBusy ? 'Importing…' : 'Import pack'}</button
      >
      <div class="card-menu-anchor">
        <button
          type="button"
          class="btn-sm"
          onclick={() => (toolbarMenuOpen = !toolbarMenuOpen)}
          data-tooltip={t('tt.moreInstanceActions')}
          aria-label={t('tt.moreInstanceActions')}
        >
          <i class="fa-solid fa-ellipsis"></i>
        </button>
        {#if toolbarMenuOpen}
          <div class="tw-dropdown-menu">
            <button
              type="button"
              class="tw-dropdown-item"
              disabled={importBusy}
              onclick={() => { closeToolbarMenu(); importOfficialMinecraft() }}
            >
              <span><i class="fa-brands fa-microsoft"></i> Import official .minecraft</span>
            </button>
            <button
              type="button"
              class="tw-dropdown-item"
              onclick={() => { closeToolbarMenu(); onNavigate?.('import-launcher') }}
            >
              <span><i class="fa-solid fa-right-to-bracket"></i> From another launcher</span>
            </button>
            <button
              type="button"
              class="tw-dropdown-item"
              onclick={() => { closeToolbarMenu(); onNavigate?.('sync-packs') }}
            >
              <span><i class="fa-solid fa-arrows-rotate"></i> Sync packs</span>
            </button>
          </div>
        {/if}
      </div>
    </div>

    {#if instancesLoading && instances.length === 0}
      <ListSkeleton rows={3} />
    {:else if instances.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-cubes-stacked"></i>
        <span>{t('empty.noInstancesYet')}</span>
        <small>{t('empty.createOneImportModpackOrBring')}</small>
      </div>
    {/if}

    {#snippet instanceCard(inst: JavaInstance)}
      {@const isActive = inst.id === activeInstance?.id}
      <div class="client-card instance-card {isActive ? 'client-active' : ''}">
        <div class="client-card-header">
          <div class="client-card-icon client-card-icon-accent">
            <i class="fa-solid fa-cubes-stacked"></i>
          </div>
          <div class="client-card-meta">
            {#if renamingInstanceId === inst.id}
              <input
                class="setting-input instance-rename"
                aria-label={t('tt.rename')}
                value={renameInstanceValue}
                oninput={(e) => (renameInstanceValue = e.currentTarget.value)}
                onkeydown={onRenameInstanceKey}
              />
            {:else if editingGroupId === inst.id}
              <span class="setting-label">{inst.name}</span>
              <input
                class="setting-input instance-rename"
                aria-label={t('label.group')}
                placeholder={t('ph.ungrouped')}
                value={editingGroupValue}
                oninput={(e) => (editingGroupValue = e.currentTarget.value)}
                onkeydown={(e) => { e.stopPropagation(); if (e.key === 'Enter') void commitGroup() }}
              />
            {:else}
              <button type="button" class="instance-name-btn" onclick={() => switchInstance(inst.id)}>
                {#if isActive}<i class="fa-solid fa-circle-check"></i>{:else}<i class="fa-regular fa-circle"></i>{/if}
                <span class="client-card-name" title={inst.name}>{inst.name}</span>
              </button>
              <span class="client-card-sub">{isActive ? 'Active instance' : 'Click name to switch'}</span>
            {/if}
          </div>
          <div class="client-card-actions">
            {#if renamingInstanceId === inst.id}
              <IconBtn ghost icon="fa-solid fa-check" title="Confirm rename" onClick={commitRenameInstance} />
            {:else if editingGroupId === inst.id}
              <IconBtn ghost icon="fa-solid fa-check" title="Confirm group" onClick={commitGroup} />
            {:else}
              <IconBtn
                ghost
                icon="fa-solid fa-thumbtack"
                title={inst.pinned ? 'Unpin' : 'Pin to top'}
                onClick={() => togglePinned(inst)}
                class={inst.pinned ? 'instance-pin-active' : ''}
              />
              <IconBtn ghost icon="fa-solid fa-arrow-up-right-from-square" title="Open details" onClick={() => { switchInstance(inst.id); onNavigate?.('instance-detail') }} />
              <IconBtn ghost icon="fa-solid fa-pen" title="Rename" onClick={() => beginRenameInstance(inst.id, inst.name)} />
              {#if confirmDeleteInstanceId === inst.id}
                <IconBtn ghost icon="fa-solid fa-check" title="Confirm delete" onClick={() => deleteJavaInstance(inst.id)} />
                <IconBtn ghost icon="fa-solid fa-xmark" title="Cancel" onClick={cancelDeleteInstance} />
              {:else}
                <IconBtn ghost icon="fa-solid fa-trash" title="Delete" onClick={() => confirmDeleteInstance(inst.id)} />
              {/if}
              <div class="card-menu-anchor">
                <IconBtn
                  ghost
                  icon="fa-solid fa-ellipsis-vertical"
                  title="More"
                  onClick={() => toggleCardMenu(inst.id)}
                />
                {#if openCardMenuId === inst.id}
                  <div class="tw-dropdown-menu">
                    <button
                      type="button"
                      class="tw-dropdown-item"
                      onclick={() => { closeCardMenu(); beginEditGroup(inst) }}
                    >
                      <span><i class="fa-solid fa-folder"></i> Set group/folder</span>
                    </button>
                    <button
                      type="button"
                      class="tw-dropdown-item"
                      onclick={() => { closeCardMenu(); cloneJavaInstance(inst.id) }}
                    >
                      <span><i class="fa-solid fa-copy"></i> Clone</span>
                    </button>
                    <button
                      type="button"
                      class="tw-dropdown-item"
                      disabled={exportingId === inst.id}
                      onclick={() => { closeCardMenu(); exportInstance(inst) }}
                    >
                      <span><i class={exportingId === inst.id ? 'fa-solid fa-spinner fa-spin' : 'fa-solid fa-file-export'}></i> {exportingId === inst.id ? 'Exporting…' : 'Export as .zip'}</span>
                    </button>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        </div>

        {#if renamingInstanceId !== inst.id && editingGroupId !== inst.id}
          <div class="instance-card-footer">
            <select
              class="setting-select instance-version-select"
              value={inst.jvm_args_preset ?? ''}
              onchange={(e) => onInstancePresetChange(inst.id, e.currentTarget.value)}
              onclick={(e) => e.stopPropagation()}
              aria-label={t('tt.jvmArgsPresetThisInstance')}
              data-tooltip={t('tt.jvmPresetAppliedWhenThisInstance')}
            >
              <option value="" disabled>{inst.jvm_args_preset ? '' : 'Custom'}</option>
              {#each jvmPresets as p (p.id)}
                <option value={p.id}>{p.label}</option>
              {/each}
            </select>
          </div>
        {/if}
      </div>
    {/snippet}

    {#if pinnedInstances.length > 0}
      <details class="instance-group" open>
        <summary class="instance-group-summary">
          <i class="fa-solid fa-thumbtack group-icon"></i>
          <span class="group-label">{t('label.pinned')}</span>
          <span class="settings-nav-btn-count">{pinnedInstances.length}</span>
        </summary>
        <div class="client-card-cols">
          {#each pinnedInstances as inst (inst.id)}
            {@render instanceCard(inst)}
          {/each}
        </div>
      </details>
    {/if}

    <!--
      Same "too many controls in one row" fix as InstancesView (Bedrock) and
      ProfileView's old Instances tab: each group's instances are a
      `.client-card-cols` grid of cards instead of stacked full-width rows.
    -->
    {#each groupedInstances as [groupName, groupInstances] (groupName)}
      <details class="instance-group" open={!settings.CollapseInstanceGroups}>
        <summary class="instance-group-summary">
          <i class="fa-solid fa-folder group-icon"></i>
          <span class="group-label" title={groupName}>{groupName}</span>
          <span class="settings-nav-btn-count">{groupInstances.length}</span>
        </summary>
        <div class="client-card-cols">
        {#each groupInstances as inst (inst.id)}
          {@render instanceCard(inst)}
        {/each}
        </div>
      </details>
    {/each}
  </div>
</div>

<style>
  /* Instance management rhythm: section gutter and band gaps between the
     memory card, the instances card and each group; card padding on every
     card; row padding on group summaries; hover/disabled on buttons. */
  .panel-body {
    gap: var(--panel-band);
  }
  .panel-body > .instance-card {
    margin: 0;
    padding: var(--card-pad-y) var(--card-pad-x);
  }
  .instance-card-head {
    margin-bottom: var(--panel-band);
  }
  .instance-toolbar {
    gap: var(--panel-band);
    row-gap: var(--panel-band);
  }
  .toolbar-spacer {
    flex: 1;
    min-width: 0;
  }
  .card-menu-anchor {
    position: relative;
  }
  .instance-ram-row {
    gap: var(--panel-band);
  }
  .instance-hint {
    margin-top: var(--panel-band);
  }
  .client-card-icon-accent {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .client-card-cols {
    gap: var(--panel-band);
  }
  .instance-card-footer {
    gap: var(--panel-band);
    padding-top: var(--row-pad-y);
    margin-top: 0;
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
  .instance-name-btn {
    min-width: 0;
    padding: 4px var(--row-pad-x) 4px 4px;
    border-radius: var(--r-sm);
    transition: background-color var(--t);
  }
  .instance-name-btn:hover:not(:disabled) {
    background: var(--bg-item-hover);
  }
  .instance-name-btn:disabled,
  .instance-version-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .instance-group {
    margin-top: var(--panel-band);
  }
  .instance-group-summary {
    padding: var(--row-pad-y) var(--row-pad-x);
    transition: background-color var(--t);
  }
  .instance-group-summary:hover {
    background: var(--bg-item-hover);
  }
  .group-icon {
    color: var(--accent);
  }
  .group-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn-sm:not(.btn-primary):hover:not(:disabled) {
    background: var(--bg-item-hover);
  }
  .btn-sm:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .tw-dropdown-item:hover:not(:disabled) {
    background: var(--bg-item-hover);
  }
  .tw-dropdown-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
