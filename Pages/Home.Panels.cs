using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Components;
using GlacierLauncher.Models;
using GlacierLauncher.Services;
using Microsoft.AspNetCore.Components;
using Microsoft.JSInterop;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private void OpenCredits() => _ = NavigateAsync(() => currentView = "credits");

    private bool   serverModalOpen   = false;
    private SavedServer? editingServer = null;
    private string serverInputName    = "";
    private string serverInputAddress = "";
    private int    serverInputPort    = 19132;
    private string serverInputError   = "";

    private static readonly SavedServer[] _serverSuggestions =
    {
        new() { Name = "Hive",          Address = "geo.hivebedrock.network", Port = 19132, Icon = "fa-solid fa-hexagon-nodes", IconColor = "#f5a623" },
        new() { Name = "CubeCraft",     Address = "play.cubecraft.net",      Port = 19132, Icon = "fa-solid fa-cube",       IconColor = "#4a90e2" },
        new() { Name = "Mineplex",      Address = "mco.mineplex.com",        Port = 19132, Icon = "fa-solid fa-tower-cell", IconColor = "#e94e4e" },
        new() { Name = "Lifeboat",      Address = "play.lbsg.net",           Port = 19132, Icon = "fa-solid fa-life-ring",  IconColor = "#00bcd4" },
        new() { Name = "Galaxite",      Address = "play.galaxite.net",       Port = 19132, Icon = "fa-solid fa-star",       IconColor = "#9b51e0" },
    };

    // Live status keyed by "address:port". A present-but-null value means a ping
    // is in flight; an absent key means we haven't started one yet.
    private readonly Dictionary<string, ServerStatus?> _serverStatus = new();
    private System.Threading.CancellationTokenSource? _serverPingCts;

    private void OpenServers()
    {
        _ = NavigateAsync(() =>
        {
            currentView = "servers";
            _ = PingAllServersAsync();
        });
    }

    private static string ServerKey(SavedServer s) => $"{s.Address}:{s.Port}";

    // Inline live-status badge for a server row: "Pinging…" → players/latency, or "Offline".
    private RenderFragment RenderServerStatus(SavedServer s) => builder =>
    {
        if (!_serverStatus.TryGetValue(ServerKey(s), out var st)) return;   // not started yet

        if (st == null)
        {
            builder.OpenElement(0, "span");
            builder.AddAttribute(1, "class", "server-ping checking");
            builder.AddContent(2, "Pinging…");
            builder.CloseElement();
            return;
        }

        builder.OpenElement(10, "span");
        builder.AddAttribute(11, "class", st.Online ? "server-ping online" : "server-ping offline");
        builder.OpenElement(12, "span");
        builder.AddAttribute(13, "class", "ping-dot");
        builder.CloseElement();
        builder.AddContent(14, st.Online
            ? $"{st.PlayersOnline:N0}/{st.PlayersMax:N0} · {st.LatencyMs} ms"
            : "Offline");
        builder.CloseElement();
    };

    /// <summary>Pings every saved + suggested server in parallel, refreshing the UI as each returns.</summary>
    private async Task PingAllServersAsync()
    {
        _serverPingCts?.Cancel();
        _serverPingCts = new System.Threading.CancellationTokenSource();
        var token = _serverPingCts.Token;

        var targets = SettingsService.Settings.SavedServers
            .Concat(_serverSuggestions)
            .GroupBy(ServerKey)
            .Select(g => g.First())
            .ToList();

        foreach (var s in targets) _serverStatus[ServerKey(s)] = null;   // mark "pinging" (UI thread)
        StateHasChanged();

        await Task.WhenAll(targets.Select(async s =>
        {
            var status = await ServerPingService.PingAsync(s.Address, s.Port);
            if (token.IsCancellationRequested) return;
            await InvokeAsync(() =>                                       // marshal dict write to UI thread
            {
                _serverStatus[ServerKey(s)] = status;
                StateHasChanged();
            });
        }));
    }

    private void OpenAddServerModal()
    {
        editingServer       = null;
        serverInputName     = "";
        serverInputAddress  = "";
        serverInputPort     = 19132;
        serverInputError    = "";
        serverModalOpen     = true;
        StateHasChanged();
    }

    private void OpenEditServerModal(SavedServer s)
    {
        editingServer       = s;
        serverInputName     = s.Name;
        serverInputAddress  = s.Address;
        serverInputPort     = s.Port;
        serverInputError    = "";
        serverModalOpen     = true;
        StateHasChanged();
    }

    private void CloseServerModal()
    {
        serverModalOpen = false;
        editingServer   = null;
        StateHasChanged();
    }

    private void SaveServerModal()
    {
        if (string.IsNullOrWhiteSpace(serverInputAddress))
        {
            serverInputError = "Address is required.";
            return;
        }
        if (serverInputPort <= 0 || serverInputPort > 65535)
        {
            serverInputError = "Port must be between 1 and 65535.";
            return;
        }

        var name = string.IsNullOrWhiteSpace(serverInputName) ? serverInputAddress : serverInputName.Trim();

        if (editingServer == null)
        {
            SettingsService.Settings.SavedServers.Add(new SavedServer
            {
                Name    = name,
                Address = serverInputAddress.Trim(),
                Port    = serverInputPort
            });
        }
        else
        {
            editingServer.Name    = name;
            editingServer.Address = serverInputAddress.Trim();
            editingServer.Port    = serverInputPort;
        }
        SettingsService.Save();
        CloseServerModal();
        _ = ShowToast(editingServer == null ? "Server saved" : "Server updated", "success");
    }

    private void DeleteServer(SavedServer s)
    {
        SettingsService.Settings.SavedServers.Remove(s);
        SettingsService.Save();
        StateHasChanged();
    }

    private void SaveSuggestedServer(SavedServer s)
    {
        if (SettingsService.Settings.SavedServers.Any(x =>
                string.Equals(x.Address, s.Address, StringComparison.OrdinalIgnoreCase) && x.Port == s.Port))
        {
            _ = ShowToast(s.Name + " is already saved", "info");
            return;
        }
        SettingsService.Settings.SavedServers.Add(new SavedServer
        {
            Name      = s.Name,
            Address   = s.Address,
            Port      = s.Port,
            Icon      = s.Icon,
            IconColor = s.IconColor
        });
        SettingsService.Save();
        _ = ShowToast(s.Name + " saved", "success");
        StateHasChanged();
    }

    private bool IsSuggestionUnsaved(SavedServer s) =>
        !SettingsService.Settings.SavedServers.Any(x =>
            string.Equals(x.Address, s.Address, StringComparison.OrdinalIgnoreCase) && x.Port == s.Port);

    private static string GetServerIconClass(SavedServer s) =>
        string.IsNullOrWhiteSpace(s.Icon) ? "fa-solid fa-server" : s.Icon;

    private string GetServerIconStyle(SavedServer s)
    {
        var color = string.IsNullOrWhiteSpace(s.IconColor) ? SettingsService.Settings.AccentColor : s.IconColor;
        return $"background:{color};";
    }

    private async Task CopyServerAddress(SavedServer s)
    {
        await JS.InvokeVoidAsync("copyToClipboard", $"{s.Address}:{s.Port}");
        _ = ShowToast($"Copied {s.Address}:{s.Port}", "info");
    }

    private async Task OpenMcVersions()
    {
        await NavigateAsync(() =>
        {
            currentView = "mcversions";
            mcVersionsFilter = "";
        });
        if (mcVersionsList.Count == 0) await LoadMcVersionsAsync();
    }

    private async Task RefreshMcVersionsAsync() => await LoadMcVersionsAsync();

    private async Task LoadMcVersionsAsync()
    {
        isLoadingMcVersions = true; mcVersionsError = null; StateHasChanged();
        mcVersionsList = await VanillaVersions.GetVersionsAsync();
        mcVersionsError = VanillaVersions.LastError;
        isLoadingMcVersions = false; StateHasChanged();
    }

    private async Task HandleMcVersionDownload(VanillaVersion v)
    {
        if (v.IsDownloading) return;
        v.DownloadCts = new CancellationTokenSource();
        v.IsDownloading = true; v.ErrorMessage = null; v.Progress = 0; StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(pct =>
            { v.Progress = pct; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
            await VanillaVersions.DownloadVersionAsync(v, prog, v.DownloadCts.Token);
            _ = ShowToast($"Minecraft {v.Version} downloaded!", "success");
        }
        catch (OperationCanceledException) { _ = ShowToast($"Minecraft {v.Version} download cancelled.", "info"); }
        catch (Exception ex) { v.ErrorMessage = ex.Message; }
        finally { v.IsDownloading = false; v.Progress = 0; var cts = v.DownloadCts; v.DownloadCts = null; cts?.Dispose(); }
        StateHasChanged();
    }

    private async Task HandleMcVersionSwitch(VanillaVersion v)
    {
        v.IsSwitching = true; v.ErrorMessage = null; StateHasChanged();
        try
        {
            var result = await VanillaVersions.SwitchVersionAsync(v);
            if (!string.IsNullOrEmpty(result))
            {
                v.ErrorMessage = result;
                _ = ShowToast(result, "error");
            }
            else
            {
                foreach (var mv in mcVersionsList) mv.IsActive = (mv.Version == v.Version);
                _ = ShowToast($"Switched to Minecraft {v.Version}!", "success");
            }
        }
        catch (Exception ex) { v.ErrorMessage = ex.Message; _ = ShowToast(ex.Message, "error"); }
        finally { v.IsSwitching = false; }
        StateHasChanged();
    }

    private void HandleMcVersionDelete(VanillaVersion v)
    {
        VanillaVersions.DeleteVersion(v);
        if (v.IsActive)
            foreach (var mv in mcVersionsList) mv.IsActive = false;
        StateHasChanged();
    }

    private RenderFragment McVersionActions(VanillaVersion v) => b =>
    {
        if (v.IsDownloading)
        {
            b.OpenComponent<InlineProgress>(0);
            b.AddAttribute(1, "Progress", v.Progress);
            b.AddAttribute(2, "OnCancel", EventCallback.Factory.Create(this, () => v.DownloadCts?.Cancel()));
            b.CloseComponent();
        }
        else if (v.IsSwitching)
        {
            b.OpenElement(0, "div");
            b.AddAttribute(1, "class", "version-actions");
            b.OpenElement(2, "button");
            b.AddAttribute(3, "class", "icon-btn");
            b.AddAttribute(4, "disabled", true);
            b.OpenElement(5, "span");
            b.AddAttribute(6, "class", "spinner");
            b.CloseElement();
            b.CloseElement();
            b.CloseElement();
        }
        else
        {
            b.OpenElement(20, "div");
            b.AddAttribute(21, "class", "version-actions");

            if (v.IsDownloaded)
            {
                b.OpenElement(30, "button");
                b.AddAttribute(31, "class", "icon-btn icon-btn-ghost");
                b.AddAttribute(32, "title", "Delete");
                b.AddAttribute(33, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, () => HandleMcVersionDelete(v)));
                b.OpenElement(34, "i");
                b.AddAttribute(35, "class", "fa-solid fa-trash");
                b.CloseElement();
                b.CloseElement();

                if (!v.IsActive)
                {
                    b.OpenElement(40, "button");
                    b.AddAttribute(41, "class", "icon-btn mcv-switch-btn");
                    b.AddAttribute(42, "title", "Switch to this version");
                    b.AddAttribute(43, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, async () => await HandleMcVersionSwitch(v)));
                    b.OpenElement(44, "i");
                    b.AddAttribute(45, "class", "fa-solid fa-right-left");
                    b.CloseElement();
                    b.CloseElement();
                }
            }
            else
            {
                b.OpenElement(50, "button");
                b.AddAttribute(51, "class", "icon-btn");
                b.AddAttribute(52, "title", "Download");
                b.AddAttribute(53, "onclick", Microsoft.AspNetCore.Components.EventCallback.Factory.Create(this, async () => await HandleMcVersionDownload(v)));
                b.OpenElement(54, "i");
                b.AddAttribute(55, "class", "fa-solid fa-download");
                b.CloseElement();
                b.CloseElement();
            }

            b.CloseElement();
        }
    };

    private async Task InstallFromStoreAsync(bool preview)
    {
        if (storeInstallBusy) return;
        storeInstallBusy   = true;
        storeInstallPct    = 0;
        storeInstallStage  = preview ? "Installing Minecraft Preview…" : "Installing Minecraft…";
        storeInstallDetail = "Talking to the Microsoft Store…";
        StateHasChanged();

        var product = preview
            ? StoreInstallService.Product.MinecraftPreview
            : StoreInstallService.Product.MinecraftRelease;

        var progress = new Progress<StoreInstallService.InstallProgress>(p =>
        {
            storeInstallPct    = p.Percent;
            storeInstallStage  = $"{p.Stage} {p.ProductName}";
            storeInstallDetail = p.BytesTotal > 0
                ? $"{FormatSize(p.BytesDownloaded)} of {FormatSize(p.BytesTotal)}"
                : "Preparing…";
            InvokeAsync(StateHasChanged);
        });

        try
        {
            var finalState = await StoreInstall.InstallAsync(product, progress);

            if (finalState == Windows.ApplicationModel.Store.Preview.InstallControl.AppInstallState.Completed)
            {
                _ = ShowToast($"{product.DisplayName} installed from the Store.", "success");
                if (!string.IsNullOrEmpty(SettingsService.Settings.ActiveVanillaVersion))
                {
                    SettingsService.Settings.ActiveVanillaVersion = "";
                    SettingsService.Save();
                    foreach (var mv in mcVersionsList) mv.IsActive = false;
                }
            }
            else
            {
                _ = ShowToast($"Install ended: {finalState}", "info");
            }
        }
        catch (Exception ex)
        {
            _ = ShowToast($"Store install failed: {ex.Message}. You can still sideload a build below.", "error");
        }
        finally
        {
            storeInstallBusy   = false;
            storeInstallPct    = 0;
            storeInstallStage  = "";
            storeInstallDetail = "";
            StateHasChanged();
        }
    }

    private void CancelStoreInstall()
    {
        StoreInstall.Cancel();
    }

    private async Task OpenJavaVersions()
    {
        await NavigateAsync(() =>
        {
            currentView = "javaversions";
            javaVersionsFilter = "";
        });
        if (javaVersionsList.Count == 0) await LoadJavaVersionsAsync();
    }

    private async Task RefreshJavaVersionsAsync() => await LoadJavaVersionsAsync();

    private async Task LoadJavaVersionsAsync()
    {
        isLoadingJavaVersions = true; javaVersionsError = null; StateHasChanged();
        try
        {
            var fromManifest = await JavaVersions.GetVersionsAsync();
            var known        = fromManifest.Select(v => v.Id).ToList();
            var custom       = JavaVersions.ScanCustomInstalledVersions(known);
            javaVersionsList = custom.Concat(fromManifest).ToList();
            javaVersionsError = JavaVersions.LastError;
        }
        catch (Exception ex)
        {
            javaVersionsError = ex.Message;
        }
        finally
        {
            isLoadingJavaVersions = false;
            StateHasChanged();
        }
    }

    private void ToggleJavaSnapshots()
    {
        SettingsService.Settings.JavaShowSnapshots = !SettingsService.Settings.JavaShowSnapshots;
        SettingsService.Save();
        StateHasChanged();
    }

    private void ToggleJavaHistorical()
    {
        SettingsService.Settings.JavaShowHistorical = !SettingsService.Settings.JavaShowHistorical;
        SettingsService.Save();
        StateHasChanged();
    }

    private void SetActiveJavaVersion(JavaVersion v)
    {
        foreach (var jv in javaVersionsList) jv.IsActive = false;
        v.IsActive = true;
        SettingsService.Settings.JavaActiveVersion = v.Id;
        SettingsService.Save();
        _ = ShowToast($"Default Java version: {v.Id}", "success");
        StateHasChanged();
    }

    private void OnJavaRamChanged(ChangeEventArgs e)
    {
        if (int.TryParse(e.Value?.ToString(), out var v))
        {
            SettingsService.Settings.JavaMaxRamMb = Math.Clamp(v, 512, 16384);
            if (SettingsService.Settings.JavaMinRamMb > SettingsService.Settings.JavaMaxRamMb)
                SettingsService.Settings.JavaMinRamMb = SettingsService.Settings.JavaMaxRamMb;
            SettingsService.Save();
        }
    }

    private void OnJavaMinRamChanged(ChangeEventArgs e)
    {
        if (int.TryParse(e.Value?.ToString(), out var v))
        {
            SettingsService.Settings.JavaMinRamMb = Math.Clamp(v, 256, SettingsService.Settings.JavaMaxRamMb);
            SettingsService.Save();
        }
    }

    private void OnJavaWindowWidthChanged(ChangeEventArgs e)
    {
        if (int.TryParse(e.Value?.ToString(), out var v))
        {
            SettingsService.Settings.JavaWindowWidth = Math.Clamp(v, 320, 7680);
            SettingsService.Save();
        }
    }

    private void OnJavaWindowHeightChanged(ChangeEventArgs e)
    {
        if (int.TryParse(e.Value?.ToString(), out var v))
        {
            SettingsService.Settings.JavaWindowHeight = Math.Clamp(v, 240, 4320);
            SettingsService.Save();
        }
    }

    private void OnJavaServerAddressChanged(ChangeEventArgs e)
    {
        SettingsService.Settings.JavaServerAddress = (e.Value?.ToString() ?? "").Trim();
        SettingsService.Save();
    }

    private void OnJavaServerPortChanged(ChangeEventArgs e)
    {
        if (int.TryParse(e.Value?.ToString(), out var v))
        {
            SettingsService.Settings.JavaServerPort = Math.Clamp(v, 1, 65535);
            SettingsService.Save();
        }
    }

    private void OpenJavaClients() => _ = OpenJavaClientsAsync();

    private async Task OpenJavaClientsAsync()
    {
        // Detect() walks the filesystem for Lunar/Badlion installs — worker
        // thread, so the click never stalls the UI before the panel opens.
        await Task.Run(LunarBadlion.Detect);
        if (string.IsNullOrEmpty(lunarSelectedVersion))
            lunarSelectedVersion = LunarBadlion.LunarInstalledVersions.FirstOrDefault() ?? "";
        await NavigateAsync(() =>
        {
            currentView = "javaclients";
            _ = EnsureGlacierManifestAsync();
        });
    }

    private void RefreshJavaClients()
    {
        LunarBadlion.Detect();
        if (!LunarBadlion.LunarInstalledVersions.Contains(lunarSelectedVersion))
            lunarSelectedVersion = LunarBadlion.LunarInstalledVersions.FirstOrDefault() ?? "";
        StateHasChanged();
        _ = EnsureGlacierManifestAsync(force: true);
    }

    private string lunarSelectedVersion = "";

    private void LaunchLunar()
    {
        try
        {
            LunarBadlion.LaunchLunar();
            Discord.SetJavaInGamePresence(lunarSelectedVersion, "Lunar");
            AddToRecentlyLaunched("Java Lunar");
            _ = ShowToast("Lunar Client launching…", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void LaunchLunarDirect()
    {
        var ver = !string.IsNullOrEmpty(lunarSelectedVersion)
            ? lunarSelectedVersion
            : LunarBadlion.LunarInstalledVersions.FirstOrDefault() ?? "";
        try
        {
            LunarBadlion.LaunchLunar(ver);
            Discord.SetJavaInGamePresence(ver, "Lunar");
            AddToRecentlyLaunched("Java Lunar " + ver);
            _ = ShowToast(
                string.IsNullOrEmpty(ver)
                    ? "Lunar Client launching…"
                    : $"Lunar {ver} launching (skip-splash)…",
                "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void LaunchBadlion()
    {
        try
        {
            LunarBadlion.LaunchBadlion();
            Discord.SetJavaInGamePresence("", "Badlion");
            AddToRecentlyLaunched("Java Badlion");
            _ = ShowToast("Badlion Client launching…", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private void OnJavaMinecraftDirChanged(ChangeEventArgs e)
    {
        var v = (e.Value?.ToString() ?? "").Trim().Trim('"');
        SettingsService.Settings.JavaMinecraftDir = v;
        SettingsService.Save();
        javaVersionsList.Clear();
        StateHasChanged();
    }

    private void OnJavaRuntimeChanged(ChangeEventArgs e)
    {
        var v = (e.Value?.ToString() ?? "").Trim().Trim('"');
        SettingsService.Settings.JavaRuntimePath = v;
        SettingsService.Save();
        StateHasChanged();
    }

    private void OnJavaJvmArgsChanged(ChangeEventArgs e)
    {
        SettingsService.Settings.JavaCustomJvmArgs = (e.Value?.ToString() ?? "").Trim();
        SettingsService.Save();
        StateHasChanged();
    }

    private void OpenJavaMinecraftFolder()
    {
        try
        {
            var dir = JavaVersions.MinecraftDir;
            System.IO.Directory.CreateDirectory(dir);
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = dir,
                UseShellExecute = true
            });
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private string? LatestJavaCrashFile()
    {
        try
        {
            var dir = System.IO.Path.Combine(JavaVersions.MinecraftDir, "crash-reports");
            if (!System.IO.Directory.Exists(dir)) return null;
            return System.IO.Directory.EnumerateFiles(dir, "crash-*.txt")
                .OrderByDescending(System.IO.File.GetLastWriteTimeUtc)
                .FirstOrDefault();
        }
        catch { return null; }
    }

    private string LatestJavaCrashLabel()
    {
        var p = LatestJavaCrashFile();
        if (p == null) return "No crash reports yet";
        try
        {
            var when = System.IO.File.GetLastWriteTime(p);
            return $"{System.IO.Path.GetFileName(p)} · {FormatRelativeTime(when.ToUniversalTime().ToString("o"))}";
        }
        catch { return System.IO.Path.GetFileName(p) ?? ""; }
    }

    private void OpenLatestJavaCrash()
    {
        var p = LatestJavaCrashFile();
        if (p == null) { _ = ShowToast("No crash reports found.", "info"); return; }
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = p,
                UseShellExecute = true
            });
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
    }

    private async Task HandleJavaInstall(JavaVersion v)
    {
        if (v.IsInstalling) return;
        v.IsInstalling = true; v.InstallPercent = 0; v.InstallStage = "Starting…"; v.ErrorMessage = null;
        StateHasChanged();
        try
        {
            var lastTick = 0L;
            await JavaInstaller.InstallAsync(v, p =>
            {
                v.InstallPercent = p.Percent;
                v.InstallStage   = p.Stage;
                var now = Environment.TickCount64;
                if (now - lastTick >= 80 || p.Percent >= 100)
                {
                    lastTick = now;
                    InvokeAsync(StateHasChanged);
                }
            });
            v.IsInstalled = true;
            v.HasJar      = true;
            _ = ShowToast($"Minecraft Java {v.Id} installed!", "success");
        }
        catch (Exception ex)
        {
            v.ErrorMessage = ex.Message;
            _ = ShowToast(ex.Message, "error");
        }
        finally
        {
            v.IsInstalling = false;
            _ = LoadJavaVersionsAsync();
            StateHasChanged();
        }
    }

    private async Task HandleJavaLaunch(JavaVersion v)
    {
        if (v.IsLaunching) return;
        v.IsLaunching = true; v.ErrorMessage = null; StateHasChanged();
        try
        {
            await JavaLauncher.LaunchAsync(v.Id, OnJavaLaunchProgress);
            _ = ShowToast($"Minecraft Java {v.Id} launching…", "success");
            AddToRecentlyLaunched("Java " + v.Id);
            Discord.SetJavaInGamePresence(v.Id);
            if (SettingsService.Settings.CloseAfterLaunch) await CloseWindow();
        }
        catch (Exception ex)
        {
            v.ErrorMessage = ex.Message;
            _ = ShowToast(ex.Message, "error");
        }
        finally
        {
            v.IsLaunching = false;
            ClearDownloadBar();
            StateHasChanged();
        }
    }

    private async Task RunStartupChecksAsync()
    {
        try
        {
            var skipped = SettingsService.Settings.SkippedLauncherVersion;

            var launcherTask = AutoUpdate.CheckLauncherUpdateAsync();
            var flarialTask  = AutoUpdate.IsFlarialUpdateAvailableAsync();
            var odersoTask   = AutoUpdate.IsOderSoUpdateAvailableAsync();
            var sessionTask  = Xbox.ValidateSessionAsync();

            await Task.WhenAll(launcherTask, flarialTask, odersoTask, sessionTask);

            var info          = launcherTask.Result;
            var flarialUpdate = flarialTask.Result;
            var odersoUpdate  = odersoTask.Result;

            SettingsService.Settings.LastUpdateCheck = DateTime.UtcNow.ToString("o");
            SettingsService.Save();

            await InvokeAsync(() =>
            {
                lastUpdateCheckLabel = "Just now";

                if (info != null && info.Tag != skipped)
                {
                    launcherUpdateInfo      = info;
                    launcherUpdateAvailable = true;
                    Notify.Add("Launcher update available", $"Glacier Launcher {info.Tag} is ready to install.", "info");
                }

                if (flarialUpdate || odersoUpdate)
                {
                    clientsHasBadge = true;
                    var names = string.Join(" and ", new[] { flarialUpdate ? "Flarial" : null, odersoUpdate ? "OderSo" : null }.Where(n => n != null));
                    Notify.Add("Client update available", $"{names} client has a new version ready to download.", "info");
                }

                StateHasChanged();
            });
        }
        catch { }
    }

    private async Task ManualUpdateCheck()
    {
        checkingUpdatesManual = true;
        StateHasChanged();

        var skipped      = SettingsService.Settings.SkippedLauncherVersion;
        var launcherTask = AutoUpdate.CheckLauncherUpdateAsync();
        var flarialTask  = AutoUpdate.IsFlarialUpdateAvailableAsync();
        var odersoTask   = AutoUpdate.IsOderSoUpdateAvailableAsync();
        await Task.WhenAll(launcherTask, flarialTask, odersoTask);

        var info          = launcherTask.Result;
        var flarialUpdate = flarialTask.Result;
        var odersoUpdate  = odersoTask.Result;

        SettingsService.Settings.LastUpdateCheck = DateTime.UtcNow.ToString("o");
        SettingsService.Save();

        checkingUpdatesManual = false;
        lastUpdateCheckLabel  = "Just now";

        if (info != null && info.Tag != skipped)
        {
            launcherUpdateInfo      = info;
            launcherUpdateAvailable = true;
            OpenUpdateModal();
        }

        if (flarialUpdate || odersoUpdate)
            clientsHasBadge = true;

        var checkStatus = AutoUpdate.LastCheckStatus;
        if (!string.IsNullOrEmpty(checkStatus))
            _ = ShowToast(checkStatus, "info");
        else if (info == null && !flarialUpdate && !odersoUpdate)
            _ = ShowToast("Everything is up to date!", "success");
        else if (info == null && (flarialUpdate || odersoUpdate))
            _ = ShowToast("Client update available in Clients panel.", "info");

        StateHasChanged();
    }

    private void OpenUpdateModal()  { launcherUpdateModalOpen = true;  StateHasChanged(); }
    private void CloseUpdateModal() { launcherUpdateModalOpen = false; StateHasChanged(); }

    private void SkipLauncherUpdate()
    {
        if (launcherUpdateInfo == null) return;
        SettingsService.Settings.SkippedLauncherVersion = launcherUpdateInfo.Tag;
        SettingsService.Save();
        launcherUpdateAvailable = false;
        launcherUpdateModalOpen = false;
        _ = ShowToast("This version will be skipped.", "info");
        StateHasChanged();
    }

    private async Task ApplyLauncherUpdate()
    {
        if (launcherUpdateInfo == null) return;
        launcherUpdating       = true;
        launcherUpdateProgress = 0;
        StateHasChanged();
        try
        {
            var prog = new ThrottledProgress(new Progress<double>(f =>
            { launcherUpdateProgress = f; InvokeAsync(StateHasChanged); }), completeValue: 1.0, minDelta: 0.01);
            await AutoUpdate.ApplyUpdateAsync(launcherUpdateInfo, prog);
        }
        catch (Exception ex)
        {
            launcherUpdating = false;
            _ = ShowToast("Update failed: " + ex.Message, "error");
            StateHasChanged();
        }
    }

    private void OpenDiscordModal()
    {
        discordUsernameInput = SettingsService.Settings.DiscordUsername;
        discordModalOpen = true;
        StateHasChanged();
    }

    private void CloseDiscordModal() { discordModalOpen = false; StateHasChanged(); }

    private async Task OpenDiscordOAuth()
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = "https://discord.com/oauth2/authorize?client_id=1482726422094024779&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A5000%2Fcallback&scope=identify",
                UseShellExecute = true
            });

            using var listener = new System.Net.HttpListener();
            listener.Prefixes.Add("http://localhost:5000/callback/");
            listener.Start();

            var context = await listener.GetContextAsync();
            var code = context.Request.QueryString["code"];

            var responseBytes = System.Text.Encoding.UTF8.GetBytes("Success");
            context.Response.OutputStream.Write(responseBytes, 0, responseBytes.Length);
            context.Response.Close();
            listener.Stop();

            if (!string.IsNullOrEmpty(code))
            {
                using var http = new System.Net.Http.HttpClient();
                var tokenContent = new System.Net.Http.FormUrlEncodedContent(new[]
                {
                    new System.Collections.Generic.KeyValuePair<string, string>("client_id", "1482726422094024779"),
                    new System.Collections.Generic.KeyValuePair<string, string>("client_secret", "zwvBIpvo18qHSLUzlvG1AKfJKAkMLujc"),
                    new System.Collections.Generic.KeyValuePair<string, string>("grant_type", "authorization_code"),
                    new System.Collections.Generic.KeyValuePair<string, string>("code", code),
                    new System.Collections.Generic.KeyValuePair<string, string>("redirect_uri", "http://localhost:5000/callback")
                });

                var tokenResponse = await http.PostAsync("https://discord.com/api/oauth2/token", tokenContent);
                var tokenJson = await tokenResponse.Content.ReadAsStringAsync();
                var tokenDoc = System.Text.Json.JsonDocument.Parse(tokenJson);
                var accessToken = tokenDoc.RootElement.GetProperty("access_token").GetString() ?? "";

                http.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", accessToken);
                var userResponse = await http.GetAsync("https://discord.com/api/users/@me");
                var userJson = await userResponse.Content.ReadAsStringAsync();
                var userDoc = System.Text.Json.JsonDocument.Parse(userJson);

                var userId     = userDoc.RootElement.GetProperty("id").GetString()       ?? "";
                var username   = userDoc.RootElement.GetProperty("username").GetString() ?? "";
                var avatarHash = userDoc.RootElement.GetProperty("avatar").GetString()   ?? "";

                var avatarUrl = !string.IsNullOrEmpty(avatarHash)
                    ? $"https://cdn.discordapp.com/avatars/{userId}/{avatarHash}.png"
                    : "";

                SettingsService.Settings.DiscordLoggedIn = true;
                SettingsService.Settings.DiscordUsername = username;
                SettingsService.Settings.DiscordAvatar   = avatarUrl;
                SettingsService.Settings.DiscordToken    = accessToken;
                displayName   = username;
                displayHandle = "@" + username.ToLower().Replace(" ", "");
                SettingsService.Settings.Username   = displayName;
                SettingsService.Settings.UserHandle = displayHandle;
                SettingsService.Save();
                BuildDefaultSearchResults();
                discordModalOpen = false;
                _ = ShowToast("Signed in as " + displayName, "success");
                StateHasChanged();
            }
        }
        catch { }
    }

    private void SaveDiscordManual()
    {
        if (string.IsNullOrWhiteSpace(discordUsernameInput)) return;
        var name = discordUsernameInput.Trim();
        SettingsService.Settings.DiscordLoggedIn = true;
        SettingsService.Settings.DiscordUsername = name;
        displayName   = name;
        displayHandle = "@" + name.ToLower().Replace(" ", "");
        SettingsService.Settings.Username   = displayName;
        SettingsService.Settings.UserHandle = displayHandle;
        SettingsService.Save();
        BuildDefaultSearchResults();
        discordModalOpen = false;
        _ = ShowToast("Signed in as " + displayName, "success");
        StateHasChanged();
    }

    private void DisconnectDiscord()
    {
        SettingsService.Settings.DiscordLoggedIn = false;
        SettingsService.Settings.DiscordUsername = "";
        SettingsService.Settings.DiscordAvatar   = "";
        SettingsService.Settings.DiscordToken    = "";
        SettingsService.Save();
        discordModalOpen = false;
        _ = ShowToast("Discord disconnected", "info");
        StateHasChanged();
    }

    private void OpenXboxModal() { xboxModalOpen = true; StateHasChanged(); }
    private void CloseXboxModal() { xboxModalOpen = false; StateHasChanged(); }

    private async Task XboxSignIn()
    {
        xboxSigningIn = true; StateHasChanged();
        try
        {
            var success = await Xbox.SignInAsync();
            if (success)
            {
                _ = ShowToast($"Signed in as {Xbox.CurrentProfile?.Gamertag}", "success");
                xboxModalOpen = false;
            }
            else
            {
                _ = ShowToast(Xbox.LastError ?? "Sign-in failed.", "error");
            }
        }
        catch (Exception ex)
        {
            _ = ShowToast(ex.Message, "error");
        }
        finally
        {
            xboxSigningIn = false;
            StateHasChanged();
        }
    }

    private async Task XboxAddAccount()
    {
        xboxSigningIn = true; StateHasChanged();
        try
        {
            var success = await Xbox.SignInAsync(true);
            _ = ShowToast(success ? $"Signed in as {Xbox.CurrentProfile?.Gamertag}" : Xbox.LastError ?? "Sign-in failed.", success ? "success" : "error");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { xboxSigningIn = false; StateHasChanged(); }
    }

    private void SwitchJavaAccount(JavaAccount account)
    {
        if (Xbox.SwitchAccount(account.Id))
            _ = ShowToast("Switched to " + account.MinecraftUsername, "success");
        StateHasChanged();
    }

    private void RemoveJavaAccount(JavaAccount account)
    {
        Xbox.RemoveAccount(account.Id);
        _ = ShowToast("Account removed.", "info");
        StateHasChanged();
    }

    private void XboxSignOut()
    {
        Xbox.SignOut();
        xboxModalOpen = false;
        _ = ShowToast("Xbox account disconnected.", "info");
        StateHasChanged();
    }

    private void OpenLauncherFolder()
    {
        try
        {
            var dir = System.IO.Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                "Glacier Launcher");
            System.IO.Directory.CreateDirectory(dir);
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            { FileName = dir, UseShellExecute = true });
        }
        catch (Exception ex) { _ = ShowToast("Couldn't open folder: " + ex.Message, "error"); }
    }

    private void OpenMinecraftFolder()
    {
        try
        {
            var dir = System.IO.Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "Packages", "Microsoft.MinecraftUWP_8wekyb3d8bbwe", "LocalState",
                "games", "com.mojang");

            if (!System.IO.Directory.Exists(dir))
            {
                _ = ShowToast("Minecraft data folder not found. Is Minecraft installed?", "error");
                return;
            }

            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            { FileName = dir, UseShellExecute = true });
        }
        catch (Exception ex) { _ = ShowToast("Couldn't open folder: " + ex.Message, "error"); }
    }
}
