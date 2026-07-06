using System.Collections.Generic;
using System.IO;
using System.Net.Http.Json;
using System.Text.Json;

namespace GlacierLauncher.Services;

public sealed record NewsPost(string Icon, string Title, string Subtitle, string Url, string Date);

/// <summary>
/// Fetches launcher news from the Glacier site. Successful responses are cached
/// to disk so a later offline start still shows the last-known news instead of
/// an empty list. News is purely additive and never blocks anything.
/// </summary>
public static class NewsService
{
    private const string NewsUrl = "https://glacierclient.xyz/news.json";

    private static string CachePath =>
        Path.Combine(LauncherUtilityService.CacheDir, "news.json");

    public static async Task<List<NewsPost>> GetNewsAsync()
    {
        try
        {
            var posts = await HttpFactory.Shared.GetFromJsonAsync<List<NewsPost>>(NewsUrl).ConfigureAwait(false);
            if (posts != null && posts.Count > 0)
            {
                posts = Decode(posts);
                try { JsonStore.WriteAtomic(CachePath, JsonSerializer.Serialize(posts)); } catch { }
                return posts;
            }
            return posts ?? ReadCache();
        }
        catch
        {
            // Offline / server down — serve the last cached copy if we have one.
            return ReadCache();
        }
    }

    private static List<NewsPost> ReadCache()
    {
        try
        {
            if (!File.Exists(CachePath)) return new List<NewsPost>();
            var json = File.ReadAllText(CachePath);
            return Decode(JsonSerializer.Deserialize<List<NewsPost>>(json) ?? new List<NewsPost>());
        }
        catch { return new List<NewsPost>(); }
    }

    // news.json is hand-authored; titles/subtitles sometimes arrive HTML-escaped
    // ("Update &amp; fixes"). Decode once so the plain-@ bindings don't show entities.
    private static List<NewsPost> Decode(List<NewsPost> posts)
    {
        for (int i = 0; i < posts.Count; i++)
            posts[i] = posts[i] with
            {
                Title    = System.Net.WebUtility.HtmlDecode(posts[i].Title ?? ""),
                Subtitle = System.Net.WebUtility.HtmlDecode(posts[i].Subtitle ?? ""),
            };
        return posts;
    }
}
