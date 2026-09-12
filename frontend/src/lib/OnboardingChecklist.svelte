<script lang="ts">
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

  function dismiss() {
    onChange({ ...settings, OnboardingChecklistDismissed: true })
  }

  function goTo(item: ChecklistItem) {
    if (item.view) onNavigate?.(item.view)
  }
</script>

<div class="onboarding-checklist-card">
  <div class="onboarding-checklist-header">
    <span class="onboarding-checklist-title">
      <i class="fa-solid fa-list-check"></i> Get started ({doneCount}/{items.length})
    </span>
    <button
      type="button"
      class="onboarding-checklist-dismiss"
      onclick={dismiss}
      onmousedown={(e) => e.stopPropagation()}
      aria-label="Dismiss checklist"
    >
      <i class="fa-solid fa-xmark"></i>
    </button>
  </div>
  <div class="onboarding-checklist-items">
    {#each items as item (item.key)}
      {#if item.view}
        <button
          type="button"
          class="onboarding-checklist-item {item.done ? 'done' : ''}"
          onclick={() => goTo(item)}
          onmousedown={(e) => e.stopPropagation()}
          data-tooltip={item.done ? undefined : `Go to ${item.label}`}
        >
          <i class={item.done ? 'fa-solid fa-circle-check' : 'fa-regular fa-circle'}></i>
          <i class={item.icon}></i>
          <span>{item.label}</span>
        </button>
      {:else}
        <div class="onboarding-checklist-item {item.done ? 'done' : ''}">
          <i class={item.done ? 'fa-solid fa-circle-check' : 'fa-regular fa-circle'}></i>
          <i class={item.icon}></i>
          <span>{item.label}</span>
        </div>
      {/if}
    {/each}
  </div>
</div>

<style>
  /* N5100-safe: opacity/transform-only, no box-shadow/filter animation.
     Colors reuse the existing Theme Studio custom properties. */
  .onboarding-checklist-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--r-md);
    padding: 10px 14px;
    margin: 8px 0;
    animation: onboarding-checklist-in 0.18s var(--ease-out);
  }

  @keyframes onboarding-checklist-in {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .onboarding-checklist-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .onboarding-checklist-title {
    font-size: 12.5px;
    font-weight: 700;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .onboarding-checklist-title i { color: var(--accent); }

  .onboarding-checklist-dismiss {
    background: transparent;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    padding: 2px 6px;
    transition: color var(--t);
  }

  .onboarding-checklist-dismiss:hover { color: var(--text); }

  .onboarding-checklist-items {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
  }

  .onboarding-checklist-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--text-dim);
    background: transparent;
    border: none;
    padding: 2px 4px;
    margin: -2px -4px;
    border-radius: 6px;
    font-family: inherit;
  }

  button.onboarding-checklist-item {
    cursor: pointer;
    transition: background var(--t), color var(--t);
  }

  button.onboarding-checklist-item:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text);
  }

  .onboarding-checklist-item.done {
    color: var(--text);
  }

  .onboarding-checklist-item i:first-child {
    color: var(--text-dim);
    font-size: 12px;
  }

  .onboarding-checklist-item.done i:first-child {
    color: var(--accent);
  }
</style>
