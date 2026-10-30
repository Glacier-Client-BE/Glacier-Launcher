<script lang="ts">
  import { t } from './i18nStore.svelte'
  // Feature 42 (UI polish pass): a small dismissible first-run checklist,
  // separate from the OnboardingModal wizard (which only runs once and
  // collects edition/username/accessibility). This card appears once the
  // wizard is done and stays until the user dismisses it, checking off
  // items as the corresponding settings state becomes true — no separate
  // "mark done" action needed.
  import type { LauncherSettings } from './settings'

  let {
    settings,
    onChange,
    onNavigate,
  }: {
    settings: LauncherSettings
    onChange: (next: LauncherSettings) => void
    onNavigate?: (view: string) => void
  } = $props()

  type ChecklistItem = { key: string; label: string; icon: string; done: boolean; view?: string }

  const items = $derived<ChecklistItem[]>([
    {
      key: 'xbox',
      label: 'Connect Xbox account',
      icon: 'fa-brands fa-xbox',
      done: !!settings.XboxGamertag,
      view: 'settings',
    },
    {
      key: 'server',
      label: 'Add a server',
      icon: 'fa-solid fa-server',
      done: (settings.SavedServers?.length ?? 0) > 0,
      view: 'servers',
    },
    {
      key: 'client',
      label: 'Install a client',
      icon: 'fa-solid fa-puzzle-piece',
      done: !!settings.SelectedClient && settings.SelectedClient !== 'Vanilla',
      view: 'clients',
    },
    {
      key: 'launch',
      label: 'Launch the game',
      icon: 'fa-solid fa-play',
      done: settings.TotalPlaytimeSeconds > 0,
    },
  ])

  const doneCount = $derived(items.filter((i) => i.done).length)
  const allDone = $derived(doneCount === items.length)
  const nextKey = $derived(items.find((i) => !i.done)?.key ?? null)
  const progressScale = $derived(items.length ? doneCount / items.length : 0)

  function dismiss() {
    onChange({ ...settings, OnboardingChecklistDismissed: true })
  }

  function goTo(item: ChecklistItem) {
    if (item.view) onNavigate?.(item.view)
  }
</script>

<div class="onboarding-checklist-card" role="region" aria-label="Get started">
  <div class="onboarding-checklist-header">
    <span class="onboarding-checklist-title">
      {#if allDone}
        <i class="fa-solid fa-circle-check"></i> All set
      {:else}
        <i class="fa-solid fa-list-check"></i> Get started
      {/if}
      <span class="onboarding-checklist-count">{doneCount} of {items.length}</span>
    </span>
    {#if allDone}
      <button
        type="button"
        class="onboarding-checklist-dismiss"
        onclick={dismiss}
        onmousedown={(e) => e.stopPropagation()}
        aria-label={t('tt.dismissChecklist')}
      >
        <i class="fa-solid fa-xmark"></i>
      </button>
    {/if}
  </div>
  <div class="onboarding-checklist-progress" aria-hidden="true">
    <span class="onboarding-checklist-progress-bar" style="transform: scaleX({progressScale})"></span>
  </div>
  <div class="onboarding-checklist-items">
    {#each items as item (item.key)}
      {#if item.view}
        <button
          type="button"
          class="onboarding-checklist-item {item.done ? 'done' : ''} {item.key === nextKey ? 'next' : ''}"
          onclick={() => goTo(item)}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip={item.done ? undefined : `Go to ${item.label}`}
        >
          <i class={item.done ? 'fa-solid fa-circle-check' : 'fa-regular fa-circle'}></i>
          <i class={item.icon}></i>
          <span class="onboarding-checklist-label">{item.label}</span>
          <span class="onboarding-checklist-state">{item.done ? 'Done' : 'To do'}</span>
        </button>
      {:else}
        <div class="onboarding-checklist-item {item.done ? 'done' : ''} {item.key === nextKey ? 'next' : ''}">
          <i class={item.done ? 'fa-solid fa-circle-check' : 'fa-regular fa-circle'}></i>
          <i class={item.icon}></i>
          <span class="onboarding-checklist-label">{item.label}</span>
          <span class="onboarding-checklist-state">{item.done ? 'Done' : 'To do'}</span>
        </div>
      {/if}
    {/each}
  </div>
</div>

<style>
  /* N5100-safe: opacity/transform-only motion, no box-shadow or filter animation. */
  .onboarding-checklist-card {
    margin: 8px 0;
    padding: var(--card-pad-y) var(--card-pad-x);
    background: var(--bg-item);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-sm);
    animation: onboarding-checklist-in 0.18s var(--ease-out);
  }

  @keyframes onboarding-checklist-in {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  @media (prefers-reduced-motion: reduce) {
    .onboarding-checklist-card {
      animation: none;
    }
  }

  .onboarding-checklist-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .onboarding-checklist-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .onboarding-checklist-title i {
    color: var(--accent-hover);
  }

  .onboarding-checklist-dismiss {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: var(--r-pill);
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    transition: background var(--t), color var(--t);
  }

  .onboarding-checklist-dismiss:hover {
    background: var(--bg-item-hover);
    color: var(--text);
  }

  .onboarding-checklist-dismiss:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .onboarding-checklist-count {
    margin-left: 6px;
    font-weight: 500;
    font-size: 11px;
    color: var(--text-dim);
  }

  .onboarding-checklist-progress {
    height: 4px;
    margin-bottom: 10px;
    background: var(--bg-inset);
    border-radius: var(--r-pill);
    overflow: hidden;
  }

  .onboarding-checklist-progress-bar {
    display: block;
    width: 100%;
    height: 100%;
    background: var(--green);
    border-radius: var(--r-pill);
    transform-origin: left center;
    transition: transform 0.3s var(--ease-out);
  }

  @media (prefers-reduced-motion: reduce) {
    .onboarding-checklist-progress-bar {
      transition: none;
    }
  }

  .onboarding-checklist-items {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .onboarding-checklist-item.next {
    background: color-mix(in srgb, var(--accent) 16%, var(--bg-inset));
    color: var(--text);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .onboarding-checklist-item.next i:first-child {
    color: var(--accent-hover);
  }

  .onboarding-checklist-item {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 6px 10px;
    background: var(--bg-inset);
    border: none;
    border-radius: var(--r-sm);
    color: var(--text-dim);
    font-family: inherit;
    font-size: 12px;
  }

  button.onboarding-checklist-item {
    cursor: pointer;
    transition: background var(--t), color var(--t);
  }

  button.onboarding-checklist-item:hover {
    background: var(--bg-item-hover);
    color: var(--text);
  }

  button.onboarding-checklist-item:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .onboarding-checklist-item.done {
    color: var(--text);
  }

  .onboarding-checklist-item i:first-child {
    color: var(--text-dim);
    font-size: 12px;
  }

  .onboarding-checklist-item.done i:first-child {
    color: var(--green);
  }

  .onboarding-checklist-state {
    font-size: 11px;
    color: var(--text-dim);
  }

  .onboarding-checklist-item.done .onboarding-checklist-state {
    color: var(--green);
    font-weight: 600;
  }
</style>
