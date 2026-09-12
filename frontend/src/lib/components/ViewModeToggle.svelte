<script lang="ts">
  // New addition — no counterpart in the original WPF app. A two-button
  // segmented control for switching a list panel between its row layout and
  // a grid of cards, in the spirit of the toggle in Pepe's Portfolio (a
  // `view-toggle` pair of icon buttons driving a `list-view` class flip).
  import type { ViewMode } from '../viewMode'

  let { mode, onChange }: { mode: ViewMode; onChange: (m: ViewMode) => void } = $props()
</script>

<div class="view-mode-toggle" role="group" aria-label="Toggle view layout">
  <button
    type="button"
    class="view-mode-btn {mode === 'list' ? 'active' : ''}"
    onclick={() => onChange('list')}
    onmousedown={(e) => e.stopPropagation()}
    data-tooltip="List view"
    aria-label="List view"
    aria-pressed={mode === 'list'}
  >
    <i class="fa-solid fa-list"></i>
  </button>
  <button
    type="button"
    class="view-mode-btn {mode === 'grid' ? 'active' : ''}"
    onclick={() => onChange('grid')}
    onmousedown={(e) => e.stopPropagation()}
    data-tooltip="Grid view"
    aria-label="Grid view"
    aria-pressed={mode === 'grid'}
  >
    <i class="fa-solid fa-table-cells"></i>
  </button>
</div>

<!-- .view-mode-toggle / .view-mode-btn are defined in app.css, alongside
     .version-list.grid-view (the shared grid layout every consumer of this
     toggle opts into by wrapping its rows in `.version-list`). -->
