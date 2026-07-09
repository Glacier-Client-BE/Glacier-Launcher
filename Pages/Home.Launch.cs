using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Models;
using GlacierLauncher.Services;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private System.Threading.CancellationTokenSource? _launchCts;
    private bool _launchForceCancelled = false;

    private async Task HandleLaunch()
    {
        if (IsJava)
        {
            await HandleJavaLaunchFromHome();
            return;
        }

        var sel            = SettingsService.Settings.SelectedClient;
        bool isFlarial     = sel == "Flarial Client";
        bool isOderSo      = sel == "OderSo Client";
        bool isLeviLamina  = sel == "LeviLamina Client";
        bool isCustom      = sel == "Custom DLL";
        bool isVanilla     = sel == "Vanilla";

        if (isLeviLamina && string.IsNullOrEmpty(LeviLamina.FilePath))
        {
            await ShowStatus("LeviLamina isn't downloaded yet. Download it from the Clients tab first.", error: true, clearAfterMs: 8000);
            return;
        }

        var prereq = LaunchDiagnosticsService.CheckPrerequisites();
        if (prereq.Reason != LaunchDiagnosticsService.Reason.Ok)
        {
            await ShowStatus(prereq.Message, error: true, clearAfterMs: 8000);
            _ = ShowToast(prereq.Title + ": " + prereq.Message, "error");
            return;
        }

        if (isCustom && !string.IsNullOrEmpty(customDllPath))
        {
            var dllCheck = LaunchDiagnosticsService.ValidateDll(customDllPath);
            if (dllCheck.Reason != LaunchDiagnosticsService.Reason.Ok)
            {
                await ShowStatus(dllCheck.Message, error: true, clearAfterMs: 8000);
                _ = ShowToast(dllCheck.Title + ": " + dllCheck.Message, "error");
                return;
            }
        }

        isLaunching = true;
        _launchForceCancelled = false;
        _launchCts?.Cancel();
        _launchCts = new System.Threading.CancellationTokenSource();
        var token = _launchCts.Token;
        await ShowStatus(isVanilla ? "Starting Minecraft (vanilla)…" : "Starting Minecraft…", clearAfterMs: 30_000);
        try
        {
            Task launchTask = isVanilla ? Launcher.LaunchVanillaAsync()
                            : (isCustom && !string.IsNullOrEmpty(customDllPath)) ? Launcher.LaunchWithDllAsync(customDllPath)
                            : isOderSo ? Launcher.LaunchWithDllAsync(OderSo.FilePath)
                            : isLeviLamina ? Launcher.LaunchWithDllAsync(LeviLamina.FilePath!)
                            : Launcher.LaunchAsync(useFlarial: isFlarial);

            var done = await Task.WhenAny(launchTask, Task.Delay(System.Threading.Timeout.Infinite, token));
            if (done != launchTask)
                throw new OperationCanceledException();

            await launchTask;

            isLaunching = false;
            var label = isVanilla     ? "Vanilla"
                      : isCustom      ? System.IO.Path.GetFileNameWithoutExtension(customDllPath)
                      : isFlarial     ? "Flarial"
                      : isOderSo      ? "OderSo"
                      : isLeviLamina  ? "LeviLamina"
                      : SettingsService.Settings.LastUsedVersion;
            Discord.SetInGamePresence(label ?? "", sel);
            AddToRecentlyLaunched(label ?? "");

            if (SettingsService.Settings.CloseAfterLaunch) { await CloseWindow(); return; }

            var doneMsg = isVanilla ? "Minecraft launched. Have fun!" : "Injected! Have fun.";
            await ShowStatus(doneMsg, clearAfterMs: 3000);
            _ = ShowToast(isVanilla ? "Minecraft launched!" : "Injection successful!", "success");
        }
        catch (OperationCanceledException)
        {
            isLaunching = false;
            await ShowStatus("Launch cancelled.", error: false, clearAfterMs: 3000);
        }
        catch (Exception ex)
        {
            isLaunching = false;
            if (!_launchForceCancelled)
            {
                var diag = LaunchDiagnosticsService.Classify(ex);
                await ShowStatus(diag.Message, error: true, clearAfterMs: 8000);
                _ = ShowToast(diag.Title + ": " + diag.Message, "error");
            }
        }
    }

    private void OnJavaLaunchProgress(string stage, double pct)
    {
        isDownloading = true;
        downloadStage = stage;
        downloadPct   = pct;
        InvokeAsync(StateHasChanged);
    }

    private void ClearDownloadBar()
    {
        isDownloading = false;
        downloadStage = "";
        downloadPct   = 0;
    }

    private async Task HandleJavaLaunchFromHome()
    {
        var s   = SettingsService.Settings;
        var ver = !string.IsNullOrEmpty(s.JavaActiveVersion) ? s.JavaActiveVersion
                : !string.IsNullOrEmpty(s.JavaLastUsedVersion) ? s.JavaLastUsedVersion
                : "";

        if (string.IsNullOrEmpty(ver))
        {
            await ShowStatus("Pick a Java version first.", clearAfterMs: 3000);
            await OpenJavaVersions();
            return;
        }

        isLaunching = true;
        await ShowStatus($"Starting Minecraft Java {ver}…", clearAfterMs: 30_000);
        try
        {
            await JavaLauncher.LaunchAsync(ver, OnJavaLaunchProgress);
            AddToRecentlyLaunched("Java " + ver);
            Discord.SetJavaInGamePresence(ver);
            await ShowStatus("Minecraft launching!", clearAfterMs: 3000);
            _ = ShowToast($"Minecraft Java {ver} launching!", "success");
            if (SettingsService.Settings.CloseAfterLaunch) { await CloseWindow(); }
        }
        catch (Exception ex)
        {
            await ShowStatus(ex.Message, error: true, clearAfterMs: 8000);
            _ = ShowToast(ex.Message, "error");
        }
        finally
        {
            isLaunching = false;
            ClearDownloadBar();
            StateHasChanged();
        }
    }

    private void ForceCloseLaunch()
    {
        _launchForceCancelled = true;
        _launchCts?.Cancel();
        isLaunching = false;
        try
        {
            foreach (var name in new[] { "Minecraft.Windows", "Minecraft", "MinecraftUWP" })
            {
                foreach (var p in System.Diagnostics.Process.GetProcessesByName(name))
                {
                    try { p.Kill(entireProcessTree: true); } catch { }
                }
            }
        }
        catch { }
        _ = ShowToast("Launch cancelled", "info");
        StateHasChanged();
    }

    private async Task QuickLaunchRecent(string label)
    {
        if (label.StartsWith("Java ", StringComparison.Ordinal))
        {
            var ver = label[5..];
            var jv = javaVersionsList.FirstOrDefault(x => x.Id == ver);
            if (jv != null)
            {
                await HandleJavaLaunch(jv);
                return;
            }
            try
            {
                isLaunching = true; StateHasChanged();
                await JavaLauncher.LaunchAsync(ver, OnJavaLaunchProgress);
                AddToRecentlyLaunched("Java " + ver);
                _ = ShowToast($"Minecraft Java {ver} launching!", "success");
            }
            catch (Exception ex)
            {
                await ShowStatus(ex.Message, error: true, clearAfterMs: 8000);
            }
            finally
            {
                isLaunching = false; ClearDownloadBar(); StateHasChanged();
            }
            return;
        }

        if (label == "Flarial")
        {
            SettingsService.Settings.SelectedClient = "Flarial Client";
            SettingsService.Save();
            await HandleLaunch();
            return;
        }
        if (label == "OderSo")
        {
            SettingsService.Settings.SelectedClient = "OderSo Client";
            SettingsService.Save();
            await HandleLaunch();
            return;
        }
        if (label == "Vanilla")
        {
            SettingsService.Settings.SelectedClient = "Vanilla";
            SettingsService.Save();
            await HandleLaunch();
            return;
        }

        var v = versions.FirstOrDefault(x => x.Tag == label || x.DisplayName == label);
        if (v != null && v.IsDownloaded) { await HandleVersionLaunch(v); return; }

        if (!string.IsNullOrEmpty(customDllPath) &&
            System.IO.Path.GetFileNameWithoutExtension(customDllPath) == label)
        {
            SettingsService.Settings.SelectedClient = "Custom DLL";
            SettingsService.Save();
            await HandleLaunch();
            return;
        }

        await ShowStatus($"'{label}' not found in downloaded versions.", error: true);
    }

    private async Task LaunchServer(SavedServer s)
    {
        if (isLaunching) return;

        var sel           = SettingsService.Settings.SelectedClient;
        bool isFlarial    = sel == "Flarial Client";
        bool isOderSo     = sel == "OderSo Client";
        bool isLeviLamina = sel == "LeviLamina Client";
        bool isCustom     = sel == "Custom DLL";
        bool isVanilla    = sel == "Vanilla";

        string? dllPath = null;
        try
        {
            if (isCustom && !string.IsNullOrEmpty(customDllPath)) dllPath = customDllPath;
            else if (isOderSo)                                    dllPath = OderSo.FilePath;
            else if (isFlarial)                                   dllPath = FlarialService.FilePath;
            else if (isLeviLamina)                                dllPath = LeviLamina.FilePath;
            else if (!isVanilla)
            {
                var tag = SettingsService.Settings.LastUsedVersion;
                if (!string.IsNullOrEmpty(tag))
                {
                    var p = GameLauncher.GetDllPath(tag);
                    dllPath = System.IO.File.Exists(p) ? p : null;
                }
            }
        }
        catch { dllPath = null; }

        if (!isVanilla && !string.IsNullOrEmpty(dllPath) && !System.IO.File.Exists(dllPath))
            dllPath = null;

        isLaunching = true;
        _launchForceCancelled = false;
        _launchCts?.Cancel();
        _launchCts = new System.Threading.CancellationTokenSource();
        var token = _launchCts.Token;
        await ShowStatus($"Launching → {s.Name}…", clearAfterMs: 30_000);
        try
        {
            var launchTask = Launcher.LaunchServerAsync(dllPath, s.Name, s.Address, s.Port);
            var done = await Task.WhenAny(launchTask, Task.Delay(System.Threading.Timeout.Infinite, token));
            if (done != launchTask) throw new OperationCanceledException();
            await launchTask;

            isLaunching = false;
            Discord.SetInGamePresence(s.Name, sel);
            AddToRecentlyLaunched(s.Name);

            if (SettingsService.Settings.CloseAfterLaunch) { await CloseWindow(); return; }

            await ShowStatus($"Connected to {s.Name}!", clearAfterMs: 3000);
            _ = ShowToast($"Joining {s.Name}", "success");
        }
        catch (OperationCanceledException)
        {
            isLaunching = false;
            await ShowStatus("Launch cancelled.", clearAfterMs: 3000);
        }
        catch (Exception ex)
        {
            isLaunching = false;
            if (!_launchForceCancelled)
            {
                var diag = LaunchDiagnosticsService.Classify(ex);
                await ShowStatus(diag.Message, error: true, clearAfterMs: 8000);
                _ = ShowToast(diag.Title + ": " + diag.Message, "error");
            }
        }
    }
}
