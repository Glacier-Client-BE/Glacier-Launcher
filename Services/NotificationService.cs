using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Persistent notification stack (survives closing the launcher), distinct from
/// the ephemeral toasts in Home.razor.cs. Toasts announce something that just
/// happened; notifications are for things the user may not have been looking
/// at the window for — an update becoming available, a background download
/// finishing — and stay until dismissed or read.
/// </summary>
public class NotificationService
{
    private const int MaxStored = 50;

    private static string FilePath =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "notifications.json");

    private readonly List<AppNotification> _items = new();
    private readonly object _sync = new();

    public event Action? Changed;

    public NotificationService() => Load();

    public IReadOnlyList<AppNotification> Items
    {
        get { lock (_sync) return _items.OrderByDescending(n => n.TimeIso).ToList(); }
    }

    public int UnreadCount
    {
        get { lock (_sync) return _items.Count(n => !n.Read); }
    }

    public void Add(string title, string message, string kind = "info", string url = "")
    {
        var icon = kind switch
        {
            "success" => "fa-solid fa-circle-check",
            "error"   => "fa-solid fa-circle-xmark",
            _         => "fa-solid fa-circle-info"
        };
        lock (_sync)
        {
            _items.Insert(0, new AppNotification { Title = title, Message = message, Kind = kind, Icon = icon, Url = url });
            while (_items.Count > MaxStored) _items.RemoveAt(_items.Count - 1);
            Save();
        }
        Changed?.Invoke();
    }

    public void MarkRead(string id)
    {
        lock (_sync)
        {
            var n = _items.FirstOrDefault(x => x.Id == id);
            if (n == null || n.Read) return;
            n.Read = true;
            Save();
        }
        Changed?.Invoke();
    }

    public void MarkAllRead()
    {
        lock (_sync)
        {
            if (_items.All(n => n.Read)) return;
            foreach (var n in _items) n.Read = true;
            Save();
        }
        Changed?.Invoke();
    }

    public void Remove(string id)
    {
        lock (_sync)
        {
            if (_items.RemoveAll(n => n.Id == id) == 0) return;
            Save();
        }
        Changed?.Invoke();
    }

    public void Clear()
    {
        lock (_sync)
        {
            if (_items.Count == 0) return;
            _items.Clear();
            Save();
        }
        Changed?.Invoke();
    }

    private void Load()
    {
        try
        {
            if (!File.Exists(FilePath)) return;
            var json = File.ReadAllText(FilePath);
            var items = JsonSerializer.Deserialize<List<AppNotification>>(json);
            if (items != null) _items.AddRange(items);
        }
        catch { /* start empty on corrupt/missing file */ }
    }

    private void Save()
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(FilePath)!);
            var json = JsonSerializer.Serialize(_items, new JsonSerializerOptions { WriteIndented = true });
            JsonStore.WriteAtomic(FilePath, json);
        }
        catch { /* best effort */ }
    }
}
