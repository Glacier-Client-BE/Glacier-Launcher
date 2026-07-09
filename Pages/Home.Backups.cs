using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private List<BedrockBackup> bedrockBackups = new();
    private bool isLoadingBackups = false;
    private bool isCreatingBackup = false;
    private string _confirmDeleteBackupPath = "";
    private string _confirmRestoreBackupPath = "";
    private string backupLabelInput = "";

    private async Task OpenBedrockBackups()
    {
        SetEditionCore("bedrock");
        await RefreshBedrockBackupsAsync();
        await NavigateAsync(() => currentView = "bedrockbackups");
    }

    private async Task RefreshBedrockBackupsAsync()
    {
        isLoadingBackups = true;
        StateHasChanged();
        bedrockBackups = await Task.Run(() => Backups.ListBackups());
        isLoadingBackups = false;
        StateHasChanged();
    }

    private async Task CreateBedrockBackup()
    {
        isCreatingBackup = true;
        StateHasChanged();
        try
        {
            var label = backupLabelInput;
            backupLabelInput = "";
            var path = await Backups.CreateBackupAsync(label);
            Notify.Add("Backup created", $"'{System.IO.Path.GetFileName(path)}' saved.", "success");
            _ = ShowToast("Backup created.", "success");
            await RefreshBedrockBackupsAsync();
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { isCreatingBackup = false; StateHasChanged(); }
    }

    private void ConfirmDeleteBackup(string path) => _confirmDeleteBackupPath = path;
    private void CancelDeleteBackup() => _confirmDeleteBackupPath = "";

    private async Task DeleteBedrockBackup(BedrockBackup b)
    {
        try
        {
            await Task.Run(() => Backups.DeleteBackup(b.FilePath));
            _confirmDeleteBackupPath = "";
            await RefreshBedrockBackupsAsync();
            _ = ShowToast("Backup deleted.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void ConfirmRestoreBackup(string path) => _confirmRestoreBackupPath = path;
    private void CancelRestoreBackup() => _confirmRestoreBackupPath = "";

    private async Task RestoreBedrockBackup(BedrockBackup b)
    {
        _confirmRestoreBackupPath = "";
        _ = ShowToast("Restoring backup…", "info");
        StateHasChanged();
        try
        {
            await Backups.RestoreBackupAsync(b.FilePath);
            Notify.Add("Backup restored", $"'{b.Name}' was restored. Worlds and packs are back in place.", "success");
            _ = ShowToast("Backup restored.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void OnBackupLabelInput(Microsoft.AspNetCore.Components.ChangeEventArgs e) =>
        backupLabelInput = e.Value?.ToString() ?? "";

    private void OpenBedrockBackupsFolder()
    {
        try { Backups.OpenBackupsFolder(); }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }
}
