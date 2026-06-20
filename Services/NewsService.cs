using System.Collections.Generic;
using System.Net.Http.Json;

namespace GlacierLauncher.Services;

public sealed record NewsPost(string Icon, string Title, string Subtitle, string Url, string Date);

/// <summary>
/// Fetches launcher news from the Glacier site. Returns an empty list on any
/// failure so the UI can fall back to its built-in highlights — news is purely
/// additive and never blocks anything.
/// </summary>
public static class NewsService
{
    private const string NewsUrl = "https://glacierclient.xyz/news.json";

    public static async Task<List<NewsPost>> GetNewsAsync()
    {
        try
        {
            var posts = await HttpFactory.Shared.GetFromJsonAsync<List<NewsPost>>(NewsUrl).ConfigureAwait(false);
            return posts ?? new List<NewsPost>();
        }
        catch
        {
            return new List<NewsPost>();
        }
    }
}
