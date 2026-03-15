using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using EasyHook;
using MinecraftLauncher.Models;

namespace MinecraftLauncher.Services;

public class GameLauncher
{
    private const string BaseUrl = "https://github.com/Imrglop/Latite-Releases/releases/download";
    private const string ApiUrl = "https://api.github.com/repos/Imrglop/Latite-Releases/releases";

    private readonly SettingsService _settingsService;
    private readonly HttpClient _httpClient;

    public static string ClientsDirectory =>
        Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "clients");

    public GameLauncher(SettingsService settingsService)
    {
        _settingsService = settingsService;
        _httpClient = new HttpClient();
        _httpClient.DefaultRequestHeaders.Add("User-Agent", "GlacierLauncher/1.0");
        Directory.CreateDirectory(ClientsDirectory);
    }

    /// <summary>
    /// Fetch available Latite releases from GitHub and check which are downloaded.
    /// </summary>
    public async Task<List<MinecraftVersion>> GetVersionsAsync()
    {
        var versions = new List<MinecraftVersion>();
        try
        {
            var json = await _httpClient.GetStringAsync(ApiUrl);
            using var doc = JsonDocument.Parse(json);

            foreach (var release in doc.RootElement.EnumerateArray())
            {
                var tag = release.GetProperty("tag_name").GetString() ?? "";
                var name = release.GetProperty("name").GetString() ?? tag;
                var dllPath = GetDllPath(tag);

                versions.Add(new MinecraftVersion
                {
                    Tag = tag,
                    DisplayName = name,
                    IsDownloaded = File.Exists(dllPath)
                });
            }
        }
        catch (Exception ex)
        {
            versions.Add(new MinecraftVersion
            {
                Tag = "error",
                DisplayName = $"Failed to fetch: {ex.Message}",
                ErrorMessage = ex.Message
            });
        }
        return versions;
    }

    /// <summary>
    /// Download a specific Latite release DLL.
    /// </summary>
    public async Task DownloadVersionAsync(MinecraftVersion version, IProgress<double>? progress = null)
    {
        var url = $"{BaseUrl}/{version.Tag}/Latite.dll";
        var dllPath = GetDllPath(version.Tag);

        using var response = await _httpClient.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
        response.EnsureSuccessStatusCode();

        var totalBytes = response.Content.Headers.ContentLength ?? -1;
        var buffer = new byte[8192];
        long downloaded = 0;

        await using var stream = await response.Content.ReadAsStreamAsync();
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

    /// <summary>
    /// Launch Minecraft and inject the specified (or last-used) version of Latite.
    /// </summary>
    public async Task LaunchAsync(string? versionTag = null)
    {
        var tag = versionTag ?? _settingsService.Settings.LastUsedVersion;

        string dllPath;
        if (!string.IsNullOrEmpty(tag))
        {
            dllPath = GetDllPath(tag);
            if (!File.Exists(dllPath))
                throw new FileNotFoundException($"Client DLL not found for version '{tag}'. Please download it first.");
        }
        else
        {
            dllPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Latite.dll");
            if (!File.Exists(dllPath))
                throw new FileNotFoundException("No client version selected. Please download a version from the Versions panel.");
        }

        // GDK launch — shell:AppsFolder is more reliable than the minecraft: URI on GDK builds
        Process.Start(new ProcessStartInfo("explorer.exe")
        {
            Arguments = "shell:AppsFolder\\Microsoft.MinecraftUWP_8wekyb3d8bbwe!App",
            UseShellExecute = false
        });

        // Wait up to 20 seconds for the process to appear (GDK can be slow)
        Process[] procs = [];
        for (int i = 0; i < 40; i++)
        {
            procs = Process.GetProcessesByName("Minecraft.Windows");
            if (procs.Length > 0) break;
            await Task.Delay(500);
        }

        if (procs.Length == 0)
            throw new TimeoutException("Minecraft did not start in time. Please try again.");

        RemoteHooking.Inject(procs[0].Id, InjectionOptions.Default, dllPath, dllPath);

        if (!string.IsNullOrEmpty(tag))
        {
            _settingsService.Settings.LastUsedVersion = tag;
            _settingsService.Save();
        }
    }

    private static string GetDllPath(string tag) =>
        Path.Combine(ClientsDirectory, $"Latite_{tag}.dll");
}