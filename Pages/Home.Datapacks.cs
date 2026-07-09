using System;
using System.Collections.Generic;
using System.Linq;
using GlacierLauncher.Models;
using Microsoft.AspNetCore.Components;
using Microsoft.JSInterop;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private string datapacksWorldName = "";
    private List<JavaInstanceFile> javaDatapacks = new();
    private List<(string Name, string Path, long Size, string ModifiedAt)> javaWorldsList = new();

    private void OnOpenDatapacksTab()
    {
        javaModsTab = "datapacks";
        javaWorldsList = JavaInstances.ListWorlds().ToList();
        if (!string.IsNullOrEmpty(datapacksWorldName) && javaWorldsList.All(w => w.Name != datapacksWorldName))
            datapacksWorldName = "";
        RefreshDatapacks();
    }

    private void OnDatapacksWorldChanged(ChangeEventArgs e)
    {
        datapacksWorldName = e.Value?.ToString() ?? "";
        RefreshDatapacks();
    }

    private void RefreshDatapacks()
    {
        javaDatapacks = string.IsNullOrEmpty(datapacksWorldName)
            ? new()
            : JavaInstances.ListDatapacks(datapacksWorldName).ToList();
    }

    private void DeleteDatapack(JavaInstanceFile pack)
    {
        try
        {
            JavaInstances.DeleteDatapack(pack.Path);
            RefreshDatapacks();
            _ = ShowToast("Datapack removed.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        StateHasChanged();
    }

    private void OpenDatapacksFolder()
    {
        if (string.IsNullOrEmpty(datapacksWorldName)) return;
        try { JavaInstances.OpenDatapacksFolder(datapacksWorldName); }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private async System.Threading.Tasks.Task AddDatapackFile()
    {
        if (string.IsNullOrEmpty(datapacksWorldName))
        {
            _ = ShowToast("Pick a world first.", "info");
            return;
        }
        await JS.InvokeVoidAsync("pickDatapackFile", _selfRef);
    }

    [JSInvokable]
    public void OnDatapackPicked(string path)
    {
        if (string.IsNullOrWhiteSpace(path) || string.IsNullOrEmpty(datapacksWorldName)) return;
        try
        {
            JavaInstances.ImportDatapackFile(datapacksWorldName, path);
            RefreshDatapacks();
            _ = ShowToast("Datapack installed.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        InvokeAsync(StateHasChanged);
    }
}
