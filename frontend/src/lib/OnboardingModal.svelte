<script lang="ts">
  import { untrack } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { LauncherSettings } from './settings'
  import { t } from './i18n'

  let { settings, onChange }: { settings: LauncherSettings; onChange: (next: LauncherSettings) => void } =
    $props()

  let step = $state(0)
  let edition = $state('bedrock')
  let username = $state(untrack(() => settings.Username))
  let accessibilityMode = $state(untrack(() => settings.AccessibilityMode))
  let themeId = $state(untrack(() => settings.ActiveThemeId))

  type ThemeSwatch = { Id: string; Name: string; Accent: string; Bg: string; BgPanel: string }
  let themes = $state<ThemeSwatch[]>([])
  async function loadThemes() {
    try {
      themes = await invoke<ThemeSwatch[]>('list_themes')
    } catch {
      themes = []
    }
  }
  loadThemes()

  // Steps: edition, theme (skipped when there's nothing to pick from),
  // username, accessibility. There's no real .minecraft import step — the
  // backend doesn't have that command yet, and we'd rather skip it than show
  // a fake success checkmark. Import from Settings once that lands.
  const STEPS = ['edition', 'theme', 'username', 'a11y'] as const
  const visibleSteps = $derived(STEPS.filter((s) => s !== 'theme' || themes.length > 0))
  const lastStep = $derived(visibleSteps.length - 1)
  const currentStepKey = $derived(visibleSteps[step] ?? 'edition')

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
      ActiveThemeId: themeId,
      OnboardingCompleted: true,
    })
  }

  function skip() {
    onChange({ ...settings, AccessibilityMode: accessibilityMode, OnboardingCompleted: true })
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') skip()
    else if (e.key === 'Enter' && currentStepKey !== 'username' && !(e.target instanceof HTMLButtonElement)) {
      if (step < lastStep) next()
      else finish()
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="search-overlay">
  <div class="onboarding-box" role="dialog" aria-modal="true" aria-labelledby="onboarding-title">
    <img class="onboarding-logo" src="/images/icon.png" alt="" />
    <span class="onboarding-title" id="onboarding-title">Welcome to Glacier</span>

    <div class="onboarding-progress">
      {#each visibleSteps as s, i (s)}
        <span class="onboarding-progress-dot" class:done={i < step} class:active={i === step}></span>
      {/each}
    </div>

    {#if currentStepKey === 'edition'}
      <span class="onboarding-lede">Let's get you set up. Pick your edition and we'll tailor the launcher to how you play.</span>
      <div class="onboarding-edition-row">
        <button class="onboarding-edition-tile" class:selected={edition === 'bedrock'} onclick={() => pickEdition('bedrock')}>
          <img class="chip-icon onboarding-icon-bedrock" src="images/mc-logo.png" alt="">
          <span>Bedrock</span>
        </button>
        <button class="onboarding-edition-tile" class:selected={edition === 'java'} onclick={() => pickEdition('java')}>
          <img class="chip-icon onboarding-icon-java" src="images/mc-java-logo.png" alt="">
          <span>Java</span>
        </button>
        <button class="onboarding-edition-tile" class:selected={edition === 'both'} onclick={() => pickEdition('both')}>
          <span class="onboarding-both-icons">
            <img class="chip-icon onboarding-icon-bedrock" src="images/mc-logo.png" alt="">
            <img class="chip-icon onboarding-icon-java" src="images/mc-java-logo.png" alt="">
          </span>
          <span>{t('x.both', settings.Language)}</span>
        </button>
      </div>
      <small class="onboarding-hint">{t('x.youCanSwitchEditionsAnytimeFromTheHomeSc', settings.Language)}</small>
    {:else if currentStepKey === 'theme'}
      <span class="onboarding-lede">Pick a look. You can fine-tune every color later in Theme Studio.</span>
      <div class="onboarding-theme-row">
        <button
          type="button"
          class="onboarding-theme-tile"
          class:selected={themeId === ''}
          onclick={() => (themeId = '')}
        >
          <span class="onboarding-theme-swatch onboarding-theme-swatch-default">
            <span class="onboarding-theme-swatch-accent"></span>
          </span>
          <span>{t('label.default', settings.Language)}</span>
        </button>
        {#each themes as theme (theme.Id)}
          <button
            type="button"
            class="onboarding-theme-tile"
            class:selected={themeId === theme.Id}
            onclick={() => (themeId = theme.Id)}
            title={theme.Name}
          >
            <span class="onboarding-theme-swatch" style="background:{theme.Bg};">
              <span style="background:{theme.Accent};"></span>
            </span>
            <span>{theme.Name}</span>
          </button>
        {/each}
      </div>
      <small class="onboarding-hint">You can switch themes anytime in Settings → Theme Studio.</small>
    {:else if currentStepKey === 'username'}
      <span class="onboarding-lede">What should we call you?</span>
      <label class="modal-input-label" for="onboarding-username">{t('label.displayName', settings.Language)}</label>
      <input
        id="onboarding-username"
        class="modal-input"
        placeholder={settings.Username || 'Player'}
        bind:value={username}
      />
      <small class="onboarding-hint">Shown in the launcher's footer and recent-launches list. Optional.</small>
    {:else if currentStepKey === 'a11y'}
      <span class="onboarding-lede">Turn on accessibility features — reduced motion, stronger focus outlines, and larger click targets.</span>
      <button
        type="button"
        class="onboarding-a11y-toggle"
        class:on={accessibilityMode}
        onclick={() => (accessibilityMode = !accessibilityMode)}
        aria-pressed={accessibilityMode}
      >
        <i class="fa-solid fa-universal-access"></i>
        <span>{t('x.accessibilityFeatures', settings.Language)}</span>
        <span class="toggle" class:on={accessibilityMode}></span>
      </button>
      <small class="onboarding-hint">{t('x.youCanChangeThisAnytimeInSettings', settings.Language)}</small>
    {/if}

    <div class="onboarding-actions">
      {#if step > 0}
        <button class="modal-btn modal-btn-cancel" onclick={back}>{t('common.back', settings.Language)}</button>
      {:else}
        <button class="modal-btn modal-btn-cancel" onclick={skip}>{t('x.skip', settings.Language)}</button>
      {/if}
      {#if step < lastStep}
        <button class="onboarding-continue-btn" onclick={next}>{t('x.continue', settings.Language)}</button>
      {:else}
        <button class="onboarding-continue-btn" onclick={finish}>
          <i class="fa-solid fa-check"></i> Finish
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  /* Solid panel surfaces only: backdrop-filter blur is too costly on the N5100. */
  .search-overlay {
    position: absolute;
    inset: 0;
    z-index: 60;
    background: color-mix(in srgb, var(--bg-inset) 80%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    animation: search-fade-in 0.15s ease;
  }

  .onboarding-box {
    box-sizing: border-box;
    width: 460px;
    max-width: calc(100% - 32px);
    padding: 36px 40px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    text-align: center;
    background: var(--bg-panel);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-sm);
    animation: search-slide-down 0.18s var(--ease-out);
  }

  @media (prefers-reduced-motion: reduce) {
    .search-overlay,
    .onboarding-box {
      animation: none;
    }
  }

  .onboarding-logo {
    width: 64px;
    height: 64px;
    border-radius: var(--r-sm);
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
    max-width: 380px;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-dim);
  }

  .onboarding-hint {
    font-size: 11px;
    color: var(--text-dim);
  }

  .onboarding-progress {
    display: flex;
    gap: 6px;
    margin-top: -4px;
  }

  .onboarding-progress-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--bg-item-hover);
    transition: background var(--t), width var(--t);
  }

  .onboarding-progress-dot.done {
    background: var(--accent);
  }

  .onboarding-progress-dot.active {
    width: 16px;
    border-radius: var(--r-pill);
    background: var(--accent);
  }

  .onboarding-icon-bedrock {
    width: 14px;
    height: 14px;
    object-fit: contain;
  }

  .onboarding-icon-java {
    width: 18px;
    height: 18px;
    object-fit: contain;
  }

  .onboarding-edition-row {
    display: flex;
    gap: 12px;
    margin-top: 4px;
    width: 100%;
    justify-content: center;
  }

  .onboarding-edition-tile {
    flex: 1;
    min-width: 0;
    max-width: 130px;
    padding: 18px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    background: var(--bg-item);
    border: 1px solid transparent;
    border-radius: var(--r-md);
    cursor: pointer;
    color: var(--text-dim);
    font-family: inherit;
    transition: background var(--t), border-color var(--t), transform 0.18s cubic-bezier(0.32, 0.72, 0, 1);
  }

  .onboarding-edition-tile:hover {
    background: var(--bg-item-hover);
    transform: scale(1.03);
  }

  .onboarding-edition-tile:focus-visible,
  .onboarding-theme-tile:focus-visible,
  .onboarding-a11y-toggle:focus-visible,
  .onboarding-continue-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .onboarding-edition-tile i {
    font-size: 20px;
  }

  .onboarding-edition-tile span {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .onboarding-edition-tile.selected {
    background: var(--accent-bg);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }

  .onboarding-edition-tile.selected i {
    color: var(--accent);
  }

  .onboarding-both-icons {
    display: flex;
    gap: 4px;
  }

  .onboarding-both-icons i {
    font-size: 16px;
  }

  .onboarding-theme-row {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 10px;
    margin-top: 4px;
    width: 100%;
  }

  .onboarding-theme-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 8px;
    background: var(--bg-item);
    border: 1px solid transparent;
    border-radius: var(--r-md);
    cursor: pointer;
    color: var(--text-dim);
    font-family: inherit;
    font-size: 11px;
    font-weight: 600;
    transition: background var(--t), border-color var(--t), transform 0.18s cubic-bezier(0.32, 0.72, 0, 1);
  }

  .onboarding-theme-tile:hover {
    background: var(--bg-item-hover);
    transform: scale(1.03);
  }

  .onboarding-theme-tile.selected {
    background: var(--accent-bg);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
    color: var(--text);
  }

  .onboarding-theme-swatch {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    padding: 4px;
    border-radius: var(--r-sm);
    border: 1px solid var(--bg-item-hover);
  }

  .onboarding-theme-swatch-default {
    background: var(--bg-panel);
  }

  .onboarding-theme-swatch span {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid var(--bg-item-hover);
  }

  .onboarding-theme-swatch-accent {
    background: var(--accent);
  }

  .onboarding-actions {
    display: flex;
    gap: 10px;
    margin-top: 8px;
  }

  .onboarding-continue-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 40px;
    padding: 0 28px;
    background: var(--accent);
    border: none;
    border-radius: var(--r-sm);
    box-shadow: var(--shadow-sm);
    color: var(--bg-inset);
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: background var(--t), opacity var(--t);
  }

  .onboarding-continue-btn:hover {
    background: var(--accent-hover);
  }

  .onboarding-a11y-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    padding: 12px 16px;
    background: var(--bg-item);
    border: 1px solid transparent;
    border-radius: var(--r-md);
    cursor: pointer;
    color: var(--text);
    font-family: inherit;
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    transition: background var(--t), border-color var(--t);
  }

  .onboarding-a11y-toggle:hover {
    background: var(--bg-item-hover);
  }

  .onboarding-a11y-toggle.on {
    background: var(--accent-bg);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }

  .onboarding-a11y-toggle i {
    width: 18px;
    font-size: 16px;
    color: var(--text-dim);
    text-align: center;
  }

  .onboarding-a11y-toggle.on i {
    color: var(--accent);
  }

  .onboarding-a11y-toggle span:not(.toggle) {
    flex: 1;
  }
</style>
