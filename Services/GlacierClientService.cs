using System.IO;
using System.Net.Http;
using System.Net.Http.Json;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Downloads and manages Glacier Client JARs from the CDN.
/// Manifest:     https://cdn.glacierclient.xyz/versions.json
/// Install path: ~/.glacier/versions/{id}/Glacier-{id}.jar
/// </summary>
public sealed class GlacierClientService
{
    private const string ManifestUrl = "https://cdn.glacierclient.xyz/versions.json";

    private readonly HttpClient       _http;
    private readonly JavaGameLauncher _javaLauncher;
    private readonly DownloadService  _download;
    private GlacierManifest?          _cachedManifest;

    public string? LastError { get; private set; }

    public GlacierClientService(JavaGameLauncher javaLauncher, DownloadService download)
    {
        _javaLauncher = javaLauncher;
        _download     = download;
        _http         = HttpFactory.Shared;
    }

    // ── Manifest ─────────────────────────────────────────────────────────────

    public async Task<GlacierManifest?> GetManifestAsync(bool forceRefresh = false)
    {
        if (_cachedManifest != null && !forceRefresh) return _cachedManifest;
        LastError = null;
        try
        {
            _cachedManifest = await _http.GetFromJsonAsync<GlacierManifest>(ManifestUrl);
            return _cachedManifest;
        }
        catch (Exception ex)
        {
            LastError = $"Failed to fetch manifest: {ex.Message}";
            return null;
        }
    }

    // ── Install ───────────────────────────────────────────────────────────────

    public async Task InstallAsync(
        GlacierClientVersion version, IProgress<double>? progress = null, CancellationToken cancel = default) =>
        await _download.DownloadAsync(version.Url, version.JarPath, version.Sha256, progress, cancel: cancel);

    // ── Uninstall ─────────────────────────────────────────────────────────────

    public void Uninstall(GlacierClientVersion version)
    {
        if (File.Exists(version.JarPath)) File.Delete(version.JarPath);
    }

    // ── Launch ────────────────────────────────────────────────────────────────

    /// <summary>
    /// Injects the Glacier Client JAR as a -javaagent and launches the specified
    /// Minecraft version through the existing JavaGameLauncher pipeline.
    /// </summary>
    public async Task LaunchAsync(GlacierClientVersion version, string mcVersionId)
    {
        if (!version.IsInstalled)
            throw new InvalidOperationException($"Glacier {version.Id} is not installed.");

        await _javaLauncher.LaunchAsync(
            mcVersionId,
            extraJvmArgs: [$"-javaagent:{version.JarPath}"]);
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    public static string GetGlacierDir() =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".glacier");

    public static long GetInstalledSizeBytes()
    {
        var dir = GetGlacierDir();
        if (!Directory.Exists(dir)) return 0;
        return Directory.EnumerateFiles(dir, "*.jar", SearchOption.AllDirectories)
            .Sum(f => new FileInfo(f).Length);
    }
}
