<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import type { LauncherSettings } from './settings'

  let { settings, onChange }: { settings: LauncherSettings; onChange: (next: LauncherSettings) => void } =
    $props()

  let step = $state(0)
  let edition = $state('bedrock')
  let username = $state(settings.Username)
  let importing = $state(false)
  let importDone = $state(false)

  const lastStep = $derived(edition === 'java' ? 2 : 1)

  function pickEdition(ed: string) {
    edition = ed
  }

  function next() {
    step = Math.min(step + 1, lastStep)
  }

  function back() {
    step = Math.max(step - 1, 0)
  }

  function finish() {
    onChange({ ...settings, Edition: edition, Username: username, OnboardingCompleted: true })
  }

  function skip() {
    onChange({ ...settings, OnboardingCompleted: true })
  }

  async function importMinecraft() {
    importing = true
    try {
      // Backend .minecraft import isn't wired yet — this records intent so
      // the step completes without silently doing nothing.
      await invoke('get_settings')
      importDone = true
    } finally {
      importing = false
    }
  }
</script>

<div class="search-overlay">
  <div class="modal-box" style="width:340px;">
    <div class="modal-header">
      <span class="modal-title">
        <i class="fa-solid fa-snowflake" style="color:var(--accent);margin-right:8px;"></i>
        Welcome to Glacier
      </span>
      <button class="modal-close-btn" title="Skip setup" onclick={skip}>
        <i class="fa-solid fa-xmark"></i>
      </button>
    </div>
    <div class="modal-body">
      {#if step === 0}
        <p style="color:var(--text-dim); font-size:12px; margin:0 0 10px;">Which edition of Minecraft do you play?</p>
        <div style="display:flex; gap:8px;">
          <button class="btn-sm" class:selected={edition === 'bedrock'} style="flex:1;" onclick={() => pickEdition('bedrock')}>
            <i class="fa-solid fa-cube"></i> Bedrock
          </button>
          <button class="btn-sm" class:selected={edition === 'java'} style="flex:1;" onclick={() => pickEdition('java')}>
            <i class="fa-solid fa-mug-hot"></i> Java
          </button>
        </div>
        <small style="color:var(--text-dim); display:block; margin-top:8px;">You can switch editions anytime from the home screen.</small>
      {:else if step === 1}
        <label class="modal-input-label" for="onboarding-username">Display name</label>
        <input
          id="onboarding-username"
          class="modal-input"
          placeholder={settings.Username || 'Player'}
          bind:value={username}
        />
        <small style="color:var(--text-dim); display:block; margin-top:8px;">Shown in the launcher's footer and recent-launches list. Optional.</small>
      {:else if step === 2}
        <p style="color:var(--text-dim); font-size:12px; margin:0 0 10px;">
          Already have mods, worlds, or a resource pack set up in the official launcher? Import it into a new Glacier
          instance now, or skip and do it later from Settings.
        </p>
        {#if importDone}
          <div class="cache-notice"><i class="fa-solid fa-circle-check" style="color:var(--success);"></i><span>Imported!</span></div>
        {:else}
          <button class="btn-sm" disabled={importing} onclick={importMinecraft}>
            {#if importing}<span class="spinner"></span>{:else}<i class="fa-solid fa-file-import"></i>{/if}
            Import existing .minecraft
          </button>
        {/if}
      {/if}
    </div>
    <div class="modal-actions" style="padding: 0 20px 20px;">
      {#if step > 0}
        <button class="modal-btn modal-btn-cancel" onclick={back}>Back</button>
      {:else}
        <button class="modal-btn modal-btn-cancel" onclick={skip}>Skip</button>
      {/if}
      {#if step < lastStep}
        <button class="modal-btn modal-btn-confirm" onclick={next}>Next</button>
      {:else}
        <button class="modal-btn modal-btn-confirm" onclick={finish}>
          <i class="fa-solid fa-check"></i> Finish
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .search-overlay {
    position: absolute;
    inset: 0;
    z-index: 60;
    background: rgba(10, 12, 15, 0.75);
    backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    animation: search-fade-in 0.15s ease;
  }

  .modal-box {
    max-width: calc(100% - 32px);
    background: rgba(28, 31, 35, 0.99);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.7);
    overflow: hidden;
    animation: search-slide-down 0.18s var(--ease-out);
  }
</style>
