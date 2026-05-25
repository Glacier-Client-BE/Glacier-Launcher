using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Models;
using GlacierLauncher.Services;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.JSInterop;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private void OnClientChanged(ChangeEventArgs e) => SelectClient(e.Value?.ToString() ?? "Latite Client");

    private void SelectClient(string name)
    {
        SettingsService.Settings.SelectedClient = name;
        SettingsService.Save();
        BuildDefaultSearchResults();
        StateHasChanged();
    }

    private void CycleClient()
    {
        var clients = new List<string> { "Latite Client", "Flarial Client", "OderSo Client", "Vanilla" };
        if (!string.IsNullOrEmpty(customDllPath)) clients.Add("Custom DLL");
        var idx = clients.IndexOf(SettingsService.Settings.SelectedClient);
        if (idx < 0) idx = -1;
        SelectClient(clients[(idx + 1) % clients.Count]);
        _ = ShowToast("Active client: " + SettingsService.Settings.SelectedClient, "info");
    }

    private void ClearCustomDll()
    {
        customDllPath = "";
        if (SettingsService.Settings.SelectedClient == "Custom DLL") SelectClient("Latite Client");
        StateHasChanged();
    }

    private async Task CopyDllPath()
    {
        await JS.InvokeVoidAsync("copyToClipboard", customDllPath);
        copiedDllPath = true; StateHasChanged();
        await Task.Delay(1500);
        copiedDllPath = false; StateHasChanged();
    }

    private async Task OpenClients()
    {
        currentView = "clients";
        flarialDownloaded = Flarial.IsDownloaded;
        flarialUpToDate   = flarialDownloaded && await Flarial.IsUpToDateAsync();
        odersoDownloaded  = OderSo.IsDownloaded;
        odersoUpToDate    = odersoDownloaded  && await OderSo.IsUpToDateAsync();
        clientsHasBadge   = false;
        StateHasChanged();
    }

    private async Task HandleFlarialDownload()
    {
        flarialDownloading = true; flarialProgress = 0; flarialError = "";
        StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(pct =>
            { flarialProgress = pct; InvokeAsync(StateHasChanged); }));
            await Flarial.DownloadAsync(prog);
            flarialDownloaded = true;
            flarialUpToDate   = await Flarial.IsUpToDateAsync();
            _ = ShowToast("Flarial downloaded!", "success");
        }
        catch (Exception ex) { flarialError = ex.Message; }
        finally { flarialDownloading = false; flarialProgress = 0; }
        StateHasChanged();
    }

    private void HandleFlarialDelete()
    {
        Flarial.Delete();
        flarialDownloaded = false; flarialUpToDate = false;
        StateHasChanged();
    }

    private async Task HandleOderSoDownload()
    {
        odersoDownloading = true; odersoProgress = 0; odersoError = "";
        StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(pct =>
            { odersoProgress = pct; InvokeAsync(StateHasChanged); }));
            await OderSo.DownloadAsync(prog);
            odersoDownloaded = true;
            odersoUpToDate   = await OderSo.IsUpToDateAsync();
            _ = ShowToast("OderSo downloaded!", "success");
        }
        catch (Exception ex) { odersoError = ex.Message; }
        finally { odersoDownloading = false; odersoProgress = 0; }
        StateHasChanged();
    }

    private void HandleOderSoDelete()
    {
        OderSo.Delete();
        odersoDownloaded = false; odersoUpToDate = false;
        if (SettingsService.Settings.SelectedClient == "OderSo Client") SelectClient("Latite Client");
        StateHasChanged();
    }

    private string versionsBackTarget = "home";

    private async Task OpenVersions()
    {
        if (currentView != "versions") versionsBackTarget = currentView;
        currentView = "versions"; versionsFilter = "";
        versionsHasBadge = false;
        if (versionsClient == "Latite" && versions.Count == 0) await LoadVersionsAsync();
        else if (versionsClient == "OderSo" && oderSoVersions.Count == 0) await LoadOderSoVersionsAsync();
    }

    private async Task BackFromVersions()
    {
        switch (versionsBackTarget)
        {
            case "clients": await OpenClients(); break;
            case "settings": OpenSettings(); break;
            case "addons":   OpenAddons(); break;
            case "servers":  OpenServers(); break;
            case "credits":  OpenCredits(); break;
            default:         GoHome(); break;
        }
    }

    private async Task RefreshVersionsAsync()
    {
        if (versionsClient == "Latite") await LoadVersionsAsync();
        else await LoadOderSoVersionsAsync();
    }

    private async Task SwitchVersionsClient(string client)
    {
        versionsClient = client;
        versionsFilter = "";
        if (client == "Latite" && versions.Count == 0)        await LoadVersionsAsync();
        else if (client == "OderSo" && oderSoVersions.Count == 0) await LoadOderSoVersionsAsync();
        else StateHasChanged();
    }

    private string? latiteFetchError = null;

    private async Task LoadVersionsAsync()
    {
        isLoadingVersions = true; latiteFetchError = null; StateHasChanged();
        versions = await Launcher.GetVersionsAsync();
        latiteFetchError = Launcher.LastVersionsError;
        isLoadingVersions = false; StateHasChanged();
    }

    private bool IsPinned(string tag) => SettingsService.Settings.PinnedVersions.Contains(tag);

    private void TogglePin(MinecraftVersion v)
    {
        var pins = SettingsService.Settings.PinnedVersions;
        if (pins.Contains(v.Tag)) pins.Remove(v.Tag);
        else pins.Insert(0, v.Tag);
        SettingsService.Save();
        StateHasChanged();
    }

    private async Task CopyVersionTag(MinecraftVersion v)
    {
        await JS.InvokeVoidAsync("copyToClipboard", v.Tag);
        _ = ShowToast("Copied \"" + v.Tag + "\"", "info");
    }

    private RenderFragment VersionActions(MinecraftVersion v) => b =>
    {
        if (v.IsDownloading)
        {
            b.OpenElement(0, "div");
            b.AddAttribute(1, "class", "dl-progress-row");
            if (downloadProgress > 0)
            {
                b.OpenElement(2, "div");
                b.AddAttribute(3, "class", "progress-bar-wrap");
                b.OpenElement(4, "div");
                b.AddAttribute(5, "class", "progress-bar-fill");
                b.AddAttribute(6, "style", $"width:{downloadProgress:F0}%");
                b.CloseElement();
                b.CloseElement();
                b.OpenElement(7, "span");
                b.AddAttribute(8, "class", "dl-pct");
                b.AddContent(9, $"{(int)downloadProgress}%");
                b.CloseElement();
            }
            b.OpenElement(10, "button");
            b.AddAttribute(11, "class", "icon-btn");
            b.AddAttribute(12, "disabled", true);
            b.OpenElement(13, "span");
            b.AddAttribute(14, "class", "spinner");
            b.CloseElement();
            b.CloseElement();
            b.CloseElement();
        }
        else
        {
            b.OpenElement(20, "div");
            b.AddAttribute(21, "class", "version-actions");

            b.OpenElement(22, "button");
            b.AddAttribute(23, "class", IsPinned(v.Tag) ? "pin-btn pinned" : "pin-btn");
            b.AddAttribute(24, "title", IsPinned(v.Tag) ? "Unpin" : "Pin");
            b.AddAttribute(25, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, () => TogglePin(v)));
            b.OpenElement(26, "i");
            b.AddAttribute(27, "class", "fa-solid fa-thumbtack");
            b.AddAttribute(28, "style", IsPinned(v.Tag) ? "" : "opacity:0.4;");
            b.CloseElement();
            b.CloseElement();

            b.OpenElement(30, "button");
            b.AddAttribute(31, "class", "copy-btn");
            b.AddAttribute(32, "title", "Copy tag");
            b.AddAttribute(33, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, async () => await CopyVersionTag(v)));
            b.OpenElement(34, "i");
            b.AddAttribute(35, "class", "fa-solid fa-copy");
            b.CloseElement();
            b.CloseElement();

            if (v.IsDownloaded)
            {
                b.OpenElement(40, "button");
                b.AddAttribute(41, "class", "icon-btn icon-btn-ghost");
                b.AddAttribute(42, "title", "Delete");
                b.AddAttribute(43, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, () => HandleVersionDelete(v)));
                b.OpenElement(44, "i");
                b.AddAttribute(45, "class", "fa-solid fa-trash");
                b.CloseElement();
                b.CloseElement();

                b.OpenElement(50, "button");
                b.AddAttribute(51, "class", "icon-btn");
                b.AddAttribute(52, "title", "Launch");
                b.AddAttribute(53, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, async () => await HandleVersionLaunch(v)));
                b.OpenElement(54, "i");
                b.AddAttribute(55, "class", "fa-solid fa-play");
                b.CloseElement();
                b.CloseElement();
            }
            else
            {
                b.OpenElement(60, "button");
                b.AddAttribute(61, "class", "icon-btn");
                b.AddAttribute(62, "title", "Download");
                b.AddAttribute(63, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, async () => await HandleVersionDownload(v)));
                b.OpenElement(64, "i");
                b.AddAttribute(65, "class", "fa-solid fa-download");
                b.CloseElement();
                b.CloseElement();
            }

            b.CloseElement();
        }
    };

    private async Task HandleVersionDownload(MinecraftVersion v)
    {
        v.IsDownloading = true; v.ErrorMessage = null; downloadProgress = 0; StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(pct =>
            { downloadProgress = pct; InvokeAsync(StateHasChanged); }));
            await Launcher.DownloadVersionAsync(v, prog);
            _ = ShowToast(v.DisplayName + " downloaded!", "success");
        }
        catch (Exception ex) { v.ErrorMessage = ex.Message; }
        finally { v.IsDownloading = false; downloadProgress = 0; }
        StateHasChanged();
    }

    private void HandleVersionDelete(MinecraftVersion v) { Launcher.DeleteVersion(v); StateHasChanged(); }

    private async Task HandleVersionLaunch(MinecraftVersion v)
    {
        GoHome(); isLaunching = true;
        await ShowStatus($"Launching {v.DisplayName}…", clearAfterMs: 30_000);
        try
        {
            var dllPath = GameLauncher.GetDllPath(v.Tag);
            await Launcher.LaunchWithDllAsync(dllPath);
            isLaunching = false;
            SettingsService.Settings.LastUsedVersion = v.Tag;
            SettingsService.Save();
            Discord.SetInGamePresence(v.Tag, SettingsService.Settings.SelectedClient);
            AddToRecentlyLaunched(v.DisplayName);

            if (SettingsService.Settings.CloseAfterLaunch) { await CloseWindow(); return; }

            await ShowStatus("Injected! Have fun.", clearAfterMs: 3000);
            _ = ShowToast("Injection successful!", "success");
        }
        catch (Exception ex)
        {
            isLaunching = false;
            await ShowStatus(ex.Message, error: true, clearAfterMs: 8000);
            _ = ShowToast(ex.Message, "error");
        }
    }

    private string? oderSoFetchError = null;

    private async Task LoadOderSoVersionsAsync()
    {
        isLoadingOderSoVersions = true; oderSoFetchError = null; StateHasChanged();
        var dlls   = await OderSo.ListDllsAsync();
        var active = OderSo.GetActiveEntry();
        oderSoVersions = dlls.Select(e => new OderSoDllState(e)
        {
            IsDownloaded = OderSo.IsEntryDownloaded(e),
            IsActive     = active?.Name == e.Name
        }).ToList();
        oderSoFetchError = OderSo.LastError;
        isLoadingOderSoVersions = false; StateHasChanged();
    }

    private async Task HandleOderSoEntryDownload(OderSoDllState state)
    {
        state.IsDownloading = true; state.Progress = 0; state.Error = "";
        StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(pct =>
            { state.Progress = pct; InvokeAsync(StateHasChanged); }));
            await OderSo.DownloadEntryAsync(state.Entry, prog);
            state.IsDownloaded = true;
            _ = ShowToast(state.DisplayName + " downloaded!", "success");
        }
        catch (Exception ex) { state.Error = ex.Message; }
        finally { state.IsDownloading = false; state.Progress = 0; }
        StateHasChanged();
    }

    private void HandleOderSoEntryDelete(OderSoDllState state)
    {
        OderSo.DeleteEntry(state.Entry);
        state.IsDownloaded = false;
        state.IsActive = false;
        if (SettingsService.Settings.SelectedClient == "OderSo Client" && !OderSo.IsDownloaded)
            SelectClient("Latite Client");
        StateHasChanged();
    }

    private async Task HandleOderSoEntryLaunch(OderSoDllState state)
    {
        OderSo.SetActiveEntry(state.Entry);
        foreach (var s in oderSoVersions) s.IsActive = (s.Entry.Name == state.Entry.Name);
        SelectClient("OderSo Client");
        GoHome();
        isLaunching = true;
        await ShowStatus($"Launching OderSo {state.Entry.Name}…", clearAfterMs: 30_000);
        try
        {
            var dllPath = OderSo.GetEntryFilePath(state.Entry);
            await Launcher.LaunchWithDllAsync(dllPath);
            isLaunching = false;
            Discord.SetInGamePresence(state.Entry.Name, "OderSo Client");
            AddToRecentlyLaunched("OderSo");

            if (SettingsService.Settings.CloseAfterLaunch) { await CloseWindow(); return; }

            await ShowStatus("Injected! Have fun.", clearAfterMs: 3000);
            _ = ShowToast("Injection successful!", "success");
        }
        catch (Exception ex)
        {
            isLaunching = false;
            await ShowStatus(ex.Message, error: true, clearAfterMs: 8000);
            _ = ShowToast(ex.Message, "error");
        }
    }

    private RenderFragment OderSoVersionActions(OderSoDllState state) => b =>
    {
        if (state.IsDownloading)
        {
            b.OpenElement(0, "div");
            b.AddAttribute(1, "class", "dl-progress-row");
            if (state.Progress > 0)
            {
                b.OpenElement(2, "div");
                b.AddAttribute(3, "class", "progress-bar-wrap");
                b.OpenElement(4, "div");
                b.AddAttribute(5, "class", "progress-bar-fill");
                b.AddAttribute(6, "style", $"width:{state.Progress:F0}%");
                b.CloseElement();
                b.CloseElement();
                b.OpenElement(7, "span");
                b.AddAttribute(8, "class", "dl-pct");
                b.AddContent(9, $"{(int)state.Progress}%");
                b.CloseElement();
            }
            b.OpenElement(10, "button");
            b.AddAttribute(11, "class", "icon-btn");
            b.AddAttribute(12, "disabled", true);
            b.OpenElement(13, "span");
            b.AddAttribute(14, "class", "spinner");
            b.CloseElement();
            b.CloseElement();
            b.CloseElement();
        }
        else
        {
            b.OpenElement(20, "div");
            b.AddAttribute(21, "class", "version-actions");

            if (state.IsDownloaded)
            {
                b.OpenElement(30, "button");
                b.AddAttribute(31, "class", "icon-btn icon-btn-ghost");
                b.AddAttribute(32, "title", "Delete");
                b.AddAttribute(33, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, () => HandleOderSoEntryDelete(state)));
                b.OpenElement(34, "i");
                b.AddAttribute(35, "class", "fa-solid fa-trash");
                b.CloseElement();
                b.CloseElement();

                b.OpenElement(40, "button");
                b.AddAttribute(41, "class", "icon-btn");
                b.AddAttribute(42, "title", "Set active & Launch");
                b.AddAttribute(43, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, async () => await HandleOderSoEntryLaunch(state)));
                b.OpenElement(44, "i");
                b.AddAttribute(45, "class", "fa-solid fa-play");
                b.CloseElement();
                b.CloseElement();
            }
            else
            {
                b.OpenElement(50, "button");
                b.AddAttribute(51, "class", "icon-btn");
                b.AddAttribute(52, "title", "Download");
                b.AddAttribute(53, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, async () => await HandleOderSoEntryDownload(state)));
                b.OpenElement(54, "i");
                b.AddAttribute(55, "class", "fa-solid fa-download");
                b.CloseElement();
                b.CloseElement();
            }

            b.CloseElement();
        }
    };

    private record SortOption(string Key, string Label, string Icon);
    private static readonly SortOption[] _sortOptions =
    {
        new("newest",     "Newest first",     "fa-arrow-down-9-1"),
        new("oldest",     "Oldest first",     "fa-arrow-up-9-1"),
        new("name",       "Name A–Z",         "fa-arrow-down-a-z"),
        new("downloaded", "Downloaded first", "fa-circle-down"),
    };

    private string CurrentSortLabel =>
        _sortOptions.FirstOrDefault(o => o.Key == SettingsService.Settings.VersionSortMode)?.Label
            ?? "Newest first";

    private string CurrentSortIcon =>
        _sortOptions.FirstOrDefault(o => o.Key == SettingsService.Settings.VersionSortMode)?.Icon
            ?? "fa-arrow-down-wide-short";

    private IEnumerable<MinecraftVersion> SortedVersions(IEnumerable<MinecraftVersion> src)
    {
        var mode = SettingsService.Settings.VersionSortMode;
        return mode switch
        {
            "oldest"     => src.Reverse(),
            "name"       => src.OrderBy(v => v.DisplayName, StringComparer.OrdinalIgnoreCase),
            "downloaded" => src.OrderByDescending(v => v.IsDownloaded).ThenBy(v => 0),
            _            => src
        };
    }

    private void SetVersionSort(string mode)
    {
        SettingsService.Settings.VersionSortMode = mode;
        SettingsService.Save();
        StateHasChanged();
    }

    private void CycleVersionSort()
    {
        var current = SettingsService.Settings.VersionSortMode ?? "newest";
        var idx = Array.FindIndex(_sortOptions, o => o.Key == current);
        if (idx < 0) idx = -1;
        var next = _sortOptions[(idx + 1) % _sortOptions.Length];
        SetVersionSort(next.Key);
    }

    private void ToggleShowOnlyDownloaded()
    {
        SettingsService.Settings.ShowOnlyDownloaded = !SettingsService.Settings.ShowOnlyDownloaded;
        SettingsService.Save();
        StateHasChanged();
    }

    private void OpenAddons()
    {
        currentView = "addons";
        if (IsJava)
            RefreshJavaInstanceFiles();
        if (!cfHasSearched && CurseForge.IsAvailable)
            _ = CfSearchAsync();
    }

    private void RefreshJavaInstanceFiles()
    {
        javaMods = JavaInstances.ListFiles("mods").ToList();
        javaScreenshots = JavaInstances.ListFiles("screenshots").ToList();
        javaResourcePacks = JavaInstances.ListFiles("resourcepacks").ToList();
        javaShaderPacks = JavaInstances.ListFiles("shaderpacks").ToList();
        javaSchematics = JavaInstances.ListFiles("schematics").Concat(JavaInstances.ListFiles("litematica")).ToList();
    }

    private void OpenJavaFolder(string kind)
    {
        try { JavaInstances.OpenFolder(kind); }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void ToggleJavaMod(JavaInstanceFile file)
    {
        try
        {
            JavaInstances.ToggleMod(file.Path);
            RefreshJavaInstanceFiles();
            StateHasChanged();
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void DeleteJavaFile(JavaInstanceFile file)
    {
        try
        {
            JavaInstances.DeleteFile(file.Path);
            RefreshJavaInstanceFiles();
            StateHasChanged();
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private async Task BackupJavaSaves()
    {
        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var zip = await JavaInstances.BackupSavesAsync();
            _ = ShowToast(zip == null ? "No saves to back up." : "World backup created.", zip == null ? "info" : "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { javaUtilityBusy = false; StateHasChanged(); }
    }

    private async Task ExportJavaModpack()
    {
        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var zip = await JavaInstances.ExportModpackAsync();
            _ = ShowToast("Modpack exported: " + System.IO.Path.GetFileName(zip), "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { javaUtilityBusy = false; StateHasChanged(); }
    }

    private async Task DuplicateJavaInstance()
    {
        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var copy = await Task.Run(() => JavaInstances.DuplicateActive());
            JavaInstances.SetActive(copy.Id);
            javaVersionsList.Clear();
            RefreshJavaInstanceFiles();
            _ = ShowToast("Instance duplicated.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { javaUtilityBusy = false; StateHasChanged(); }
    }

    private async Task InstallFabricForActive() => await InstallLoaderForActive("fabric");

    private async Task InstallQuiltForActive() => await InstallLoaderForActive("quilt");

    private async Task InstallLoaderForActive(string loader)
    {
        var version = SettingsService.Settings.JavaActiveVersion;
        if (string.IsNullOrWhiteSpace(version))
        {
            _ = ShowToast("Select a Java version first.", "error");
            return;
        }
        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var profile = loader == "fabric"
                ? await JavaLoaders.InstallFabricAsync(version)
                : await JavaLoaders.InstallQuiltAsync(version);
            SettingsService.Settings.JavaActiveVersion = profile.Id;
            JavaInstances.SetActiveVersion(profile.Id);
            SettingsService.Save();
            await LoadJavaVersionsAsync();
            _ = ShowToast(loader == "fabric" ? "Fabric profile installed." : "Quilt profile installed.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { javaUtilityBusy = false; StateHasChanged(); }
    }

    private async Task DownloadForgeInstaller()
    {
        var version = SettingsService.Settings.JavaActiveVersion;
        if (string.IsNullOrWhiteSpace(version))
        {
            _ = ShowToast("Select a Java version first.", "error");
            return;
        }
        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var path = await JavaLoaders.DownloadForgeInstallerAsync(version);
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo { FileName = path, UseShellExecute = true });
            _ = ShowToast("Forge installer downloaded & launched.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { javaUtilityBusy = false; StateHasChanged(); }
    }

    private async Task DownloadNeoForgeInstaller()
    {
        var version = SettingsService.Settings.JavaActiveVersion;
        if (string.IsNullOrWhiteSpace(version))
        {
            _ = ShowToast("Select a Java version first.", "error");
            return;
        }
        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var path = await JavaLoaders.DownloadNeoForgeInstallerAsync(version);
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo { FileName = path, UseShellExecute = true });
            _ = ShowToast("NeoForge installer downloaded & launched.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { javaUtilityBusy = false; StateHasChanged(); }
    }

    private void OpenSettingsCurseForge() { OpenSettings(); settingsFilter = "curseforge"; StateHasChanged(); }

    private async Task OnCfSearchKeyDown(KeyboardEventArgs e)
    {
        if (e.Key == "Enter")
        {
            _cfDebounceCts?.Cancel();
            await CfSearchAsync();
        }
    }

    private void OnCfSearchInput(ChangeEventArgs e)
    {
        cfSearchQuery = e.Value?.ToString() ?? "";
        _ = DebouncedCfSearch();
    }

    private async Task SetCfCategory(string cat)
    {
        cfCategory = cat;
        StateHasChanged();
        if (cfHasSearched) await CfSearchAsync();
    }

    private async Task CfSearchAsync()
    {
        cfSearching = true; cfError = ""; cfPage = 0; cfResults.Clear();
        StateHasChanged();
        try
        {
            var result = await CurseForge.SearchAsync(cfSearchQuery, cfCategory, 20, 0);
            cfResults    = result.Addons;
            cfTotalCount = result.TotalCount;
            cfHasSearched = true;
        }
        catch (Exception ex) { cfError = ex.Message; }
        finally { cfSearching = false; }
        StateHasChanged();
    }

    private async Task CfLoadMore()
    {
        cfPage++; cfSearching = true;
        StateHasChanged();
        try
        {
            var result = await CurseForge.SearchAsync(cfSearchQuery, cfCategory, 20, cfPage);
            cfResults.AddRange(result.Addons);
            cfTotalCount = result.TotalCount;
        }
        catch (Exception ex) { cfError = ex.Message; }
        finally { cfSearching = false; }
        StateHasChanged();
    }

    private async Task HandleCfDownload(CurseForgeService.CfAddon addon)
    {
        cfDownloadingId = addon.Id; cfDownloadProgress = 0;
        StateHasChanged();
        try
        {
            var files = await CurseForge.GetFilesAsync(addon.Id);
            var file = files.FirstOrDefault();
            if (file == null) { _ = ShowToast("No downloadable file found.", "error"); return; }

            var prog = new ThrottledProgress(new Progress<double>(pct =>
            { cfDownloadProgress = pct; InvokeAsync(StateHasChanged); }));
            await CurseForge.DownloadAndInstallAsync(file, addon.ClassId, prog);
            _ = ShowToast(addon.Name + " installed!", "success");
        }
        catch (Exception ex) { _ = ShowToast("Download failed: " + ex.Message, "error"); }
        finally { cfDownloadingId = 0; cfDownloadProgress = 0; }
        StateHasChanged();
    }

    private System.Threading.CancellationTokenSource? _cfDebounceCts;

    private async Task DebouncedCfSearch()
    {
        _cfDebounceCts?.Cancel();
        _cfDebounceCts = new System.Threading.CancellationTokenSource();
        var token = _cfDebounceCts.Token;
        try
        {
            await Task.Delay(350, token);
            if (!token.IsCancellationRequested) await CfSearchAsync();
        }
        catch (TaskCanceledException) { }
    }

    // ── Modrinth ─────────────────────────────────────────────

    private async Task SwitchToModrinth()
    {
        javaModsTab = "modrinth";
        StateHasChanged();
        if (!mrHasSearched) await MrSearchAsync();
    }

    private async Task SetMrCategory(string cat)
    {
        mrCategory = cat;
        StateHasChanged();
        await MrSearchAsync();
    }

    private async Task OnMrSearchKeyDown(KeyboardEventArgs e)
    {
        if (e.Key == "Enter")
        {
            _mrDebounceCts?.Cancel();
            await MrSearchAsync();
        }
    }

    private void OnMrSearchInput(ChangeEventArgs e)
    {
        mrSearchQuery = e.Value?.ToString() ?? "";
        _ = DebouncedMrSearch();
    }

    private async Task MrSearchAsync()
    {
        mrSearching = true; mrError = ""; mrPage = 0; mrResults.Clear();
        StateHasChanged();
        try
        {
            var result = await Modrinth.SearchAsync(mrSearchQuery, mrCategory, 20, 0);
            mrResults    = result.Projects;
            mrTotalCount = result.TotalCount;
            mrHasSearched = true;
        }
        catch (Exception ex) { mrError = ex.Message; }
        finally { mrSearching = false; }
        StateHasChanged();
    }

    private async Task MrLoadMore()
    {
        mrPage++; mrSearching = true;
        StateHasChanged();
        try
        {
            var result = await Modrinth.SearchAsync(mrSearchQuery, mrCategory, 20, mrPage * 20);
            mrResults.AddRange(result.Projects);
            mrTotalCount = result.TotalCount;
        }
        catch (Exception ex) { mrError = ex.Message; }
        finally { mrSearching = false; }
        StateHasChanged();
    }

    private async Task HandleMrDownload(ModrinthService.MrProject project)
    {
        mrDownloadingId = project.Id; mrDownloadProgress = 0;
        StateHasChanged();
        try
        {
            var version = await Modrinth.GetLatestVersionAsync(project.Id);
            if (version == null) { _ = ShowToast("No downloadable file found.", "error"); return; }

            var prog = new ThrottledProgress(new Progress<double>(pct =>
            { mrDownloadProgress = pct; InvokeAsync(StateHasChanged); }));
            await Modrinth.DownloadAndInstallAsync(version, project.ProjectType, prog);
            _ = ShowToast(project.Title + " installed!", "success");
        }
        catch (Exception ex) { _ = ShowToast("Download failed: " + ex.Message, "error"); }
        finally { mrDownloadingId = ""; mrDownloadProgress = 0; }
        StateHasChanged();
    }

    private System.Threading.CancellationTokenSource? _mrDebounceCts;

    private async Task DebouncedMrSearch()
    {
        _mrDebounceCts?.Cancel();
        _mrDebounceCts = new System.Threading.CancellationTokenSource();
        var token = _mrDebounceCts.Token;
        try
        {
            await Task.Delay(350, token);
            if (!token.IsCancellationRequested) await MrSearchAsync();
        }
        catch (TaskCanceledException) { }
    }
}
