using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private List<JavaInstanceFile> bedrockScreenshots = new();
    private bool isLoadingBedrockScreenshots = false;

    private async Task OpenBedrockScreenshots()
    {
        SetEditionCore("bedrock");
        isLoadingBedrockScreenshots = true;
        StateHasChanged();
        bedrockScreenshots = await Task.Run(() => BedrockShots.ListScreenshots());
        isLoadingBedrockScreenshots = false;
        await NavigateAsync(() => currentView = "bedrockscreenshots");
    }

    private async Task RefreshBedrockScreenshotsAsync()
    {
        isLoadingBedrockScreenshots = true;
        StateHasChanged();
        bedrockScreenshots = await Task.Run(() => BedrockShots.ListScreenshots());
        isLoadingBedrockScreenshots = false;
        StateHasChanged();
    }

    private void OpenBedrockScreenshotsFolder()
    {
        try { BedrockShots.OpenFolder(); }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }
}
