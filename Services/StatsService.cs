using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Records and aggregates play sessions and launch history for the stats
/// dashboard. Persisted to stats.json (atomic; corrupt files quarantined),
/// capped at 500 sessions so the file stays small.
/// </summary>
public sealed class StatsService
{
    private const int MaxSessions = 500;
    private static readonly JsonSerializerOptions SerializerOptions = new() { WriteIndented = true };

    private readonly string _path;
    private readonly object _sync = new();
    private List<PlaySession> _sessions = new();

    public StatsService()
    {
        _path = Path.Combine(LauncherUtilityService.LauncherRoot, "stats.json");
        Load();
    }

    public IReadOnlyList<PlaySession> Sessions
    {
        get { lock (_sync) return _sessions.ToList(); }
    }

    public void RecordSession(DateTime startUtc, long seconds, string edition, string label)
    {
        if (seconds <= 0) return;
        lock (_sync)
        {
            _sessions.Add(new PlaySession
            {
                StartedAt = startUtc.ToString("o"),
                Seconds   = seconds,
                Edition   = edition,
                Label     = label,
            });
            if (_sessions.Count > MaxSessions)
                _sessions.RemoveRange(0, _sessions.Count - MaxSessions);
            Save();
        }
    }

    public long TotalSeconds { get { lock (_sync) return _sessions.Sum(s => s.Seconds); } }
    public int  SessionCount { get { lock (_sync) return _sessions.Count; } }

    public long LongestSessionSeconds { get { lock (_sync) return _sessions.Count == 0 ? 0 : _sessions.Max(s => s.Seconds); } }

    /// <summary>Total seconds played per day for the last <paramref name="days"/> days (oldest first).</summary>
    public List<(DateTime Day, long Seconds)> DailyTotals(int days = 14)
    {
        var today = DateTime.UtcNow.Date;
        var buckets = Enumerable.Range(0, days)
            .Select(i => today.AddDays(-(days - 1 - i)))
            .ToDictionary(d => d, _ => 0L);

        lock (_sync)
        {
            foreach (var s in _sessions)
            {
                if (DateTime.TryParse(s.StartedAt, null, System.Globalization.DateTimeStyles.RoundtripKind, out var dt))
                {
                    var day = dt.ToUniversalTime().Date;
                    if (buckets.ContainsKey(day)) buckets[day] += s.Seconds;
                }
            }
        }
        return buckets.OrderBy(kv => kv.Key).Select(kv => (kv.Key, kv.Value)).ToList();
    }

    public List<PlaySession> RecentSessions(int count = 20)
    {
        lock (_sync)
            return _sessions.AsEnumerable().Reverse().Take(count).ToList();
    }

    /// <summary>Most-launched labels with their session counts.</summary>
    public List<(string Label, int Count)> TopLabels(int count = 5)
    {
        lock (_sync)
            return _sessions
                .Where(s => !string.IsNullOrEmpty(s.Label))
                .GroupBy(s => s.Label)
                .Select(g => (g.Key, g.Count()))
                .OrderByDescending(x => x.Item2)
                .Take(count)
                .ToList();
    }

    private void Load()
    {
        try
        {
            if (!File.Exists(_path)) return;
            var json = File.ReadAllText(_path);
            try { _sessions = JsonSerializer.Deserialize<List<PlaySession>>(json) ?? new(); }
            catch (JsonException) { JsonStore.QuarantineCorrupt(_path); _sessions = new(); }
        }
        catch { _sessions = new(); }
    }

    private void Save()
    {
        try { JsonStore.WriteAtomic(_path, JsonSerializer.Serialize(_sessions, SerializerOptions)); }
        catch { }
    }
}
