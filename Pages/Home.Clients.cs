using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Components;
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
        await NavigateAsync(() =>
        {
            currentView = "clients";
            _flarial.Downloaded = Flarial.IsDownloaded;
            _oderso.Downloaded  = OderSo.IsDownloaded;
            clientsHasBadge     = false;
        });

        var flarialUpToDate = _flarial.Downloaded ? Flarial.IsUpToDateAsync() : Task.FromResult(false);
        var odersoUpToDate  = _oderso.Downloaded  ? OderSo.IsUpToDateAsync()  : Task.FromResult(false);
        await Task.WhenAll(flarialUpToDate, odersoUpToDate);

        _flarial.UpToDate = flarialUpToDate.Result;
        _oderso.UpToDate  = odersoUpToDate.Result;
        StateHasChanged();
    }

    /// <summary>
    /// Shared download flow for the simple single-file clients (Flarial, OderSo):
    /// flips the card into its downloading state, streams via the service while
    /// surfacing 0–1 progress, then refreshes the up-to-date flag.
    /// </summary>
    private async Task RunClientDownload(
        ClientDownloadState                              state,
        Func<IProgress<double>, CancellationToken, Task> download,
        Func<Task<bool>>                                 isUpToDate,
        string                                           clientName)
    {
        if (state.Downloading) return;
        var token = state.BeginDownload();
        StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(f =>
            { state.Progress = f; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
            await download(prog, token);
            state.Downloaded = true;
            state.UpToDate   = await isUpToDate();
            _ = ShowToast($"{clientName} downloaded!", "success");
        }
        catch (OperationCanceledException) { _ = ShowToast($"{clientName} download cancelled.", "info"); }
        catch (Exception ex) { state.Error = ex.Message; }
        finally { state.EndDownload(); }
        StateHasChanged();
    }

    private Task HandleFlarialDownload() =>
        RunClientDownload(_flarial, Flarial.DownloadAsync, Flarial.IsUpToDateAsync, "Flarial");

    private void HandleFlarialDelete()
    {
        Flarial.Delete();
        _flarial.MarkRemoved();
        StateHasChanged();
    }

    private Task HandleOderSoDownload() =>
        RunClientDownload(_oderso, OderSo.DownloadAsync, OderSo.IsUpToDateAsync, "OderSo");

    private void HandleOderSoDelete()
    {
        OderSo.Delete();
        _oderso.MarkRemoved();
        if (SettingsService.Settings.SelectedClient == "OderSo Client") SelectClient("Latite Client");
        StateHasChanged();
    }

    private string versionsBackTarget = "home";

    private async Task OpenVersions()
    {
        await NavigateAsync(() =>
        {
            if (currentView != "versions") versionsBackTarget = currentView;
            currentView = "versions"; versionsFilter = "";
            versionsHasBadge = false;
        });
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
            b.OpenComponent<InlineProgress>(0);
            b.AddAttribute(1, "Progress", v.Progress);
            b.AddAttribute(2, "OnCancel", EventCallback.Factory.Create(this, () => v.DownloadCts?.Cancel()));
            b.CloseComponent();
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
        if (v.IsDownloading) return;
        v.DownloadCts = new CancellationTokenSource();
        v.IsDownloading = true; v.ErrorMessage = null; v.Progress = 0; StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(pct =>
            { v.Progress = pct; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
            await Launcher.DownloadVersionAsync(v, prog, v.DownloadCts.Token);
            _ = ShowToast(v.DisplayName + " downloaded!", "success");
        }
        catch (OperationCanceledException) { _ = ShowToast(v.DisplayName + " download cancelled.", "info"); }
        catch (Exception ex) { v.ErrorMessage = ex.Message; }
        finally { v.IsDownloading = false; v.Progress = 0; var cts = v.DownloadCts; v.DownloadCts = null; cts?.Dispose(); }
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
        if (state.IsDownloading) return;
        state.DownloadCts = new CancellationTokenSource();
        state.IsDownloading = true; state.Progress = 0; state.Error = "";
        StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(f =>
            { state.Progress = f; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
            await OderSo.DownloadEntryAsync(state.Entry, prog, state.DownloadCts.Token);
            state.IsDownloaded = true;
            _ = ShowToast(state.DisplayName + " downloaded!", "success");
        }
        catch (OperationCanceledException) { _ = ShowToast(state.DisplayName + " download cancelled.", "info"); }
        catch (Exception ex) { state.Error = ex.Message; }
        finally { state.IsDownloading = false; state.Progress = 0; var cts = state.DownloadCts; state.DownloadCts = null; cts?.Dispose(); }
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
            b.OpenComponent<InlineProgress>(0);
            b.AddAttribute(1, "Progress", state.Progress);
            b.AddAttribute(2, "OnCancel", EventCallback.Factory.Create(this, () => state.DownloadCts?.Cancel()));
            b.CloseComponent();
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

    private void OpenAddons() => OpenAddons("");

    private void OpenAddons(string tab) => _ = OpenAddonsAsync(tab);

    // The instance-file scan happens BEFORE the navigation, on a worker thread.
    // It used to run inside the NavigateAsync mutate callback, which executes
    // inside document.startViewTransition — where the browser freezes all
    // rendering until the callback returns. Five directory scans there = the
    // "tab switch hangs" feel on slow disks/CPUs.
    private async Task OpenAddonsAsync(string tab)
    {
        if (IsJava)
        {
            await RefreshJavaInstanceFilesAsync();
            _ = AnalyzeModsAsync();
        }
        await NavigateAsync(() =>
        {
            currentView = "addons";
            if (!string.IsNullOrEmpty(tab)) javaModsTab = tab;
            if (!cfHasSearched && CurseForge.IsAvailable)
                _ = CfSearchAsync();
        });
    }

    private void RefreshJavaInstanceFiles()
    {
        javaMods = JavaInstances.ListFiles("mods").ToList();
        javaScreenshots = JavaInstances.ListFiles("screenshots").ToList();
        javaResourcePacks = JavaInstances.ListFiles("resourcepacks").ToList();
        javaShaderPacks = JavaInstances.ListFiles("shaderpacks").ToList();
        javaSchematics = JavaInstances.ListFiles("schematics").Concat(JavaInstances.ListFiles("litematica")).ToList();
    }

    // Async variant for navigation paths: scans on a worker thread, then swaps
    // the lists in on the UI thread so a concurrent render never sees a torn set.
    private async Task RefreshJavaInstanceFilesAsync()
    {
        var (mods, shots, packs, shaders, schematics) = await Task.Run(() => (
            JavaInstances.ListFiles("mods").ToList(),
            JavaInstances.ListFiles("screenshots").ToList(),
            JavaInstances.ListFiles("resourcepacks").ToList(),
            JavaInstances.ListFiles("shaderpacks").ToList(),
            JavaInstances.ListFiles("schematics").Concat(JavaInstances.ListFiles("litematica")).ToList()));
        javaMods = mods;
        javaScreenshots = shots;
        javaResourcePacks = packs;
        javaShaderPacks = shaders;
        javaSchematics = schematics;
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

    // ── Instance manager (profile panel) ─────────────────────────
    private string _renamingInstanceId = "";
    private string _renameInstanceValue = "";
    private string _confirmDeleteInstanceId = "";

    private void SwitchInstance(string id)
    {
        JavaInstances.SetActive(id);
        javaVersionsList.Clear();
        LoadInstanceRam();
        RefreshJavaInstanceFiles();
        _ = ShowToast("Instance switched.", "info");
        StateHasChanged();
    }

    private void NewJavaInstance()
    {
        var instance = JavaInstances.Create("New Instance");
        JavaInstances.SetActive(instance.Id);
        javaVersionsList.Clear();
        LoadInstanceRam();
        RefreshJavaInstanceFiles();
        _ = ShowToast("Instance created.", "success");
        StateHasChanged();
    }

    private async Task CloneJavaInstance(string id)
    {
        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var copy = await Task.Run(() => JavaInstances.Duplicate(id));
            if (copy != null) _ = ShowToast($"Cloned to \"{copy.Name}\".", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { javaUtilityBusy = false; StateHasChanged(); }
    }

    private void BeginRenameInstance(string id, string current)
    {
        _renamingInstanceId = id;
        _renameInstanceValue = current;
        StateHasChanged();
    }

    private void OnRenameInstanceInput(ChangeEventArgs e) => _renameInstanceValue = e.Value?.ToString() ?? "";

    private void OnRenameInstanceKey(Microsoft.AspNetCore.Components.Web.KeyboardEventArgs e)
    {
        if (e.Key == "Enter") CommitRenameInstance();
    }

    private void CommitRenameInstance()
    {
        if (!string.IsNullOrWhiteSpace(_renamingInstanceId) && !string.IsNullOrWhiteSpace(_renameInstanceValue))
            JavaInstances.Rename(_renamingInstanceId, _renameInstanceValue);
        _renamingInstanceId = "";
        _ = ShowToast("Instance renamed.", "success");
        StateHasChanged();
    }

    private void ConfirmDeleteInstance(string id) { _confirmDeleteInstanceId = id; StateHasChanged(); }
    private void CancelDeleteInstance()           { _confirmDeleteInstanceId = ""; StateHasChanged(); }

    private void DeleteJavaInstance(string id)
    {
        if (JavaInstances.Delete(id))
        {
            javaVersionsList.Clear();
            LoadInstanceRam();
            RefreshJavaInstanceFiles();
            _ = ShowToast("Instance deleted.", "info");
        }
        else
        {
            _ = ShowToast("Can't delete the last instance.", "error");
        }
        _confirmDeleteInstanceId = "";
        StateHasChanged();
    }

    private async Task ImportOfficialMinecraft()
    {
        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var instance = await JavaInstances.ImportOfficialMinecraftAsync();
            JavaInstances.SetActive(instance.Id);
            javaVersionsList.Clear();
            RefreshJavaInstanceFiles();
            _ = ShowToast("Imported .minecraft into a new instance.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { javaUtilityBusy = false; StateHasChanged(); }
    }

    private async Task ImportInstanceZip()
    {
        var path = await Task.Run(() =>
        {
            string? result = null;
            var thread = new System.Threading.Thread(() =>
            {
                var dlg = new Microsoft.Win32.OpenFileDialog
                {
                    Title  = "Import instance / modpack",
                    Filter = "Modpacks & instances|*.zip;*.mrpack|All files|*.*"
                };
                if (dlg.ShowDialog() == true) result = dlg.FileName;
            });
            thread.SetApartmentState(System.Threading.ApartmentState.STA);
            thread.Start();
            thread.Join();
            return result;
        });
        if (string.IsNullOrEmpty(path)) return;

        javaUtilityBusy = true; StateHasChanged();
        try
        {
            var ext = System.IO.Path.GetExtension(path).ToLowerInvariant();
            Models.JavaInstance instance;
            if (ext == ".mrpack")
                instance = await Modpacks.InstallMrpackFileAsync(path, System.IO.Path.GetFileNameWithoutExtension(path), null, default);
            else
                instance = await JavaInstances.ImportModpackAsync(path);
            JavaInstances.SetActive(instance.Id);
            javaVersionsList.Clear();
            RefreshJavaInstanceFiles();
            _ = ShowToast("Instance imported.", "success");
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

    private void OpenSettingsCurseForge() => OpenSettings("curseforge");

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

            var prog = new ThrottledProgress(new Progress<double>(f =>
            { cfDownloadProgress = f; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
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

            var prog = new ThrottledProgress(new Progress<double>(f =>
            { mrDownloadProgress = f; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
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
