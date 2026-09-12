<script lang="ts">
  import { untrack } from 'svelte'
  import type { LauncherSettings } from './settings'

  let { settings, onChange }: { settings: LauncherSettings; onChange: (next: LauncherSettings) => void } =
    $props()

  let step = $state(0)
  let edition = $state('bedrock')
  let username = $state(untrack(() => settings.Username))
  let accessibilityMode = $state(untrack(() => settings.AccessibilityMode))

  // Steps: 0 edition, 1 username, last accessibility. There's no real
  // .minecraft import step — the backend doesn't have that command yet, and
  // we'd rather skip it than show a fake success checkmark. Import from
  // Settings once that lands.
  const lastStep = 2

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
    // `LauncherSettings.Edition` only ever means "which edition's home
    // screen shows right now" — there's no dual-edition mode to persist.
    // Picking "Both" just means don't force a choice; start on Bedrock,
    // same as the hint text promises ("switch anytime from the home screen").
    const finalEdition = edition === 'both' ? 'bedrock' : edition
    onChange({
      ...settings,
      Edition: finalEdition,
      Username: username,
      AccessibilityMode: accessibilityMode,
      OnboardingCompleted: true,
    })
  }

  function skip() {
    onChange({ ...settings, AccessibilityMode: accessibilityMode, OnboardingCompleted: true })
  }

</script>

<div class="search-overlay">
  <div class="onboarding-box">
    <img class="onboarding-logo" src="/images/icon.png" alt="" />
    <span class="onboarding-title">Welcome to Glacier</span>

    {#if step === 0}
      <span class="onboarding-lede">Let's get you set up. Pick your edition and we'll tailor the launcher to how you play.</span>
      <div class="onboarding-edition-row">
        <button class="onboarding-edition-tile" class:selected={edition === 'bedrock'} onclick={() => pickEdition('bedrock')}>
          <i class="fa-solid fa-cube"></i>
          <span>Bedrock</span>
        </button>
        <button class="onboarding-edition-tile" class:selected={edition === 'java'} onclick={() => pickEdition('java')}>
          <i class="fa-brands fa-java"></i>
          <span>Java</span>
        </button>
        <button class="onboarding-edition-tile" class:selected={edition === 'both'} onclick={() => pickEdition('both')}>
          <span class="onboarding-both-icons">
            <i class="fa-solid fa-cube"></i>
            <i class="fa-brands fa-java"></i>
          </span>
          <span>Both</span>
        </button>
      </div>
      <small class="onboarding-hint">You can switch editions anytime from the home screen.</small>
    {:else if step === 1}
      <span class="onboarding-lede">What should we call you?</span>
      <label class="modal-input-label" for="onboarding-username">Display name</label>
      <input
        id="onboarding-username"
        class="modal-input"
        placeholder={settings.Username || 'Player'}
        bind:value={username}
      />
      <small class="onboarding-hint">Shown in the launcher's footer and recent-launches list. Optional.</small>
    {:else if step === lastStep}
      <span class="onboarding-lede">Turn on accessibility features — reduced motion, stronger focus outlines, and larger click targets.</span>
      <button
        type="button"
        class="onboarding-a11y-toggle"
        class:on={accessibilityMode}
        onclick={() => (accessibilityMode = !accessibilityMode)}
        aria-pressed={accessibilityMode}
      >
        <i class="fa-solid fa-universal-access"></i>
        <span>Accessibility features</span>
        <span class="toggle" class:on={accessibilityMode}></span>
      </button>
      <small class="onboarding-hint">You can change this anytime in Settings.</small>
    {/if}

    <div class="onboarding-actions">
      {#if step > 0}
        <button class="modal-btn modal-btn-cancel" onclick={back}>Back</button>
      {:else}
        <button class="modal-btn modal-btn-cancel" onclick={skip}>Skip</button>
      {/if}
      {#if step < lastStep}
        <button class="onboarding-continue-btn" onclick={next}>Continue</button>
      {:else}
        <button class="onboarding-continue-btn" onclick={finish}>
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

  .onboarding-box {
    width: 460px;
    max-width: calc(100% - 32px);
    background: rgba(35, 39, 42, 0.45);
    backdrop-filter: blur(var(--blur, 14px));
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: var(--r-lg);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.14);
    animation: search-slide-down 0.18s var(--ease-out);
    padding: 40px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    text-align: center;
  }

  .onboarding-logo {
    width: 64px;
    height: 64px;
    border-radius: 16px;
    object-fit: contain;
  }

  .onboarding-title {
    font-family: var(--font-display);
    font-weight: 800;
    font-size: 26px;
    letter-spacing: -0.025em;
    color: var(--text);
  }

  .onboarding-lede {
    font-size: 13.5px;
    color: var(--text-dim);
    max-width: 380px;
    line-height: 1.6;
  }

  .onboarding-hint {
    color: var(--text-dim);
    font-size: 11.5px;
  }

  .onboarding-edition-row {
    display: flex;
    gap: 12px;
    margin-top: 8px;
    width: 100%;
    justify-content: center;
  }

  .onboarding-edition-tile {
    flex: 1;
    min-width: 0;
    max-width: 130px;
    padding: 18px 0;
    border-radius: var(--r-md);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    color: var(--text-dim);
    font-family: inherit;
    transition: background var(--t), border-color var(--t), transform 0.18s cubic-bezier(0.32, 0.72, 0, 1);
  }

  .onboarding-both-icons {
    display: flex;
    gap: 4px;
  }

  .onboarding-both-icons i {
    font-size: 16px;
  }

  .onboarding-edition-tile:hover {
    transform: scale(1.03);
  }

  .onboarding-edition-tile i {
    font-size: 20px;
  }

  .onboarding-edition-tile span {
    font-size: 12.5px;
    font-weight: 700;
    color: var(--text);
  }

  .onboarding-edition-tile.selected {
    background: var(--accent-bg);
    border-color: rgba(114, 137, 218, 0.5);
  }

  .onboarding-edition-tile.selected i {
    color: var(--accent);
  }

  .onboarding-actions {
    display: flex;
    gap: 10px;
    margin-top: 8px;
  }

  .onboarding-continue-btn {
    padding: 11px 30px;
    border-radius: var(--r-sm);
    background: var(--accent);
    border: none;
    color: #fff;
    font-family: inherit;
    font-weight: 700;
    font-size: 13.5px;
    cursor: pointer;
    box-shadow: 0 0 22px var(--accent-glow);
    transition: background var(--t), filter var(--t);
  }

  .onboarding-continue-btn:hover {
    filter: brightness(1.1);
  }

  .onboarding-a11y-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 12px 16px;
    border-radius: var(--r-md);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    cursor: pointer;
    color: var(--text);
    font-family: inherit;
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    transition: background var(--t), border-color var(--t);
  }

  .onboarding-a11y-toggle:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .onboarding-a11y-toggle.on {
    background: var(--accent-bg);
    border-color: rgba(114, 137, 218, 0.5);
  }

  .onboarding-a11y-toggle i {
    font-size: 16px;
    color: var(--text-dim);
    width: 18px;
    text-align: center;
  }

  .onboarding-a11y-toggle.on i {
    color: var(--accent);
  }

  .onboarding-a11y-toggle span:not(.toggle) {
    flex: 1;
  }
</style>
