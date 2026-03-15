using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using EasyHook;
using MinecraftLauncher.Models;

namespace MinecraftLauncher.Services;

public class GameLauncher
{
    private const string BaseUrl = "https://github.com/Imrglop/Latite-Releases/releases/download";
    private const string ApiUrl  = "https://api.github.com/repos/Imrglop/Latite-Releases/releases";

    private readonly SettingsService _settingsService;
    private readonly HttpClient _httpClient;

    public static string ClientsDirectory =>
        Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "clients");

    public GameLauncher(SettingsService settingsService)
    {
        _settingsService = settingsService;
        _httpClient      = new HttpClient();
        _httpClient.DefaultRequestHeaders.Add("User-Agent", "GlacierLauncher/1.0");
        Directory.CreateDirectory(ClientsDirectory);
    }

    public async Task<List<MinecraftVersion>> GetVersionsAsync()
    {
        var versions = new List<MinecraftVersion>();
        try
        {
            var json = await _httpClient.GetStringAsync(ApiUrl);
            using var doc = JsonDocument.Parse(json);

            foreach (var release in doc.RootElement.EnumerateArray())
            {
                var tag      = release.GetProperty("tag_name").GetString() ?? "";
                var name     = release.GetProperty("name").GetString() ?? tag;
                var dllPath  = GetDllPath(tag);

                versions.Add(new MinecraftVersion
                {
                    Tag          = tag,
                    DisplayName  = name,
                    IsDownloaded = File.Exists(dllPath)
                });
            }
        }
        catch (Exception ex)
        {
            versions.Add(new MinecraftVersion
            {
                Tag          = "error",
                DisplayName  = $"Failed to fetch: {ex.Message}",
                ErrorMessage = ex.Message
            });
        }
        return versions;
    }

    public async Task DownloadVersionAsync(MinecraftVersion version, IProgress<double>? progress = null)
    {
        var url     = $"{BaseUrl}/{version.Tag}/Latite.dll";
        var dllPath = GetDllPath(version.Tag);

        using var response = await _httpClient.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
        response.EnsureSuccessStatusCode();

        var  totalBytes = response.Content.Headers.ContentLength ?? -1;
        var  buffer     = new byte[8192];
        long downloaded = 0;

        await using var stream     = await response.Content.ReadAsStreamAsync();
        await using var fileStream = File.Create(dllPath);

        int read;
        while ((read = await stream.ReadAsync(buffer)) > 0)
        {
            await fileStream.WriteAsync(buffer.AsMemory(0, read));
            downloaded += read;
            if (totalBytes > 0)
                progress?.Report((double)downloaded / totalBytes * 100);
        }

        version.IsDownloaded = true;
        _settingsService.Settings.LastUsedVersion = version.Tag;
        _settingsService.Save();
    }

    public void DeleteVersion(MinecraftVersion version)
    {
        var dllPath = GetDllPath(version.Tag);
        if (File.Exists(dllPath))
            File.Delete(dllPath);

        version.IsDownloaded = false;

        if (_settingsService.Settings.LastUsedVersion == version.Tag)
        {
            _settingsService.Settings.LastUsedVersion = "";
            _settingsService.Save();
        }
    }

    public async Task LaunchAsync(string? versionTag = null)
    {
        var tag = versionTag ?? _settingsService.Settings.LastUsedVersion;

        string dllPath;
        if (!string.IsNullOrEmpty(tag))
        {
            dllPath = GetDllPath(tag);
            if (!File.Exists(dllPath))
                throw new FileNotFoundException($"DLL not found for '{tag}'. Please download it first.");
        }
        else
        {
            dllPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Latite.dll");
            if (!File.Exists(dllPath))
                throw new FileNotFoundException("No version selected. Download one from the Versions panel.");
        }

        // Launch Minecraft for Windows (GDK) via cmd to ensure shell: resolves correctly
        Process.Start(new ProcessStartInfo
        {
            FileName        = "cmd.exe",
            Arguments       = "/c start \"\" \"shell:AppsFolder\\Microsoft.MinecraftUWP_8wekyb3d8bbwe!App\"",
            UseShellExecute = false,
            CreateNoWindow  = true,
            WindowStyle     = ProcessWindowStyle.Hidden
        });

        // Wait up to 20 s for Minecraft.Windows to appear
        Process[] procs = [];
        for (int i = 0; i < 40; i++)
        {
            procs = Process.GetProcessesByName("Minecraft.Windows");
            if (procs.Length > 0) break;
            await Task.Delay(500);
        }

        if (procs.Length == 0)
            throw new TimeoutException("Minecraft.Windows did not start. Make sure Minecraft for Windows is installed.");

        // Wait an additional 4 s for the game engine to fully initialise its module list
        // before injecting — injecting too early causes the hook to fail silently
        await Task.Delay(4000);

        // Re-fetch process in case it restarted during initialisation
        procs = Process.GetProcessesByName("Minecraft.Windows");
        if (procs.Length == 0)
            throw new TimeoutException("Minecraft.Windows exited before injection could complete.");

        RemoteHooking.Inject(procs[0].Id, InjectionOptions.Default, dllPath, dllPath);

        if (!string.IsNullOrEmpty(tag))
        {
            _settingsService.Settings.LastUsedVersion = tag;
            _settingsService.Save();
        }
    }

    public static string GetDllPath(string tag) =>
        Path.Combine(ClientsDirectory, $"Latite_{tag}.dll");
}
