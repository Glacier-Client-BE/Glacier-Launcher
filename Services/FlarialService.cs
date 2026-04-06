using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

public class FlarialService
{
    private const string DownloadUri = "https://cdn.flarial.xyz/dll/latest.dll";
    private const string HashesUri   = "https://cdn.flarial.xyz/dll_hashes.json";
    private const string HashKey     = "Release";

    public static string FileName => "Flarial.Client.Release.dll";

    public static string FilePath =>
        Path.Combine(GameLauncher.ClientsDirectory, FileName);

    private readonly HttpClient _http;

    public FlarialService()
    {
        _http = new HttpClient();
        _http.DefaultRequestHeaders.Add("User-Agent", "GlacierLauncher/1.0");
    }

    public bool IsDownloaded => File.Exists(FilePath);

    public async Task<bool> IsUpToDateAsync()
    {
        if (!IsDownloaded) return false;
        try
        {
            var localHash  = await GetLocalHashAsync();
            var remoteHash = await GetRemoteHashAsync();
            return string.Equals(localHash, remoteHash, StringComparison.OrdinalIgnoreCase);
        }
        catch { return false; }
    }

    public async Task DownloadAsync(IProgress<double>? progress = null)
    {
        // Skip if already up-to-date
        if (await IsUpToDateAsync()) return;

        // Remove stale copy
        if (File.Exists(FilePath))
        {
            try { File.Delete(FilePath); } catch { }
        }

        using var response = await _http.GetAsync(DownloadUri, HttpCompletionOption.ResponseHeadersRead);
        response.EnsureSuccessStatusCode();

        var  total      = response.Content.Headers.ContentLength ?? -1;
        var  buffer     = new byte[8192];
        long downloaded = 0;

        await using var stream     = await response.Content.ReadAsStreamAsync();
        await using var fileStream = File.Create(FilePath);

        int read;
        while ((read = await stream.ReadAsync(buffer)) > 0)
        {
            await fileStream.WriteAsync(buffer.AsMemory(0, read));
            downloaded += read;
            if (total > 0) progress?.Report((double)downloaded / total * 100);
        }
    }

    public void Delete()
    {
        if (File.Exists(FilePath))
            File.Delete(FilePath);
    }

    private async Task<string> GetRemoteHashAsync()
    {
        var json = await _http.GetStringAsync(HashesUri);
        using var doc = JsonDocument.Parse(json);
        return doc.RootElement.GetProperty(HashKey).GetString() ?? "";
    }

    private static Task<string> GetLocalHashAsync() => Task.Run(() =>
    {
        try
        {
            using var sha    = SHA256.Create();
            using var stream = File.OpenRead(FilePath);
            var bytes  = sha.ComputeHash(stream);
            return BitConverter.ToString(bytes).Replace("-", "");
        }
        catch { return ""; }
    });
}
