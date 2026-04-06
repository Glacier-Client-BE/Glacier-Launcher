using System.IO;
using System.Text.Json;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public class SettingsService
{
    private static readonly string SettingsPath =
        Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "glacier-settings.json");

    public LauncherSettings Settings { get; private set; } = new();

    public SettingsService()
    {
        Load();
    }

    public void Load()
    {
        try
        {
            if (File.Exists(SettingsPath))
            {
                var json = File.ReadAllText(SettingsPath);
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
            File.WriteAllText(SettingsPath, json);
        }
        catch { }
    }
}
