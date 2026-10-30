// Shared formatting helpers extracted from duplicated logic in several
// *View.svelte files. Each function here matches, byte-for-byte behavior,
// the implementation it replaces — see call sites for provenance comments.

/** Port of Home.razor.cs:677 `FormatFileSize` (Bedrock backups/worlds views). */
export function formatSize(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit++
  }
  return `${value.toFixed(2).replace(/\.?0+$/, '')} ${units[unit]}`
}

/** Port of Home.razor.cs:677 `FormatRelativeTime`. */
export function formatRelativeTime(iso: string): string {
  try {
    const dt = new Date(iso)
    if (Number.isNaN(dt.getTime())) return ''
    const diffMs = Date.now() - dt.getTime()
    const minutes = diffMs / 60000
    if (minutes < 2) return 'just now'
    const hours = minutes / 60
    if (hours < 1) return `${Math.floor(minutes)}m ago`
    const days = hours / 24
    if (days < 1) return `${Math.floor(hours)}h ago`
    if (days < 7) return `${Math.floor(days)}d ago`
    return dt.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  } catch {
    return ''
  }
}

/** Tiny markdown-ish subset (bold, links, line breaks) used to render GitHub
 * release changelogs — shared by NewsView and the update-available modal so
 * "what's in this release" renders identically everywhere it's shown. */
export function formatChangelog(md: string): string {
  const escapeHtml = (s: string) =>
    s
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;')

  let html = escapeHtml(md)
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
  html = html.replace(
    /\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g,
    '<a href="$2" target="_blank" rel="noopener" style="color:var(--accent);text-decoration:underline;cursor:pointer;">$1</a>',
  )
  html = html.replace(/\r\n/g, '<br/>').replace(/\n/g, '<br/>')
  return html
}

/** Shared by Bedrock backups/screenshots/worlds views to recover a folder's
 * parent directory from a listed item's file path, since the corresponding
 * Tauri commands don't expose the containing directory directly. */
export function parentDir(path: string): string {
  const idx = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return idx > 0 ? path.slice(0, idx) : ''
}
