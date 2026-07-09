using System.IO;
using System.Net.Http.Json;
using System.Text.Json;

namespace GlacierLauncher.Services;

public sealed record Announcement(string Id, string Title, string Message, string Kind, string Url);

/// <summary>
/// Fetches an optional remote announcement/maintenance banner from the same
/// site NewsService already reads from. Absent or unreachable is the normal
/// case — this only shows something when Glacier's team publishes a file, and
/// fails silently (no banner) otherwise so it can never block startup.
/// </summary>
public static class AnnouncementService
{
    private const string AnnouncementUrl = "https://glacierclient.xyz/announcement.json";

    private static string CachePath =>
        Path.Combine(LauncherUtilityService.CacheDir, "announcement.json");

    public static async Task<Announcement?> GetAnnouncementAsync()
    {
        try
        {
            var a = await HttpFactory.Shared.GetFromJsonAsync<Announcement>(AnnouncementUrl).ConfigureAwait(false);
            if (a != null && !string.IsNullOrWhiteSpace(a.Id))
            {
                try { JsonStore.WriteAtomic(CachePath, JsonSerializer.Serialize(a)); } catch { }
                return a;
            }
            return null;
        }
        catch
        {
            // Offline / 404 / server down — no cached fallback here, unlike news:
            // a stale "maintenance in progress" banner surviving after the outage
            // ended would be actively misleading rather than just less fresh.
            return null;
        }
    }
}
