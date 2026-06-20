using System;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using System.Xml.Linq;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public sealed class JavaModLoaderService
{
    private readonly JavaVersionService _versions;
    private readonly HttpClient _http;
    private readonly DownloadService _download = new();

    public JavaModLoaderService(JavaVersionService versions)
    {
        _versions = versions;
        _http = HttpFactory.Shared;
    }

    public async Task<JavaVersion> InstallFabricAsync(string minecraftVersion, string loaderVersion = "")
    {
        var loader = string.IsNullOrWhiteSpace(loaderVersion) ? await LatestFabricLoaderAsync() : loaderVersion;
        var url = $"https://meta.fabricmc.net/v2/versions/loader/{Uri.EscapeDataString(minecraftVersion)}/{Uri.EscapeDataString(loader)}/profile/json";
        var id = $"fabric-loader-{loader}-{minecraftVersion}";
        return await InstallProfileAsync(id, url, "Fabric", minecraftVersion);
    }

    public async Task<JavaVersion> InstallQuiltAsync(string minecraftVersion, string loaderVersion = "")
    {
        var loader = string.IsNullOrWhiteSpace(loaderVersion) ? await LatestQuiltLoaderAsync() : loaderVersion;
        var url = $"https://meta.quiltmc.org/v3/versions/loader/{Uri.EscapeDataString(minecraftVersion)}/{Uri.EscapeDataString(loader)}/profile/json";
        var id = $"quilt-loader-{loader}-{minecraftVersion}";
        return await InstallProfileAsync(id, url, "Quilt", minecraftVersion);
    }

    public async Task<string> DownloadForgeInstallerAsync(string minecraftVersion, string forgeVersion = "")
    {
        var build = string.IsNullOrWhiteSpace(forgeVersion) ? await LatestForgeVersionAsync(minecraftVersion) : forgeVersion;
        var artifact = await ResolveForgeArtifactAsync(minecraftVersion, build);
        var file = $"forge-{artifact}-installer.jar";
        var url = $"https://maven.minecraftforge.net/net/minecraftforge/forge/{artifact}/{file}";
        var dest = Path.Combine(JavaInstanceService.RootDir, "loaders", "forge", file);
        await _download.DownloadAsync(url, dest);
        return dest;
    }

    public async Task<string> DownloadNeoForgeInstallerAsync(string minecraftVersion)
    {
        var ver = await LatestNeoForgeVersionAsync(minecraftVersion);
        var file = $"neoforge-{ver}-installer.jar";
        var url = $"https://maven.neoforged.net/releases/net/neoforged/neoforge/{ver}/{file}";
        var dest = Path.Combine(JavaInstanceService.RootDir, "loaders", "neoforge", file);
        await _download.DownloadAsync(url, dest);
        return dest;
    }

    private async Task<JavaVersion> InstallProfileAsync(string id, string url, string loaderName, string minecraftVersion)
    {
        using var resp = await _http.GetAsync(url);
        if (resp.StatusCode is HttpStatusCode.NotFound or HttpStatusCode.BadRequest)
            throw new InvalidOperationException($"{loaderName} does not support Minecraft {minecraftVersion}.");
        resp.EnsureSuccessStatusCode();
        var json = await resp.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(json);
        var profileId = doc.RootElement.TryGetProperty("id", out var idProp) ? idProp.GetString() ?? id : id;
        var dir = Path.Combine(_versions.VersionsDir, profileId);
        Directory.CreateDirectory(dir);
        await File.WriteAllTextAsync(Path.Combine(dir, profileId + ".json"), json);
        return new JavaVersion
        {
            Id = profileId,
            Type = "custom",
            IsInstalled = true,
            HasJar = false
        };
    }

    private async Task<string> LatestFabricLoaderAsync()
    {
        using var resp = await _http.GetAsync("https://meta.fabricmc.net/v2/versions/loader");
        resp.EnsureSuccessStatusCode();
        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        foreach (var item in doc.RootElement.EnumerateArray())
            if (item.TryGetProperty("stable", out var stable) && stable.GetBoolean())
                return item.GetProperty("version").GetString() ?? "";
        return doc.RootElement[0].GetProperty("version").GetString() ?? "";
    }

    private async Task<string> LatestQuiltLoaderAsync()
    {
        using var resp = await _http.GetAsync("https://meta.quiltmc.org/v3/versions/loader");
        resp.EnsureSuccessStatusCode();
        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        string? newest = null;
        foreach (var item in doc.RootElement.EnumerateArray())
        {
            var version = item.GetProperty("version").GetString() ?? "";
            newest ??= version;
            if (!IsPreRelease(version))
                return version;
        }
        return newest ?? "";
    }

    private static bool IsPreRelease(string version) =>
        version.Contains("-beta", StringComparison.OrdinalIgnoreCase)
        || version.Contains("-pre", StringComparison.OrdinalIgnoreCase)
        || version.Contains("-rc", StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Resolves the exact Forge Maven artifact coordinate for a Minecraft
    /// version and build number. Legacy Minecraft versions (e.g. 1.7.10, 1.8.9)
    /// publish their installer under a branch-suffixed coordinate such as
    /// <c>1.8.9-11.15.1.2318-1.8.9</c>, which cannot be reconstructed from the
    /// promotions feed alone; the authoritative value is read from
    /// <c>maven-metadata.xml</c>. Falls back to <c>{mc}-{build}</c> when the
    /// metadata is unavailable or lists no match.
    /// </summary>
    private async Task<string> ResolveForgeArtifactAsync(string minecraftVersion, string build)
    {
        var coordinate = $"{minecraftVersion}-{build}";
        try
        {
            using var resp = await _http.GetAsync("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml");
            resp.EnsureSuccessStatusCode();
            var xml = await resp.Content.ReadAsStringAsync();
            var matches = XDocument.Parse(xml)
                .Descendants("version")
                .Select(v => v.Value)
                .Where(v => v == coordinate || v.StartsWith(coordinate + "-", StringComparison.Ordinal))
                .ToList();
            return matches.FirstOrDefault(v => v == coordinate) ?? matches.FirstOrDefault() ?? coordinate;
        }
        catch
        {
            return coordinate;
        }
    }

    private async Task<string> LatestForgeVersionAsync(string minecraftVersion)
    {
        using var resp = await _http.GetAsync("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json");
        resp.EnsureSuccessStatusCode();
        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        if (doc.RootElement.TryGetProperty("promos", out var promos))
        {
            if (promos.TryGetProperty($"{minecraftVersion}-recommended", out var rec))
                return rec.GetString() ?? "";
            if (promos.TryGetProperty($"{minecraftVersion}-latest", out var lat))
                return lat.GetString() ?? "";
        }
        throw new InvalidOperationException($"No Forge version found for Minecraft {minecraftVersion}. You can enter a version manually from files.minecraftforge.net.");
    }

    private async Task<string> LatestNeoForgeVersionAsync(string minecraftVersion)
    {
        // NeoForge versions follow the pattern: mcMajor.mcMinor.patch (e.g. 21.1.172 for MC 1.21.1)
        // The API lists all versions; we pick the latest matching the MC version.
        using var resp = await _http.GetAsync($"https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge");
        resp.EnsureSuccessStatusCode();
        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        if (!doc.RootElement.TryGetProperty("versions", out var versions))
            throw new InvalidOperationException("Could not fetch NeoForge versions.");

        var parts = minecraftVersion.Split('.');
        // MC 1.X.Y → NeoForge prefix is "X.Y." (or "X.0." if no Y)
        var prefix = parts.Length >= 3 ? $"{parts[1]}.{parts[2]}." : $"{parts[1]}.0.";

        string? best = null;
        foreach (var v in versions.EnumerateArray())
        {
            var ver = v.GetString() ?? "";
            if (ver.StartsWith(prefix))
                best = ver;
        }

        if (best == null)
            throw new InvalidOperationException($"No NeoForge version found for Minecraft {minecraftVersion}. NeoForge supports 1.20.2+.");
        return best;
    }
}
