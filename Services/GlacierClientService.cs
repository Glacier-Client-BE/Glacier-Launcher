using System.Net.Http.Json;
using System.Security.Cryptography;
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

    private readonly HttpClient      _http;
    private readonly JavaGameLauncher _javaLauncher;
    private GlacierManifest?         _cachedManifest;

    public string? LastError { get; private set; }

    public GlacierClientService(JavaGameLauncher javaLauncher)
    {
        _javaLauncher = javaLauncher;
        _http         = HttpFactory.Create("GlacierLauncher/1.0");
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

    public async Task InstallAsync(GlacierClientVersion version, IProgress<double>? progress = null)
    {
        Directory.CreateDirectory(version.InstallDir);
        var tmp = version.JarPath + ".tmp";
        try
        {
            using var response = await _http.GetAsync(version.Url, HttpCompletionOption.ResponseHeadersRead);
            response.EnsureSuccessStatusCode();

            var total = response.Content.Headers.ContentLength ?? -1L;
            using var sha  = SHA256.Create();
            await using var net  = await response.Content.ReadAsStreamAsync();
            await using var file = new FileStream(tmp, FileMode.Create, FileAccess.Write, FileShare.None, 81920, true);

            var buf = new byte[81920];
            long downloaded = 0;
            int  read;
            while ((read = await net.ReadAsync(buf)) > 0)
            {
                await file.WriteAsync(buf.AsMemory(0, read));
                sha.TransformBlock(buf, 0, read, null, 0);
                downloaded += read;
                if (total > 0) progress?.Report((double)downloaded / total);
            }

            sha.TransformFinalBlock([], 0, 0);
            var actual = Convert.ToHexStringLower(sha.Hash!);

            if (!string.IsNullOrEmpty(version.Sha256)
                && !version.Sha256.StartsWith("0000")
                && !actual.Equals(version.Sha256, StringComparison.OrdinalIgnoreCase))
            {
                File.Delete(tmp);
                throw new InvalidDataException(
                    $"SHA256 mismatch for {version.Id}: expected {version.Sha256}, got {actual}");
            }

            File.Move(tmp, version.JarPath, overwrite: true);
        }
        catch
        {
            if (File.Exists(tmp)) File.Delete(tmp);
            throw;
        }
    }

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
