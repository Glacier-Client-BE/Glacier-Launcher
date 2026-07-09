using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using GlacierLauncher.Services;
using Microsoft.AspNetCore.Components;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private string levModsQuery      = "";
    private bool   levModsSearching  = false;
    private bool   levModsHasSearched = false;
    private string? levModsError;
    private List<LeviLaminaModsService.LeviLaminaMod> levModsResults = new();
    private string? levModsInstallingRepo;
    private double levModsInstallProgress;

    private async Task OpenLeviLaminaMods()
    {
        await NavigateAsync(() => currentView = "levimods");
        if (!levModsHasSearched) await LevModsSearchAsync();
    }

    private void CloseLeviLaminaMods() => _ = OpenClients();

    private void OnLevModsSearchInput(ChangeEventArgs e)
    {
        levModsQuery = e.Value?.ToString() ?? "";
        _ = DebouncedLevModsSearch();
    }

    private CancellationTokenSource? _levModsDebounceCts;

    private async Task DebouncedLevModsSearch()
    {
        _levModsDebounceCts?.Cancel();
        _levModsDebounceCts = new CancellationTokenSource();
        var token = _levModsDebounceCts.Token;
        try
        {
            await Task.Delay(350, token);
            if (!token.IsCancellationRequested) await LevModsSearchAsync();
        }
        catch (TaskCanceledException) { }
    }

    private async Task LevModsSearchAsync()
    {
        levModsSearching = true; levModsError = null;
        StateHasChanged();
        try
        {
            levModsResults = await LeviLaminaMods.SearchAsync(levModsQuery);
            levModsError = LeviLaminaMods.LastError;
            levModsHasSearched = true;
        }
        catch (Exception ex) { levModsError = ex.Message; }
        finally { levModsSearching = false; }
        StateHasChanged();
    }

    private async Task HandleLevModInstall(LeviLaminaModsService.LeviLaminaMod mod)
    {
        levModsInstallingRepo = mod.RepoName; levModsInstallProgress = 0;
        StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(f =>
            { levModsInstallProgress = f; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
            await LeviLaminaMods.DownloadAndInstallAsync(mod, prog);
            _ = ShowToast(mod.Name + " installed!", "success");
        }
        catch (Exception ex) { _ = ShowToast("Install failed: " + ex.Message, "error"); }
        finally { levModsInstallingRepo = null; levModsInstallProgress = 0; }
        StateHasChanged();
    }

    private void HandleLevModDelete(LeviLaminaModsService.LeviLaminaMod mod)
    {
        LeviLaminaMods.Delete(mod);
        _ = ShowToast(mod.Name + " removed.", "info");
        StateHasChanged();
    }
}
