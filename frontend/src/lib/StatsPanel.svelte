<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import Spinner from './components/Spinner.svelte'

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

<PanelHeader title="Stats" {onBack}>
  {#snippet extraActions()}
    <button type="button" class="panel-icon-btn {loading ? 'spinning' : ''}" onclick={load} data-tooltip="Refresh" aria-label="Refresh">
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if loading}
    <div class="versions-loading"><Spinner size={16} /><span>Loading stats…</span></div>
  {:else if error}
    <span class="error-text">{error}</span>
  {:else if !stats || stats.session_count === 0}
    <div class="empty-state">
      <i class="fa-solid fa-chart-simple"></i>
      <span>No play sessions recorded yet.</span>
    </div>
  {:else}
    <div class="stats-cards">
      <div class="stats-card">
        <i class="fa-solid fa-clock"></i>
        <span class="stats-val">{fmtDuration(stats.total_seconds)}</span>
        <span class="stats-label">Total played</span>
      </div>
      <div class="stats-card">
        <i class="fa-solid fa-gamepad"></i>
        <span class="stats-val">{stats.session_count}</span>
        <span class="stats-label">Sessions</span>
      </div>
      <div class="stats-card">
        <i class="fa-solid fa-hourglass-half"></i>
        <span class="stats-val">{fmtDuration(stats.longest_session_seconds)}</span>
        <span class="stats-label">Longest session</span>
      </div>
      <div class="stats-card">
        <i class="fa-solid fa-star"></i>
        <span class="stats-val">{stats.top_labels[0]?.[0] ?? '—'}</span>
        <span class="stats-label">Most played</span>
      </div>
    </div>

    <span class="panel-section-label">Last 14 days</span>
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

    <span class="panel-section-label">Recent sessions</span>
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
