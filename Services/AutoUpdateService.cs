using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Reflection;
using System.Text.Json;
using System.Threading.Tasks;
using System.Windows;

namespace GlacierLauncher.Services;

public record LauncherUpdateInfo(
    string Tag,
    string DownloadUrl,
    string Changelog,
    long   AssetSize
);

public record LauncherRelease(string Tag, string PublishedAt, string Changelog);

public class AutoUpdateService
{
    // ── Launcher identity ─────────────────────────────────────────
    public static readonly string CurrentVersion =
        Assembly.GetEntryAssembly()?.GetName().Version?.ToString(3) ?? "0.0.0";

    // Change these to your actual GitHub org/repo once you push releases.
    private const string LauncherOrg  = "Glacier-Client-BE";
    private const string LauncherRepo = "Glacier-Launcher";

    // We list all releases (not /releases/latest) so hotfixes that weren't promoted
    // to "Latest" on GitHub — or were published as pre-releases — still get picked up.
    private static string ReleasesApiUrl =>
        $"https://api.github.com/repos/{LauncherOrg}/{LauncherRepo}/releases?per_page=30";

    // ── Injected services ─────────────────────────────────────────
    private readonly FlarialService  _flarial;
    private readonly OderSoService   _oderso;
    private readonly HttpClient      _http;
    private readonly DownloadService _download = new();

    public AutoUpdateService(FlarialService flarial, OderSoService oderso)
    {
        _flarial = flarial;
        _oderso  = oderso;
        _http    = HttpFactory.Shared;
    }

    // ── Launcher update check ─────────────────────────────────────

    /// <summary>Last status message — cache fallback / rate-limit notice. Null if last fetch was clean.</summary>
    public string? LastCheckStatus { get; private set; }

