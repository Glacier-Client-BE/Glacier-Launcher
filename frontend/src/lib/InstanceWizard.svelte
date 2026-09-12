<script lang="ts">
  /**
   * Phase 8 — multi-step "New Instance" wizard.
   *
   * Steps: version -> loader (disabled-with-tooltip until Phase 2 ships
   * `list_loader_versions`/`install_loader`) -> name/icon/group ->
   * RAM/Java (via Phase 1's `get_instance_settings`/`set_instance_settings`,
   * already registered) -> review/create.
   *
   * The pack-import step described in the plan is intentionally omitted:
   * `preview_pack` and the CurseForge/Modrinth pack-install commands it
   * depends on do not exist in `src-tauri/src/lib.rs` yet (Phase 6/7), and
   * per the porting rules a step with no real backend is skipped rather
   * than shown broken. `search_modrinth_modpacks` (modpack.rs) exists but
   * only searches — there is no "apply this pack to a fresh instance"
   * command yet, so it would still dead-end mid-step.
   */
  import { invoke } from '@tauri-apps/api/core'
  import Spinner from './components/Spinner.svelte'
  import type { JavaInstance, JavaVersion, InstanceSettings } from './settings'

  let {
    onDone,
    onCancel,
    onToast,
  }: {
    /** Called with the newly created instance once the wizard finishes. */
    onDone?: (instance: JavaInstance) => void
    onCancel?: () => void
    onToast?: (message: string, kind?: string) => void
  } = $props()

  function toast(message: string, kind = 'info') {
    onToast?.(message, kind)
  }

  const MISSING_LOADER_VERSIONS =
    'Unavailable: needs list_loader_versions/install_loader commands (Phase 2)'

  type StepId = 'version' | 'loader' | 'details' | 'ram' | 'review'
  const STEPS: { id: StepId; label: string }[] = [
    { id: 'version', label: 'Version' },
    { id: 'loader', label: 'Loader' },
    { id: 'details', label: 'Name & group' },
    { id: 'ram', label: 'RAM & Java' },
    { id: 'review', label: 'Review' },
  ]

  let stepIndex = $state(0)
  const step = $derived(STEPS[stepIndex].id)

  // -- step 1: version --------------------------------------------------
  let versions = $state<JavaVersion[]>([])
  let versionsLoading = $state(true)
  let versionFilter = $state<'release' | 'snapshot' | 'all'>('release')
  let selectedVersionId = $state('')
  let installingVersion = $state(false)

  const filteredVersions = $derived(
    versions.filter((v) => versionFilter === 'all' || v.type === versionFilter),
  )

  async function loadVersions() {
    versionsLoading = true
    try {
      versions = await invoke<JavaVersion[]>('list_java_versions')
      if (!selectedVersionId) {
        selectedVersionId = filteredVersions[0]?.id ?? versions[0]?.id ?? ''
      }
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      versionsLoading = false
    }
  }
  loadVersions()

  const selectedVersion = $derived(versions.find((v) => v.id === selectedVersionId) ?? null)

  async function ensureVersionInstalled() {
    if (!selectedVersion || selectedVersion.is_installed) return
    installingVersion = true
    try {
      await invoke('install_java_version', { version: selectedVersion })
      selectedVersion.is_installed = true
      toast(`Installed ${selectedVersion.id}.`, 'success')
    } catch (e) {
      toast(String(e), 'error')
      throw e
    } finally {
      installingVersion = false
    }
  }

  // -- step 2: loader (degraded — no backend yet) ------------------------
  type LoaderChoice = 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge'
  let selectedLoader = $state<LoaderChoice>('vanilla')

  // -- step 3: name/icon/group -------------------------------------------
  let instanceName = $state('New Instance')
  let instanceGroup = $state('')
  const ACCENTS = ['#7289da', '#43b581', '#faa61a', '#f04747', '#c471ed', '#4fc3f7']
  let accentColor = $state(ACCENTS[0])

  // -- step 4: RAM & Java --------------------------------------------------
  let minRamMb = $state(1024)
  let maxRamMb = $state(4096)
  let javaPath = $state('')

  function canAdvance(): boolean {
    switch (step) {
      case 'version':
        return !!selectedVersionId
      case 'loader':
        return true
      case 'details':
        return instanceName.trim().length > 0
      case 'ram':
        return maxRamMb >= minRamMb && minRamMb > 0
      default:
        return true
    }
  }

  async function next() {
    if (!canAdvance()) return
    if (step === 'version') {
      try {
        await ensureVersionInstalled()
      } catch {
        return
      }
    }
    if (stepIndex < STEPS.length - 1) stepIndex += 1
  }

  function back() {
    if (stepIndex > 0) stepIndex -= 1
    else onCancel?.()
  }

  let creating = $state(false)

  async function create() {
    creating = true
    try {
      const instance = await invoke<JavaInstance>('create_java_instance_from_template', {
        name: instanceName.trim(),
        versionId: selectedVersionId,
        template: null,
      })

      const settings: InstanceSettings = {
        memory: [minRamMb, maxRamMb],
        java_path: javaPath.trim() || null,
      }
      await invoke('set_instance_settings', { id: instance.id, settings })

      if (instanceGroup.trim()) {
        await invoke('set_instance_group', { id: instance.id, group: instanceGroup.trim() })
      }
      await invoke('set_java_instance_accent_color', { id: instance.id, color: accentColor })

      // Loader install is Phase 2's job; nothing to call here yet since the
      // loader step is disabled-with-tooltip (selectedLoader stays 'vanilla').

      toast(`Created "${instance.name}".`, 'success')
      onDone?.(instance)
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      creating = false
    }
  }
