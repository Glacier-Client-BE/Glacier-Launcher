using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Security.Cryptography;

namespace GlacierLauncher.Services;

public enum DownloadStatus { Downloading, Completed, Failed, Cancelled }

/// <summary>One tracked download, shown on the Download Manager panel. Mutated internally by DownloadService as the transfer progresses.</summary>
public sealed class DownloadEntry
{
    public Guid    Id              { get; } = Guid.NewGuid();
    public string  Label           { get; internal set; } = "";
    public DownloadStatus Status   { get; internal set; } = DownloadStatus.Downloading;
    public double  Progress        { get; internal set; }
    public long    DownloadedBytes { get; internal set; }
    public long    TotalBytes      { get; internal set; }
    public string  StartedAt       { get; } = DateTime.UtcNow.ToString("o");
    public string? FinishedAt      { get; internal set; }
    public string? Error           { get; internal set; }

    internal readonly CancellationTokenSource Cts = new();

    /// <summary>Requests cancellation of this download. Safe to call after it has already finished.</summary>
    public void Cancel() { try { Cts.Cancel(); } catch { } }
}

/// <summary>
/// Streams a remote file to disk with progress reporting, optional SHA-256
/// verification, an atomic temp-file swap and automatic retry with exponential
/// backoff on transient failures. Replaces the hand-rolled download loops that
/// were duplicated across the client, version and runtime services.
///
/// <para>Concurrency: every operation targeting the same destination is
/// serialized by a process-wide per-path lock, so a download can never race
/// another download — or a hash read / delete — of the same file. Network
/// failures retry the download; a destination that's momentarily locked retries
/// only the final swap, and a destination that's genuinely in use (e.g. a client
/// loaded in a running game) surfaces a clear, actionable error.</para>
/// </summary>
public sealed class DownloadService
{
    private const int BufferSize     = 81920;
    private const int DefaultRetries = 3;
    private const int CommitRetries  = 5;

    // One gate per destination path, shared across every DownloadService instance.
    // The launcher only ever touches a bounded handful of paths, so the dictionary
    // stays tiny and the gates intentionally live for the lifetime of the process.
    private static readonly ConcurrentDictionary<string, SemaphoreSlim> _fileLocks =
        new(StringComparer.OrdinalIgnoreCase);

    private readonly HttpClient _http;

    public DownloadService() => _http = HttpFactory.Shared;

    public DownloadService(HttpClient http) =>
        _http = http ?? throw new ArgumentNullException(nameof(http));

    // ── Download tracking (for the Download Manager panel) ───────────────────
    // Static because DownloadService is created ad-hoc in several other services
    // (not always resolved through DI), so a per-instance list would miss most
    // downloads. Capped so a long session doesn't grow this unbounded.
    private const int MaxHistory = 60;
    private static readonly ConcurrentDictionary<Guid, DownloadEntry> _entries = new();

    public static event Action? Changed;

    public static IReadOnlyList<DownloadEntry> ActiveDownloads =>
        _entries.Values.OrderByDescending(e => e.StartedAt).ToList();

    private static void RaiseChanged() => Changed?.Invoke();

    private static DownloadEntry TrackStart(string label)
    {
        var entry = new DownloadEntry { Label = label };
        _entries[entry.Id] = entry;
        TrimHistory();
        RaiseChanged();
        return entry;
    }

    private static void TrimHistory()
    {
        if (_entries.Count <= MaxHistory) return;
        var finished = _entries.Values
            .Where(e => e.Status != DownloadStatus.Downloading)
            .OrderBy(e => e.FinishedAt)
            .Take(_entries.Count - MaxHistory);
        foreach (var e in finished) _entries.TryRemove(e.Id, out _);
    }

    /// <summary>Removes a single finished entry from the tracked list (never removes an in-flight download).</summary>
    public static void Remove(Guid id)
    {
        if (_entries.TryGetValue(id, out var entry) && entry.Status != DownloadStatus.Downloading)
        {
            _entries.TryRemove(id, out _);
            RaiseChanged();
        }
    }

    // ── Download ─────────────────────────────────────────────────────────────

