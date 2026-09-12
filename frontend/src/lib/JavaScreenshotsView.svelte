<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { convertFileSrc } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'
  import IconBtn from './components/IconBtn.svelte'
  import type { JavaInstance } from './settings'

  // Ported from Home.razor lines 5357-5407 ("Java Screenshots" panel).
  // Java's screenshot source is simpler than Bedrock's (one folder,
  // `<instance>/screenshots/*.png`, vs. Bedrock's two-source merge in
  // BedrockScreenshotService) so this is a small standalone command
  // (java_screenshots.rs) rather than reusing gl_bedrock's service.
  let {
    onBack,
    onNavigate,
    embedded = false,
    instanceId,
  }: {
    onBack?: () => void
    onNavigate?: (view: string) => void
    /** Phase 8: hosted inside `InstanceDetailView`'s Screenshots tab —
     * suppress the standalone PanelHeader/back-button. */
    embedded?: boolean
    /** When embedded, pin the instance selector to this id instead of
     * letting the user pick from the multi-instance dropdown. */
    instanceId?: string
  } = $props()

  type ScreenshotFile = { name: string; path: string; kind: string; size_bytes: number; modified_at: string }

  let instances = $state<JavaInstance[]>([])
  let activeInstanceId = $state('')
  let shots = $state<ScreenshotFile[]>([])
  let loading = $state(true)
  let error = $state('')

  async function loadInstances() {
    try {
      instances = await invoke<JavaInstance[]>('list_java_instances')
      if (instanceId && instances.some((i) => i.id === instanceId)) {
        activeInstanceId = instanceId
      } else if (!activeInstanceId || !instances.some((i) => i.id === activeInstanceId)) {
        activeInstanceId = instances[0]?.id ?? ''
      }
    } catch (e) {
      error = String(e)
    }
  }

  async function loadShots() {
    if (!activeInstanceId) {
      shots = []
      return
    }
    loading = true
    error = ''
    try {
      shots = await invoke<ScreenshotFile[]>('list_java_screenshots', { instanceId: activeInstanceId })
    } catch (e) {
      error = String(e)
    } finally {
      loading = false
    }
  }

  async function remove(shot: ScreenshotFile) {
    try {
      await invoke('delete_java_screenshot', { path: shot.path })
      await loadShots()
    } catch (e) {
      error = String(e)
    }
  }

  async function openFolder() {
    try {
      await invoke('open_java_screenshots_folder', { instanceId: activeInstanceId })
    } catch (e) {
      error = String(e)
    }
  }

  loadInstances().then(loadShots)

  $effect(() => {
    activeInstanceId
    loadShots()
  })
</script>

{#if !embedded}
  <PanelHeader title="Java Screenshots" {onBack}>
    {#snippet extraActions()}
      <button
        type="button"
        class="panel-icon-btn"
        onclick={openFolder}
        onmousedown={(e) => e.stopPropagation()}
        data-tooltip="Open screenshots folder"
        aria-label="Open screenshots folder"
      >
        <i class="fa-solid fa-folder-open"></i>
      </button>
    {/snippet}
  </PanelHeader>
{/if}

<div class="panel-body">
  {#if error}<p class="error-text">{error}</p>{/if}

  {#if instances.length > 1}
    <select class="setting-select" bind:value={activeInstanceId} style="width:100%; margin-bottom:8px;">
      {#each instances as inst (inst.id)}
        <option value={inst.id}>{inst.name}</option>
      {/each}
    </select>
  {/if}

  {#if loading}
    <Spinner />
  {:else if shots.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-image"></i>
      <span>No screenshots</span>
      <small>Press F2 in-game to take one — they'll show up here.</small>
    </div>
  {:else}
    {#each shots as shot (shot.path)}
      <div class="version-row">
        <img
          src={convertFileSrc(shot.path)}
          alt={shot.name}
          loading="lazy"
          style="width:64px; height:36px; object-fit:cover; border-radius:4px; margin-right:8px;"
        />
        <div class="version-meta">
          <span class="version-name">{shot.name}</span>
          <span class="version-sub">{(shot.size_bytes / 1024).toFixed(0)} KB</span>
        </div>
        <IconBtn ghost icon="fa-solid fa-trash" title="Delete" onClick={() => remove(shot)} />
      </div>
    {/each}
  {/if}
</div>

<!-- Home.razor:5400-5403: this panel ends with `JavaTabs("photos")`. It had no
     bottom strip at all, which made it a dead end. -->
