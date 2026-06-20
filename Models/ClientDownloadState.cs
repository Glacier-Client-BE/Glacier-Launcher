namespace GlacierLauncher.Models;

/// <summary>
/// Mutable UI state for a downloadable client (Flarial, OderSo, …): whether it
/// is installed, whether it matches the latest remote build, whether a download
/// is in flight, its progress as a 0–1 fraction and the last surfaced error.
/// </summary>
public sealed class ClientDownloadState
{
    public bool   Downloaded  { get; set; }
    public bool   UpToDate    { get; set; }
    public bool   Downloading { get; set; }
    public double Progress    { get; set; }
    public string Error       { get; set; } = "";

    private CancellationTokenSource? _cts;

    public bool HasError => !string.IsNullOrEmpty(Error);

    // ── Transitions ──────────────────────────────────────────────────────────

    /// <summary>Flips into the downloading state and hands back a fresh token to drive the download.</summary>
    public CancellationToken BeginDownload()
    {
        _cts?.Dispose();
        _cts        = new CancellationTokenSource();
        Downloading = true;
        Progress    = 0;
        Error       = "";
        return _cts.Token;
    }

    public void EndDownload()
    {
        Downloading = false;
        Progress    = 0;
        var cts = _cts;
        _cts = null;
        cts?.Dispose();
    }

    public void Cancel()
    {
        try { _cts?.Cancel(); }
        catch (ObjectDisposedException) { }
    }

    public void MarkRemoved()
    {
        Downloaded = false;
        UpToDate   = false;
    }
}
