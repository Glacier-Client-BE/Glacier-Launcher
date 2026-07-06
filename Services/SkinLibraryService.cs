using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

/// <summary>
/// Manages the local saved-skins collection under ~/Glacier Launcher/skins.
/// Skins are plain PNGs; the folder is served to the WebView through the
/// existing glacier-files.local virtual host, so the UI can preview them.
/// </summary>
public sealed class SkinLibraryService
{
    private readonly HttpClient _http = HttpFactory.Shared;

    public static string SkinsDir => Path.Combine(LauncherUtilityService.LauncherRoot, "skins");

    public SkinLibraryService() => Directory.CreateDirectory(SkinsDir);

    public sealed record SavedSkin(string Name, string Path, string Url, bool Slim);

    /// <summary>Lists saved skins, newest first. Slim variants are flagged by a ".slim" name marker.</summary>
    public IReadOnlyList<SavedSkin> List()
    {
        Directory.CreateDirectory(SkinsDir);
        return Directory.EnumerateFiles(SkinsDir, "*.png")
            .Select(p => new FileInfo(p))
            .OrderByDescending(fi => fi.LastWriteTimeUtc)
            .Select(fi => new SavedSkin(
                Path.GetFileNameWithoutExtension(fi.Name),
                fi.FullName,
                "https://glacier-files.local/skins/" + Uri.EscapeDataString(fi.Name) + "?t=" + fi.LastWriteTimeUtc.Ticks,
                fi.Name.Contains(".slim.", StringComparison.OrdinalIgnoreCase)))
            .ToList();
    }

    /// <summary>Copies a PNG into the library. Returns the stored path.</summary>
    public async Task<string> AddFromFileAsync(string sourcePath, bool slim)
    {
        var fi = new FileInfo(sourcePath);
        if (!fi.Exists) throw new FileNotFoundException("Skin file not found.", sourcePath);
        if (fi.Length > 5 * 1024 * 1024) throw new InvalidOperationException("That PNG is too large to be a skin.");
        var baseName = Path.GetFileNameWithoutExtension(sourcePath);
        var dest = UniquePath(baseName, slim);
        await Task.Run(() => File.Copy(sourcePath, dest, false));
        return dest;
    }

    /// <summary>
    /// Fetches a player's current skin by username into the library using Mojang's
    /// official endpoints only: username -> UUID -> signed texture URL on
    /// textures.minecraft.net. Third-party proxies like crafatar are avoided
    /// deliberately - they sit behind Cloudflare and return 521 ("origin down")
    /// whenever their backend is offline, which is the failure this replaces.
    /// The slim (Alex) model is detected from the texture metadata.
    /// Throws <see cref="InvalidOperationException"/> with a user-friendly message
    /// on any failure; never surfaces a raw HTTP/JSON exception.
    /// </summary>
    public async Task<string> AddFromUsernameAsync(string username)
    {
        if (string.IsNullOrWhiteSpace(username)) throw new ArgumentException("Enter a username.");
        var name = username.Trim();

        // 1) Username -> UUID.
        string uuid = await ResolveUuidAsync(name);

        // 2) UUID -> signed texture URL (+ model) from the session server.
        var (skinUrl, slim) = await ResolveSkinTextureAsync(uuid, name);

        // 3) Download the PNG from Mojang's own CDN (force https for the WebView).
        skinUrl = skinUrl.Replace("http://", "https://");
        byte[] bytes;
        try { bytes = await _http.GetByteArrayAsync(skinUrl); }
        catch { throw new InvalidOperationException("Couldn't download the skin texture - try again shortly."); }
        if (bytes.Length < 100) throw new InvalidOperationException("The downloaded skin looked empty.");

        var dest = UniquePath(name, slim);
        await File.WriteAllBytesAsync(dest, bytes);
        return dest;
    }

