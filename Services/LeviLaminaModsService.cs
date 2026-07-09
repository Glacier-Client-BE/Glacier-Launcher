using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

/// <summary>
/// Browses and installs mods for LeviLamina by reading the same package
/// registry the official "lip" package manager and LeviLauncher use
/// (LiteLDev/lipr's index.json) — each entry maps to a GitHub repo, and we
/// resolve the chosen version to that repo's matching GitHub release asset
/// (the same download mechanism used for LeviLamina itself), rather than
/// re-implementing lip's tooth.json install protocol.
/// </summary>
public class LeviLaminaModsService
{
    private const string RegistryUrl = "https://raw.githubusercontent.com/LiteLDev/lipr/main/index.json";

    public static string ModsDirectory =>
        Path.Combine(LeviLaminaService.LeviLaminaDirectory, "plugins");

    private readonly HttpClient      _http;
    private readonly DownloadService _download = new();
    private List<LeviLaminaMod>?     _cache;

    public LeviLaminaModsService()
    {
        _http = HttpFactory.Shared;
    }

    public string? LastError { get; private set; }

    public record LeviLaminaMod(
        string RepoOwner,
        string RepoName,
        string Name,
        string Description,
        string AvatarUrl,
        string LatestVersion,
        int    Stars);

    public bool IsInstalled(LeviLaminaMod mod) =>
        Directory.Exists(Path.Combine(ModsDirectory, SanitizeFolderName(mod.RepoName)));

    public async Task<List<LeviLaminaMod>> SearchAsync(string query)
    {
        var all = await LoadAllAsync();
        if (string.IsNullOrWhiteSpace(query)) return all;
        return all.Where(m =>
            m.Name.Contains(query, StringComparison.OrdinalIgnoreCase) ||
            m.Description.Contains(query, StringComparison.OrdinalIgnoreCase)).ToList();
    }

    public async Task DownloadAndInstallAsync(LeviLaminaMod mod, IProgress<double>? progress = null, CancellationToken cancel = default)
    {
        // The registry lists bare version numbers ("2.4.0"), but most repos tag
        // their releases with a "v" prefix ("v2.4.0") — try both rather than
        // guessing wrong and surfacing a 404.
        var tagCandidates = mod.LatestVersion.StartsWith("v", StringComparison.OrdinalIgnoreCase)
            ? new[] { mod.LatestVersion, mod.LatestVersion[1..] }
            : new[] { mod.LatestVersion, "v" + mod.LatestVersion };

        string? assetUrl  = null;
        string? assetName = null;
        Exception? lastError = null;

        foreach (var tag in tagCandidates)
        {
            try
            {
                var releaseUrl = $"https://api.github.com/repos/{mod.RepoOwner}/{mod.RepoName}/releases/tags/{tag}";
                var cached = await GitHubApiCache.GetJsonAsync(_http, releaseUrl);
                if (!string.IsNullOrEmpty(cached.Error) && cached.FromCache == false)
                    throw new InvalidOperationException(cached.Error);

                using var doc = JsonDocument.Parse(cached.Body);
                if (doc.RootElement.TryGetProperty("message", out var msg) &&
                    string.Equals(msg.GetString(), "Not Found", StringComparison.OrdinalIgnoreCase))
                    continue;

                if (doc.RootElement.TryGetProperty("assets", out var assets) && assets.ValueKind == JsonValueKind.Array)
                {
                    var candidates = assets.EnumerateArray()
                        .Select(a => (Name: a.TryGetProperty("name", out var n) ? n.GetString() ?? "" : "",
                                       Url:  a.TryGetProperty("browser_download_url", out var u) ? u.GetString() ?? "" : ""))
                        .Where(a => a.Url.Length > 0)
                        .ToList();

                    var pick = candidates.FirstOrDefault(a => a.Name.EndsWith(".zip", StringComparison.OrdinalIgnoreCase));
                    if (pick.Url == null) pick = candidates.FirstOrDefault();
                    assetUrl  = pick.Url;
                    assetName = pick.Name;
                }

                if (!string.IsNullOrEmpty(assetUrl)) break;
            }
            catch (Exception ex) { lastError = ex; }
        }

        if (string.IsNullOrEmpty(assetUrl) || string.IsNullOrEmpty(assetName))
            throw new InvalidOperationException(
                $"No downloadable release asset found for {mod.Name} {mod.LatestVersion}." +
                (lastError != null ? $" ({lastError.Message})" : ""));

        Directory.CreateDirectory(ModsDirectory);
        var destDir = Path.Combine(ModsDirectory, SanitizeFolderName(mod.RepoName));
        var isZip   = assetName.EndsWith(".zip", StringComparison.OrdinalIgnoreCase);
        var tmp     = Path.Combine(Path.GetTempPath(),
            "glacier_levimod_" + Guid.NewGuid().ToString("N") + (isZip ? ".zip" : Path.GetExtension(assetName)));

        try
        {
            await _download.DownloadAsync(assetUrl, tmp, progress: progress, cancel: cancel, label: mod.Name);

            if (Directory.Exists(destDir)) Directory.Delete(destDir, recursive: true);
            Directory.CreateDirectory(destDir);

            if (isZip)
                await Task.Run(() => ZipFile.ExtractToDirectory(tmp, destDir, overwriteFiles: true), cancel);
            else
                File.Copy(tmp, Path.Combine(destDir, assetName), overwrite: true);
        }
        finally
        {
            try { if (File.Exists(tmp)) File.Delete(tmp); } catch { }
        }
    }

