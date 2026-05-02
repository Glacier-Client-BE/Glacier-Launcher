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

public class AutoUpdateService
{
    // ── Launcher identity ─────────────────────────────────────────
    public static readonly string CurrentVersion =
        Assembly.GetEntryAssembly()?.GetName().Version?.ToString(3) ?? "0.0.0";

    // Change these to your actual GitHub org/repo once you push releases.
    private const string LauncherOrg  = "Glacier-Client-BE";
    private const string LauncherRepo = "Glacier-Launcher";

    private static string LatestReleaseApiUrl =>
        $"https://api.github.com/repos/{LauncherOrg}/{LauncherRepo}/releases/latest";

    // ── Injected services ─────────────────────────────────────────
    private readonly FlarialService _flarial;
    private readonly OderSoService  _oderso;
    private readonly HttpClient     _http;

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
            var cached = await GitHubApiCache.GetJsonAsync(_http, LatestReleaseApiUrl);
            LastCheckStatus = cached.Error;
            using var doc = JsonDocument.Parse(cached.Body);
            var root = doc.RootElement;

            var tag = root.TryGetProperty("tag_name", out var tagProp)
                ? tagProp.GetString()?.TrimStart('v') ?? ""
                : "";

            if (string.IsNullOrEmpty(tag)) return null;
            if (!IsNewerVersion(tag, CurrentVersion)) return null;

            var changelog = root.TryGetProperty("body", out var bodyProp)
                ? bodyProp.GetString() ?? ""
                : "";

            // Pick the first .exe asset, fallback to first asset of any type.
            string? downloadUrl = null;
            long    assetSize   = 0;

            if (root.TryGetProperty("assets", out var assets))
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

            return new LauncherUpdateInfo(tag, downloadUrl, changelog, assetSize);
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

        using var resp = await _http.GetAsync(info.DownloadUrl, HttpCompletionOption.ResponseHeadersRead);
        resp.EnsureSuccessStatusCode();

        var  total      = resp.Content.Headers.ContentLength ?? info.AssetSize;
        using var src   = await resp.Content.ReadAsStreamAsync();
        using var dest  = File.Create(tmpPath);

        var  buf        = new byte[81920];
        long downloaded = 0;
        int  read;
        while ((read = await src.ReadAsync(buf)) > 0)
        {
            await dest.WriteAsync(buf.AsMemory(0, read));
            downloaded += read;
            if (total > 0) progress?.Report(downloaded * 100.0 / total);
        }

        dest.Close();

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
                copy /Y "{tmpPath}" "{currentExe}" >NUL
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
