using System;
using System.IO;
using System.Text.Json;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public class SettingsService
{
    private static readonly JsonSerializerOptions SerializerOptions = new() { WriteIndented = true };

    private readonly string _settingsPath;
    private readonly object _sync = new();
    private string _lastSerialized = "";

    public LauncherSettings Settings { get; private set; } = new();

    public SettingsService()
    {
        var userFolder = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var appFolder = Path.Combine(userFolder, "Glacier Launcher");
        
        Directory.CreateDirectory(appFolder);

        _settingsPath = Path.Combine(appFolder, "glacier-settings.json");
        Load();
    }

    public void Load()
    {
        try
        {
            if (!File.Exists(_settingsPath)) return;

            var json = File.ReadAllText(_settingsPath);
            try
            {
                Settings        = JsonSerializer.Deserialize<LauncherSettings>(json) ?? new();
                _lastSerialized = JsonSerializer.Serialize(Settings, SerializerOptions);
            }
            catch (JsonException)
            {
                // The file is readable but malformed. Set it aside (keeping a copy)
                // and start from defaults instead of failing the same way forever.
                JsonStore.QuarantineCorrupt(_settingsPath);
                Settings        = new();
                _lastSerialized = "";
            }
        }
        catch
        {
            // Transient read error (e.g. the file was momentarily locked). Use
            // defaults in memory but leave the on-disk file untouched so we don't
            // discard good settings over a hiccup.
            Settings        = new();
            _lastSerialized = "";
        }
    }

    public void Save()
    {
        try
        {
            lock (_sync)
            {
                // Serialize under the lock so two concurrent saves can't interleave
                // and so the dedupe check and write stay consistent.
                var json = JsonSerializer.Serialize(Settings, SerializerOptions);
                if (string.Equals(json, _lastSerialized, StringComparison.Ordinal))
                    return;

                JsonStore.WriteAtomic(_settingsPath, json);
                _lastSerialized = json;
            }
        }
        catch { }
    }
}