    public void Delete(LeviLaminaMod mod)
    {
        var dir = Path.Combine(ModsDirectory, SanitizeFolderName(mod.RepoName));
        if (Directory.Exists(dir)) Directory.Delete(dir, recursive: true);
    }

    // ── Internals ────────────────────────────────────────────────

    private async Task<List<LeviLaminaMod>> LoadAllAsync()
    {
        if (_cache != null) return _cache;

        var cached = await GitHubApiCache.GetJsonAsync(_http, RegistryUrl);
        LastError = cached.Error;

        var list = new List<LeviLaminaMod>();
        using var doc = JsonDocument.Parse(cached.Body);
        if (doc.RootElement.TryGetProperty("packages", out var packages))
        {
            foreach (var entry in packages.EnumerateObject())
            {
                // Keys look like "github.com/Owner/Repo".
                var parts = entry.Name.Split('/');
                if (parts.Length < 3 || !parts[0].Equals("github.com", StringComparison.OrdinalIgnoreCase))
                    continue;
                var owner = parts[1];
                var repo  = parts[2];

                var pkg = entry.Value;
                if (!pkg.TryGetProperty("info", out var info)) continue;

                var tags = info.TryGetProperty("tags", out var tagsEl) && tagsEl.ValueKind == JsonValueKind.Array
                    ? tagsEl.EnumerateArray().Select(t => t.GetString() ?? "").ToList()
                    : new List<string>();

                var isLevilaminaMod =
                    tags.Any(t => t.Contains("levilamina", StringComparison.OrdinalIgnoreCase)) &&
                    tags.Any(t => t.Contains("mod", StringComparison.OrdinalIgnoreCase));
                if (!isLevilaminaMod) continue;

                var name   = info.TryGetProperty("name", out var n) ? n.GetString() ?? repo : repo;
                var desc   = info.TryGetProperty("description", out var d) ? d.GetString() ?? "" : "";
                var avatar = info.TryGetProperty("avatar_url", out var a) ? a.GetString() ?? "" : "";
                var stars  = pkg.TryGetProperty("stargazer_count", out var sc) ? sc.GetInt32() : 0;

                var latestVersion = "";
                if (pkg.TryGetProperty("variants", out var variants) && variants.ValueKind == JsonValueKind.Object)
                {
                    foreach (var variant in variants.EnumerateObject())
                    {
                        if (variant.Value.TryGetProperty("versions", out var versions) &&
                            versions.ValueKind == JsonValueKind.Array && versions.GetArrayLength() > 0)
                        {
                            latestVersion = versions[versions.GetArrayLength() - 1].GetString() ?? "";
                            if (!string.IsNullOrEmpty(latestVersion)) break;
                        }
                    }
                }
                if (string.IsNullOrEmpty(latestVersion)) continue;

                list.Add(new LeviLaminaMod(owner, repo, name, desc, avatar, latestVersion, stars));
            }
        }

        list = list.OrderByDescending(m => m.Stars).ToList();
        _cache = list;
        return list;
    }

    private static string SanitizeFolderName(string name)
    {
        foreach (var c in Path.GetInvalidFileNameChars())
            name = name.Replace(c, '_');
        return name.Trim();
    }
}
