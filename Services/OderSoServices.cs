using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;

namespace MinecraftLauncher.Services;

public class OderSoService
{
    // ── GitHub raw content endpoints ─────────────────────────────
    private const string Org    = "MasonOderSo";
    private const string Repo   = "oderso-data";
    private const string Branch = "main";

    private static string ContentsApiUrl =>
        $"https://api.github.com/repos/{Org}/{Repo}/contents/";

    private static string RawBaseUrl =>
        $"https://raw.githubusercontent.com/{Org}/{Repo}/{Branch}/";

    // ── Local paths ───────────────────────────────────────────────
    public static string ClientsDirectory => GameLauncher.ClientsDirectory;

    private static string VerPath =>
        Path.Combine(ClientsDirectory, "OderSo.ver");

    // ── Active-entry facade (used by GameLauncher) ────────────────
    public bool IsDownloaded
    {
        get
        {
            if (!File.Exists(VerPath)) return false;
            var stored = GetStoredEntry();
            return stored != null && File.Exists(GetDllPath(stored.Name));
        }
    }

    public string FilePath
    {
        get
        {
            var entry = GetStoredEntry();
            if (entry != null) return GetDllPath(entry.Name);
            return Path.Combine(ClientsDirectory, "OderSo.dll");
        }
    }

    private readonly HttpClient _http;

    public OderSoService()
    {
        _http = new HttpClient();
        _http.DefaultRequestHeaders.Add("User-Agent", "GlacierLauncher/1.0");
    }

    // ── GitHub Contents API ───────────────────────────────────────

    public record DllEntry(string Name, string Sha, long Size);

    /// <summary>Lists all .dll files in the repo root via Contents API.</summary>
    public async Task<List<DllEntry>> ListDllsAsync()
    {
        var result = new List<DllEntry>();
        try
        {
            var json = await _http.GetStringAsync(ContentsApiUrl);
            using var doc = JsonDocument.Parse(json);
            foreach (var item in doc.RootElement.EnumerateArray())
            {
                var type = item.TryGetProperty("type", out var t) ? t.GetString() : null;
                var name = item.TryGetProperty("name", out var n) ? n.GetString() ?? "" : "";
                var sha  = item.TryGetProperty("sha",  out var s) ? s.GetString() ?? "" : "";
                var size = item.TryGetProperty("size", out var z) ? z.GetInt64() : 0L;

                if (type == "file" && name.EndsWith(".dll", StringComparison.OrdinalIgnoreCase))
                    result.Add(new DllEntry(name, sha, size));
            }
        }
        catch { /* return empty — caller handles */ }
        return result;
    }

    public async Task<DllEntry?> GetLatestDllAsync()
    {
        var list = await ListDllsAsync();
        return list.Count > 0 ? list[0] : null;
    }

    // ── Per-entry helpers (multi-version) ─────────────────────────

    public bool IsEntryDownloaded(DllEntry entry) =>
        File.Exists(GetDllPath(entry.Name));

    public string GetEntryFilePath(DllEntry entry) =>
        GetDllPath(entry.Name);

    public DllEntry? GetActiveEntry() => GetStoredEntry();

    public void SetActiveEntry(DllEntry entry) => SaveStoredEntry(entry);

    public async Task DownloadEntryAsync(DllEntry entry, IProgress<double>? progress = null)
    {
        Directory.CreateDirectory(ClientsDirectory);

        var rawUrl  = RawBaseUrl + Uri.EscapeDataString(entry.Name);
        var dllPath = GetDllPath(entry.Name);
        var tmpPath = dllPath + ".tmp";

        using var resp = await _http.GetAsync(rawUrl, HttpCompletionOption.ResponseHeadersRead);
        resp.EnsureSuccessStatusCode();

        var total      = resp.Content.Headers.ContentLength ?? entry.Size;
        using var src  = await resp.Content.ReadAsStreamAsync();
        using var dest = File.Create(tmpPath);

        var  buf        = new byte[81920];
        long downloaded = 0;
        int  read;
        while ((read = await src.ReadAsync(buf)) > 0)
        {
            await dest.WriteAsync(buf.AsMemory(0, read));
            downloaded += read;
            if (total > 0) progress?.Report(downloaded * 100.0 / total);
        }

        dest.Close();
        if (File.Exists(dllPath)) File.Delete(dllPath);
        File.Move(tmpPath, dllPath);
    }

    public void DeleteEntry(DllEntry entry)
    {
        var path = GetDllPath(entry.Name);
        if (File.Exists(path)) File.Delete(path);

        var stored = GetStoredEntry();
        if (stored?.Name == entry.Name && File.Exists(VerPath))
            File.Delete(VerPath);
    }

    // ── Active-entry convenience wrappers ─────────────────────────

    public async Task<bool> IsUpToDateAsync()
    {
        if (!IsDownloaded) return false;
        try
        {
            var stored = GetStoredEntry();
            if (stored == null) return false;
            var remote = await GetLatestDllAsync();
            return remote != null && remote.Sha == stored.Sha;
        }
        catch { return true; }
    }

    public async Task DownloadAsync(IProgress<double>? progress = null)
    {
        var entry = await GetLatestDllAsync()
            ?? throw new Exception("No .dll files found in MasonOderSo/oderso-data.");

        await DownloadEntryAsync(entry, progress);
        SaveStoredEntry(entry);
    }

    public void Delete()
    {
        var stored = GetStoredEntry();
        if (stored != null)
        {
            var path = GetDllPath(stored.Name);
            if (File.Exists(path)) File.Delete(path);
        }
        if (File.Exists(VerPath)) File.Delete(VerPath);
    }

    // ── Helpers ───────────────────────────────────────────────────

    private static string GetDllPath(string fileName) =>
        Path.Combine(ClientsDirectory, "OderSo_" + fileName);

    private DllEntry? GetStoredEntry()
    {
        try
        {
            if (!File.Exists(VerPath)) return null;
            var parts = File.ReadAllText(VerPath).Split('|');
            if (parts.Length < 2) return null;
            return new DllEntry(parts[0], parts[1], 0);
        }
        catch { return null; }
    }

    private static void SaveStoredEntry(DllEntry entry) =>
        File.WriteAllText(VerPath, $"{entry.Name}|{entry.Sha}");
}