    /// <summary>
    /// Downloads <paramref name="url"/> to <paramref name="destinationPath"/>,
    /// reporting progress as a fraction in the range 0–1. The body is streamed
    /// to a sibling <c>.tmp</c> file and atomically moved into place only after
    /// the (optional) hash check succeeds. When <paramref name="expectedSha256"/>
    /// is supplied and is not a placeholder, the computed hash is verified
    /// before the file is committed. <paramref name="configureRequest"/> lets a
    /// caller attach per-request headers (e.g. an API key) before the request is
    /// sent. Transient network/IO failures are retried up to
    /// <paramref name="maxAttempts"/> times with exponential backoff. Returns the
    /// lowercase-hex SHA-256 of the downloaded bytes.
    /// </summary>
    public async Task<string> DownloadAsync(
        string                      url,
        string                      destinationPath,
        string?                     expectedSha256   = null,
        IProgress<double>?          progress         = null,
        long                        knownTotalBytes  = -1,
        Action<HttpRequestMessage>? configureRequest = null,
        int                         maxAttempts      = DefaultRetries,
        CancellationToken           cancel           = default,
        string?                     label            = null)
    {
        if (string.IsNullOrWhiteSpace(url))
            throw new ArgumentException("Download URL is required.", nameof(url));
        if (string.IsNullOrWhiteSpace(destinationPath))
            throw new ArgumentException("Destination path is required.", nameof(destinationPath));
        if (maxAttempts < 1) maxAttempts = 1;

        // Normalize so "C:\a\b.jar" and "C:\a\.\b.jar" share one gate, and so the
        // tmp sibling lands in a real, existing directory.
        var fullPath = Path.GetFullPath(destinationPath);
        var dir = Path.GetDirectoryName(fullPath)
            ?? throw new ArgumentException(
                $"Destination '{destinationPath}' has no parent directory.", nameof(destinationPath));
        Directory.CreateDirectory(dir);

        var entry = TrackStart(label ?? Path.GetFileName(fullPath));
        entry.TotalBytes = Math.Max(0, knownTotalBytes);

        using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(cancel, entry.Cts.Token);
        var linkedToken = linkedCts.Token;

        var lastRaise = DateTime.MinValue;
        var trackedProgress = new DelegateProgress<double>(fraction =>
        {
            entry.Progress = fraction;
            if (entry.TotalBytes > 0) entry.DownloadedBytes = (long)(entry.TotalBytes * fraction);
            progress?.Report(fraction);

            // Throttle UI notifications — progress fires per network buffer (as
            // often as every 80KB), far more often than a panel needs to repaint.
            var now = DateTime.UtcNow;
            if (fraction >= 1.0 || (now - lastRaise).TotalMilliseconds >= 250)
            {
                lastRaise = now;
                RaiseChanged();
            }
        });

        var gate = _fileLocks.GetOrAdd(fullPath, static _ => new SemaphoreSlim(1, 1));
        try
        {
            await gate.WaitAsync(linkedToken).ConfigureAwait(false);
            try
            {
                var hash = await DownloadLockedAsync(
                        url, fullPath, expectedSha256, trackedProgress,
                        knownTotalBytes, configureRequest, maxAttempts, linkedToken)
                    .ConfigureAwait(false);

                entry.Status = DownloadStatus.Completed;
                entry.Progress = 1.0;
                return hash;
            }
            finally
            {
                gate.Release();
            }
        }
        catch (OperationCanceledException)
        {
            entry.Status = DownloadStatus.Cancelled;
            throw;
        }
        catch (Exception ex)
        {
            entry.Status = DownloadStatus.Failed;
            entry.Error = ex.Message;
            throw;
        }
        finally
        {
            entry.FinishedAt = DateTime.UtcNow.ToString("o");
            RaiseChanged();
        }
    }

    // Runs under the per-destination gate: fetch to a unique temp file (retrying
    // network failures), then atomically swap it into place (retrying lock
    // contention). The temp file is always cleaned up, success or failure.
    private async Task<string> DownloadLockedAsync(
        string url, string destinationPath, string? expectedSha256,
        IProgress<double>? progress, long knownTotalBytes,
        Action<HttpRequestMessage>? configureRequest, int maxAttempts,
        CancellationToken cancel)
    {
        var tmp = destinationPath + "." + Guid.NewGuid().ToString("N") + ".tmp";
        try
        {
            var hash = await DownloadToTempWithRetryAsync(
                url, tmp, expectedSha256, progress, knownTotalBytes,
                configureRequest, maxAttempts, cancel).ConfigureAwait(false);

            await CommitAsync(tmp, destinationPath, cancel).ConfigureAwait(false);
            return hash;
        }
        finally
        {
            TryDelete(tmp);
        }
    }

    // ── Stage 1: download to temp (network-transient retry) ──────────────────

    private async Task<string> DownloadToTempWithRetryAsync(
        string url, string tmp, string? expectedSha256,
        IProgress<double>? progress, long knownTotalBytes,
        Action<HttpRequestMessage>? configureRequest, int maxAttempts,
        CancellationToken cancel)
    {
        for (var attempt = 1; ; attempt++)
        {
            try
            {
                return await DownloadToTempOnceAsync(
                        url, tmp, expectedSha256, progress, knownTotalBytes, configureRequest, cancel)
                    .ConfigureAwait(false);
            }
            catch (Exception ex)
                when (attempt < maxAttempts && !cancel.IsCancellationRequested && IsTransient(ex))
            {
                progress?.Report(0);
                await Task.Delay(BackoffFor(attempt), cancel).ConfigureAwait(false);
            }
        }
    }

