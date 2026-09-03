<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'
  import SettingRow from './components/SettingRow.svelte'
  import Toggle from './components/Toggle.svelte'
  import Spinner from './components/Spinner.svelte'

  // Ported from Pages/Home.LevelDat.cs. The backend
  // (src-tauri/src/commands/leveldat.rs) backs up level.dat, writes to a
  // temp file, re-reads + verifies it matches what was intended, and only
  // then atomically renames it over the original — so a save here either
  // fully succeeds with a `.bak-<timestamp>` left behind, or fails leaving
  // the original file completely untouched.
  let { worldDir, onBack }: { worldDir: string; onBack?: () => void } = $props()

  type Summary = {
    game_type: number
    difficulty: number
    cheats_enabled: boolean
    generator: number
    random_seed: number
    has_random_seed: boolean
    experiments: Record<string, boolean>
  }

  const gameTypeOptions = [
    [0, 'Survival'],
    [1, 'Creative'],
    [2, 'Adventure'],
    [3, 'Spectator'],
  ] as const
  const difficultyOptions = [
    [0, 'Peaceful'],
    [1, 'Easy'],
    [2, 'Normal'],
    [3, 'Hard'],
  ] as const
  const generatorOptions = [
    [0, 'Legacy'],
    [1, 'Infinite'],
    [2, 'Flat'],
  ] as const

  let summary = $state<Summary | null>(null)
  let loading = $state(true)
  let error = $state('')
  let saving = $state(false)
  let saveMessage = $state('')

  async function load() {
    loading = true
    error = ''
    saveMessage = ''
    try {
      summary = await invoke<Summary>('read_level_dat_summary', { worldDir })
    } catch (e) {
      error = String(e)
      summary = null
    } finally {
      loading = false
    }
  }

  function update<K extends keyof Summary>(key: K, value: Summary[K]) {
    if (!summary) return
    summary = { ...summary, [key]: value }
  }

  function toggleExperiment(name: string) {
    if (!summary) return
    summary = { ...summary, experiments: { ...summary.experiments, [name]: !summary.experiments[name] } }
  }

  async function save() {
    if (!summary) return
    saving = true
    error = ''
    saveMessage = ''
    try {
      const backupPath = await invoke<string>('write_level_dat_summary', { worldDir, summary })
      saveMessage = `Saved. Backup kept at ${backupPath}`
    } catch (e) {
      error = String(e)
    } finally {
      saving = false
    }
  }

  load()
</script>

<PanelHeader title="Level.dat Editor" {onBack}>
  {#snippet extraActions()}
    <button type="button" class="panel-icon-btn {loading ? 'spinning' : ''}" onclick={load} disabled={loading} data-tooltip="Reload" aria-label="Reload">
      <i class="fa-solid fa-rotate"></i>
    </button>
  {/snippet}
</PanelHeader>

<div class="panel-body">
  {#if loading}
    <div class="versions-loading"><Spinner size={16} /><span>Reading level.dat…</span></div>
  {:else if !summary}
    <div class="empty-state">
      <i class="fa-solid fa-triangle-exclamation"></i>
      <span>Couldn't read level.dat</span>
      <small>{error}</small>
      <button type="button" class="btn-sm" style="margin-top:10px;" onclick={load}>
        <i class="fa-solid fa-rotate"></i> Retry
      </button>
    </div>
  {:else}
    <SettingRow label="Game type">
      {#snippet trailing()}
        <select
          class="setting-select"
          value={summary.game_type}
          onchange={(e) => update('game_type', Number((e.target as HTMLSelectElement).value))}
        >
          {#each gameTypeOptions as [value, label] (value)}
            <option {value}>{label}</option>
          {/each}
        </select>
      {/snippet}
    </SettingRow>

    <SettingRow label="Difficulty">
      {#snippet trailing()}
        <select
          class="setting-select"
          value={summary.difficulty}
          onchange={(e) => update('difficulty', Number((e.target as HTMLSelectElement).value))}
        >
          {#each difficultyOptions as [value, label] (value)}
            <option {value}>{label}</option>
          {/each}
        </select>
      {/snippet}
    </SettingRow>

    <SettingRow label="Cheats enabled">
      {#snippet trailing()}
        <Toggle on={summary.cheats_enabled} onChanged={(v) => update('cheats_enabled', v)} />
      {/snippet}
    </SettingRow>

    <SettingRow label="World generator">
      {#snippet trailing()}
        <select
          class="setting-select"
          value={summary.generator}
          onchange={(e) => update('generator', Number((e.target as HTMLSelectElement).value))}
        >
          {#each generatorOptions as [value, label] (value)}
            <option {value}>{label}</option>
          {/each}
        </select>
      {/snippet}
    </SettingRow>

    {#if summary.has_random_seed}
      <SettingRow label="Random seed" hint="Read-only — regenerating terrain from a new seed isn't supported.">
        {#snippet trailing()}
          <span class="server-sub">{summary.random_seed}</span>
        {/snippet}
      </SettingRow>
    {/if}

    {#if Object.keys(summary.experiments).length > 0}
      <span class="panel-section-label">Experiments</span>
      {#each Object.entries(summary.experiments) as [name, enabled] (name)}
        <SettingRow label={name}>
          {#snippet trailing()}
            <Toggle on={enabled} onChanged={() => toggleExperiment(name)} />
          {/snippet}
        </SettingRow>
      {/each}
    {/if}

    {#if error}
      <span class="error-text">{error}</span>
    {/if}
    {#if saveMessage}
      <div class="logs-share-url" style="margin-top:10px;">
        <i class="fa-solid fa-circle-check"></i>
        <span style="flex:1;overflow:hidden;text-overflow:ellipsis;">{saveMessage}</span>
      </div>
    {/if}
  {/if}
</div>

{#if summary}
  <div class="panel-footer-bar">
    <span style="font-size:12px;color:var(--text-dim);flex:1;">
      A backup is made automatically before every save.
    </span>
    <button type="button" class="action-btn" disabled={saving} onclick={save}>
      {#if saving}
        <Spinner size={14} />
        <span>Saving…</span>
      {:else}
        <i class="fa-solid fa-floppy-disk"></i> Save
      {/if}
    </button>
  </div>
{/if}