</script>

<div class="wizard-overlay">
  <div class="wizard-card">
    <div class="wizard-head">
      <h2>New Instance</h2>
      <button class="panel-icon-btn" onclick={onCancel} aria-label="Close wizard">
        <i class="fa-solid fa-xmark"></i>
      </button>
    </div>

    <div class="wizard-steps">
      {#each STEPS as s, i (s.id)}
        <div class="wizard-step-dot {i === stepIndex ? 'active' : ''} {i < stepIndex ? 'done' : ''}">
          <span class="wizard-step-num">{i < stepIndex ? '✓' : i + 1}</span>
          <span class="wizard-step-label">{s.label}</span>
        </div>
      {/each}
    </div>

    <div class="wizard-body">
      {#if step === 'version'}
        <div class="wizard-field-row">
          <label>
            <input type="radio" name="vf" checked={versionFilter === 'release'} onchange={() => (versionFilter = 'release')} />
            Releases
          </label>
          <label>
            <input type="radio" name="vf" checked={versionFilter === 'snapshot'} onchange={() => (versionFilter = 'snapshot')} />
            Snapshots
          </label>
          <label>
            <input type="radio" name="vf" checked={versionFilter === 'all'} onchange={() => (versionFilter = 'all')} />
            All
          </label>
        </div>
        {#if versionsLoading}
          <div class="versions-loading"><Spinner size={18} /><span>Loading versions...</span></div>
        {:else}
          <div class="wizard-version-list">
            {#each filteredVersions.slice(0, 60) as v (v.id)}
              <button
                type="button"
                class="wizard-version-row {selectedVersionId === v.id ? 'active' : ''}"
                onclick={() => (selectedVersionId = v.id)}
              >
                <span>{v.id}</span>
                {#if v.is_installed}<i class="fa-solid fa-circle-check" title="Installed"></i>{/if}
              </button>
            {/each}
          </div>
        {/if}
      {:else if step === 'loader'}
        <p class="wizard-hint">Choose a mod loader, or stick with vanilla.</p>
        <div class="wizard-loader-grid">
          {#each ['vanilla', 'fabric', 'quilt', 'forge', 'neoforge'] as l}
            <button
              type="button"
              class="wizard-loader-tile {selectedLoader === l ? 'active' : ''}"
              disabled={l !== 'vanilla'}
              data-tooltip={l !== 'vanilla' ? MISSING_LOADER_VERSIONS : undefined}
              onclick={() => (selectedLoader = l as LoaderChoice)}
            >
              {l}
            </button>
          {/each}
        </div>
        <p class="wizard-hint">
          Fabric/Quilt/Forge/NeoForge installers land in a later phase — this instance will be
          created vanilla and you can attach a loader from its Settings tab once that ships.
        </p>
      {:else if step === 'details'}
        <label class="wizard-field">
          <span>Name</span>
          <input class="setting-input" bind:value={instanceName} />
        </label>
        <label class="wizard-field">
          <span>Group (optional)</span>
          <input class="setting-input" bind:value={instanceGroup} placeholder="Ungrouped" />
        </label>
        <div class="wizard-field">
          <span>Accent color</span>
          <div class="wizard-accent-row">
            {#each ACCENTS as c}
              <button
                type="button"
                class="wizard-accent-dot {accentColor === c ? 'active' : ''}"
                style="background:{c}"
                aria-label={`Accent ${c}`}
                onclick={() => (accentColor = c)}
              ></button>
            {/each}
          </div>
        </div>
      {:else if step === 'ram'}
        <label class="wizard-field">
          <span>Min RAM (MB)</span>
          <input class="setting-input" type="number" min="256" step="256" bind:value={minRamMb} />
        </label>
        <label class="wizard-field">
          <span>Max RAM (MB)</span>
          <input class="setting-input" type="number" min="512" step="256" bind:value={maxRamMb} />
        </label>
        <label class="wizard-field">
          <span>Custom Java path (optional)</span>
          <input class="setting-input" bind:value={javaPath} placeholder="Use global default" />
        </label>
      {:else if step === 'review'}
        <div class="wizard-review">
          <div><strong>Name</strong><span>{instanceName}</span></div>
          <div><strong>Version</strong><span>{selectedVersionId}</span></div>
          <div><strong>Loader</strong><span>{selectedLoader}</span></div>
          <div><strong>Group</strong><span>{instanceGroup || 'Ungrouped'}</span></div>
          <div><strong>RAM</strong><span>{minRamMb} – {maxRamMb} MB</span></div>
        </div>
      {/if}
    </div>

    <div class="wizard-footer">
      <button class="btn-sm" onclick={back} disabled={creating || installingVersion}>
        {stepIndex === 0 ? 'Cancel' : 'Back'}
      </button>
      {#if step === 'review'}
        <button class="btn-sm btn-primary" onclick={create} disabled={creating}>
          {#if creating}<Spinner size={14} />{:else}<i class="fa-solid fa-check"></i>{/if} Create Instance
        </button>
      {:else}
        <button class="btn-sm btn-primary" onclick={next} disabled={!canAdvance() || installingVersion}>
          {#if installingVersion}<Spinner size={14} /> Installing…{:else}Next{/if}
        </button>
      {/if}
    </div>
  </div>
</div>
