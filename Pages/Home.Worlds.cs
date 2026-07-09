using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private List<BedrockWorld> bedrockWorlds = new();
    private bool isLoadingBedrockWorlds = false;
    private string _confirmDeleteWorldPath = "";

    private async Task OpenBedrockWorlds()
    {
        SetEditionCore("bedrock");
        await RefreshBedrockWorldsAsync();
        await NavigateAsync(() => currentView = "bedrockworlds");
    }

    private async Task RefreshBedrockWorldsAsync()
    {
        isLoadingBedrockWorlds = true;
        StateHasChanged();
        bedrockWorlds = await Task.Run(() => Worlds.ListWorlds());
        isLoadingBedrockWorlds = false;
        StateHasChanged();
    }

    private void OpenBedrockWorldFolder(BedrockWorld w)
    {
        try { Worlds.OpenFolder(w.FolderPath); }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void ConfirmDeleteWorld(string folderPath) => _confirmDeleteWorldPath = folderPath;
    private void CancelDeleteWorld() => _confirmDeleteWorldPath = "";

    private async Task DeleteBedrockWorld(BedrockWorld w)
    {
        try
        {
            await Task.Run(() => Worlds.DeleteWorld(w.FolderPath));
            _confirmDeleteWorldPath = "";
            await RefreshBedrockWorldsAsync();
            _ = ShowToast($"Deleted '{w.Name}'.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private async Task ExportBedrockWorld(BedrockWorld w)
    {
        try
        {
            _ = ShowToast($"Exporting '{w.Name}'…", "info");
            var path = await Worlds.ExportWorldAsync(w.FolderPath);
            Notify.Add("World exported", $"'{w.Name}' was exported to {path}.", "success");
            _ = ShowToast("World exported.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void OpenBedrockWorldsFolder()
    {
        try { System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            { FileName = Services.BedrockWorldService.WorldsDir, UseShellExecute = true }); }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }
}
