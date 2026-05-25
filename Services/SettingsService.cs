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
            if (File.Exists(_settingsPath))
            {
                var json = File.ReadAllText(_settingsPath);
                Settings = JsonSerializer.Deserialize<LauncherSettings>(json) ?? new();
                _lastSerialized = JsonSerializer.Serialize(Settings, SerializerOptions);
            }
        }
        catch
        {
            Settings = new();
            _lastSerialized = "";
        }
    }

    public void Save()
    {
        try
        {
            var json = JsonSerializer.Serialize(Settings, SerializerOptions);
            lock (_sync)
            {
                if (string.Equals(json, _lastSerialized, StringComparison.Ordinal))
                    return;

                File.WriteAllText(_settingsPath, json);
                _lastSerialized = json;
            }
        }
        catch { }
    }
}
