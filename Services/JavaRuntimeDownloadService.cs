using System;
using System.IO;
using System.IO.Compression;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

/// <summary>
/// Downloads a JDK from Adoptium (Eclipse Temurin) when the user doesn't have
/// the required Java version installed. Downloads are cached under
/// <c>~/Glacier Launcher/runtimes/java-{major}/</c> and reused across launches.
/// </summary>
public sealed class JavaRuntimeDownloadService
{
    private readonly HttpClient      _http     = HttpFactory.Shared;
    private readonly DownloadService _download = new();

    /// <summary>Root directory for all Glacier-managed JDKs.</summary>
    public static string RuntimesRoot { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
        "Glacier Launcher", "runtimes");

    /// <summary>
    /// Returns the path to javaw.exe for a Glacier-managed JDK of the given
    /// major version, or null if it hasn't been downloaded yet.
    /// </summary>
    public static string? GetCachedJavaw(int major)
    {
        var dir = Path.Combine(RuntimesRoot, $"java-{major}");
        if (!Directory.Exists(dir)) return null;

        // The extracted archive has a single top-level folder like
        // "jdk-21.0.10+7" — we search for javaw.exe under it.
        try
        {
            foreach (var f in Directory.EnumerateFiles(dir, "javaw.exe", SearchOption.AllDirectories))
                return f;
        }
        catch { }
        return null;
    }

    /// <summary>
    /// Downloads and extracts an Adoptium JDK for the given Java major version.
    /// Returns the path to javaw.exe inside the extracted tree.
    /// </summary>
    public async Task<string> DownloadAsync(
        int major,
        Action<string, double>? onProgress = null,
        CancellationToken cancel = default)
    {
        // Check cache first.
        var cached = GetCachedJavaw(major);
        if (cached != null) return cached;

        var destDir = Path.Combine(RuntimesRoot, $"java-{major}");
        Directory.CreateDirectory(destDir);

        // Adoptium API: ask for the latest GA release for this feature version.
        // We want a zip (not msi/tar.gz) for easy extraction.
        var arch = RuntimeInformation.OSArchitecture switch
        {
            Architecture.Arm64 => "aarch64",
            _ => "x64"
        };

        // First, query for the actual download URL + filename via the API.
        var infoUrl = $"https://api.adoptium.net/v3/assets/latest/{major}/hotspot?" +
                      $"os=windows&architecture={arch}&image_type=jdk";

        onProgress?.Invoke($"Querying Adoptium for Java {major}…", 0);

        var infoResp = await _http.GetAsync(infoUrl, cancel);
        if (!infoResp.IsSuccessStatusCode)
            throw new InvalidOperationException(
                $"Adoptium API returned {(int)infoResp.StatusCode} for Java {major}. " +
                $"This version may not be available yet — install it manually and set Settings → Java Runtime.");

        var infoJson = await infoResp.Content.ReadAsStringAsync(cancel);
        using var doc = JsonDocument.Parse(infoJson);
        var arr = doc.RootElement;
        if (arr.GetArrayLength() == 0)
            throw new InvalidOperationException(
                $"No Adoptium release found for Java {major} on Windows {arch}.");

        // Find the .zip package (prefer it over .msi).
        string? downloadUrl = null;
        long totalSize = 0;
        foreach (var release in arr.EnumerateArray())
        {
            if (!release.TryGetProperty("binary", out var binary)) continue;
            if (!binary.TryGetProperty("package", out var pkg)) continue;

            var link = pkg.GetProperty("link").GetString();
            var name = pkg.TryGetProperty("name", out var n) ? n.GetString() ?? "" : "";
            totalSize = pkg.TryGetProperty("size", out var sz) ? sz.GetInt64() : 0;

            if (name.EndsWith(".zip", StringComparison.OrdinalIgnoreCase))
            {
                downloadUrl = link;
                break;
            }
            // Fall back to whatever is available.
            downloadUrl ??= link;
        }

        if (string.IsNullOrEmpty(downloadUrl))
            throw new InvalidOperationException($"No downloadable package found for Java {major}.");

        // Download the archive.
        var zipPath = Path.Combine(destDir, $"java-{major}.zip");
        onProgress?.Invoke($"Downloading Java {major}…", 5);

        var progress = new DelegateProgress<double>(f =>
        {
            var pct = Math.Min(5 + f * 80.0, 85);
            if (totalSize > 0)
                onProgress?.Invoke(
                    $"Downloading Java {major}… ({(long)(f * totalSize) / (1024 * 1024)} / {totalSize / (1024 * 1024)} MB)",
                    pct);
            else
                onProgress?.Invoke($"Downloading Java {major}…", pct);
        });

        await _download.DownloadAsync(downloadUrl, zipPath, progress: progress, knownTotalBytes: totalSize, cancel: cancel);

        // Extract.
        onProgress?.Invoke($"Extracting Java {major}…", 88);
        try
        {
            ZipFile.ExtractToDirectory(zipPath, destDir, overwriteFiles: true);
        }
        finally
        {
            // Clean up the zip regardless of extraction success.
            try { File.Delete(zipPath); } catch { }
        }

        onProgress?.Invoke($"Java {major} ready.", 100);

        var javaw = GetCachedJavaw(major);
        if (javaw == null)
            throw new InvalidOperationException(
                $"Extracted Java {major} but couldn't find javaw.exe in {destDir}. " +
                "The archive layout may have changed — install Java manually.");

        return javaw;
    }
}
