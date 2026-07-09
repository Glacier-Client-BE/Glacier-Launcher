using System;
using System.Collections.Generic;
using System.Linq;
using GlacierLauncher.Services;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private void OpenDownloadManager() =>
        _ = NavigateAsync(() => currentView = "downloads");

    // DownloadService.Changed fires from background download threads (possibly
    // many times a second while throttled). Subscribed once in OnInitialized and
    // torn down in Dispose, same lifecycle as the notification subscription.
    private void OnDownloadsChanged() => InvokeAsync(StateHasChanged);

    private List<DownloadEntry> DownloadsList =>
        DownloadService.ActiveDownloads.ToList();

    private static string DownloadStatusLabel(DownloadStatus status) => status switch
    {
        DownloadStatus.Downloading => "Downloading",
        DownloadStatus.Completed   => "Completed",
        DownloadStatus.Failed      => "Failed",
        DownloadStatus.Cancelled   => "Cancelled",
        _                          => status.ToString()
    };

    private static string DownloadStatusIcon(DownloadStatus status) => status switch
    {
        DownloadStatus.Downloading => "fa-solid fa-arrow-down",
        DownloadStatus.Completed   => "fa-solid fa-circle-check",
        DownloadStatus.Failed      => "fa-solid fa-circle-xmark",
        DownloadStatus.Cancelled   => "fa-solid fa-ban",
        _                          => "fa-solid fa-circle-question"
    };

    private void CancelDownload(DownloadEntry entry) => entry.Cancel();

    private void ClearFinishedDownloads()
    {
        foreach (var e in DownloadService.ActiveDownloads.Where(e => e.Status != DownloadStatus.Downloading))
            DownloadService.Remove(e.Id);
        StateHasChanged();
    }
}
