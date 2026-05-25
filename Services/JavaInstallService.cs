using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public sealed class JavaInstallService
{
    private const string AssetsBaseUrl = "https://resources.download.minecraft.net";
    private const int ParallelDownloads = 6;

    private readonly JavaVersionService _versions;
    private readonly HttpClient         _http;

    public JavaInstallService(JavaVersionService versions)
    {
        _versions = versions;
        _http     = HttpFactory.Shared;
    }

    public sealed class Progress
    {
        public string Stage { get; internal set; } = "";
        public double Percent { get; internal set; }
        public int FilesDone { get; internal set; }
        public int FilesTotal { get; internal set; }
    }

    public async Task InstallAsync(
        JavaVersion version,
        Action<Progress>? report   = null,
        CancellationToken cancel    = default)
    {
        if (string.IsNullOrEmpty(version.Url))
            throw new InvalidOperationException($"Version {version.Id} is missing a manifest URL — refresh the Java versions list.");

        var progress = new Progress();
        void Tick(string stage, double pct, int done = 0, int total = 0)
        {
            progress.Stage = stage; progress.Percent = pct;
            progress.FilesDone = done; progress.FilesTotal = total;
            report?.Invoke(progress);
        }

        var mcDir      = _versions.MinecraftDir;
        var versionDir = Path.Combine(mcDir, "versions", version.Id);
        Directory.CreateDirectory(versionDir);

        Tick("Downloading version metadata…", 1);
        var versionJsonPath = Path.Combine(versionDir, version.Id + ".json");
        var versionJsonText = await GetStringAsync(version.Url, cancel).ConfigureAwait(false);
        await File.WriteAllTextAsync(versionJsonPath, versionJsonText, cancel).ConfigureAwait(false);

        using var versionDoc = JsonDocument.Parse(versionJsonText);
        var root = versionDoc.RootElement;

        Tick("Downloading client jar…", 4);
        if (root.TryGetProperty("downloads", out var downloads)
            && downloads.TryGetProperty("client", out var clientDl))
        {
            var url    = clientDl.GetProperty("url").GetString()!;
            var sha1   = clientDl.TryGetProperty("sha1", out var s) ? s.GetString() : null;
            var jarPath = Path.Combine(versionDir, version.Id + ".jar");
            await DownloadFileAsync(url, jarPath, sha1, cancel).ConfigureAwait(false);
        }

        Tick("Downloading libraries…", 8);
        var (libFiles, nativeJars) = CollectLibraries(root, mcDir);
        await DownloadManyAsync(libFiles, cancel,
            (done, total) => Tick($"Libraries ({done}/{total})", 8 + done * 22.0 / Math.Max(1, total), done, total)).ConfigureAwait(false);

        Tick("Extracting natives…", 32);
        var nativesDir = Path.Combine(versionDir, "natives");
        Directory.CreateDirectory(nativesDir);
        foreach (var nj in nativeJars)
        {
            try
            {
                if (!File.Exists(nj)) continue;
                using var zip = ZipFile.OpenRead(nj);
                foreach (var entry in zip.Entries)
                {
                    if (entry.FullName.StartsWith("META-INF/", StringComparison.OrdinalIgnoreCase)) continue;
                    if (string.IsNullOrEmpty(entry.Name)) continue;
                    var dest = Path.Combine(nativesDir, entry.Name);
                    entry.ExtractToFile(dest, overwrite: true);
                }
            }
            catch { }
        }

        Tick("Downloading asset index…", 34);
        var (assetFiles, isLegacy, isVirtual, assetIndexName) = await ResolveAssetsAsync(root, mcDir, cancel).ConfigureAwait(false);
        Tick($"Downloading assets…", 36, 0, assetFiles.Count);
        await DownloadManyAsync(assetFiles, cancel,
            (done, total) => Tick($"Assets ({done}/{total})", 36 + done * 60.0 / Math.Max(1, total), done, total)).ConfigureAwait(false);

        if (isLegacy || isVirtual)
        {
            Tick("Linking legacy/virtual assets…", 96);
            var virtRoot = isLegacy
                ? Path.Combine(mcDir, "assets", "virtual", "legacy")
                : Path.Combine(mcDir, "assets", "virtual", assetIndexName);
            Directory.CreateDirectory(virtRoot);
            foreach (var f in assetFiles)
            {
                if (f.VirtualPath is null) continue;
                try
                {
                    var dest = Path.Combine(virtRoot, f.VirtualPath);
                    Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
                    if (!File.Exists(dest)) File.Copy(f.Destination, dest);
                }
                catch { }
            }
        }

        Tick("Done", 100);
    }

    private (List<DownloadJob> libs, List<string> natives) CollectLibraries(JsonElement root, string mcDir)
    {
        var jobs    = new List<DownloadJob>();
        var natives = new List<string>();

        if (!root.TryGetProperty("libraries", out var libs)) return (jobs, natives);

        foreach (var lib in libs.EnumerateArray())
        {
            if (!RulesAllow(lib)) continue;

            if (lib.TryGetProperty("downloads", out var dl))
            {
                if (dl.TryGetProperty("artifact", out var art)
                    && art.TryGetProperty("url", out var u)
                    && art.TryGetProperty("path", out var p))
                {
                    var url  = u.GetString() ?? "";
                    var path = Path.Combine(mcDir, "libraries", p.GetString()!.Replace('/', Path.DirectorySeparatorChar));
                    var sha1 = art.TryGetProperty("sha1", out var s) ? s.GetString() : null;
                    if (!string.IsNullOrEmpty(url))
                        jobs.Add(new DownloadJob(url, path, sha1));
                }

                if (dl.TryGetProperty("classifiers", out var classifiers)
                    && classifiers.ValueKind == JsonValueKind.Object
                    && lib.TryGetProperty("natives", out var nativesElem)
                    && nativesElem.TryGetProperty("windows", out var key))
                {
                    var k = key.GetString()?.Replace("${arch}", "64") ?? "";
                    if (classifiers.TryGetProperty(k, out var native)
                        && native.TryGetProperty("url",  out var nu)
                        && native.TryGetProperty("path", out var np))
                    {
                        var url  = nu.GetString() ?? "";
                        var path = Path.Combine(mcDir, "libraries", np.GetString()!.Replace('/', Path.DirectorySeparatorChar));
                        var sha1 = native.TryGetProperty("sha1", out var s) ? s.GetString() : null;
                        if (!string.IsNullOrEmpty(url))
                        {
                            jobs.Add(new DownloadJob(url, path, sha1));
                            natives.Add(path);
                        }
                    }
                }
            }
        }

        return (jobs, natives);
    }

    private static bool RulesAllow(JsonElement el)
    {
        if (!el.TryGetProperty("rules", out var rules) || rules.ValueKind != JsonValueKind.Array)
            return true;
        bool allowed = false;
        foreach (var rule in rules.EnumerateArray())
        {
            var action = rule.TryGetProperty("action", out var a) ? a.GetString() : "allow";
            if (!OsMatches(rule)) continue;
            allowed = action == "allow";
        }
        return allowed;
    }

    private static bool OsMatches(JsonElement rule)
    {
        if (!rule.TryGetProperty("os", out var os)) return true;
        if (os.TryGetProperty("name", out var n))
        {
            var name = n.GetString();
            if (!string.Equals(name, "windows", StringComparison.OrdinalIgnoreCase))
                return false;
        }
        return true;
    }

    private async Task<(List<AssetJob> jobs, bool isLegacy, bool isVirtual, string indexName)>
        ResolveAssetsAsync(JsonElement versionRoot, string mcDir, CancellationToken cancel)
    {
        if (!versionRoot.TryGetProperty("assetIndex", out var idx))
            return (new(), false, false, "legacy");

        var url       = idx.GetProperty("url").GetString()!;
        var indexId   = idx.GetProperty("id").GetString() ?? "legacy";
        var sha1      = idx.TryGetProperty("sha1", out var s) ? s.GetString() : null;
        var indexPath = Path.Combine(mcDir, "assets", "indexes", indexId + ".json");

        await DownloadFileAsync(url, indexPath, sha1, cancel).ConfigureAwait(false);

        var indexText = await File.ReadAllTextAsync(indexPath, cancel).ConfigureAwait(false);
        using var indexDoc = JsonDocument.Parse(indexText);
        var indexRoot = indexDoc.RootElement;

        bool isLegacy  = indexRoot.TryGetProperty("map_to_resources", out var l) && l.GetBoolean();
        bool isVirtual = indexRoot.TryGetProperty("virtual", out var v) && v.GetBoolean();

        var jobs = new List<AssetJob>();
        if (indexRoot.TryGetProperty("objects", out var objects))
        {
            var objectsDir = Path.Combine(mcDir, "assets", "objects");
            foreach (var prop in objects.EnumerateObject())
            {
                var hash = prop.Value.GetProperty("hash").GetString() ?? "";
                if (hash.Length < 2) continue;
                var sub   = hash[..2];
                var dest  = Path.Combine(objectsDir, sub, hash);
                var dlUrl = $"{AssetsBaseUrl}/{sub}/{hash}";
                jobs.Add(new AssetJob(dlUrl, dest, hash, (isLegacy || isVirtual) ? prop.Name : null));
            }
        }

        return (jobs, isLegacy, isVirtual, indexId);
    }

    private record DownloadJob(string Url, string Destination, string? Sha1);

    private sealed record AssetJob(string Url, string Destination, string? Sha1, string? VirtualPath)
        : DownloadJob(Url, Destination, Sha1);

    private async Task DownloadManyAsync<T>(
        List<T> jobs, CancellationToken cancel, Action<int, int>? report = null)
        where T : DownloadJob
    {
        if (jobs.Count == 0) return;

        int done = 0;
        var options = new ParallelOptions
        {
            MaxDegreeOfParallelism = ParallelDownloads,
            CancellationToken = cancel
        };

        await Parallel.ForEachAsync(jobs, options, async (j, ct) =>
        {
            try
            {
                await DownloadFileAsync(j.Url, j.Destination, j.Sha1, ct).ConfigureAwait(false);
            }
            catch
            {
            }
            finally
            {
                var n = Interlocked.Increment(ref done);
                report?.Invoke(n, jobs.Count);
            }
        }).ConfigureAwait(false);
    }

    private async Task DownloadFileAsync(string url, string dest, string? sha1, CancellationToken cancel)
    {
        if (File.Exists(dest))
        {
            if (string.IsNullOrEmpty(sha1)) return;
            if (await VerifySha1Async(dest, sha1!, cancel).ConfigureAwait(false)) return;
        }

        Directory.CreateDirectory(Path.GetDirectoryName(dest)!);

        var tmp = dest + ".part";
        using (var resp = await _http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, cancel).ConfigureAwait(false))
        {
            resp.EnsureSuccessStatusCode();
            await using var src  = await resp.Content.ReadAsStreamAsync(cancel).ConfigureAwait(false);
            await using var fs   = File.Create(tmp);
            await src.CopyToAsync(fs, cancel).ConfigureAwait(false);
        }

        if (File.Exists(dest)) File.Delete(dest);
        File.Move(tmp, dest);
    }

    private async Task<string> GetStringAsync(string url, CancellationToken cancel)
    {
        using var resp = await _http.GetAsync(url, cancel).ConfigureAwait(false);
        resp.EnsureSuccessStatusCode();
        return await resp.Content.ReadAsStringAsync(cancel).ConfigureAwait(false);
    }

    private static async Task<bool> VerifySha1Async(string path, string expected, CancellationToken cancel)
    {
        try
        {
            await using var fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read, 4096, useAsync: true);
            byte[] hash = await SHA1.HashDataAsync(fs, cancel).ConfigureAwait(false);
            Span<byte> expectedBytes = stackalloc byte[20];
            if (TryParseHexToBytes(expected, expectedBytes))
            {
                return expectedBytes.SequenceEqual(hash);
            }
            return false;
        }
        catch { return false; }
    }

    private static bool TryParseHexToBytes(string hex, Span<byte> bytes)
    {
        if (hex.Length != 40 || bytes.Length < 20) return false;
        for (int i = 0; i < 20; i++)
        {
            int high = HexVal(hex[i * 2]);
            int low = HexVal(hex[i * 2 + 1]);
            if (high < 0 || low < 0) return false;
            bytes[i] = (byte)((high << 4) | low);
        }
        return true;
    }

    private static int HexVal(char hex)
    {
        int val = hex;
        return val switch
        {
            >= '0' and <= '9' => val - '0',
            >= 'A' and <= 'F' => val - 'A' + 10,
            >= 'a' and <= 'f' => val - 'a' + 10,
            _ => -1
        };
    }
}
