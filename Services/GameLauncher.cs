using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public class GameLauncher
{
    // LatiteClient/Latite is the active source. The old Imrglop/Latite-Releases is
    // archived and no longer receives builds, so we fetch via the JSON API and use
    // each asset's browser_download_url (asset names vary: LatiteNightly.dll,
    // LatiteDebug.dll, etc., and tags like nightly-YYYYMMDD-HHMMSS aren't constructable).
    private const string ApiUrl        = "https://api.github.com/repos/LatiteClient/Latite/releases";
    private const string UwpAppModelId = "Microsoft.MinecraftUWP_8wekyb3d8bbwe!App";
    private const string ProcessName   = "Minecraft.Windows";

    // ────────────────────────────────────────────────────────────

    private readonly SettingsService _settingsService;
    private readonly HttpClient      _httpClient;

    public static string DownloadsDirectory =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "downloads");

    public static string LatiteDirectory =>
        Path.Combine(DownloadsDirectory, "Latite");

    public GameLauncher(SettingsService settingsService)
    {
        _settingsService = settingsService;
        _httpClient      = HttpFactory.Shared;
        Directory.CreateDirectory(LatiteDirectory);
    }

    /// <summary>Last fetch error message (e.g. rate-limit notice). Null if last fetch was clean.</summary>
    public string? LastVersionsError { get; private set; }

    public async Task<List<MinecraftVersion>> GetVersionsAsync()
    {
        var versions = new List<MinecraftVersion>();
        LastVersionsError = null;
        try
        {
            var cached = await GitHubApiCache.GetJsonAsync(_httpClient, ApiUrl + "?per_page=60");
            LastVersionsError = cached.Error;
            var json = cached.Body;

            using var doc = JsonDocument.Parse(json);
            foreach (var release in doc.RootElement.EnumerateArray())
            {
                var tag = release.TryGetProperty("tag_name", out var t) ? t.GetString() ?? "" : "";
                if (string.IsNullOrEmpty(tag)) continue;

                // Skip floating tags — they duplicate the most recent timestamped build.
                if (tag is "nightly" or "debug" or "latest") continue;
                if (release.TryGetProperty("draft", out var d) && d.GetBoolean()) continue;

                var name        = release.TryGetProperty("name", out var n) ? n.GetString() ?? tag : tag;
                var publishedAt = release.TryGetProperty("published_at", out var pa) ? pa.GetString() : null;

                // Pick the runnable .dll. Prefer Release > Nightly > anything; never debug
                // (giant .pdb-paired builds aren't useful for end users).
                string? bestUrl = null; long bestSize = 0; string? bestAssetName = null; int bestRank = int.MaxValue;
                if (release.TryGetProperty("assets", out var assets) && assets.ValueKind == JsonValueKind.Array)
                {
                    foreach (var asset in assets.EnumerateArray())
                    {
                        var aName = asset.TryGetProperty("name", out var an) ? an.GetString() ?? "" : "";
                        if (!aName.EndsWith(".dll", StringComparison.OrdinalIgnoreCase)) continue;
                        if (aName.Contains("Debug", StringComparison.OrdinalIgnoreCase)) continue;

                        int rank = aName.Equals("Latite.dll", StringComparison.OrdinalIgnoreCase)        ? 0
                                 : aName.Contains("Nightly",  StringComparison.OrdinalIgnoreCase)        ? 1
                                                                                                          : 2;
                        if (rank < bestRank)
                        {
                            bestRank      = rank;
                            bestAssetName = aName;
                            bestUrl       = asset.TryGetProperty("browser_download_url", out var u) ? u.GetString() : null;
                            bestSize      = asset.TryGetProperty("size",                  out var s) ? s.GetInt64() : 0;
                        }
                    }
                }

                if (string.IsNullOrEmpty(bestUrl)) continue; // no usable .dll → skip release

                var variant = bestAssetName!.Contains("Nightly", StringComparison.OrdinalIgnoreCase) ? "Nightly"
                            : bestAssetName.Contains("Debug",   StringComparison.OrdinalIgnoreCase) ? "Debug"
                                                                                                    : "Release";

                versions.Add(new MinecraftVersion
                {
                    Tag          = tag,
                    DisplayName  = FormatLatiteDisplayName(name, tag, publishedAt),
                    IsDownloaded = File.Exists(GetDllPath(tag)),
                    DownloadUrl  = bestUrl,
                    AssetSize    = bestSize,
                    PublishedAt  = publishedAt,
                    Variant      = variant
                });
            }
        }
        catch (HttpRequestException ex) when (ex.StatusCode == System.Net.HttpStatusCode.Forbidden)
        {
            LastVersionsError = "GitHub rate limit reached. Try again in a few minutes.";
        }
        catch (Exception ex)
        {
            LastVersionsError = $"Failed to fetch: {ex.Message}";
        }
        return versions;
    }

    private static readonly Regex _latiteTagRegex =
        new(@"^(nightly|debug|release)-(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})$",
            RegexOptions.IgnoreCase | RegexOptions.Compiled);

    private static string FormatLatiteDisplayName(string apiName, string tag, string? publishedAt)
    {
        var m = _latiteTagRegex.Match(tag);
        if (m.Success)
        {
            var variant = char.ToUpperInvariant(m.Groups[1].Value[0]) + m.Groups[1].Value[1..].ToLowerInvariant();
            var date = $"{m.Groups[2].Value}-{m.Groups[3].Value}-{m.Groups[4].Value} {m.Groups[5].Value}:{m.Groups[6].Value}";
            return $"Latite {variant} · {date}";
        }
        if (!string.IsNullOrEmpty(publishedAt) &&
            DateTime.TryParse(publishedAt, CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal, out var dt))
        {
            return $"{apiName} · {dt:yyyy-MM-dd HH:mm}";
        }
        return apiName;
    }

    public async Task DownloadVersionAsync(MinecraftVersion version, IProgress<double>? progress = null)
    {
        if (string.IsNullOrEmpty(version.DownloadUrl))
            throw new InvalidOperationException("This version has no download URL. Refresh the versions list and try again.");

        var dllPath = GetDllPath(version.Tag);

        using var response = await _httpClient.GetAsync(version.DownloadUrl, HttpCompletionOption.ResponseHeadersRead);
        response.EnsureSuccessStatusCode();

        var total = response.Content.Headers.ContentLength ?? -1;
        var buf   = new byte[8192];
        long dl   = 0;

        await using var stream = await response.Content.ReadAsStreamAsync();
        await using var fs     = File.Create(dllPath);

        int read;
        while ((read = await stream.ReadAsync(buf)) > 0)
        {
            await fs.WriteAsync(buf.AsMemory(0, read));
            dl += read;
            if (total > 0) progress?.Report((double)dl / total * 100);
        }

        version.IsDownloaded = true;
        _settingsService.Settings.LastUsedVersion = version.Tag;
        _settingsService.Save();
    }

    public void DeleteVersion(MinecraftVersion version)
    {
        var path = GetDllPath(version.Tag);
        if (File.Exists(path)) File.Delete(path);
        version.IsDownloaded = false;
        if (_settingsService.Settings.LastUsedVersion == version.Tag)
        { _settingsService.Settings.LastUsedVersion = ""; _settingsService.Save(); }
    }

    public Task LaunchWithDllAsync(string dllPath)
    {
        if (!File.Exists(dllPath)) throw new FileNotFoundException($"DLL not found: {dllPath}");
        return LaunchWithPathAsync(dllPath);
    }

    /// <summary>
    /// Launches Minecraft with no DLL injection at all (vanilla / un-modified).
    /// </summary>
    public Task LaunchVanillaAsync() => LaunchWithPathAsync(null);

    /// <summary>
    /// Launches MC + injects (if a DLL is given), then asks Minecraft to add the
    /// server to the user's external-server list via the well-known
    /// <c>minecraft://?addExternalServer=Name|Host:Port</c> URI scheme. This
    /// pops the server up at the top of the Servers list so the user only has
    /// to click once to join.
    /// </summary>
    public async Task LaunchServerAsync(string? dllPath, string serverName, string host, int port)
    {
        await LaunchWithPathAsync(dllPath);

        // Bedrock's URI handler accepts `addExternalServer=<DisplayName>|<Host>:<Port>`.
        // We URL-encode Name/Host because both can contain spaces, special chars,
        // or IPv6 brackets that would otherwise break the query string.
        var encodedName = Uri.EscapeDataString(string.IsNullOrWhiteSpace(serverName) ? host : serverName);
        var encodedHost = Uri.EscapeDataString(host);
        var uri = $"minecraft://?addExternalServer={encodedName}|{encodedHost}:{port}";
        try
        {
            Process.Start(new ProcessStartInfo(uri) { UseShellExecute = true });
        }
        catch
        {
            // Some shells don't honour the URI handler the same way — fall back to cmd start.
            try { Process.Start(new ProcessStartInfo("cmd.exe") { Arguments = $"/c start \"\" \"{uri}\"", UseShellExecute = false, CreateNoWindow = true, WindowStyle = ProcessWindowStyle.Hidden }); }
            catch { }
        }
    }

    public async Task LaunchAsync(string? versionTag = null, bool useFlarial = false)
    {
        // Determine DLL path — null means launch MC only, no injection
        string? dllPath = null;

        if (useFlarial)
        {
            dllPath = FlarialService.FilePath;
            if (!File.Exists(dllPath))
                throw new FileNotFoundException("Flarial DLL not found. Download it from the Clients tab first.");
        }
        else
        {
            var tag = versionTag ?? _settingsService.Settings.LastUsedVersion;
            if (!string.IsNullOrEmpty(tag))
            {
                var path = GetDllPath(tag);
                // If a version is selected but not downloaded yet, still launch MC — just skip injection
                dllPath = File.Exists(path) ? path : null;
            }
            // If no version is selected at all, dllPath stays null — launch MC without injection
        }

        await LaunchWithPathAsync(dllPath);

        if (!useFlarial && !string.IsNullOrEmpty(versionTag ?? _settingsService.Settings.LastUsedVersion))
        {
            _settingsService.Settings.LastUsedVersion = versionTag ?? _settingsService.Settings.LastUsedVersion;
            _settingsService.Save();
        }
    }

    public static string GetDllPath(string tag) =>
        Path.Combine(LatiteDirectory, $"Latite_{tag}.dll");

    /// <summary>True if a Minecraft.Windows process is currently running.</summary>
    public static bool IsMinecraftRunning() => GetMinecraftProcess() != null;

    // ── Core launch + inject ─────────────────────────────────────

    private async Task LaunchWithPathAsync(string? dllPath)
    {
        // Check if Minecraft is already running before launching
        var alreadyRunning = GetMinecraftProcess();
        uint pid = 0;

        if (alreadyRunning == null)
        {
            bool launched = false;

            // 1. Try COM-based IApplicationActivationManager (InjectionService)
            try { pid = InjectionService.LaunchMinecraft(); } catch { pid = 0; }
            
            // Wait briefly to see if process actually spawns
            for (int i = 0; i < 6; i++) // 3 seconds
            {
                await Task.Delay(500);
                if (GetMinecraftProcess() != null) { launched = true; break; }
            }

            if (!launched)
            {
                // 2. Fallback: Try URI and cmd methods (GameLauncher Service)
                launched = await TryLaunchMinecraftAsync();
            }

            if (!launched)
            {
                // 3. Vice versa fallback: Try COM again just in case the system was busy
                try { pid = InjectionService.LaunchMinecraft(); } catch { pid = 0; }
                for (int i = 0; i < 6; i++) // 3 seconds
                {
                    await Task.Delay(500);
                    if (GetMinecraftProcess() != null) { launched = true; break; }
                }
            }

            // Wait for the process to be ready (main window handle exists).
            for (int i = 0; i < 60; i++) // 30 seconds max
            {
                await Task.Delay(500);
                var proc = GetMinecraftProcess();
                if (proc != null)
                {
                    pid = (uint)proc.Id;
                    // Wait until the process has a main window — means the game engine is initialising
                    if (proc.MainWindowHandle != IntPtr.Zero)
                        break;
                }
            }
        }
        else
        {
            pid = (uint)alreadyRunning.Id;
        }

        var mcProcess = GetMinecraftProcess();
        if (mcProcess == null)
            throw new TimeoutException("Minecraft did not start. Please launch Minecraft manually and then click Launch again.");

        pid = (uint)mcProcess.Id;

        // If no DLL was provided, just launch MC without injecting
        if (string.IsNullOrEmpty(dllPath)) return;

        // Wait for the game engine to initialise before injecting.
        // The delay is shorter now because we already waited for the main window above.
        await Task.Delay(_settingsService.Settings.InjectionDelayMs);

        // Re-acquire — GDK sometimes restarts the process during init
        mcProcess = GetMinecraftProcess()
            ?? throw new TimeoutException("Minecraft exited before injection could complete.");
        pid = (uint)mcProcess.Id;

        // Grant UWP access to the DLL and its parent directory, then inject
        // using the consolidated InjectionService which handles all P/Invoke correctly.
        InjectionService.InjectDll(pid, dllPath);
    }

    // Checks all known Minecraft process names
    private static Process? GetMinecraftProcess()
    {
        foreach (var name in new[] { "Minecraft.Windows", "Minecraft", "MinecraftUWP" })
        {
            var procs = Process.GetProcessesByName(name);
            if (procs.Length > 0) return procs[0];
        }
        return null;
    }

    // Tries each launch method in order, silently moving to the next on failure
    private static async Task<bool> TryLaunchMinecraftAsync()
    {
        // Method 1: minecraft:// URI — standard protocol, works on UWP and GDK
        if (await TryLaunchAsync(() =>
            Process.Start(new ProcessStartInfo("minecraft:")
            {
                UseShellExecute = true
            }))) return true;

        // Method 2: start command via cmd — fallback if URI handler isn't registered
        if (await TryLaunchAsync(() =>
            Process.Start(new ProcessStartInfo("cmd.exe")
            {
                Arguments       = "/c start minecraft:",
                UseShellExecute = false,
                CreateNoWindow  = true,
                WindowStyle     = ProcessWindowStyle.Hidden
            }))) return true;

        return false;
    }

    private static async Task<bool> TryLaunchAsync(Func<Process?> launcher)
    {
        try
        {
            launcher();
            // Give it 3 seconds to see if the process appears before trying the next method
            for (int i = 0; i < 6; i++)
            {
                await Task.Delay(500);
                if (GetMinecraftProcess() != null) return true;
            }
            return false;
        }
        catch { return false; }
    }
}