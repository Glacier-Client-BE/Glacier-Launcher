using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Java Edition version manifest service. Pulls the Mojang piston-meta
/// version_manifest_v2.json (the same list the official launcher uses) and
/// cross-references it with the installed versions in
/// <c>%APPDATA%\.minecraft\versions\</c>.
/// </summary>
public sealed class JavaVersionService
{
    private const string ManifestUrl =
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

    private readonly SettingsService _settings;
    private readonly HttpClient      _http;

    public string? LastError      { get; private set; }
    public string? LatestRelease  { get; private set; }
    public string? LatestSnapshot { get; private set; }

    public JavaVersionService(SettingsService settings)
    {
        _settings = settings;
        _http     = HttpFactory.Shared;
        Directory.CreateDirectory(Path.GetDirectoryName(CacheFile)!);
    }

    /// <summary>The user's .minecraft directory (override or %APPDATA%\.minecraft).</summary>
    public string MinecraftDir
    {
        get
        {
            var s = _settings.Settings.JavaMinecraftDir;
            if (!string.IsNullOrEmpty(s)) return s;
            return Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                ".minecraft");
        }
    }

    public string VersionsDir => Path.Combine(MinecraftDir, "versions");

    public static string CacheFile =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "cache", "java-versions.json");

    public async Task<List<JavaVersion>> GetVersionsAsync()
    {
        LastError = null;
        List<JavaVersion> versions;

        try
        {
            versions = await FetchManifestAsync();
            SaveCache(versions);
        }
        catch (Exception ex)
        {
            LastError = $"Failed to fetch Mojang manifest: {ex.Message}";
            versions  = LoadCache();
            if (versions.Count > 0)
                LastError += " (showing cached data)";
        }

        // Cross-reference with what's actually on disk so the UI can show
        // installed vs. needs-download without us re-walking the folder on every render.
        var active = _settings.Settings.JavaActiveVersion;
        foreach (var v in versions)
        {
            var dir = Path.Combine(VersionsDir, v.Id);
            v.IsInstalled = File.Exists(Path.Combine(dir, v.Id + ".json"));
            v.HasJar      = File.Exists(Path.Combine(dir, v.Id + ".jar"));
            v.IsActive    = v.Id == active;
        }

        return versions;
    }

    /// <summary>
    /// Returns versions found under .minecraft\versions that aren't in the
    /// upstream manifest — typically modloader profiles (Fabric, Forge,
    /// OptiFine) and ancient versions Mojang has dropped.
    /// </summary>
    public List<JavaVersion> ScanCustomInstalledVersions(IReadOnlyCollection<string> knownIds)
    {
        var results = new List<JavaVersion>();
        if (!Directory.Exists(VersionsDir)) return results;

        var active = _settings.Settings.JavaActiveVersion;
        var known  = new HashSet<string>(knownIds, StringComparer.OrdinalIgnoreCase);

        foreach (var dir in Directory.EnumerateDirectories(VersionsDir))
        {
            var id   = Path.GetFileName(dir);
            if (known.Contains(id)) continue;

            var jsonPath = Path.Combine(dir, id + ".json");
            if (!File.Exists(jsonPath)) continue;

            results.Add(new JavaVersion
            {
                Id           = id,
                Type         = "custom",
                IsInstalled  = true,
                HasJar       = File.Exists(Path.Combine(dir, id + ".jar")),
                IsActive     = id == active
            });
        }

        return results;
    }

    private async Task<List<JavaVersion>> FetchManifestAsync()
    {
        using var resp = await _http.GetAsync(ManifestUrl);
        resp.EnsureSuccessStatusCode();
        var json = await resp.Content.ReadAsStringAsync();

        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;

        if (root.TryGetProperty("latest", out var latest))
        {
            if (latest.TryGetProperty("release",  out var rel)) LatestRelease  = rel.GetString();
            if (latest.TryGetProperty("snapshot", out var snap)) LatestSnapshot = snap.GetString();
        }

        var list = new List<JavaVersion>(800);
        if (root.TryGetProperty("versions", out var arr) && arr.ValueKind == JsonValueKind.Array)
        {
            foreach (var v in arr.EnumerateArray())
            {
                list.Add(new JavaVersion
                {
                    Id              = v.TryGetProperty("id",              out var i)  ? i.GetString()  ?? "" : "",
                    Type            = v.TryGetProperty("type",            out var t)  ? t.GetString()  ?? "" : "",
                    Url             = v.TryGetProperty("url",             out var u)  ? u.GetString()  ?? "" : "",
                    Sha1            = v.TryGetProperty("sha1",            out var s)  ? s.GetString()  ?? "" : "",
                    ReleaseTime     = v.TryGetProperty("releaseTime",     out var rt) ? rt.GetString() ?? "" : "",
                    Time            = v.TryGetProperty("time",            out var tm) ? tm.GetString() ?? "" : "",
                    ComplianceLevel = v.TryGetProperty("complianceLevel", out var cl) ? cl.GetInt32() : 0,
                });
            }
        }

        return list;
    }

    private static void SaveCache(List<JavaVersion> versions)
    {
        try
        {
            var data = versions.Select(v => new
            {
                v.Id, v.Type, v.Url, v.Sha1, v.ReleaseTime, v.Time, v.ComplianceLevel
            }).ToList();
            File.WriteAllText(CacheFile,
                JsonSerializer.Serialize(data, new JsonSerializerOptions { WriteIndented = false }));
        }
        catch { /* cache is best-effort */ }
    }

    private List<JavaVersion> LoadCache()
    {
        try
        {
            if (!File.Exists(CacheFile)) return new();
            var json = File.ReadAllText(CacheFile);
            using var doc = JsonDocument.Parse(json);
            var list = new List<JavaVersion>();
            foreach (var el in doc.RootElement.EnumerateArray())
            {
                list.Add(new JavaVersion
                {
                    Id              = el.GetProperty("Id").GetString()      ?? "",
                    Type            = el.GetProperty("Type").GetString()    ?? "",
                    Url             = el.GetProperty("Url").GetString()     ?? "",
                    Sha1            = el.GetProperty("Sha1").GetString()    ?? "",
                    ReleaseTime     = el.GetProperty("ReleaseTime").GetString() ?? "",
                    Time            = el.GetProperty("Time").GetString()    ?? "",
                    ComplianceLevel = el.TryGetProperty("ComplianceLevel", out var cl) ? cl.GetInt32() : 0,
                });
            }
            return list;
        }
        catch { return new(); }
    }
}
