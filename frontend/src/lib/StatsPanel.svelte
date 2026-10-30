<script lang="ts">
  import { t } from './i18nStore.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import ListSkeleton from './components/ListSkeleton.svelte'

  // Ported from Components/StatsPanel.razor + Services/StatsService.cs.
  let { onBack }: { onBack?: () => void } = $props()

  type PlaySession = { started_at: string; seconds: number; edition: string; label: string }
  type StatsSummary = {
    total_seconds: number
    session_count: number
    longest_session_seconds: number
    daily_totals: [string, number][]
    recent_sessions: PlaySession[]
    top_labels: [string, number][]
  }

  let stats = $state<StatsSummary | null>(null)
  let loading = $state(true)
  let error = $state('')

  function fmtDuration(seconds: number): string {
    const h = Math.floor(seconds / 3600)
    const m = Math.floor((seconds % 3600) / 60)
    if (h > 0) return `${h}h ${m}m`
    return `${m}m`
  }

  function fmtDay(iso: string): string {
    const [, m, d] = iso.split('-')
    return `${d}/${m}`
  }

  async function load() {
    loading = true
    error = ''
    try {
      stats = await invoke<StatsSummary>('get_stats_summary', { days: 14, recentCount: 20 })
    } catch (e) {
      error = String(e)
    } finally {
      loading = false
    }
  }

  load()
</script>

<PanelHeader title={t('panel.stats')} {onBack}>
  {#snippet extraActions()}
    <button type="button" class="panel-icon-btn {loading ? 'spinning' : ''}" onclick={load} data-tooltip={t('tt.refresh')} aria-label={t('tt.refresh')}>
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if loading}
    <ListSkeleton rows={3} />
  {:else if error}
    <span class="error-text">{error}</span>
  {:else if !stats || stats.session_count === 0}
    <div class="empty-state">
      <i class="fa-solid fa-chart-simple"></i>
      <span>{t('empty.noPlaySessionsRecordedYet')}</span>
    </div>
  {:else}
    <div class="stats-cards">
      <div class="stats-card">
        <i class="fa-solid fa-clock"></i>
        <span class="stats-val">{fmtDuration(stats.total_seconds)}</span>
        <span class="stats-label">{t('x.totalPlayed')}</span>
      </div>
      <div class="stats-card">
        <i class="fa-solid fa-gamepad"></i>
        <span class="stats-val">{stats.session_count}</span>
        <span class="stats-label">{t('x.sessions')}</span>
      </div>
      <div class="stats-card">
        <i class="fa-solid fa-hourglass-half"></i>
        <span class="stats-val">{fmtDuration(stats.longest_session_seconds)}</span>
        <span class="stats-label">{t('x.longestSession')}</span>
      </div>
      <div class="stats-card">
        <i class="fa-solid fa-star"></i>
        <span class="stats-val">{stats.top_labels[0]?.[0] ?? '—'}</span>
        <span class="stats-label">{t('x.mostPlayed')}</span>
      </div>
    </div>

    {#if stats.top_labels.length > 0}
      <span class="panel-section-label">{t('x.byInstanceVersion')}</span>
      <div class="stats-category-list">
        {#each stats.top_labels as [label, count] (label)}
          {@const maxCount = Math.max(...stats.top_labels.map(([, c]) => c), 1)}
          {@const pct = Math.round((count / maxCount) * 100)}
          <div class="stats-category-row">
            <span class="stats-category-name">{label}</span>
            <div class="stats-category-bar-track"><div class="stats-category-bar-fill" style="width:{pct}%"></div></div>
            <span class="stats-category-count">{count} {count === 1 ? 'session' : 'sessions'}</span>
          </div>
        {/each}
      </div>
    {/if}

    <span class="panel-section-label">{t('x.last14Days')}</span>
    <div class="stats-chart">
      {#each stats.daily_totals as [day, seconds] (day)}
        {@const maxSeconds = Math.max(...stats.daily_totals.map(([, s]) => s), 1)}
        <div class="stats-bar-col">
          <div class="stats-bar-track">
            <div class="stats-bar-fill" style="height:{Math.max((seconds / maxSeconds) * 100, seconds > 0 ? 4 : 0)}%;" title={fmtDuration(seconds)}></div>
          </div>
          <span class="stats-bar-day">{fmtDay(day)}</span>
        </div>
      {/each}
    </div>

    <span class="panel-section-label">{t('x.recentSessions')}</span>
    {#each stats.recent_sessions as s, i (i)}
      <div class="setting-row">
        <div class="setting-meta">
          <span class="setting-label">{s.label || s.edition || 'Session'}</span>
          <span class="setting-hint">{new Date(s.started_at).toLocaleString()}</span>
        </div>
        <span class="server-sub">{fmtDuration(s.seconds)}</span>
      </div>
    {/each}
  {/if}
</div>

<style>
  .stats-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 8px;
  }
  .stats-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: var(--card-pad-y, 12px) var(--card-pad-x, 14px);
    border-radius: var(--r-md);
    background: var(--bg-item);
    box-shadow: var(--shadow-sm);
    transition: background var(--t, 0.15s);
  }
  .stats-card:hover {
    background: var(--bg-item-hover);
  }
  .stats-card > i {
    font-size: 13px;
    color: var(--accent-hover);
    margin-bottom: 4px;
  }
  .stats-val {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 700;
    line-height: 1.1;
    color: var(--text);
    text-align: left;
    font-variant-numeric: tabular-nums;
  }
  .stats-label {
    font-size: 11px;
    color: var(--text-dim);
    text-align: left;
  }
</style>
