<script lang="ts">
  /**
   * Phase 7 UI: sync packs (`gl-sync`) — named sets of targets (directory
   * links, merged files, plain-copy files, mod-id lists) kept consistent
   * across Java instances. Backed by the Tauri commands in
   * `src-tauri/src/commands/sync_packs.rs`, which operate on the single
   * shared `AppState.sync_engine` (so pack state persists across calls
   * within one app session — see `src-tauri/src/state.rs`).
   *
   * Safety rule mirrored from the engine itself: `apply_sync_pack` is never
   * offered without first showing a `preview_sync` report. There is no
   * "just apply" shortcut in this UI.
   */
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import Modal from './components/Modal.svelte'
  import type { JavaInstance } from './settings'

  let {
    onBack,
    onToast,
  }: {
    onBack?: () => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  // ------------------------------------------------------------- model types
  // Mirrors `gl_sync::model` / `gl_sync::report` (Rust, serde `tag = "kind"`
  // for `SyncTarget`).

  type MergeFormat = 'OptionsTxt'

  type SyncTarget =
    | { kind: 'DirLink'; relative_path: string }
    | { kind: 'FileMerge'; relative_path: string; format: MergeFormat }
    | { kind: 'FileCopy'; relative_path: string }
    | { kind: 'Mods'; mod_ids: string[] }

  type AdoptStrategy = 'KeepExisting' | 'TakePackVersion' | 'Merge'
  type DetachMode = 'Materialize' | 'Discard'

  type VersionOverride = {
    instance_id: string
    target_label: string
    adopt_strategy: AdoptStrategy
  }

  type SyncPack = {
    id: string
    name: string
    targets: SyncTarget[]
    overrides: VersionOverride[]
    created_at: string
    updated_at: string
  }

  type ReportAction =
    | 'LinkCreated'
    | 'CopiedInsteadOfLinked'
    | 'Merged'
    | 'Copied'
    | 'ModInstalled'
    | 'NoOp'
    | 'ConflictNeedsResolution'
    | 'DetachedMaterialized'
    | 'DetachedDiscarded'
    | 'Failed'

  type ReportEntry = {
    instance_id: string
    target_label: string
    action: ReportAction
    detail: string
  }

  type SyncReport = {
    pack_id: string
    dry_run: boolean
    entries: ReportEntry[]
  }

  // ------------------------------------------------------------------ state

  let packs = $state<SyncPack[]>([])
  let instances = $state<JavaInstance[]>([])
  let isLoading = $state(true)
  let busy = $state(false)

  // Create-pack modal.
  let showCreateModal = $state(false)
  let newPackId = $state('')
  let newPackName = $state('')
  let newTargetKind = $state<SyncTarget['kind']>('DirLink')
  let newTargetPath = $state('')
  let newTargetModIds = $state('')
  let draftTargets = $state<SyncTarget[]>([])

  // Apply/preview modal, scoped to one pack.
  let activePack = $state<SyncPack | null>(null)
  let selectedInstanceIds = $state<Set<string>>(new Set())
  let report = $state<SyncReport | null>(null)
  let previewing = $state(false)
  let applying = $state(false)

  function instanceName(id: string): string {
    return instances.find((i) => i.id === id)?.name ?? id
  }

  function targetLabel(t: SyncTarget): string {
    switch (t.kind) {
      case 'DirLink':
        return `Directory link · ${t.relative_path}`
      case 'FileMerge':
        return `Merged file · ${t.relative_path} (${t.format})`
      case 'FileCopy':
        return `Copied file · ${t.relative_path}`
      case 'Mods':
        return `Mods · ${t.mod_ids.join(', ') || '(none)'}`
    }
  }

  function actionLabel(a: ReportAction): string {
    switch (a) {
      case 'LinkCreated': return 'Junction created'
      case 'CopiedInsteadOfLinked': return 'Copied (different volume)'
      case 'Merged': return 'Merged'
      case 'Copied': return 'Copied'
      case 'ModInstalled': return 'Mod installed'
      case 'NoOp': return 'No change'
      case 'ConflictNeedsResolution': return 'Needs a decision'
      case 'DetachedMaterialized': return 'Detached (kept content)'
      case 'DetachedDiscarded': return 'Detached (discarded)'
      case 'Failed': return 'Failed'
    }
  }

  function actionClass(a: ReportAction): string {
    if (a === 'Failed' || a === 'ConflictNeedsResolution') return 'sync-report-bad'
    if (a === 'NoOp') return 'sync-report-neutral'
    return 'sync-report-good'
  }

  // -------------------------------------------------------------- loading

  async function reload() {
    try {
      const [p, i] = await Promise.all([
        invoke<SyncPack[]>('list_sync_packs'),
        invoke<JavaInstance[]>('list_java_instances'),
      ])
      packs = p
      instances = i
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      isLoading = false
    }
  }

  reload()

  // ------------------------------------------------------------ create pack

  function openCreateModal() {
    newPackId = ''
    newPackName = ''
    newTargetKind = 'DirLink'
    newTargetPath = ''
    newTargetModIds = ''
    draftTargets = []
    showCreateModal = true
  }

  function addDraftTarget() {
    if (newTargetKind === 'Mods') {
      const ids = newTargetModIds.split(',').map((s) => s.trim()).filter(Boolean)
      if (ids.length === 0) return
      draftTargets = [...draftTargets, { kind: 'Mods', mod_ids: ids }]
      newTargetModIds = ''
      return
    }
    const path = newTargetPath.trim()
    if (!path) return
    if (newTargetKind === 'DirLink') {
      draftTargets = [...draftTargets, { kind: 'DirLink', relative_path: path }]
    } else if (newTargetKind === 'FileCopy') {
      draftTargets = [...draftTargets, { kind: 'FileCopy', relative_path: path }]
    } else if (newTargetKind === 'FileMerge') {
      draftTargets = [...draftTargets, { kind: 'FileMerge', relative_path: path, format: 'OptionsTxt' }]
    }
    newTargetPath = ''
  }

  function removeDraftTarget(index: number) {
    draftTargets = draftTargets.filter((_, i) => i !== index)
  }

  async function createPack() {
    const id = newPackId.trim()
    const name = newPackName.trim()
    if (!id || !name || draftTargets.length === 0) {
      onToast?.('Pack id, name, and at least one target are required.', 'error')
      return
    }
    busy = true
    try {
      await invoke('create_sync_pack', { id, name, targets: draftTargets })
      showCreateModal = false
      await reload()
      onToast?.(`Created sync pack "${name}".`, 'success')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }

  async function deletePack(pack: SyncPack) {
    busy = true
    try {
      await invoke('delete_sync_pack', { packId: pack.id })
      await reload()
      onToast?.(`Deleted "${pack.name}".`, 'info')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }

  // ------------------------------------------------------------ apply flow

  function openApplyModal(pack: SyncPack) {
    activePack = pack
    // Pre-select instances already tracking this pack.
    selectedInstanceIds = new Set(instances.filter((i) => i.sync_pack_ids?.includes(pack.id)).map((i) => i.id))
    report = null
  }

  function closeApplyModal() {
    activePack = null
    report = null
    selectedInstanceIds = new Set()
  }

  function toggleInstance(id: string) {
    const next = new Set(selectedInstanceIds)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    selectedInstanceIds = next
    // Selecting different instances invalidates any prior preview — force a
    // fresh one before Apply is offered again.
    report = null
  }

  async function runPreview() {
    if (!activePack || selectedInstanceIds.size === 0) return
    previewing = true
    try {
      report = await invoke<SyncReport>('preview_sync', {
        packId: activePack.id,
        instanceIds: [...selectedInstanceIds],
      })
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      previewing = false
    }
  }

  /** Only true once a preview has been run for exactly this selection and
   * came back clean — the gate that unlocks the Apply button. */
  const canApply = $derived(
    report !== null &&
      report.dry_run &&
      !report.entries.some((e) => e.action === 'ConflictNeedsResolution' || e.action === 'Failed'),
  )

  async function runApply() {
    if (!activePack || !canApply) return
    applying = true
    try {
      report = await invoke<SyncReport>('apply_sync_pack', {
        packId: activePack.id,
        instanceIds: [...selectedInstanceIds],
      })
      await reload()
      if (report.entries.some((e) => e.action === 'Failed')) {
        onToast?.('Applied with some failures — see the report.', 'error')
      } else {
        onToast?.(`Applied "${activePack.name}".`, 'success')
      }
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      applying = false
    }
  }

  async function runDetach(instanceId: string, mode: DetachMode) {
    if (!activePack) return
    busy = true
    try {
      report = await invoke<SyncReport>('detach_sync_pack', {
        packId: activePack.id,
        instanceId,
        mode,
      })
      await reload()
      onToast?.(`Detached "${instanceName(instanceId)}" from "${activePack.name}".`, 'info')
    } catch (e) {
      onToast?.(String(e), 'error')
    } finally {
      busy = false
    }
  }
</script>

<PanelHeader title="Sync Packs" {onBack}>
  {#snippet extraActions()}
    <button
      type="button"
      class="panel-icon-btn {isLoading ? 'spinning' : ''}"
      disabled={isLoading}
      onclick={reload}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="Refresh"
      aria-label="Refresh"
    >
      <i class="fa-solid fa-rotate"></i>
    </button>
    <button
      type="button"
      class="panel-icon-btn"
      onclick={openCreateModal}
      onmousedown={(e) => e.stopPropagation()}
      data-tooltip="New sync pack"
      aria-label="New sync pack"
    >
      <i class="fa-solid fa-plus"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  <p class="instance-hint" style="margin-bottom:10px;">
    A sync pack keeps a directory, a merged file, or a mod list identical across the Java instances it's applied to.
    Nothing is ever touched without a preview first.
  </p>

  {#if isLoading}
    <div class="versions-loading"><span class="spinner"></span><span>Loading sync packs…</span></div>
  {:else if packs.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-arrows-turn-to-dots"></i>
      <span>No sync packs yet. Create one to share resourcepacks, options, or mod lists across instances.</span>
    </div>
  {:else}
    {#each packs as pack (pack.id)}
      {@const trackingCount = instances.filter((i) => i.sync_pack_ids?.includes(pack.id)).length}
      <div class="instance-card">
        <div class="instance-card-head">
          <span><i class="fa-solid fa-arrows-turn-to-dots"></i> {pack.name}</span>
          <div style="flex:1"></div>
          <span class="instance-hint">{trackingCount} instance{trackingCount === 1 ? '' : 's'}</span>
        </div>
        <div class="instance-hint" style="margin-bottom:8px;">
          {#each pack.targets as t, i (i)}
            <div>{targetLabel(t)}</div>
          {/each}
        </div>
        <div class="profile-actions">
          <button class="btn-sm" onclick={() => openApplyModal(pack)} disabled={busy}>
            <i class="fa-solid fa-arrows-rotate"></i> Apply to instances
          </button>
          <button class="btn-sm skin-btn-ghost" onclick={() => deletePack(pack)} disabled={busy}>
            <i class="fa-solid fa-trash"></i> Delete
          </button>
        </div>
      </div>
    {/each}
  {/if}
</div>

<!-- Create pack -->
<Modal open={showCreateModal} title="New Sync Pack" icon="fa-solid fa-plus" onClose={() => (showCreateModal = false)}>
  <div class="setting-row">
    <label for="sync-pack-id">Pack id</label>
    <input id="sync-pack-id" class="setting-input" placeholder="shared-resourcepacks" bind:value={newPackId} />
  </div>
  <div class="setting-row">
    <label for="sync-pack-name">Name</label>
    <input id="sync-pack-name" class="setting-input" placeholder="Shared resourcepacks" bind:value={newPackName} />
  </div>

  <span class="panel-section-label">Targets</span>
  <div class="setting-row">
    <select class="setting-select" bind:value={newTargetKind} aria-label="Target kind">
      <option value="DirLink">Directory link (e.g. resourcepacks)</option>
      <option value="FileMerge">Merged file (options.txt)</option>
      <option value="FileCopy">Copied file (e.g. icon.png)</option>
      <option value="Mods">Mod id list</option>
    </select>
  </div>
  {#if newTargetKind === 'Mods'}
    <div class="setting-row">
      <input
        class="setting-input"
        placeholder="sodium, lithium, iris"
        bind:value={newTargetModIds}
        onkeydown={(e) => { if (e.key === 'Enter') addDraftTarget() }}
      />
      <button class="vcs-btn" onclick={addDraftTarget} aria-label="Add target"><i class="fa-solid fa-plus"></i></button>
    </div>
  {:else}
    <div class="setting-row">
      <input
        class="setting-input"
        placeholder={newTargetKind === 'FileMerge' ? 'options.txt' : 'resourcepacks'}
        bind:value={newTargetPath}
        onkeydown={(e) => { if (e.key === 'Enter') addDraftTarget() }}
      />
      <button class="vcs-btn" onclick={addDraftTarget} aria-label="Add target"><i class="fa-solid fa-plus"></i></button>
    </div>
  {/if}

  {#if draftTargets.length > 0}
    <div class="instance-hint">
      {#each draftTargets as t, i (i)}
        <div class="instance-row">
          <span>{targetLabel(t)}</span>
          <div style="flex:1"></div>
          <button class="instance-icon-btn" onclick={() => removeDraftTarget(i)} aria-label="Remove target">
            <i class="fa-solid fa-xmark"></i>
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="profile-actions" style="margin-top:14px;">
    <button class="btn-sm" onclick={createPack} disabled={busy}>
      <i class="fa-solid fa-check"></i> Create pack
    </button>
    <button class="btn-sm skin-btn-ghost" onclick={() => (showCreateModal = false)}>Cancel</button>
  </div>
</Modal>

<!-- Apply / preview -->
<Modal
  open={activePack !== null}
  title={activePack ? `Apply "${activePack.name}"` : ''}
  icon="fa-solid fa-arrows-rotate"
  onClose={closeApplyModal}
  class="sync-apply-modal"
>
  {#if activePack}
    <span class="panel-section-label">Instances</span>
    <div class="instance-hint">
      {#each instances as inst (inst.id)}
        <label class="instance-row" style="cursor:pointer;">
          <input
            type="checkbox"
            checked={selectedInstanceIds.has(inst.id)}
            onchange={() => toggleInstance(inst.id)}
          />
          <span>{inst.name}</span>
          <div style="flex:1"></div>
          {#if inst.sync_pack_ids?.includes(activePack.id)}
            <span class="instance-hint">tracking</span>
            <button
              class="instance-icon-btn"
              data-tooltip="Detach: keep current content"
              aria-label="Detach and keep content"
              onclick={() => runDetach(inst.id, 'Materialize')}
            >
              <i class="fa-solid fa-link-slash"></i>
            </button>
          {/if}
        </label>
      {/each}
    </div>

    <div class="profile-actions" style="margin-top:10px;">
      <button class="btn-sm" onclick={runPreview} disabled={previewing || selectedInstanceIds.size === 0}>
        <i class="fa-solid fa-magnifying-glass"></i> {previewing ? 'Previewing…' : 'Preview'}
      </button>
      <button
        class="btn-sm"
        onclick={runApply}
        disabled={!canApply || applying}
        data-tooltip={report ? '' : 'Run a preview first'}
      >
        <i class="fa-solid fa-check"></i> {applying ? 'Applying…' : 'Apply'}
      </button>
    </div>

    {#if report}
      <span class="panel-section-label">
        {report.dry_run ? 'Preview' : 'Result'} ({report.entries.length} {report.entries.length === 1 ? 'entry' : 'entries'})
      </span>
      {#if report.entries.length === 0}
        <div class="empty-state"><span>Nothing to do — already in sync.</span></div>
      {:else}
        <div class="instance-hint">
          {#each report.entries as e, i (i)}
            <div class="instance-row {actionClass(e.action)}">
              <span>{instanceName(e.instance_id)} · {e.target_label}</span>
              <div style="flex:1"></div>
              <span>{actionLabel(e.action)}</span>
            </div>
            <div class="instance-hint" style="padding-left:8px;">{e.detail}</div>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}
</Modal>

<style>
  .sync-report-bad { color: var(--danger, #e05a5a); }
  .sync-report-good { color: var(--success, #4caf7d); }
  .sync-report-neutral { opacity: 0.75; }
</style>
