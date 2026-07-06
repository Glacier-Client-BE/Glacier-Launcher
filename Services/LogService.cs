using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

/// <summary>
/// Reads Java logs and crash reports from the active instance, detects crashes,
/// and shares logs to mclo.gs. Access tokens are redacted before any upload.
/// </summary>
public sealed class LogService
{
    private readonly JavaInstanceService _instances;
    private readonly HttpClient _http;

    public LogService(JavaInstanceService instances)
    {
        _instances = instances;
        _http = HttpFactory.Shared;
    }

    public sealed record LogFile(string Name, string Path, long Size, string ModifiedAt, bool IsCrash);

    /// <summary>Lists log + crash-report files for the active instance, newest first.</summary>
    public IReadOnlyList<LogFile> ListLogs()
    {
        var result = new List<LogFile>();
        void Scan(string dir, bool crash)
        {
            if (!Directory.Exists(dir)) return;
            foreach (var f in Directory.EnumerateFiles(dir))
            {
                var ext = Path.GetExtension(f).ToLowerInvariant();
                if (ext is ".log" or ".txt" or ".gz")
                {
                    var fi = new FileInfo(f);
                    result.Add(new LogFile(fi.Name, fi.FullName, fi.Length, fi.LastWriteTimeUtc.ToString("o"), crash));
                }
            }
        }
        Scan(_instances.PathFor("logs"), crash: false);
        Scan(_instances.PathForInstance(_instances.ActiveInstance.Id, "crash-reports"), crash: true);
        return result.OrderByDescending(l => l.ModifiedAt).ToList();
    }

    /// <summary>Reads a log file, decompressing .gz, returning the last <paramref name="maxLines"/> lines.</summary>
    public async Task<string> ReadLogAsync(string path, int maxLines = 2000)
    {
        try
        {
            string text;
            if (path.EndsWith(".gz", StringComparison.OrdinalIgnoreCase))
            {
                await using var fs = File.OpenRead(path);
                await using var gz = new System.IO.Compression.GZipStream(fs, System.IO.Compression.CompressionMode.Decompress);
                using var reader = new StreamReader(gz);
                text = await reader.ReadToEndAsync();
            }
            else
            {
                text = await File.ReadAllTextAsync(path);
            }
            var lines = text.Split('\n');
            if (lines.Length > maxLines)
                text = string.Join('\n', lines.Skip(lines.Length - maxLines));
            return text;
        }
        catch (Exception ex) { return "Could not read log: " + ex.Message; }
    }

    /// <summary>Heuristic crash check for a finished log/session text.</summary>
    public static bool LooksLikeCrash(string text) =>
        text.Contains("---- Minecraft Crash Report ----", StringComparison.Ordinal)
        || text.Contains("Exception in thread \"main\"", StringComparison.Ordinal)
        || Regex.IsMatch(text, @"\bA fatal error has been detected\b", RegexOptions.IgnoreCase);

    private static readonly Regex[] SecretPatterns =
    {
        // Launch args and log lines can leak the session access token.
        new(@"(--accessToken\s+)\S+", RegexOptions.Compiled),
        new(@"(accessToken[""']?\s*[:=]\s*[""']?)[A-Za-z0-9\.\-_]+", RegexOptions.Compiled | RegexOptions.IgnoreCase),
        new(@"(--session\s+)\S+", RegexOptions.Compiled),
        new(@"(eyJ[A-Za-z0-9_\-]{6,})\.[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+", RegexOptions.Compiled), // JWTs
    };

    public static string Redact(string text)
    {
        text = SecretPatterns[0].Replace(text, "$1<redacted>");
        text = SecretPatterns[1].Replace(text, "$1<redacted>");
        text = SecretPatterns[2].Replace(text, "$1<redacted>");
        text = SecretPatterns[3].Replace(text, "<redacted-token>");
        return text;
    }

    /// <summary>Uploads a log to mclo.gs (redacted) and returns the shareable URL.</summary>
    public async Task<string> ShareAsync(string content)
    {
        var redacted = Redact(content);
        using var form = new FormUrlEncodedContent(new[]
        {
            new KeyValuePair<string, string>("content", redacted),
        });
        using var resp = await _http.PostAsync("https://api.mclo.gs/1/log", form);
        resp.EnsureSuccessStatusCode();
        using var doc = System.Text.Json.JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        var root = doc.RootElement;
        if (root.TryGetProperty("success", out var ok) && ok.GetBoolean()
            && root.TryGetProperty("url", out var url))
            return url.GetString() ?? "";
        var err = root.TryGetProperty("error", out var e) ? e.GetString() : "unknown error";
        throw new InvalidOperationException("mclo.gs rejected the log: " + err);
    }
}
