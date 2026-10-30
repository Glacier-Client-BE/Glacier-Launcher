<script lang="ts">
  import { t } from './i18nStore.svelte'
  /**
   * Phase 5: import instances from an external launcher (MultiMC/Prism,
   * CurseForge app, ATLauncher, Modrinth App) into Glacier.
   *
   * Flow: detected-launcher list -> per-launcher instance list with
   * size/loader/mod-count badges and checkboxes -> a preview step (mod list
   * for the selected candidates) -> import with per-item progress (reusing
   * `InlineProgress`, same as the rest of the app).
   *
   * WEBVIEW2 CONSTRAINT: the "point me at a custom root" affordance below
   * uses the host's native picker (`@tauri-apps/plugin-dialog` `open`), the
   * same pattern as `ProfileView.svelte`'s instance-zip import - never an
   * HTML `<input type=file>` or a drop target.
   */
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import PanelHeader from './components/PanelHeader.svelte'
  import InlineProgress from './components/InlineProgress.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import type { JavaInstance } from './settings'

  let {
    onBack,
    onToast,
    onImported,
  }: {
    onBack?: () => void
    onToast?: (message: string, kind?: string) => void
    onImported?: () => void
  } = $props()

  function toast(message: string, kind = 'info') {
    onToast?.(message, kind)
  }

  type SourceLauncher = 'multi_mc_or_prism' | 'curse_forge_app' | 'a_t_launcher' | 'modrinth_app'

  const LAUNCHER_LABELS: Record<SourceLauncher, string> = {
    multi_mc_or_prism: 'MultiMC / Prism Launcher',
    curse_forge_app: 'CurseForge app',
    a_t_launcher: 'ATLauncher',
    modrinth_app: 'Modrinth App',
  }

  type DetectedLauncher = { launcher: SourceLauncher; root_path: string }

  type ImportCandidate = {
    source_launcher: SourceLauncher
    name: string
    mc_version: string
    loader: string | null
    loader_version: string | null
    instance_path: string
    mod_count: number
    world_count: number
    size_bytes: number
    icon_path: string | null
  }

  type ImportOutcome = {
    name: string
    instance: JavaInstance | null
    error: string | null
  }

  type Step = 'detect' | 'list' | 'preview' | 'importing' | 'done'

  let step = $state<Step>('detect')
  let loading = $state(false)
  let detected = $state<DetectedLauncher[]>([])
  let activeLauncher = $state<DetectedLauncher | null>(null)
  let candidates = $state<ImportCandidate[]>([])
  let selected = $state<Set<number>>(new Set())
  let previewMods = $state<Record<number, string[]>>({})
  let outcomes = $state<ImportOutcome[]>([])

  function formatSize(bytes: number): string {
    if (bytes <= 0) return '0 MB'
    const mb = bytes / 1_048_576
    return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(0)} MB`
  }

  async function runDetection() {
    loading = true
    try {
      detected = await invoke<DetectedLauncher[]>('detect_external_launchers')
      if (detected.length === 0) {
        toast('No other launchers were found under %APPDATA%. You can still point at a custom folder below.', 'info')
      }
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      loading = false
    }
  }

  runDetection()

  /** WEBVIEW2 CONSTRAINT: native directory picker only. */
  async function pickCustomRoot(launcher: SourceLauncher) {
    const dir = await open({ directory: true, title: `Select ${LAUNCHER_LABELS[launcher]} folder` })
    if (typeof dir !== 'string') return
    await openLauncher({ launcher, root_path: dir })
  }

  async function openLauncher(entry: DetectedLauncher) {
    activeLauncher = entry
    loading = true
    try {
      candidates = await invoke<ImportCandidate[]>('scan_launcher_instances', {
        launcher: entry.launcher,
        rootPath: entry.root_path,
      })
      selected = new Set(candidates.map((_, i) => i))
      step = 'list'
      if (candidates.length === 0) {
        toast('No instances found at that location.', 'info')
      }
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      loading = false
    }
  }

  function toggle(i: number) {
    const next = new Set(selected)
    if (next.has(i)) next.delete(i)
    else next.add(i)
    selected = next
  }

  async function goToPreview() {
    loading = true
    try {
      const mods: Record<number, string[]> = {}
      for (const i of selected) {
        mods[i] = await invoke<string[]>('preview_import', { candidate: candidates[i] })
      }
      previewMods = mods
      step = 'preview'
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      loading = false
    }
  }

  async function runImport() {
    step = 'importing'
    outcomes = []
    const chosen = candidates.filter((_, i) => selected.has(i))
    try {
      outcomes = await invoke<ImportOutcome[]>('import_instances', { candidates: chosen })
    } catch (e) {
      toast(String(e), 'error')
      step = 'preview'
      return
    }
    step = 'done'
    const okCount = outcomes.filter((o) => o.instance).length
    if (okCount > 0) {
      toast(`Imported ${okCount} of ${outcomes.length} instance(s).`, okCount === outcomes.length ? 'success' : 'info')
      onImported?.()
    } else {
      toast('No instances were imported.', 'error')
    }
  }

  function backToList() {
    step = 'list'
  }

  function startOver() {
    step = 'detect'
    activeLauncher = null
    candidates = []
    selected = new Set()
    outcomes = []
  }
</script>

<PanelHeader title={t('panel.importFromAnotherLauncher')} {onBack} />

<div class="panel-body">
  {#if step === 'detect'}
    {#if loading}
      <ListSkeleton rows={4} />
    {:else}
      <p class="wizard-hint">{t('x.chooseALauncherToImportInstancesFrom')}</p>
      {#each (['multi_mc_or_prism', 'curse_forge_app', 'a_t_launcher', 'modrinth_app'] as SourceLauncher[]) as kind (kind)}
        {@const found = detected.find((d) => d.launcher === kind)}
        <div class="instance-row">
          <span class="row-grow">
            {LAUNCHER_LABELS[kind]}
            {#if found}<br /><small>{found.root_path}</small>{:else}<br /><small>Not found</small>{/if}
          </span>
          {#if found}
            <button class="btn-sm btn-primary" onclick={() => openLauncher(found)}>{t('x.scan')}</button>
          {/if}
          <button class="btn-sm" onclick={() => pickCustomRoot(kind)}>
            <i class="fa-solid fa-folder-open"></i> Choose folder
          </button>
        </div>
      {/each}
    {/if}
  {:else if step === 'list'}
    <div class="instance-row">
      <button class="btn-sm" onclick={startOver}><i class="fa-solid fa-arrow-left"></i> Launchers</button>
      <span class="row-grow">{activeLauncher ? LAUNCHER_LABELS[activeLauncher.launcher] : ''}</span>
    </div>
    {#if loading}
      <ListSkeleton rows={3} />
    {:else if candidates.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-folder-open"></i>
        <span>{t('x.noInstancesFound')}</span>
        <button class="btn-sm" onclick={startOver}>Choose another launcher</button>
      </div>
    {:else}
      {#each candidates as c, i (c.instance_path)}
        <label class="instance-row">
          <input type="checkbox" checked={selected.has(i)} onchange={() => toggle(i)} />
          <span class="row-grow">
            {c.name}
            <br />
            <small>
              MC {c.mc_version || '?'}
              {#if c.loader}&middot; {c.loader}{c.loader_version ? ` ${c.loader_version}` : ''}{/if}
              &middot; {c.mod_count} mods &middot; {c.world_count} worlds &middot; {formatSize(c.size_bytes)}
            </small>
          </span>
        </label>
      {/each}
      <div class="instance-row">
        <button class="btn-sm" onclick={backToList}><i class="fa-solid fa-arrow-left"></i> Back</button>
        <span class="row-grow"></span>
        <button class="btn-sm btn-primary" disabled={selected.size === 0} onclick={goToPreview}>
          Preview {selected.size} instance(s)
        </button>
      </div>
    {/if}
  {:else if step === 'preview'}
    <div class="instance-row">
      <button class="btn-sm" onclick={backToList}><i class="fa-solid fa-arrow-left"></i> Back</button>
      <span class="row-grow"></span>
      <button class="btn-sm btn-primary" onclick={runImport}>Import {selected.size} instance(s)</button>
    </div>
    {#each candidates as c, i (c.instance_path)}
      {#if selected.has(i)}
        <div class="instance-row row-stack">
          <strong>{c.name}</strong>
          <small>{(previewMods[i] ?? []).length} mod file(s){#if (previewMods[i] ?? []).length > 0}: {(previewMods[i] ?? []).slice(0, 8).join(', ')}{(previewMods[i] ?? []).length > 8 ? '…' : ''}{/if}</small>
        </div>
      {/if}
    {/each}
  {:else if step === 'importing'}
    <p class="wizard-hint">Importing…</p>
    <InlineProgress progress={0.5} showSpinner={true} />
  {:else if step === 'done'}
    <p class="wizard-hint">{t('x.importFinished')}</p>
    {#each outcomes as o, oi (oi)}
      <div class="instance-row">
        <span class="row-grow">{o.name}</span>
        {#if o.instance}
          <span class="badge-ok"><i class="fa-solid fa-check"></i> Imported</span>
        {:else}
          <span class="badge-err" data-tooltip={o.error}><i class="fa-solid fa-xmark"></i> Failed</span>
        {/if}
      </div>
    {/each}
    <div class="instance-row">
      <span class="row-grow"></span>
      <button class="btn-sm btn-primary" onclick={startOver}>{t('x.importMore')}</button>
    </div>
  {/if}
</div>

<style>
  /* Outcome markers after an import ran: were unstyled plain text before. */
  .badge-ok,
  .badge-err {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 12px;
  }
  .badge-ok {
    color: var(--green);
    background: var(--green-bg);
  }
  .badge-err {
    color: var(--red);
    background: var(--red-bg);
  }

  /* Import rhythm: band gaps between stacked rows, row padding inside each
     row, hover on clickable (label) rows and buttons, dimmed disabled. */
  .panel-body {
    gap: var(--panel-band);
  }
  .instance-row {
    gap: var(--panel-band);
    padding: var(--row-pad-y) var(--row-pad-x);
    margin-top: 0;
    transition: background-color var(--t);
  }
  .instance-row:hover {
    background: var(--bg-item-hover);
  }
  .instance-row.row-stack {
    flex-direction: column;
    align-items: flex-start;
  }
  .row-grow {
    flex: 1;
    min-width: 0;
  }
  label.instance-row {
    cursor: pointer;
  }
  .instance-row button:not(.btn-primary):hover:not(:disabled) {
    background: var(--bg-item-hover);
  }
  .instance-row button:disabled,
  .instance-row input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .instance-row button,
  .empty-state button {
    transition: background-color var(--t), opacity var(--t);
  }
  .instance-row small {
    overflow-wrap: anywhere;
  }
</style>
