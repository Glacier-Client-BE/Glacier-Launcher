using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

/// <summary>
/// Downloads LeviLamina (LiteLDev/LeviLamina) and injects its loader DLL the
/// same way Latite/Flarial/OderSo are injected via InjectionService — unlike
/// those single-file clients, LeviLamina ships as a multi-file release zip
/// (the loader plus its runtime dependencies), so this extracts the whole
/// archive into its own folder and locates the loader DLL inside it rather
/// than pointing at one fixed file name.
/// </summary>
public class LeviLaminaService
{
    private const string ApiUrl = "https://api.github.com/repos/LiteLDev/LeviLamina/releases/latest";

    public static string LeviLaminaDirectory =>
        Path.Combine(GameLauncher.DownloadsDirectory, "LeviLamina");

    private static string VersionFile =>
        Path.Combine(LeviLaminaDirectory, ".version");

    private readonly HttpClient      _http;
    private readonly DownloadService _download = new();

    public LeviLaminaService()
    {
        _http = HttpFactory.Shared;
        Directory.CreateDirectory(LeviLaminaDirectory);
    }

    public string? LastError { get; private set; }

    /// <summary>The loader DLL to inject, or null if LeviLamina hasn't been downloaded / extracted yet.</summary>
    public string? FilePath => FindLoaderDll();

    public bool IsDownloaded => FilePath != null;

    public async Task<bool> IsUpToDateAsync()
    {
        if (!IsDownloaded) return false;
        try
        {
            var (_, tag) = await FetchLatestReleaseAsync();
            var local = File.Exists(VersionFile) ? File.ReadAllText(VersionFile).Trim() : "";
            return string.Equals(local, tag, StringComparison.OrdinalIgnoreCase);
        }
        catch { return false; }
    }

    public async Task DownloadAsync(IProgress<double>? progress = null, CancellationToken cancel = default)
    {
        LastError = null;
        var (zipUrl, tag) = await FetchLatestReleaseAsync();
        if (string.IsNullOrEmpty(zipUrl))
            throw new InvalidOperationException("Couldn't find a downloadable LeviLamina release asset.");

        var tmpZip = Path.Combine(Path.GetTempPath(), "glacier_levilamina_" + Guid.NewGuid().ToString("N") + ".zip");
        try
        {
            await _download.DownloadAsync(zipUrl, tmpZip, progress: progress, cancel: cancel, label: "LeviLamina");

            // Wipe any previous extraction so a version swap can't leave stale
            // dependency DLLs alongside the new loader.
            if (Directory.Exists(LeviLaminaDirectory))
                Directory.Delete(LeviLaminaDirectory, recursive: true);
            Directory.CreateDirectory(LeviLaminaDirectory);

            await Task.Run(() => ZipFile.ExtractToDirectory(tmpZip, LeviLaminaDirectory, overwriteFiles: true), cancel);

            if (FindLoaderDll() == null)
                throw new InvalidOperationException(
                    "LeviLamina was downloaded but no loader DLL could be found inside the release archive.");

            File.WriteAllText(VersionFile, tag);
        }
        finally
        {
            try { if (File.Exists(tmpZip)) File.Delete(tmpZip); } catch { }
        }
    }

    public void Delete()
    {
        if (Directory.Exists(LeviLaminaDirectory))
            Directory.Delete(LeviLaminaDirectory, recursive: true);
        Directory.CreateDirectory(LeviLaminaDirectory);
    }

    // ── Internals ────────────────────────────────────────────────

    private async Task<(string zipUrl, string tag)> FetchLatestReleaseAsync()
    {
        var cached = await GitHubApiCache.GetJsonAsync(_http, ApiUrl);
        LastError = cached.Error;

        using var doc = JsonDocument.Parse(cached.Body);
        var root = doc.RootElement;
        var tag = root.TryGetProperty("tag_name", out var t) ? t.GetString() ?? "" : "";

        string? zipUrl = null;
        if (root.TryGetProperty("assets", out var assets) && assets.ValueKind == JsonValueKind.Array)
        {
            // Prefer an asset that looks like the Windows x64 build; fall back to
            // the first .zip if naming doesn't match (release naming has changed
            // before and may again).
            var candidates = assets.EnumerateArray()
                .Select(a => (Name: a.TryGetProperty("name", out var n) ? n.GetString() ?? "" : "",
                               Url: a.TryGetProperty("browser_download_url", out var u) ? u.GetString() ?? "" : ""))
                .Where(a => a.Name.EndsWith(".zip", StringComparison.OrdinalIgnoreCase))
                .ToList();

            zipUrl = candidates
                .FirstOrDefault(a => a.Name.Contains("windows", StringComparison.OrdinalIgnoreCase)
                                   || a.Name.Contains("win", StringComparison.OrdinalIgnoreCase)).Url
                ?? candidates.FirstOrDefault().Url;
        }

        return (zipUrl ?? "", tag);
    }

    private static string? FindLoaderDll()
    {
        if (!Directory.Exists(LeviLaminaDirectory)) return null;
        try
        {
            var dlls = Directory.GetFiles(LeviLaminaDirectory, "*.dll", SearchOption.AllDirectories);
            if (dlls.Length == 0) return null;

            return dlls.FirstOrDefault(f => Path.GetFileName(f).Contains("Lamina", StringComparison.OrdinalIgnoreCase))
                ?? dlls.FirstOrDefault(f => Path.GetFileName(f).Contains("Loader", StringComparison.OrdinalIgnoreCase))
                ?? (dlls.Length == 1 ? dlls[0] : null); // ambiguous with no name match — don't guess wrong
        }
        catch { return null; }
    }
}
