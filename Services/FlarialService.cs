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

    public static string FlarialDirectory =>
        Path.Combine(GameLauncher.DownloadsDirectory, "Flarial");

    public static string FilePath =>
        Path.Combine(FlarialDirectory, FileName);

    private readonly HttpClient      _http;
    private readonly DownloadService _download = new();

    public FlarialService()
    {
        _http = HttpFactory.Shared;
        Directory.CreateDirectory(FlarialDirectory);
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

    public async Task DownloadAsync(IProgress<double>? progress = null, CancellationToken cancel = default)
    {
        if (await IsUpToDateAsync()) return;
        await _download.DownloadAsync(DownloadUri, FilePath, progress: progress, cancel: cancel);
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
