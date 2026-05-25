using System;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public sealed class JavaModLoaderService
{
    private readonly JavaVersionService _versions;
    private readonly HttpClient _http;

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
        return await InstallProfileAsync(id, url);
    }

    public async Task<JavaVersion> InstallQuiltAsync(string minecraftVersion, string loaderVersion = "")
    {
        var loader = string.IsNullOrWhiteSpace(loaderVersion) ? await LatestQuiltLoaderAsync() : loaderVersion;
        var url = $"https://meta.quiltmc.org/v3/versions/loader/{Uri.EscapeDataString(minecraftVersion)}/{Uri.EscapeDataString(loader)}/profile/json";
        var id = $"quilt-loader-{loader}-{minecraftVersion}";
        return await InstallProfileAsync(id, url);
    }

    public async Task<string> DownloadForgeInstallerAsync(string minecraftVersion, string forgeVersion = "")
    {
        var ver = string.IsNullOrWhiteSpace(forgeVersion) ? await LatestForgeVersionAsync(minecraftVersion) : forgeVersion;
        var id = $"{minecraftVersion}-{ver}";
        var file = $"forge-{id}-installer.jar";
        var url = $"https://maven.minecraftforge.net/net/minecraftforge/forge/{id}/{file}";
        var destDir = Path.Combine(JavaInstanceService.RootDir, "loaders", "forge");
        Directory.CreateDirectory(destDir);
        var dest = Path.Combine(destDir, file);
        using var resp = await _http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
        resp.EnsureSuccessStatusCode();
        await using var stream = await resp.Content.ReadAsStreamAsync();
        await using var output = File.Create(dest);
        await stream.CopyToAsync(output);
        return dest;
    }

    public async Task<string> DownloadNeoForgeInstallerAsync(string minecraftVersion)
    {
        var ver = await LatestNeoForgeVersionAsync(minecraftVersion);
        var file = $"neoforge-{ver}-installer.jar";
        var url = $"https://maven.neoforged.net/releases/net/neoforged/neoforge/{ver}/{file}";
        var destDir = Path.Combine(JavaInstanceService.RootDir, "loaders", "neoforge");
        Directory.CreateDirectory(destDir);
        var dest = Path.Combine(destDir, file);
        using var resp = await _http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
        resp.EnsureSuccessStatusCode();
        await using var stream = await resp.Content.ReadAsStreamAsync();
        await using var output = File.Create(dest);
        await stream.CopyToAsync(output);
        return dest;
    }

    private async Task<JavaVersion> InstallProfileAsync(string id, string url)
    {
        using var resp = await _http.GetAsync(url);
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
        foreach (var item in doc.RootElement.EnumerateArray())
            if (item.TryGetProperty("separator", out _) == false)
                return item.GetProperty("version").GetString() ?? "";
        return "";
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