    private async Task<string> DownloadToTempOnceAsync(
        string url, string tmp, string? expectedSha256,
        IProgress<double>? progress, long knownTotalBytes,
        Action<HttpRequestMessage>? configureRequest, CancellationToken cancel)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, url);
            configureRequest?.Invoke(request);
            using var response = await _http
                .SendAsync(request, HttpCompletionOption.ResponseHeadersRead, cancel)
                .ConfigureAwait(false);
            response.EnsureSuccessStatusCode();

            var total = response.Content.Headers.ContentLength ?? knownTotalBytes;
            using var sha = SHA256.Create();

            // Stream the body into the temp file inside a nested scope so both
            // handles are disposed *before* the commit. File.Move opens the source
            // with DELETE access, which a still-open FileShare.None write handle
            // would reject with a sharing violation ("the process cannot access
            // the file because it is being used by another process").
            await using (var net  = await response.Content.ReadAsStreamAsync(cancel).ConfigureAwait(false))
            await using (var file = new FileStream(
                tmp, FileMode.Create, FileAccess.Write, FileShare.None, BufferSize, useAsync: true))
            {
                var  buffer     = new byte[BufferSize];
                long downloaded = 0;
                int  read;
                while ((read = await net.ReadAsync(buffer, cancel).ConfigureAwait(false)) > 0)
                {
                    await file.WriteAsync(buffer.AsMemory(0, read), cancel).ConfigureAwait(false);
                    sha.TransformBlock(buffer, 0, read, null, 0);
                    downloaded += read;
                    if (total > 0) progress?.Report(Math.Min(1.0, (double)downloaded / total));
                }
            }

            sha.TransformFinalBlock([], 0, 0);
            var actual = Convert.ToHexStringLower(sha.Hash!);

            if (HasRealHash(expectedSha256)
                && !actual.Equals(expectedSha256, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidDataException(
                    $"SHA256 mismatch for {url}: expected {expectedSha256}, got {actual}");
            }

            progress?.Report(1.0);
            return actual;
        }
        catch
        {
            // Drop the partial temp file so a later attempt starts clean.
            TryDelete(tmp);
            throw;
        }
    }

    // ── Stage 2: commit (lock-contention retry) ──────────────────────────────

    /// <summary>
    /// Atomically swaps the verified temp file into place. The destination can be
    /// briefly held by an AV scanner, the search indexer or a process that just
    /// exited, so the swap is retried a few times. A <em>persistent</em> lock means
    /// the file is genuinely in use (e.g. the client is loaded in a running game),
    /// which is surfaced with an actionable message rather than a raw Win32 error.
    /// </summary>
    private static async Task CommitAsync(string tmp, string destinationPath, CancellationToken cancel)
    {
        for (var attempt = 1; ; attempt++)
        {
            try
            {
                File.Move(tmp, destinationPath, overwrite: true);
                return;
            }
            catch (Exception ex) when (IsDestinationBusy(ex))
            {
                if (attempt >= CommitRetries)
                    throw new IOException(
                        $"Couldn't update \"{Path.GetFileName(destinationPath)}\" — the file is in use or " +
                        "write-protected. Close Minecraft (or the running client) and try again.", ex);

                await Task.Delay(200 * attempt, cancel).ConfigureAwait(false);
            }
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    private static void TryDelete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); }
        catch { /* best effort — never mask the original failure */ }
    }

    private static bool HasRealHash(string? sha256) =>
        !string.IsNullOrEmpty(sha256) && !sha256.StartsWith("0000");

    /// <summary>
    /// Connection resets, dropped sockets, read timeouts and partial-file IO
    /// errors are worth retrying; a hash mismatch or a 4xx is not. A user
    /// cancellation surfaces as a cancelled token and is filtered out by the
    /// caller before this is consulted.
    /// </summary>
    private static bool IsTransient(Exception ex) =>
        ex is HttpRequestException or IOException or TimeoutException or TaskCanceledException;

    /// <summary>
    /// True when the destination can't be replaced because it's open in another
    /// process. That surfaces two ways: a sharing/lock violation (IOException), or
    /// — when File.Move can't delete the existing file to swap it — an access-denied
    /// UnauthorizedAccessException. Both deserve a brief retry; a genuine permission
    /// problem simply exhausts the retries and reports the same actionable message.
    /// </summary>
    private static bool IsDestinationBusy(Exception ex) => ex switch
    {
        IOException io              => (io.HResult & 0xFFFF) is 32   // ERROR_SHARING_VIOLATION
                                                            or 33,  // ERROR_LOCK_VIOLATION
        UnauthorizedAccessException => true,
        _                           => false
    };

    private static TimeSpan BackoffFor(int attempt) =>
        TimeSpan.FromMilliseconds(400 * Math.Pow(2, attempt - 1));
}

/// <summary>
/// Adapts a plain callback to <see cref="IProgress{T}"/> and invokes it
/// synchronously on the reporting thread, without capturing a
/// <see cref="SynchronizationContext"/>. Useful for re-shaping the shared
/// 0–1 fraction into a service-specific status or percentage.
/// </summary>
public sealed class DelegateProgress<T> : IProgress<T>
{
    private readonly Action<T> _onReport;

    public DelegateProgress(Action<T> onReport) => _onReport = onReport;

    public void Report(T value) => _onReport(value);
}
