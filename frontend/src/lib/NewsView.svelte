<script lang="ts">
  import { t } from './i18nStore.svelte'
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
  import { invokeCached } from './invokeCache'
  import ListSkeleton from './components/ListSkeleton.svelte'
  import { formatChangelog } from './format'

  const RELEASES_TTL_MS = 10 * 60 * 1000

  let { onBack }: {
    onBack?: () => void
  } = $props()

  type NewsPost = { icon: string; title: string; subtitle: string; url: string; date: string }
  type Announcement = { id: string; title: string; message: string; kind: string; url: string }
  type LauncherRelease = { tag: string; published_at: string; changelog: string }

  let posts = $state<NewsPost[]>([])
  let announcement = $state<Announcement | null>(null)
  let launcherReleases = $state<LauncherRelease[]>([])
  let clientReleases = $state<LauncherRelease[]>([])
  let glacierReleases = $state<LauncherRelease[]>([])
  /** Redesign: `.news-row`s restyled as `.client-card`s, single column (not
   *  a multi-column grid — these are read top-to-bottom, not scanned tile by
   *  tile) with a real expanding body for the changelog entries. The
   *  changelog source picker stays tabs (`.mcv-channel-tabs`, one source
   *  shown at a time), same as before the redesign. */
  type Section = 'glacier' | 'latite' | 'launcher'
  let changelogTab = $state<Section>('glacier')
  let expandedTag = $state<string | null>(null)
  let loading = $state(true)

  const CHANGELOG_SECTIONS: { key: Section; label: string }[] = [
    { key: 'glacier', label: 'Glacier' },
    { key: 'latite', label: 'Latite' },
    { key: 'launcher', label: 'Launcher' },
  ]

  const releases = $derived(
    changelogTab === 'glacier' ? glacierReleases :
    changelogTab === 'latite' ? clientReleases :
    launcherReleases
  )

  /**
   * Port of `OpenUrl` (Home.razor.cs:670). `window.open` is inert inside the
   * WebView2 host, so this goes through the `open_url` command.
   */
  function openUrl(url: string) {
    if (url) invoke('open_url', { url }).catch(() => {})
  }


  function formatDate(iso: string): string {
    if (!iso) return ''
    const d = new Date(iso)
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString()
  }

  function toggleRelease(tag: string) {
    expandedTag = expandedTag === tag ? null : tag
  }

  function selectChangelogTab(s: Section) {
    changelogTab = s
    expandedTag = null
  }

  async function load() {
    loading = true
    try {
      const [news, ann, glacierRel, clientRel, launcherRel] = await Promise.all([
        invoke<NewsPost[] | null>('get_news').catch(() => null),
        invoke<Announcement | null>('get_announcement').catch(() => null),
        invokeCached<LauncherRelease[]>('list_glacier_client_releases', { max: 12 }, RELEASES_TTL_MS).catch(() => []),
        invokeCached<LauncherRelease[]>('list_client_releases', { max: 12 }, RELEASES_TTL_MS).catch(() => []),
        invokeCached<LauncherRelease[]>('list_launcher_releases', { max: 12 }, RELEASES_TTL_MS).catch(() => []),
      ])
      posts = news ?? []
      announcement = ann
      glacierReleases = glacierRel ?? []
      clientReleases = clientRel ?? []
      launcherReleases = launcherRel ?? []
    } finally {
      loading = false
    }
  }

  onMount(load)
</script>

<style>
  .news-announce {
    margin: 0 0 12px;
  }
  .news-card-col {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .client-card-link {
    cursor: pointer;
  }
  .client-card:focus-visible,
  .news-row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .news-icon-accent {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .news-ext {
    opacity: 0.5;
    flex-shrink: 0;
    font-size: 12px;
  }
  .news-date {
    font-size: 11px;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .news-changelog-body {
    max-height: 260px;
    overflow: auto;
    white-space: normal;
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--bg-item-hover);
  }
</style>

<PanelHeader title={t('panel.newsAndUpdates')} {onBack}>
  {#snippet extraActions()}
    <button type="button" class="panel-icon-btn" class:spinning={loading} disabled={loading} data-tooltip={t('tt.refresh')} aria-label={t('tt.refresh')} onclick={load}>
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if loading}
    <ListSkeleton rows={3} />
  {:else}
    {#if announcement}
      <div class="news-row news-announce" role="link" tabindex="0" onclick={() => announcement && openUrl(announcement.url)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); announcement && openUrl(announcement.url) } }}>
        <div class="news-icon"><i class="fa-solid fa-bullhorn"></i></div>
        <div class="news-meta">
          <span class="news-title">{announcement.title}</span>
          <span class="news-sub">{announcement.message}</span>
        </div>
        {#if announcement.url}<i class="fa-solid fa-arrow-up-right-from-square news-ext"></i>{/if}
      </div>
    {/if}

    <span class="panel-section-label">{t('x.latest')}</span>
    {#if posts.length > 0}
      <div class="news-card-col">
        {#each posts as p}
          <div
            class="client-card client-card-link"
            role="link"
            tabindex="0"
            onclick={() => openUrl(p.url)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openUrl(p.url) } }}
          >
            <div class="client-card-header">
              <div class="client-card-icon news-icon-accent">
                <i class={p.icon || 'fa-solid fa-newspaper'}></i>
              </div>
              <div class="client-card-meta">
                <span class="client-card-name">{p.title}</span>
                <span class="client-card-sub">{p.subtitle}</span>
              </div>
              {#if p.url}<i class="fa-solid fa-arrow-up-right-from-square news-ext"></i>{/if}
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>No news available (offline or rate-limited).</span></div>
    {/if}

    <span class="panel-section-label">{t('x.changelog')}</span>
    <div class="mcv-channel-tabs">
      {#each CHANGELOG_SECTIONS as cs (cs.key)}
        <button
          type="button"
          class="mcv-channel-tab {changelogTab === cs.key ? 'active' : ''}"
          onclick={() => selectChangelogTab(cs.key)}
        >{cs.label}</button>
      {/each}
    </div>

    {#if releases.length > 0}
      <div class="news-card-col">
        {#each releases as r (r.tag)}
          <div
            class="client-card client-card-link"
            role="button"
            tabindex="0"
            aria-expanded={expandedTag === r.tag}
            onclick={() => toggleRelease(r.tag)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleRelease(r.tag) } }}
          >
            <div class="client-card-header">
              <div class="client-card-icon news-icon-accent">
                <i class="fa-solid fa-code-branch"></i>
              </div>
              <div class="client-card-meta">
                <span class="client-card-name">{r.tag}</span>
                <span class="client-card-sub news-date">{formatDate(r.published_at)}</span>
              </div>
              <i class="fa-solid {expandedTag === r.tag ? 'fa-chevron-up' : 'fa-chevron-down'} news-ext"></i>
            </div>
            {#if expandedTag === r.tag}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              <p class="client-card-desc news-changelog-body">{@html formatChangelog(r.changelog)}</p>
            {/if}
          </div>
        {/each}
      </div>
    {:else}
      <div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>No {changelogTab === 'glacier' ? 'Glacier Client' : changelogTab === 'latite' ? 'Latite' : 'launcher'} changelog available.</span></div>
    {/if}
  {/if}
</div>
