using System;
using System.IO;
using System.Text.Json;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public class SettingsService
{
    private readonly string _settingsPath;

    public LauncherSettings Settings { get; private set; } = new();

    public SettingsService()
    {
        string userFolder = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        string appFolder = Path.Combine(userFolder, "Glacier Launcher");
        
        if (!Directory.Exists(appFolder))
        {
            Directory.CreateDirectory(appFolder);
        }

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
            }
        }
        catch
        {
            Settings = new();
        }
    }

    public void Save()
    {
        try
        {
            var json = JsonSerializer.Serialize(Settings, new JsonSerializerOptions { WriteIndented = true });
            File.WriteAllText(_settingsPath, json);
        }
        catch { }
    }
}