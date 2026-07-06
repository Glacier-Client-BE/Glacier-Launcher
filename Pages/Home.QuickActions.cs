using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using GlacierLauncher.Models;
using GlacierLauncher.Services;
using Microsoft.JSInterop;

namespace GlacierLauncher.Pages;

public partial class Home
{
    // Order mirrors the appearance dropdown so cycling feels predictable.
    private static readonly string[] _themePresets =
        { "dark", "darker", "midnight", "slate", "ocean", "forest", "sunset", "light" };

    private const string RepoUrl    = "https://github.com/Glacier-Client-BE/Glacier-Launcher";
    private const string IssuesUrl  = RepoUrl + "/issues/new";
    private const string DiscordUrl = "https://discord.glacierclient.xyz";
    private const string SiteUrl    = "https://glacierclient.xyz";

    /// <summary>
    /// Extra command-palette entries appended to the default set. Each is a real,
    /// keyboard-discoverable feature; grouping by Category keeps the palette tidy.
    /// </summary>
    private IEnumerable<SearchResult> ExtraQuickActions()
    {
        // ── Appearance ───────────────────────────────────────────────────────
        yield return new("fa-solid fa-circle-half-stroke", "Cycle Theme", "Appearance", "Switch to the next colour theme", "Ctrl+Shift+T", () => { CloseSearch(); _ = CycleTheme(); });
        yield return new("fa-solid fa-palette", "Cycle Accent Colour", "Appearance", "Step through accent swatches", "Ctrl+Shift+A", () => { CloseSearch(); _ = CycleAccent(); });
        yield return new("fa-solid fa-shuffle", "Random Accent Colour", "Appearance", "Surprise me", "", () => { CloseSearch(); _ = RandomAccent(); });
        yield return new("fa-solid fa-plus", "Increase Blur", "Appearance", "More background blur", "", () => { CloseSearch(); _ = NudgeBlur(4); });
        yield return new("fa-solid fa-minus", "Decrease Blur", "Appearance", "Less background blur", "", () => { CloseSearch(); _ = NudgeBlur(-4); });
        yield return new("fa-solid fa-rotate-left", "Reset Appearance", "Appearance", "Default accent, theme and blur", "", () => { CloseSearch(); _ = ResetAppearance(); });
        yield return new("fa-solid fa-compress", "Toggle Compact Mode", "Appearance", "Tighter spacing throughout", "", () => { CloseSearch(); ToggleCompact(); });
        yield return new("fa-solid fa-wand-magic-sparkles", "Toggle Animations", "Appearance", "Disable for low-end machines", "", () => { CloseSearch(); ToggleAnimations(); });

        // ── Launch behaviour ─────────────────────────────────────────────────
        yield return new("fa-solid fa-door-closed", "Toggle Close After Launch", "Launch", "Quit the launcher once the game starts", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.CloseAfterLaunch, v => SettingsService.Settings.CloseAfterLaunch = v, "Close after launch: on", "Close after launch: off"); });
        yield return new("fa-solid fa-syringe", "Toggle Auto-inject", "Launch", "Inject as soon as the game is detected", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.AutoInject, v => SettingsService.Settings.AutoInject = v, "Auto-inject: on", "Auto-inject: off"); });
        yield return new("fa-solid fa-window-minimize", "Toggle Minimize to Tray", "Launch", "Send to the system tray instead of taskbar", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.MinimizeToTray, v => SettingsService.Settings.MinimizeToTray = v, "Minimize to tray: on", "Minimize to tray: off"); });
        yield return new("fa-solid fa-terminal", "Toggle Launch Console", "Launch", "Show the game console window on launch", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.ShowLaunchConsole, v => SettingsService.Settings.ShowLaunchConsole = v, "Launch console: on", "Launch console: off"); });
        yield return new("fa-brands fa-discord", "Toggle Discord Rich Presence", "Launch", "Show what you're playing on Discord", "",
            () => { CloseSearch(); ToggleDiscordRpc(); });
        yield return new("fa-solid fa-rotate", "Toggle Update Check on Startup", "Launch", "Look for updates when the launcher opens", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.CheckUpdatesOnStartup, v => SettingsService.Settings.CheckUpdatesOnStartup = v, "Startup update check: on", "Startup update check: off"); });
        yield return new("fa-solid fa-up-right-and-down-left-from-center", "Toggle Remember Window Size", "Launch", "Restore the window size next launch", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.RememberWindowSize, v => SettingsService.Settings.RememberWindowSize = v, "Remembering window size", "Window size not remembered"); });

        // ── Versions / lists ─────────────────────────────────────────────────
        yield return new("fa-solid fa-filter", "Toggle Downloaded-only", "Versions", "Only show versions you've installed", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.ShowOnlyDownloaded, v => SettingsService.Settings.ShowOnlyDownloaded = v, "Showing downloaded only", "Showing all versions", rerender: true); });
        yield return new("fa-solid fa-flask", "Toggle Snapshots (Java)", "Versions", "Include snapshot builds in the Java list", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.JavaShowSnapshots, v => SettingsService.Settings.JavaShowSnapshots = v, "Snapshots shown", "Snapshots hidden", rerender: true); });
        yield return new("fa-solid fa-clock-rotate-left", "Toggle Historical (Java)", "Versions", "Include old beta/alpha builds", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.JavaShowHistorical, v => SettingsService.Settings.JavaShowHistorical = v, "Historical shown", "Historical hidden", rerender: true); });
        yield return new("fa-solid fa-clock", "Toggle Recently Launched", "Versions", "Show the recent-launches strip on home", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.ShowRecentlyLaunched, v => SettingsService.Settings.ShowRecentlyLaunched = v, "Recent strip on", "Recent strip off", rerender: true); });

        // ── Edition ──────────────────────────────────────────────────────────
        yield return new("fa-brands fa-java", "Switch to Java Edition", "Edition", "Drive Java Edition (.minecraft)", "",
            () => { CloseSearch(); SetEdition("java"); _ = ShowToast("Switched to Java Edition", "info"); });
        yield return new("fa-solid fa-cube", "Switch to Bedrock Edition", "Edition", "Drive Bedrock + client injection", "",
            () => { CloseSearch(); SetEdition("bedrock"); _ = ShowToast("Switched to Bedrock Edition", "info"); });

        // ── Java settings ────────────────────────────────────────────────────
        yield return new("fa-solid fa-floppy-disk", "Toggle Backup Saves on Launch (Java)", "Java", "Zip worlds before each Java launch", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.JavaBackupSavesBeforeLaunch, v => SettingsService.Settings.JavaBackupSavesBeforeLaunch = v, "Save backups: on", "Save backups: off"); });
        yield return new("fa-solid fa-expand", "Toggle Java Fullscreen", "Java", "Launch Java Edition in fullscreen", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.JavaFullscreen, v => SettingsService.Settings.JavaFullscreen = v, "Java fullscreen: on", "Java fullscreen: off"); });

        // ── Folders ──────────────────────────────────────────────────────────
        yield return new("fa-solid fa-image", "Open Screenshots Folder (Java)", "Folders", "Browse captured screenshots", "", () => { CloseSearch(); OpenJavaFolder("screenshots"); });
        yield return new("fa-solid fa-earth-americas", "Open Saves Folder (Java)", "Folders", "Your single-player worlds", "", () => { CloseSearch(); OpenJavaFolder("saves"); });
        yield return new("fa-solid fa-puzzle-piece", "Open Mods Folder (Java)", "Folders", "Installed Java mods", "", () => { CloseSearch(); OpenJavaFolder("mods"); });
        yield return new("fa-solid fa-palette", "Open Resource Packs Folder (Java)", "Folders", "Installed resource packs", "", () => { CloseSearch(); OpenJavaFolder("resourcepacks"); });
        yield return new("fa-solid fa-sun", "Open Shader Packs Folder (Java)", "Folders", "Installed shader packs", "", () => { CloseSearch(); OpenJavaFolder("shaderpacks"); });
        yield return new("fa-solid fa-box-archive", "Open Backups Folder (Java)", "Folders", "Your world backups", "", () => { CloseSearch(); OpenJavaFolder("backups"); });

        // ── Copy / clipboard ─────────────────────────────────────────────────
        yield return new("fa-solid fa-clipboard-list", "Copy Diagnostics", "Clipboard", "System report for bug threads", "Ctrl+Shift+D", () => { CloseSearch(); _ = CopyDiagnostics(); });
        yield return new("fa-solid fa-tag", "Copy Launcher Version", "Clipboard", "v" + AutoUpdateService.CurrentVersion, "",
            () => { CloseSearch(); _ = CopyText("v" + AutoUpdateService.CurrentVersion, "Version copied"); });
        yield return new("fa-solid fa-fingerprint", "Copy Java UUID", "Clipboard", "Your Minecraft account UUID", "",
            () => { CloseSearch(); _ = CopyText(SettingsService.Settings.JavaUuid, "UUID copied"); });
        yield return new("fa-solid fa-gamepad", "Copy Xbox Gamertag", "Clipboard", "Your signed-in gamertag", "",
            () => { CloseSearch(); _ = CopyText(SettingsService.Settings.XboxGamertag, "Gamertag copied"); });
        yield return new("fa-solid fa-folder", "Copy Launcher Folder Path", "Clipboard", LauncherUtilityService.LauncherRoot, "",
            () => { CloseSearch(); _ = CopyText(LauncherUtilityService.LauncherRoot, "Path copied"); });

        // ── Stats ────────────────────────────────────────────────────────────
        yield return new("fa-solid fa-stopwatch", "Show Total Playtime", "Stats", "Time tracked across all sessions", "",
            () => { CloseSearch(); _ = ShowToast(FormatPlaytime(SettingsService.Settings.TotalPlaytimeSeconds) + " played in total", "info"); });

        // ── Maintenance ──────────────────────────────────────────────────────
        yield return new("fa-solid fa-broom", "Clear Download Cache", "Maintenance", "Free cached API/version data", "", () => { CloseSearch(); _ = ClearCacheAction(); });
        yield return new("fa-solid fa-hard-drive", "Show Disk Usage", "Maintenance", "How much space the launcher uses", "", () => { CloseSearch(); _ = ShowDiskUsageAction(); });

        // ── Themes (direct picks) ────────────────────────────────────────────
        yield return new("fa-solid fa-moon", "Set Dark Theme", "Appearance", "Classic dark", "", () => { CloseSearch(); _ = SetThemePreset("dark"); });
        yield return new("fa-solid fa-sun", "Set Light Theme", "Appearance", "Bright theme", "", () => { CloseSearch(); _ = SetThemePreset("light"); });
        yield return new("fa-solid fa-water", "Set Ocean Theme", "Appearance", "Cool blues", "", () => { CloseSearch(); _ = SetThemePreset("ocean"); });
        yield return new("fa-solid fa-tree", "Set Forest Theme", "Appearance", "Deep greens", "", () => { CloseSearch(); _ = SetThemePreset("forest"); });
        yield return new("fa-solid fa-mountain-sun", "Set Sunset Theme", "Appearance", "Warm tones", "", () => { CloseSearch(); _ = SetThemePreset("sunset"); });

        // ── Navigation ───────────────────────────────────────────────────────
        yield return new("fa-solid fa-user", "View Java Skin / Profile", "Navigation", "3D skin viewer (NameMC-style)", "", () => { CloseSearch(); OpenJavaProfile(); });
        yield return new("fa-solid fa-images", "Screenshot Gallery (Java)", "Navigation", "Browse your in-game screenshots", "", () => { CloseSearch(); OpenJavaScreenshots(); });
        yield return new("fa-solid fa-newspaper", "What's New / Changelog", "Navigation", "Launcher news and release notes", "", () => { CloseSearch(); _ = OpenNews(); });
        yield return new("fa-solid fa-shirt", "Change Minecraft Skin", "Account", "Upload a new skin (signed-in)", "", () => { CloseSearch(); _ = ChangeSkinAsync(); });
        yield return new("fa-solid fa-triangle-exclamation", "Check Mod Conflicts (Java)", "Java", "Scan installed mods for missing deps & duplicates", "", () => { CloseSearch(); OpenAddons("mods"); });
        yield return new("fa-solid fa-house", "Go Home", "Navigation", "Back to the launch screen", "", () => { CloseSearch(); GoHome(); StateHasChanged(); });
        yield return new("fa-solid fa-id-badge", "Toggle Footer Profile", "Navigation", "Cycle Auto / Xbox / Discord display", "", () => { CloseSearch(); CycleProfileDisplay(); });

        // ── Updates ──────────────────────────────────────────────────────────
        yield return new("fa-solid fa-rotate", "Check for Updates Now", "Updates", "Re-check launcher and client updates", "", () => { CloseSearch(); _ = ManualUpdateCheck(); });

        // ── More clipboard / Java ────────────────────────────────────────────
        yield return new("fa-solid fa-puzzle-piece", "Copy Active Client", "Clipboard", SettingsService.Settings.SelectedClient, "",
            () => { CloseSearch(); _ = CopyText(SettingsService.Settings.SelectedClient, "Client copied"); });
        yield return new("fa-brands fa-discord", "Copy Discord Username", "Clipboard", "Your linked Discord name", "",
            () => { CloseSearch(); _ = CopyText(SettingsService.Settings.DiscordUsername, "Discord name copied"); });
        yield return new("fa-solid fa-folder-tree", "Copy Minecraft Folder Path (Java)", "Clipboard", "Active .minecraft directory", "",
            () => { CloseSearch(); _ = CopyText(JavaInstances.ActiveMinecraftDir, "Path copied"); });
        yield return new("fa-solid fa-display", "Toggle Custom Resolution (Java)", "Java", "Use a fixed game window size", "",
            () => { CloseSearch(); ToggleSetting(() => SettingsService.Settings.JavaUseCustomResolution, v => SettingsService.Settings.JavaUseCustomResolution = v, "Custom resolution: on", "Custom resolution: off"); });

        // ── Links / help ─────────────────────────────────────────────────────
        yield return new("fa-brands fa-github", "Open GitHub Repository", "Help", "Source code and releases", "", () => { CloseSearch(); OpenUrl(RepoUrl); });
        yield return new("fa-solid fa-bug", "Report a Bug", "Help", "Open a new GitHub issue", "", () => { CloseSearch(); OpenUrl(IssuesUrl); });
        yield return new("fa-brands fa-discord", "Open Discord", "Help", "Join the community", "", () => { CloseSearch(); OpenUrl(DiscordUrl); });
        yield return new("fa-solid fa-globe", "Open Website", "Help", SiteUrl, "", () => { CloseSearch(); OpenUrl(SiteUrl); });
    }

    // ── Handlers ─────────────────────────────────────────────────────────────

    private async Task CycleTheme()
    {
        var idx  = Array.IndexOf(_themePresets, SettingsService.Settings.ThemePreset);
        var next = _themePresets[(idx + 1 + _themePresets.Length) % _themePresets.Length];
        SettingsService.Settings.ThemePreset = next;
        SettingsService.Save();
        await JS.InvokeVoidAsync("setTheme", next);
        _ = ShowToast("Theme: " + Capitalize(next), "success");
        StateHasChanged();
    }

    private async Task CycleAccent()
    {
        var idx  = Array.IndexOf(_accentSwatches, SettingsService.Settings.AccentColor);
        var next = _accentSwatches[(idx + 1 + _accentSwatches.Length) % _accentSwatches.Length];
        await ApplyAccent(next);
    }

    private async Task RandomAccent() =>
        await ApplyAccent(_accentSwatches[Random.Shared.Next(_accentSwatches.Length)]);

    private async Task SetThemePreset(string preset)
    {
        SettingsService.Settings.ThemePreset = preset;
        SettingsService.Save();
        await JS.InvokeVoidAsync("setTheme", preset);
        _ = ShowToast("Theme: " + Capitalize(preset), "success");
        StateHasChanged();
    }

    private async Task ApplyAccent(string color)
    {
        SettingsService.Settings.AccentColor = color;
        SettingsService.Save();
        await JS.InvokeVoidAsync("setAccentColor", color);
        _ = ShowToast("Accent updated", "success");
        StateHasChanged();
    }

    private async Task NudgeBlur(int delta)
    {
        var v = Math.Clamp(SettingsService.Settings.BlurIntensity + delta, 0, 30);
        SettingsService.Settings.BlurIntensity = v;
        SettingsService.Save();
        await JS.InvokeVoidAsync("setBlurIntensity", v);
        _ = ShowToast($"Blur: {v}px", "info");
        StateHasChanged();
    }

    private async Task ResetAppearance()
    {
        SettingsService.Settings.AccentColor   = "#7289da";
        SettingsService.Settings.ThemePreset   = "dark";
        SettingsService.Settings.BlurIntensity = 14;
        SettingsService.Save();
        await JS.InvokeVoidAsync("setAccentColor", "#7289da");
        await JS.InvokeVoidAsync("setTheme", "dark");
        await JS.InvokeVoidAsync("setBlurIntensity", 14);
        _ = ShowToast("Appearance reset to defaults", "success");
        StateHasChanged();
    }

    private void ToggleCompact()
    {
        var v = !SettingsService.Settings.CompactMode;
        SettingsService.Settings.CompactMode = v;
        SettingsService.Save();
        _ = JS.InvokeVoidAsync("setCompactMode", v);
        _ = ShowToast(v ? "Compact mode on" : "Compact mode off", "info");
        StateHasChanged();
    }

    private void ToggleAnimations()
    {
        var v = !SettingsService.Settings.AnimationsEnabled;
        SettingsService.Settings.AnimationsEnabled = v;
        SettingsService.Save();
        _ = JS.InvokeVoidAsync("setAnimationsEnabled", v);
        _ = ShowToast(v ? "Animations on" : "Animations off", "info");
        StateHasChanged();
    }

    // Flip a boolean setting, persist, toast, and optionally re-render.
    private void ToggleSetting(Func<bool> get, Action<bool> set, string onMsg, string offMsg, bool rerender = false)
    {
        var v = !get();
        set(v);
        SettingsService.Save();
        _ = ShowToast(v ? onMsg : offMsg, "info");
        if (rerender) StateHasChanged();
    }

    private async Task CopyText(string text, string toast)
    {
        if (string.IsNullOrWhiteSpace(text)) { _ = ShowToast("Nothing to copy.", "error"); return; }
        await JS.InvokeVoidAsync("copyToClipboard", text);
        _ = ShowToast(toast, "success");
    }

    private async Task CopyDiagnostics()
    {
        var diag = LauncherUtilityService.Diagnostics(
            AutoUpdateService.CurrentVersion,
            SettingsService.Settings.Edition,
            SettingsService.Settings.SelectedClient);
        await CopyText(diag, "Diagnostics copied to clipboard");
    }

    private async Task ClearCacheAction()
    {
        var freed = await Task.Run(LauncherUtilityService.ClearCache);
        _ = ShowToast(
            freed > 0 ? $"Cleared {LauncherUtilityService.FormatBytes(freed)} of cache" : "Cache already empty",
            "success");
    }

    private async Task ShowDiskUsageAction()
    {
        var size = await Task.Run(() => LauncherUtilityService.DirectorySize(LauncherUtilityService.LauncherRoot));
        _ = ShowToast($"Launcher data: {LauncherUtilityService.FormatBytes(size)}", "info");
    }

    private void OpenJavaProfile()
    {
        _ = NavigateAsync(() =>
        {
            SetEditionCore("java");   // no-op if already on Java
            LoadInstanceRam();
            currentView = "javaprofile";
            _ = LoadCapesAsync();
        });
    }

    private void OpenJavaScreenshots()
    {
        _ = NavigateAsync(() =>
        {
            SetEditionCore("java");
            RefreshJavaInstanceFiles();
            currentView = "javascreenshots";
        });
    }

    private void OpenScreenshot(JavaInstanceFile shot) => OpenUrl(shot.Path);

    // Convert an absolute path under the Glacier Launcher folder into a URL the
    // WebView can load, via the glacier-files.local virtual host mapped by the host.
    private static string FileToLocalUrl(string absolutePath)
    {
        try
        {
            var root = LauncherUtilityService.LauncherRoot;
            var full = Path.GetFullPath(absolutePath);
            if (!full.StartsWith(root, StringComparison.OrdinalIgnoreCase)) return "";
            var rel = full[root.Length..].TrimStart('\\', '/').Replace('\\', '/');
            var encoded = string.Join('/', rel.Split('/').Select(Uri.EscapeDataString));
            return $"https://glacier-files.local/{encoded}";
        }
        catch { return ""; }
    }

    // Called from the running-state poll when the game launches/exits: start a
    // session clock on launch, bank the elapsed time on exit.
    private void AccumulatePlaytime(bool running)
    {
        if (running)
        {
            _sessionStart   = DateTime.UtcNow;
            _sessionEdition = IsJava ? "java" : "bedrock";
            _sessionLabel   = IsJava
                ? (string.IsNullOrEmpty(SettingsService.Settings.JavaActiveVersion) ? "Java" : SettingsService.Settings.JavaActiveVersion)
                : GetFooterVersionLabel();
            return;
        }
        if (_sessionStart is { } start)
        {
            var secs = (long)(DateTime.UtcNow - start).TotalSeconds;
            if (secs > 0)
            {
                SettingsService.Settings.TotalPlaytimeSeconds += secs;
                SettingsService.Settings.LastPlayed = DateTime.UtcNow.ToString("o");
                SettingsService.Save();
                Stats.RecordSession(start, secs, _sessionEdition, _sessionLabel);
            }
            _sessionStart = null;
        }
    }

    // After a Java session ends, look for a crash report written in the last
    // 30 seconds and offer the user a jump to the log viewer.
    private async Task CheckForCrashAsync()
    {
        try
        {
            var logs = await Task.Run(() => Logs.ListLogs());
            var fresh = logs.FirstOrDefault(l => l.IsCrash
                && DateTime.TryParse(l.ModifiedAt, null, System.Globalization.DateTimeStyles.RoundtripKind, out var dt)
                && (DateTime.UtcNow - dt).TotalSeconds < 30);
            if (fresh != null)
                _ = ShowToast("Minecraft crashed — open Logs to view the report.", "error");
        }
        catch { }
    }

    private static string FormatPlaytime(long seconds)
    {
        if (seconds <= 0) return "0m";
        var h = seconds / 3600;
        var m = (seconds % 3600) / 60;
        return h > 0 ? $"{h}h {m}m" : $"{m}m";
    }

    private static string Capitalize(string s) =>
        string.IsNullOrEmpty(s) ? s : char.ToUpper(s[0]) + s[1..];
}
