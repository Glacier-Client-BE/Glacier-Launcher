using System;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

/// <summary>
/// Caches GitHub REST API responses to disk to dodge the unauthenticated 60 req/hr limit.
/// Uses If-None-Match conditional requests (304s don't count against the rate limit) and
/// falls back to the last good cached body on 403/network failure.
/// </summary>
public static class GitHubApiCache
{
    private const int FreshTtlSeconds = 600; // 10 min — within this window we don't even hit the network

    private static string CacheDir =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "cache", "github");

    public sealed record Result(string Body, bool FromCache, string? Error);

    public static async Task<Result> GetJsonAsync(HttpClient http, string url)
    {
        Directory.CreateDirectory(CacheDir);
        var key      = Hash(url);
        var bodyPath = Path.Combine(CacheDir, key + ".body");
        var metaPath = Path.Combine(CacheDir, key + ".meta");

        var cached = TryReadCache(bodyPath, metaPath);

        // Fast path: serve from cache without touching the network if still fresh.
        if (cached != null && (DateTime.UtcNow - cached.Value.fetchedAt).TotalSeconds < FreshTtlSeconds)
            return new Result(cached.Value.body, FromCache: true, Error: null);

        try
        {
            using var req = new HttpRequestMessage(HttpMethod.Get, url);
            req.Headers.Accept.ParseAdd("application/vnd.github+json");
            req.Headers.TryAddWithoutValidation("X-GitHub-Api-Version", "2022-11-28");
            if (cached != null && !string.IsNullOrEmpty(cached.Value.etag))
                req.Headers.TryAddWithoutValidation("If-None-Match", cached.Value.etag);

            using var resp = await http.SendAsync(req);

            if (resp.StatusCode == HttpStatusCode.NotModified && cached != null)
            {
                // Refresh the timestamp so we serve from cache for the full TTL again.
                WriteMeta(metaPath, cached.Value.etag, DateTime.UtcNow);
                return new Result(cached.Value.body, FromCache: true, Error: null);
            }

            if (resp.StatusCode == HttpStatusCode.Forbidden && cached != null)
            {
                var rateMsg = ExtractRateLimitMessage(resp);
                return new Result(cached.Value.body, FromCache: true, Error: rateMsg);
            }

            resp.EnsureSuccessStatusCode();
            var body = await resp.Content.ReadAsStringAsync();
            var etag = resp.Headers.ETag?.Tag ?? "";
            File.WriteAllText(bodyPath, body);
            WriteMeta(metaPath, etag, DateTime.UtcNow);
            return new Result(body, FromCache: false, Error: null);
        }
        catch (HttpRequestException ex) when (cached != null)
        {
            var msg = ex.StatusCode == HttpStatusCode.Forbidden
                ? "GitHub rate limit reached. Showing cached results."
                : $"Network error — showing cached results. ({ex.Message})";
            return new Result(cached.Value.body, FromCache: true, Error: msg);
        }
        catch (Exception ex) when (cached != null)
        {
            return new Result(cached.Value.body, FromCache: true, Error: ex.Message);
        }
    }

    private static (string body, string etag, DateTime fetchedAt)? TryReadCache(string bodyPath, string metaPath)
    {
        try
        {
            if (!File.Exists(bodyPath) || !File.Exists(metaPath)) return null;
            var body = File.ReadAllText(bodyPath);
            var meta = File.ReadAllText(metaPath).Split('\n', 2);
            if (meta.Length < 2) return null;
            if (!DateTime.TryParse(meta[1], null, System.Globalization.DateTimeStyles.RoundtripKind, out var ts))
                return null;
            return (body, meta[0], ts);
        }
        catch { return null; }
    }

    private static void WriteMeta(string metaPath, string etag, DateTime fetchedAt)
    {
        try { File.WriteAllText(metaPath, $"{etag}\n{fetchedAt:o}"); } catch { }
    }

    private static string ExtractRateLimitMessage(HttpResponseMessage resp)
    {
        if (resp.Headers.TryGetValues("X-RateLimit-Reset", out var vals))
        {
            foreach (var v in vals)
            {
                if (long.TryParse(v, out var unix))
                {
                    var resetIn = DateTimeOffset.FromUnixTimeSeconds(unix) - DateTimeOffset.UtcNow;
                    if (resetIn.TotalMinutes > 0)
                        return $"GitHub rate limit reached. Showing cached results (resets in {(int)resetIn.TotalMinutes + 1} min).";
                }
            }
        }
        return "GitHub rate limit reached. Showing cached results.";
    }

    private static string Hash(string s)
    {
        var bytes = SHA256.HashData(Encoding.UTF8.GetBytes(s));
        var sb = new StringBuilder(32);
        for (int i = 0; i < 16; i++) sb.Append(bytes[i].ToString("x2"));
        return sb.ToString();
    }
}