    /// <summary>
    /// Fetches the latest GitHub release for the launcher.
    /// Returns null if already up-to-date, repo unreachable, or no .exe asset found.
    /// </summary>
    public async Task<LauncherUpdateInfo?> CheckLauncherUpdateAsync()
    {
        LastCheckStatus = null;
        try
        {
            var cached = await GitHubApiCache.GetJsonAsync(_http, ReleasesApiUrl);
            LastCheckStatus = cached.Error;
            using var doc = JsonDocument.Parse(cached.Body);
            var root = doc.RootElement;

            if (root.ValueKind != JsonValueKind.Array) return null;

            // Walk every release and keep the highest semver. Skips drafts but
            // includes pre-releases so hotfixes published that way still apply.
            JsonElement? bestRelease = null;
            string       bestTag     = "";

            foreach (var release in root.EnumerateArray())
            {
                if (release.TryGetProperty("draft", out var draftProp) && draftProp.GetBoolean())
                    continue;

                var tag = release.TryGetProperty("tag_name", out var tagProp)
                    ? tagProp.GetString()?.TrimStart('v', 'V') ?? ""
                    : "";
                if (string.IsNullOrEmpty(tag)) continue;

                if (bestRelease == null || IsNewerVersion(tag, bestTag))
                {
                    bestRelease = release;
                    bestTag     = tag;
                }
            }

            if (bestRelease == null) return null;
            if (!IsNewerVersion(bestTag, CurrentVersion)) return null;

            var picked = bestRelease.Value;
            var changelog = picked.TryGetProperty("body", out var bodyProp)
                ? bodyProp.GetString() ?? ""
                : "";

            // Pick the first .exe asset, fallback to first asset of any type.
            string? downloadUrl = null;
            long    assetSize   = 0;

            if (picked.TryGetProperty("assets", out var assets))
            {
                foreach (var asset in assets.EnumerateArray())
                {
                    var name = asset.TryGetProperty("name", out var n) ? n.GetString() ?? "" : "";
                    var url  = asset.TryGetProperty("browser_download_url", out var u) ? u.GetString() ?? "" : "";
                    var size = asset.TryGetProperty("size", out var s) ? s.GetInt64() : 0L;

                    if (name.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
                    {
                        downloadUrl = url;
                        assetSize   = size;
                        break;
                    }

                    downloadUrl ??= url;
                    assetSize     = size;
                }
            }

            if (string.IsNullOrEmpty(downloadUrl)) return null;

            return new LauncherUpdateInfo(bestTag, downloadUrl, changelog, assetSize);
        }
        catch (Exception ex)
        {
            LastCheckStatus = $"Update check failed: {ex.Message}";
            return null;
        }
    }

    /// <summary>
    /// Downloads the update asset to %TEMP%, replaces the original exe, relaunches, then shuts down.
    /// </summary>
    public async Task ApplyUpdateAsync(LauncherUpdateInfo info, IProgress<double>? progress = null)
    {
        var tmpPath = Path.Combine(Path.GetTempPath(), $"GlacierLauncher_{info.Tag}.exe");

        await _download.DownloadAsync(info.DownloadUrl, tmpPath, progress: progress, knownTotalBytes: info.AssetSize);

        // Replace the original exe so shortcuts keep working.
        // A running exe can't overwrite itself, so we use a small batch script
        // that waits for this process to exit, copies the new exe over, then launches it.
        var currentExe = Environment.ProcessPath ?? Process.GetCurrentProcess().MainModule?.FileName;
        if (!string.IsNullOrEmpty(currentExe) && File.Exists(currentExe))
        {
            var pid = Environment.ProcessId;
            var batPath = Path.Combine(Path.GetTempPath(), "GlacierLauncher_update.bat");
            var script = $"""
                @echo off
                :waitloop
                tasklist /FI "PID eq {pid}" 2>NUL | find /I "{pid}" >NUL
                if not errorlevel 1 (
                    timeout /t 1 /nobreak >NUL
                    goto waitloop
                )
                set RETRIES=0
                :copyloop
                copy /Y "{tmpPath}" "{currentExe}" >NUL
                if errorlevel 1 (
                    set /a RETRIES+=1
                    if %RETRIES% LSS 10 (
                        rem The old exe's file handle (antivirus scan, OS cleanup) can
                        rem outlive the process for a moment — retry instead of silently
                        rem relaunching the un-updated exe.
                        timeout /t 1 /nobreak >NUL
                        goto copyloop
                    )
                    rem Copy never succeeded — leave the download in place and relaunch
                    rem the old exe rather than deleting the update we couldn't apply.
                    start "" "{currentExe}"
                    del "%~f0"
                    exit /b 1
                )
                del "{tmpPath}" >NUL
                start "" "{currentExe}"
                del "%~f0"
                """;
            File.WriteAllText(batPath, script);
            Process.Start(new ProcessStartInfo(batPath)
            {
                UseShellExecute = true,
                CreateNoWindow  = true,
                WindowStyle     = ProcessWindowStyle.Hidden
            });
        }
        else
        {
            // Fallback: can't determine own path, just launch from temp.
            Process.Start(new ProcessStartInfo(tmpPath) { UseShellExecute = true });
        }

        Application.Current.Dispatcher.Invoke(() => Application.Current.Shutdown());
    }

    /// <summary>Recent launcher releases (newest first) for the in-app changelog viewer.</summary>
    public async Task<List<LauncherRelease>> GetRecentReleasesAsync(int max = 12)
    {
        var list = new List<LauncherRelease>();
        try
        {
            var cached = await GitHubApiCache.GetJsonAsync(_http, ReleasesApiUrl);
            using var doc = JsonDocument.Parse(cached.Body);
            if (doc.RootElement.ValueKind != JsonValueKind.Array) return list;

            foreach (var rel in doc.RootElement.EnumerateArray())
            {
                if (rel.TryGetProperty("draft", out var dr) && dr.GetBoolean()) continue;
                var tag  = rel.TryGetProperty("tag_name", out var t)     ? t.GetString() ?? "" : "";
                var date = rel.TryGetProperty("published_at", out var p) ? p.GetString() ?? "" : "";
                var body = rel.TryGetProperty("body", out var b)         ? b.GetString() ?? "" : "";
                if (string.IsNullOrEmpty(tag)) continue;
                list.Add(new LauncherRelease(tag, date, body));
                if (list.Count >= max) break;
            }
        }
        catch { /* offline / rate-limited — return whatever we have */ }
        return list;
    }

    // ── Client update checks (delegates) ─────────────────────────

    public async Task<bool> IsFlarialUpdateAvailableAsync()
    {
        try
        {
            if (!_flarial.IsDownloaded) return false;
            return !await _flarial.IsUpToDateAsync();
        }
        catch { return false; }
    }

    public async Task<bool> IsOderSoUpdateAvailableAsync()
    {
        try
        {
            if (!_oderso.IsDownloaded) return false;
            return !await _oderso.IsUpToDateAsync();
        }
        catch { return false; }
    }

    // ── Version comparison ────────────────────────────────────────

    private static bool IsNewerVersion(string remote, string local)
    {
        try
        {
            return new Version(remote) > new Version(local);
        }
        catch
        {
            return string.Compare(remote, local, StringComparison.Ordinal) > 0;
        }
    }
}