    private async Task<string> ResolveUuidAsync(string name)
    {
        try
        {
            using var resp = await _http.GetAsync($"https://api.mojang.com/users/profiles/minecraft/{Uri.EscapeDataString(name)}");
            if (resp.StatusCode == HttpStatusCode.NoContent || resp.StatusCode == HttpStatusCode.NotFound)
                throw new InvalidOperationException($"No Minecraft player named '{name}'.");
            if (!resp.IsSuccessStatusCode)
                throw new InvalidOperationException($"Mojang is unavailable (HTTP {(int)resp.StatusCode}) - try again shortly.");
            var json = await resp.Content.ReadAsStringAsync();
            using var doc = JsonDocument.Parse(json);
            var id = doc.RootElement.TryGetProperty("id", out var el) ? el.GetString() : null;
            if (string.IsNullOrEmpty(id))
                throw new InvalidOperationException($"No Minecraft player named '{name}'.");
            return id;
        }
        catch (HttpRequestException) { throw new InvalidOperationException("No internet connection, or Mojang is unreachable."); }
        catch (TaskCanceledException) { throw new InvalidOperationException("Mojang timed out - try again shortly."); }
    }

    private async Task<(string url, bool slim)> ResolveSkinTextureAsync(string uuid, string name)
    {
        try
        {
            using var resp = await _http.GetAsync($"https://sessionserver.mojang.com/session/minecraft/profile/{uuid}");
            if (!resp.IsSuccessStatusCode)
                throw new InvalidOperationException($"Mojang profile lookup failed (HTTP {(int)resp.StatusCode}).");
            var json = await resp.Content.ReadAsStringAsync();
            using var doc = JsonDocument.Parse(json);

            string? value = null;
            if (doc.RootElement.TryGetProperty("properties", out var props) && props.ValueKind == JsonValueKind.Array)
                foreach (var p in props.EnumerateArray())
                    if (p.TryGetProperty("name", out var n) && n.GetString() == "textures" && p.TryGetProperty("value", out var v))
                    { value = v.GetString(); break; }
            if (string.IsNullOrEmpty(value))
                throw new InvalidOperationException($"Couldn't read the textures for '{name}'.");

            var decoded = Encoding.UTF8.GetString(Convert.FromBase64String(value));
            using var tex = JsonDocument.Parse(decoded);
            if (!tex.RootElement.TryGetProperty("textures", out var textures) ||
                !textures.TryGetProperty("SKIN", out var skin) ||
                !skin.TryGetProperty("url", out var urlEl))
                throw new InvalidOperationException($"'{name}' is using the default skin.");

            var url = urlEl.GetString() ?? "";
            if (string.IsNullOrEmpty(url))
                throw new InvalidOperationException($"'{name}' is using the default skin.");

            bool slim = skin.TryGetProperty("metadata", out var meta) &&
                        meta.TryGetProperty("model", out var model) &&
                        string.Equals(model.GetString(), "slim", StringComparison.OrdinalIgnoreCase);
            return (url, slim);
        }
        catch (InvalidOperationException) { throw; }
        catch (HttpRequestException) { throw new InvalidOperationException("No internet connection, or Mojang is unreachable."); }
        catch (TaskCanceledException) { throw new InvalidOperationException("Mojang timed out - try again shortly."); }
        catch { throw new InvalidOperationException($"Couldn't read the skin for '{name}'."); }
    }

    public void Delete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); } catch { }
    }

    private static string UniquePath(string baseName, bool slim)
    {
        foreach (var c in Path.GetInvalidFileNameChars()) baseName = baseName.Replace(c, '_');
        if (string.IsNullOrWhiteSpace(baseName)) baseName = "skin";
        var suffix = slim ? ".slim" : "";
        var candidate = Path.Combine(SkinsDir, baseName + suffix + ".png");
        var n = 2;
        while (File.Exists(candidate))
            candidate = Path.Combine(SkinsDir, $"{baseName}-{n++}{suffix}.png");
        return candidate;
    }
}
