// New addition — no counterpart in the original WPF app, which had no
// grid/list switching anywhere. Small localStorage-backed helper so each
// panel that opts into a grid/list toggle (see ViewModeToggle.svelte) can
// remember its own choice across sessions without wiring settings.json for
// what is purely a per-viewer display preference.

export type ViewMode = 'grid' | 'list'

const PREFIX = 'glacier.viewMode.'

export function loadViewMode(key: string, fallback: ViewMode = 'list'): ViewMode {
  try {
    const v = localStorage.getItem(PREFIX + key)
    return v === 'grid' || v === 'list' ? v : fallback
  } catch {
    return fallback
  }
}

export function saveViewMode(key: string, mode: ViewMode): void {
  try {
    localStorage.setItem(PREFIX + key, mode)
  } catch {
    // Private browsing / storage disabled — the toggle still works for the
    // rest of the session, it just won't be remembered next launch.
  }
}
