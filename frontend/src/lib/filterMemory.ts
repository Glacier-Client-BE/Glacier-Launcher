// QOL: same localStorage-backed pattern as viewMode.ts, generalized to any
// string preference — panels' category/sort/provider filter chips remember
// their last-used value across restarts instead of always resetting to the
// first option.

const PREFIX = 'glacier.filter.'

export function loadFilterPref(key: string, fallback: string): string {
  try {
    const v = localStorage.getItem(PREFIX + key)
    return v ?? fallback
  } catch {
    return fallback
  }
}

export function saveFilterPref(key: string, value: string): void {
  try {
    localStorage.setItem(PREFIX + key, value)
  } catch {
    // Private browsing / storage disabled — filter still works for the
    // rest of the session, it just won't be remembered next launch.
  }
}
