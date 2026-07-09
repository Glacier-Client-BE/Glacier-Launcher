using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private List<BedrockPack> bedrockPacks = new();
    private bool isLoadingBedrockPacks = false;
    private string bedrockPacksKind = "resource"; // resource | behavior | skin
    private string _confirmDeletePackPath = "";

    private async Task OpenBedrockPacks() => await OpenBedrockPacks("resource");

    private async Task OpenBedrockPacks(string kind)
    {
        SetEditionCore("bedrock");
        bedrockPacksKind = kind;
        await RefreshBedrockPacksAsync();
        await NavigateAsync(() => currentView = "bedrockpacks");
    }

    private async Task SwitchBedrockPacksKind(string kind)
    {
        if (bedrockPacksKind == kind) return;
        bedrockPacksKind = kind;
        await RefreshBedrockPacksAsync();
        StateHasChanged();
    }

    private async Task RefreshBedrockPacksAsync()
    {
        isLoadingBedrockPacks = true;
        StateHasChanged();
        var kind = bedrockPacksKind;
        bedrockPacks = await Task.Run(() => Packs.ListPacks(kind));
        isLoadingBedrockPacks = false;
        StateHasChanged();
    }

    private void OpenBedrockPacksFolder()
    {
        try { Packs.OpenFolder(Services.BedrockPackService.DirFor(bedrockPacksKind)); }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void OpenBedrockPackFolder(BedrockPack p)
    {
        try { Packs.OpenFolder(p.FolderPath); }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void ConfirmDeletePack(string folderPath) => _confirmDeletePackPath = folderPath;
    private void CancelDeletePack() => _confirmDeletePackPath = "";

    private async Task DeleteBedrockPack(BedrockPack p)
    {
        try
        {
            await Task.Run(() => Packs.DeletePack(p.FolderPath));
            _confirmDeletePackPath = "";
            await RefreshBedrockPacksAsync();
            _ = ShowToast($"Deleted '{p.Name}'.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }
}
