<script lang="ts">
  // Ported from the "news" panel in Home.razor (lines ~5407-5493) /
  // Home.Announcement.cs + NewsService.cs. Uses the `get_news` /
  // `get_announcement` commands that already exist for the home-screen
  // ticker (src-tauri/src/commands/integrations.rs) — this gives them a
  // full list view instead of a scrolling one-line strip.
  //
  // Changelog section uses the new `list_launcher_releases` command
  // (src-tauri/src/commands/update.rs, backed by gl-update's
  // UpdateChecker::recent_releases — a port of
  // AutoUpdateService.GetRecentReleasesAsync). `formatChangelog` below is a
  // direct port of Home.razor.cs's `FormatChangelog`: HTML-escape, then
  // regex-convert **bold** and [text](url) markdown to <strong>/<a>, then
  // newlines to <br/>.
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import PanelHeader from './components/PanelHeader.svelte'
  import PanelTabs from './components/PanelTabs.svelte'
  import Spinner from './components/Spinner.svelte'

  let { onBack, onOpenSettings, onOpenCredits }: {
    onBack?: () => void
    onOpenSettings?: () => void
    onOpenCredits?: () => void
  } = $props()

  type NewsPost = { icon: string; title: string; subtitle: string; url: string; date: string }
  type Announcement = { id: string; title: string; message: string; kind: string; url: string }
  type LauncherRelease = { tag: string; published_at: string; changelog: string }

  let posts = $state<NewsPost[]>([])
  let announcement = $state<Announcement | null>(null)
  let releases = $state<LauncherRelease[]>([])
  let expandedTag = $state<string | null>(null)
  let loading = $state(true)

  /**
   * Port of `OpenUrl` (Home.razor.cs:670). `window.open` is inert inside the
   * WebView2 host, so this goes through the `open_url` command.
   */
  function openUrl(url: string) {
    if (url) invoke('open_url', { url }).catch(() => {})
  }

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;')
  }

  function formatChangelog(md: string): string {
    let html = escapeHtml(md)
    html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    html = html.replace(
      /\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g,
      '<a href="$2" target="_blank" rel="noopener" style="color:var(--accent);text-decoration:underline;cursor:pointer;">$1</a>',
    )
    html = html.replace(/\r\n/g, '<br/>').replace(/\n/g, '<br/>')
    return html
  }

  function formatDate(iso: string): string {
    if (!iso) return ''
    const d = new Date(iso)
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString()
  }

  function toggleRelease(tag: string) {
    expandedTag = expandedTag === tag ? null : tag
  }

  async function load() {
    loading = true
    try {
      const [news, ann, rel] = await Promise.all([
        invoke<NewsPost[] | null>('get_news'),
        invoke<Announcement | null>('get_announcement'),
        invoke<LauncherRelease[]>('list_launcher_releases', { max: 12 }).catch(() => []),
      ])
      posts = news ?? []
      announcement = ann
      releases = rel ?? []
    } finally {
      loading = false
    }
  }

  onMount(load)
</script>

<PanelHeader title="News & Updates" {onBack}>
  {#snippet extraActions()}
    <button type="button" class="panel-icon-btn" class:spinning={loading} disabled={loading} data-tooltip="Refresh" aria-label="Refresh" onclick={load}>
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if loading}
    <div class="versions-loading"><Spinner /><span>Loading news…</span></div>
  {:else}
    {#if announcement}
      <span class="panel-section-label">Announcement</span>
      <div class="news-row" role="link" tabindex="0" onclick={() => announcement && openUrl(announcement.url)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); announcement && openUrl(announcement.url) } }}>
        <div class="news-icon"><i class="fa-solid fa-bullhorn"></i></div>
        <div class="news-meta">
          <span class="news-title">{announcement.title}</span>
          <span class="news-sub">{announcement.message}</span>
        </div>
        {#if announcement.url}<i class="fa-solid fa-arrow-up-right-from-square news-ext"></i>{/if}
      </div>
    {/if}

    <span class="panel-section-label">Latest</span>
    {#if posts.length > 0}
      {#each posts as p}
        <div class="news-row" role="link" tabindex="0" onclick={() => openUrl(p.url)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openUrl(p.url) } }}>
          <div class="news-icon"><i class={p.icon || 'fa-solid fa-newspaper'}></i></div>
          <div class="news-meta">
            <span class="news-title">{p.title}</span>
            <span class="news-sub">{p.subtitle}</span>
          </div>
          {#if p.url}<i class="fa-solid fa-arrow-up-right-from-square news-ext"></i>{/if}
        </div>
      {/each}
    {:else}
      <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>No news available (offline or rate-limited).</span></div>
    {/if}

    {#if releases.length > 0}
      <span class="panel-section-label">Changelog</span>
      {#each releases as r (r.tag)}
        <div class="news-row" role="button" tabindex="0" onclick={() => toggleRelease(r.tag)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleRelease(r.tag) } }}>
          <div class="news-icon"><i class="fa-solid fa-code-branch"></i></div>
          <div class="news-meta">
            <span class="news-title">{r.tag}</span>
            <span class="news-sub">{formatDate(r.published_at)}</span>
            {#if expandedTag === r.tag}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              <div class="news-changelog-body">{@html formatChangelog(r.changelog)}</div>
            {/if}
          </div>
          <i class="fa-solid {expandedTag === r.tag ? 'fa-chevron-up' : 'fa-chevron-down'} news-ext"></i>
        </div>
      {/each}
    {/if}
  {/if}
</div>

<PanelTabs
  tabs={[
    { id: 'settings', label: 'Settings', icon: 'fa-solid fa-gear' },
    { id: 'home', label: 'Home', icon: 'fa-solid fa-house' },
    { id: 'news', label: 'News', icon: 'fa-solid fa-newspaper' },
    { id: 'credits', label: 'Credits', icon: 'fa-solid fa-heart' },
  ]}
  active="news"
  onSelect={(id) => {
    if (id === 'settings') onOpenSettings?.()
    else if (id === 'home') onBack?.()
    else if (id === 'credits') onOpenCredits?.()
  }}
/>
