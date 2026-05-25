using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public sealed class JavaVersionService
{
    private const string ManifestUrl =
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

    private readonly SettingsService _settings;
    private readonly JavaInstanceService _instances;
    private readonly HttpClient      _http;

    public string? LastError      { get; private set; }
    public string? LatestRelease  { get; private set; }
    public string? LatestSnapshot { get; private set; }

    public JavaVersionService(SettingsService settings, JavaInstanceService instances)
    {
        _settings = settings;
        _instances = instances;
        _http     = HttpFactory.Shared;
        Directory.CreateDirectory(Path.GetDirectoryName(CacheFile)!);
    }

    public string MinecraftDir
    {
        get
        {
            var s = _settings.Settings.JavaMinecraftDir;
            if (!string.IsNullOrEmpty(s)) return s;
            return _instances.ActiveMinecraftDir;
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
            versions = await FetchManifestAsync().ConfigureAwait(false);
            SaveCache(versions);
        }
        catch (Exception ex)
        {
            LastError = $"Failed to fetch Mojang manifest: {ex.Message}";
            versions  = LoadCache();
            if (versions.Count > 0)
                LastError += " (showing cached data)";
        }

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
        using var resp = await _http.GetAsync(ManifestUrl, HttpCompletionOption.ResponseHeadersRead).ConfigureAwait(false);
        resp.EnsureSuccessStatusCode();
        await using var stream = await resp.Content.ReadAsStreamAsync().ConfigureAwait(false);

        using var doc = await JsonDocument.ParseAsync(stream).ConfigureAwait(false);
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
        catch { }
    }

    private List<JavaVersion> LoadCache()
    {
        try
        {
            if (!File.Exists(CacheFile)) return new();
            using var fs = File.OpenRead(CacheFile);
            using var doc = JsonDocument.Parse(fs);
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
