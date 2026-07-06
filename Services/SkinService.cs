using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;

namespace GlacierLauncher.Services;

/// <summary>One cape the player owns. <see cref="Active"/> marks the one currently worn.</summary>
public sealed record CapeInfo(string Id, string Alias, string Url, bool Active);

/// <summary>
/// Changes the signed-in player's Minecraft skin through the official Minecraft
/// Services API using the stored bearer token. Every method returns null on
/// success or a human-readable error string — never throws.
/// </summary>
public static class SkinService
{
    private const string SkinsEndpoint = "https://api.minecraftservices.com/minecraft/profile/skins";
    private const string ActiveSkin    = "https://api.minecraftservices.com/minecraft/profile/skins/active";

    public static async Task<string?> UploadSkinAsync(string token, string pngPath, bool slim)
    {
        if (string.IsNullOrWhiteSpace(token)) return "Not signed in — sign in with Microsoft first.";
        if (!File.Exists(pngPath))            return "Skin file not found.";

        try
        {
            var bytes = await File.ReadAllBytesAsync(pngPath).ConfigureAwait(false);

            using var form = new MultipartFormDataContent();
            form.Add(new StringContent(slim ? "slim" : "classic"), "variant");
            var file = new ByteArrayContent(bytes);
            file.Headers.ContentType = new MediaTypeHeaderValue("image/png");
            form.Add(file, "file", Path.GetFileName(pngPath));

            using var req = new HttpRequestMessage(HttpMethod.Post, SkinsEndpoint) { Content = form };
            req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token);

            using var resp = await HttpFactory.Shared.SendAsync(req).ConfigureAwait(false);
            if (resp.IsSuccessStatusCode) return null;
            if ((int)resp.StatusCode == 401)
                return "Your session expired — re-sign in with Microsoft.";
            var body = await resp.Content.ReadAsStringAsync().ConfigureAwait(false);
            var detail = ExtractApiMessage(body);
            return string.IsNullOrEmpty(detail)
                ? $"Minecraft rejected the skin (HTTP {(int)resp.StatusCode})."
                : $"Minecraft rejected the skin: {detail}";
        }
        catch (Exception ex) { return ex.Message; }
    }

    /// <summary>Pulls the human-readable "errorMessage" out of a Minecraft API error body.</summary>
    private static string ExtractApiMessage(string body)
    {
        if (string.IsNullOrWhiteSpace(body)) return "";
        try
        {
            using var doc = JsonDocument.Parse(body);
            if (doc.RootElement.TryGetProperty("errorMessage", out var m)) return m.GetString() ?? "";
        }
        catch { /* not JSON */ }
        return "";
    }

    public static async Task<string?> ResetSkinAsync(string token)
    {
        if (string.IsNullOrWhiteSpace(token)) return "Not signed in.";
        try
        {
            using var req = new HttpRequestMessage(HttpMethod.Delete, ActiveSkin);
            req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token);
            using var resp = await HttpFactory.Shared.SendAsync(req).ConfigureAwait(false);
            return resp.IsSuccessStatusCode ? null : $"Minecraft API returned HTTP {(int)resp.StatusCode}.";
        }
        catch (Exception ex) { return ex.Message; }
    }

    // ── Capes ────────────────────────────────────────────────────────────────
    private const string ProfileEndpoint = "https://api.minecraftservices.com/minecraft/profile";
    private const string ActiveCape      = "https://api.minecraftservices.com/minecraft/profile/capes/active";

    /// <summary>Lists every cape the signed-in player owns (empty on failure).</summary>
    public static async Task<List<CapeInfo>> GetCapesAsync(string token)
    {
        var list = new List<CapeInfo>();
        if (string.IsNullOrWhiteSpace(token)) return list;
        try
        {
            using var req = new HttpRequestMessage(HttpMethod.Get, ProfileEndpoint);
            req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token);
            using var resp = await HttpFactory.Shared.SendAsync(req).ConfigureAwait(false);
            if (!resp.IsSuccessStatusCode) return list;

            var json = await resp.Content.ReadAsStringAsync().ConfigureAwait(false);
            using var doc = JsonDocument.Parse(json);
            if (!doc.RootElement.TryGetProperty("capes", out var capes) || capes.ValueKind != JsonValueKind.Array)
                return list;

            foreach (var c in capes.EnumerateArray())
            {
                var id    = c.TryGetProperty("id", out var i)    ? i.GetString() ?? "" : "";
                var alias = c.TryGetProperty("alias", out var a)  ? a.GetString() ?? "" : "";
                var url   = c.TryGetProperty("url", out var u)    ? u.GetString() ?? "" : "";
                var active = c.TryGetProperty("state", out var s)
                          && string.Equals(s.GetString(), "ACTIVE", StringComparison.OrdinalIgnoreCase);
                if (string.IsNullOrEmpty(id)) continue;
                // textures.minecraft.net serves https; the API hands back http, which a
                // secure WebView would block as mixed content.
                url = url.Replace("http://", "https://");
                list.Add(new CapeInfo(id, string.IsNullOrEmpty(alias) ? "Cape" : alias, url, active));
            }
        }
        catch { /* offline / unauthorized — return what we have */ }
        return list;
    }

    /// <summary>Sets the active cape. Returns null on success or an error string.</summary>
    public static async Task<string?> SetCapeAsync(string token, string capeId)
    {
        if (string.IsNullOrWhiteSpace(token)) return "Not signed in.";
        try
        {
            using var content = new StringContent(
                JsonSerializer.Serialize(new { capeId }), Encoding.UTF8, "application/json");
            using var req = new HttpRequestMessage(HttpMethod.Put, ActiveCape) { Content = content };
            req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token);
            using var resp = await HttpFactory.Shared.SendAsync(req).ConfigureAwait(false);
            return resp.IsSuccessStatusCode ? null : $"Minecraft API returned HTTP {(int)resp.StatusCode}.";
        }
        catch (Exception ex) { return ex.Message; }
    }

    /// <summary>Hides the cape (no active cape).</summary>
    public static async Task<string?> HideCapeAsync(string token)
    {
        if (string.IsNullOrWhiteSpace(token)) return "Not signed in.";
        try
        {
            using var req = new HttpRequestMessage(HttpMethod.Delete, ActiveCape);
            req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token);
            using var resp = await HttpFactory.Shared.SendAsync(req).ConfigureAwait(false);
            return resp.IsSuccessStatusCode ? null : $"Minecraft API returned HTTP {(int)resp.StatusCode}.";
        }
        catch (Exception ex) { return ex.Message; }
    }
}
