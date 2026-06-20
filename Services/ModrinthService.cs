using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

public class ModrinthService
{
    private const string BaseUrl = "https://api.modrinth.com/v2";

    private readonly HttpClient _http;
    private readonly SettingsService _settings;
    private readonly JavaVersionService? _javaVersions;
    private readonly DownloadService _download = new();

    public ModrinthService(SettingsService settings, JavaVersionService javaVersions)
    {
        _settings = settings;
        _javaVersions = javaVersions;
        _http = HttpFactory.Shared;
    }

    private bool IsJava => string.Equals(_settings.Settings.Edition, "java", StringComparison.OrdinalIgnoreCase);

    public IReadOnlyList<(string Key, string Label, string Icon)> AvailableCategories => IsJava
        ? new (string, string, string)[]
        {
            ("mod",          "Mods",           "fa-solid fa-puzzle-piece"),
            ("modpack",      "Modpacks",       "fa-solid fa-cubes"),
            ("resourcepack", "Resource Packs", "fa-solid fa-palette"),
            ("shader",       "Shaders",        "fa-solid fa-droplet"),
        }
        : new (string, string, string)[]
        {
            ("resourcepack", "Resource Packs", "fa-solid fa-palette"),
        };

    public record MrProject(
        string Id,
        string Slug,
        string Title,
        string Description,
        string IconUrl,
        string Author,
        long   Downloads,
        string ProjectType,
        string? CategoryName);

    public record MrVersion(
        string Id,
        string Name,
        string FileName,
        string Url,
        long   Size);

    public record MrSearchResult(List<MrProject> Projects, int TotalCount);

    public async Task<MrSearchResult> SearchAsync(string query, string facetType = "", int limit = 20, int offset = 0)
    {
        var facets = new List<string>();
        if (!string.IsNullOrEmpty(facetType))
            facets.Add($"[\"project_type:{facetType}\"]");

        var facetParam = facets.Count > 0 ? $"&facets=[{string.Join(",", facets)}]" : "";
        var queryParam = string.IsNullOrWhiteSpace(query) ? "" : $"&query={Uri.EscapeDataString(query)}";
        var url = $"{BaseUrl}/search?limit={limit}&offset={offset}{queryParam}{facetParam}";

        using var req = new HttpRequestMessage(HttpMethod.Get, url);
        req.Headers.TryAddWithoutValidation("User-Agent", "GlacierLauncher/1.0 (glacier-launcher)");
        using var resp = await _http.SendAsync(req);
        resp.EnsureSuccessStatusCode();

        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        var projects = new List<MrProject>();

        if (doc.RootElement.TryGetProperty("hits", out var hits))
        {
            foreach (var item in hits.EnumerateArray())
            {
                var id = item.GetProperty("project_id").GetString() ?? "";
                var slug = item.TryGetProperty("slug", out var sl) ? sl.GetString() ?? "" : "";
                var title = item.TryGetProperty("title", out var t) ? t.GetString() ?? "" : "";
                var desc = item.TryGetProperty("description", out var d) ? d.GetString() ?? "" : "";
                var icon = item.TryGetProperty("icon_url", out var ic) ? ic.GetString() ?? "" : "";
                var author = item.TryGetProperty("author", out var a) ? a.GetString() ?? "" : "";
                var downloads = item.TryGetProperty("downloads", out var dl) ? dl.GetInt64() : 0;
                var projType = item.TryGetProperty("project_type", out var pt) ? pt.GetString() ?? "" : "";

                string? catName = null;
                if (item.TryGetProperty("categories", out var cats) && cats.GetArrayLength() > 0)
                    catName = cats[0].GetString();

                projects.Add(new MrProject(id, slug, title, desc, icon, author, downloads, projType, catName));
            }
        }

        var total = doc.RootElement.TryGetProperty("total_hits", out var th) ? th.GetInt32() : 0;
        return new MrSearchResult(projects, total);
    }

    public async Task<MrVersion?> GetLatestVersionAsync(string projectId)
    {
        var url = $"{BaseUrl}/project/{projectId}/version?limit=1";
        using var req = new HttpRequestMessage(HttpMethod.Get, url);
        req.Headers.TryAddWithoutValidation("User-Agent", "GlacierLauncher/1.0 (glacier-launcher)");
        using var resp = await _http.SendAsync(req);
        resp.EnsureSuccessStatusCode();

        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        foreach (var ver in doc.RootElement.EnumerateArray())
        {
            if (!ver.TryGetProperty("files", out var files) || files.GetArrayLength() == 0)
                continue;

            var primary = files.EnumerateArray().FirstOrDefault(f =>
                f.TryGetProperty("primary", out var p) && p.GetBoolean());
            if (primary.ValueKind == JsonValueKind.Undefined)
                primary = files[0];

            var fileName = primary.GetProperty("filename").GetString() ?? "";
            var dlUrl = primary.GetProperty("url").GetString() ?? "";
            var size = primary.TryGetProperty("size", out var sz) ? sz.GetInt64() : 0;
            var name = ver.TryGetProperty("name", out var n) ? n.GetString() ?? fileName : fileName;
            var verId = ver.GetProperty("id").GetString() ?? "";

            return new MrVersion(verId, name, fileName, dlUrl, size);
        }
        return null;
    }

    public async Task DownloadAndInstallAsync(MrVersion version, string projectType, IProgress<double>? progress = null)
    {
        var tmpPath = Path.Combine(Path.GetTempPath(), version.FileName);
        try
        {
            await _download.DownloadAsync(
                version.Url, tmpPath,
                progress: progress,
                knownTotalBytes: version.Size,
                configureRequest: req => req.Headers.TryAddWithoutValidation(
                    "User-Agent", "GlacierLauncher/1.0 (glacier-launcher)"));

            InstallFile(tmpPath, projectType);
        }
        finally
        {
            try { if (File.Exists(tmpPath)) File.Delete(tmpPath); } catch { }
        }
    }

    private void InstallFile(string filePath, string projectType)
    {
        var mcDir = _javaVersions?.MinecraftDir ?? Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), ".minecraft");

        var targetDir = projectType switch
        {
            "mod"          => Path.Combine(mcDir, "mods"),
            "resourcepack" => Path.Combine(mcDir, "resourcepacks"),
            "shader"       => Path.Combine(mcDir, "shaderpacks"),
            _              => Path.Combine(mcDir, "mods"),
        };

        Directory.CreateDirectory(targetDir);
        File.Copy(filePath, Path.Combine(targetDir, Path.GetFileName(filePath)), overwrite: true);
    }
}
