using System;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Models;
using GlacierLauncher.Services;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private GlacierManifest?         glacierManifest;
    private bool                     glacierLoading;
    private bool                     glacierBusy;
    private double                   glacierProgress;
    private string?                  glacierError;
    private CancellationTokenSource? glacierCts;

    /// <summary>Latest published Glacier build, or the newest entry as a fallback.</summary>
    private GlacierClientVersion? GlacierLatest =>
        glacierManifest?.Versions.FirstOrDefault(v => v.Id == glacierManifest.LatestRelease)
        ?? glacierManifest?.Versions.FirstOrDefault();

    private async Task EnsureGlacierManifestAsync(bool force = false)
    {
        if (glacierManifest != null && !force) return;
        glacierLoading = true; glacierError = null; StateHasChanged();
        glacierManifest = await Glacier.GetManifestAsync(force);
        glacierError    = Glacier.LastError;
        glacierLoading  = false; StateHasChanged();
    }

    private async Task InstallGlacierAsync(GlacierClientVersion version)
    {
        if (glacierBusy) return;
        glacierCts?.Dispose();
        glacierCts  = new CancellationTokenSource();
        glacierBusy = true; glacierProgress = 0; glacierError = null; StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(f =>
            { glacierProgress = f; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
            await Glacier.InstallAsync(version, prog, glacierCts.Token);
            _ = ShowToast($"Glacier {version.Name} installed!", "success");
        }
        catch (OperationCanceledException) { _ = ShowToast("Glacier install cancelled.", "info"); }
        catch (Exception ex)
        {
            glacierError = ex.Message;
            _ = ShowToast("Install failed: " + ex.Message, "error");
        }
        finally { glacierBusy = false; glacierProgress = 0; var cts = glacierCts; glacierCts = null; cts?.Dispose(); StateHasChanged(); }
    }

    private void CancelGlacier() => glacierCts?.Cancel();

    private void UninstallGlacier(GlacierClientVersion version)
    {
        Glacier.Uninstall(version);
        _ = ShowToast($"Glacier {version.Name} removed.", "info");
        StateHasChanged();
    }

    private async Task LaunchGlacierAsync(GlacierClientVersion version)
    {
        if (glacierBusy) return;

        var mcVersion = SettingsService.Settings.JavaActiveVersion;
        if (string.IsNullOrWhiteSpace(mcVersion))
        {
            _ = ShowToast("Pick a Java version first in the Versions panel.", "error");
            return;
        }

        glacierBusy = true; glacierError = null; StateHasChanged();
        try
        {
            await Glacier.LaunchAsync(version, mcVersion);
            AddToRecentlyLaunched("Java Glacier " + version.Id);
            Discord.SetJavaInGamePresence(mcVersion, "Glacier");
            _ = ShowToast($"Glacier {version.Name} launching…", "success");
            if (SettingsService.Settings.CloseAfterLaunch) await CloseWindow();
        }
        catch (Exception ex)
        {
            glacierError = ex.Message;
            _ = ShowToast(ex.Message, "error");
        }
        finally { glacierBusy = false; StateHasChanged(); }
    }
}
