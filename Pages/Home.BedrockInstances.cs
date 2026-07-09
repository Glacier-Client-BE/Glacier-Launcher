using System;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private bool isSwitchingBedrockInstance = false;
    private string _renamingBedrockInstanceId = "";
    private string _renameBedrockInstanceValue = "";
    private string _confirmDeleteBedrockInstanceId = "";

    private async Task OpenBedrockInstances()
    {
        SetEditionCore("bedrock");
        await NavigateAsync(() => currentView = "bedrockinstances");
    }

    private async Task SwitchBedrockInstance(string id)
    {
        if (isSwitchingBedrockInstance) return;
        if (id == BedrockInstances.ActiveInstance.Id) return;
        isSwitchingBedrockInstance = true;
        StateHasChanged();
        try
        {
            await BedrockInstances.SwitchToAsync(id);
            await RefreshBedrockWorldsAsync();
            _ = ShowToast($"Switched to '{BedrockInstances.ActiveInstance.Name}'.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { isSwitchingBedrockInstance = false; StateHasChanged(); }
    }

    private void NewBedrockInstance()
    {
        var instance = BedrockInstances.Create("New Instance");
        _ = ShowToast($"Created '{instance.Name}'. Switch to it to start using its own worlds and packs.", "info");
        StateHasChanged();
    }

    private void BeginRenameBedrockInstance(string id, string current)
    {
        _renamingBedrockInstanceId = id;
        _renameBedrockInstanceValue = current;
    }

    private void OnRenameBedrockInstanceInput(Microsoft.AspNetCore.Components.ChangeEventArgs e) =>
        _renameBedrockInstanceValue = e.Value?.ToString() ?? "";

    private void CommitRenameBedrockInstance()
    {
        if (!string.IsNullOrWhiteSpace(_renameBedrockInstanceValue))
            BedrockInstances.Rename(_renamingBedrockInstanceId, _renameBedrockInstanceValue);
        _renamingBedrockInstanceId = "";
        StateHasChanged();
    }

    private void ConfirmDeleteBedrockInstance(string id) => _confirmDeleteBedrockInstanceId = id;
    private void CancelDeleteBedrockInstance() => _confirmDeleteBedrockInstanceId = "";

    private void DeleteBedrockInstance(string id)
    {
        var ok = BedrockInstances.Delete(id);
        _confirmDeleteBedrockInstanceId = "";
        _ = ShowToast(ok ? "Instance deleted." : "Can't delete the active instance — switch to another one first.", ok ? "success" : "error");
        StateHasChanged();
    }

    private async Task BackupBedrockInstance(BedrockInstance instance)
    {
        try
        {
            _ = ShowToast($"Backing up '{instance.Name}'…", "info");
            var path = await BedrockInstances.BackupInstanceAsync(instance);
            Notify.Add("Instance backed up", $"'{instance.Name}' saved to {path}.", "success");
            _ = ShowToast("Instance backed up.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }
}
