using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Models;
using GlacierLauncher.Services;
using Microsoft.AspNetCore.Components;
using Microsoft.JSInterop;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private void OpenSettings() { currentView = "settings"; settingsFilter = ""; settingsCategory = "all"; }

    private void SetSettingsCategory(string cat)
    {
        settingsCategory = cat;
        StateHasChanged();
    }

    private bool CategoryMatches(string section)
    {
        if (IsJava    && section == "injection") return false;
        if (IsBedrock && section == "java")      return false;
        if (!string.IsNullOrWhiteSpace(settingsFilter)) return true;
        return settingsCategory == "all" || settingsCategory == section;
    }

    private void ToggleProp(Action toggle, bool discordToggle = false)
    {
        toggle();
        if (discordToggle) Discord.Toggle();
        SettingsService.Save();
        StateHasChanged();
    }

    private string SH(string label, string keywords)
    {
        var q = (settingsFilter ?? "").Trim().ToLower();
        if (q.Length == 0) return "";
        return (label.ToLower().Contains(q) || keywords.ToLower().Contains(q)) ? "" : "hidden-by-search";
    }

    private bool SectionMatch(params (string l, string k)[] items)
    {
        var q = (settingsFilter ?? "").Trim().ToLower();
        if (q.Length == 0) return true;
        return items.Any(i => i.l.ToLower().Contains(q) || i.k.ToLower().Contains(q));
    }

    private static readonly (string Cat, string Label, string Keywords)[] _settingDefs =
    {
        ("injection",  "Active client",                  "active client injection vanilla"),
        ("injection",  "Injection delay",                "injection delay wait"),
        ("injection",  "Auto-inject",                    "auto inject automatic"),
        ("injection",  "Close after launch",             "close minimize launch"),

        ("java",       "Maximum RAM",                    "ram memory java heap"),
        ("java",       "Minecraft folder",               "minecraft folder dot directory"),
        ("java",       "Java runtime",                   "java runtime jre jdk javaw"),
        ("java",       "Show snapshots",                 "snapshots experimental"),
        ("java",       "Show beta and alpha",            "beta alpha historical old"),
        ("java",       "Close after launch",             "close minimize launch java"),
        ("java",       "Custom JVM args",                "jvm args java options garbage collector gc"),
        ("java",       "Open Minecraft folder",          "minecraft folder dot directory open"),
        ("java",       "Latest crash report",            "crash report log latest open view java"),

        ("appearance", "Accent colour",                  "accent color theme"),
        ("appearance", "Theme preset",                   "theme dark midnight slate ocean forest sunset light"),
        ("appearance", "Compact mode",                   "compact dense small"),
        ("appearance", "Animations",                     "animations smooth motion"),
        ("appearance", "Background blur",                "blur background"),
        ("appearance", "Remember window size",           "window size remember"),
        ("appearance", "Custom background wallpaper",    "background wallpaper image custom"),

        ("account",    "Display name",                   "username name display account"),
        ("account",    "Profile display",                "profile display footer xbox discord switch which"),
        ("account",    "Discord Rich Presence",          "discord rpc social"),
        ("account",    "Discord account",                "discord login account"),
        ("account",    "Xbox profile",                   "xbox gamertag account microsoft"),

        ("system",     "Show recently launched",         "recent history"),
        ("system",     "Show launch console",            "console log launch window java bedrock output stdout"),
        ("system",     "Minimize to tray",               "tray minimize system"),
        ("system",     "Clear recent history",           "clear history recent"),
        ("system",     "Check for updates on startup",   "update auto check startup"),
        ("system",     "Check for updates now",          "update manual check"),
        ("system",     "Open launcher folder",           "launcher folder downloads location"),
        ("system",     "Open Minecraft folder",          "minecraft com mojang folder"),
        ("system",     "CurseForge API key",             "curseforge api key addons"),
        ("system",     "Export settings",                "export backup settings save"),
        ("system",     "Import settings",                "import restore settings load"),
        ("system",     "Reset settings",                 "reset default factory settings"),
        ("system",     "Keyboard shortcuts",             "shortcuts keyboard help"),
        ("system",     "Credits",                        "credits about launcher"),
    };

    private int VisibleSettingsCount()
    {
        var q = (settingsFilter ?? "").Trim().ToLower();
        var hasFilter = q.Length > 0;
        return _settingDefs.Count(s =>
        {
            if (IsJava    && s.Cat == "injection") return false;
            if (IsBedrock && s.Cat == "java")      return false;
            if (!hasFilter && settingsCategory != "all" && settingsCategory != s.Cat) return false;
            if (hasFilter && !s.Label.ToLower().Contains(q) && !s.Keywords.ToLower().Contains(q)) return false;
            return true;
        });
    }

    private void ClearSettingsFilter()
    {
        settingsFilter = "";
        StateHasChanged();
    }

    private void OnDelayChanged(ChangeEventArgs e)
    {
        if (int.TryParse(e.Value?.ToString(), out int v))
        { SettingsService.Settings.InjectionDelayMs = v; SettingsService.Save(); StateHasChanged(); }
    }

    private void OnUsernameChanged(ChangeEventArgs e)
    {
        var val = e.Value?.ToString() ?? "";
        SettingsService.Settings.Username = val;
        displayName = string.IsNullOrEmpty(val) ? System.Environment.UserName : val;
        SettingsService.Save();
        StateHasChanged();
    }

    private void OnProfileDisplayChanged(ChangeEventArgs e)
    {
        var val = e.Value?.ToString() ?? "auto";
        if (val != "auto" && val != "xbox" && val != "discord") val = "auto";
        SettingsService.Settings.ProfileDisplayMode = val;
        SettingsService.Save();
        StateHasChanged();
    }

    private async void SetAccent(string color)
    {
        SettingsService.Settings.AccentColor = color;
        SettingsService.Save();
        await JS.InvokeVoidAsync("setAccentColor", color);
        StateHasChanged();
    }

    private async void OnThemeChanged(ChangeEventArgs e)
    {
        var val = e.Value?.ToString() ?? "dark";
        SettingsService.Settings.ThemePreset = val;
        SettingsService.Save();
        await JS.InvokeVoidAsync("setTheme", val);
        StateHasChanged();
    }

    private async void OnBlurChanged(ChangeEventArgs e)
    {
        if (int.TryParse(e.Value?.ToString(), out int v))
        {
            SettingsService.Settings.BlurIntensity = v;
            SettingsService.Save();
            await JS.InvokeVoidAsync("setBlurIntensity", v);
            StateHasChanged();
        }
    }

    private void OnCfApiKeyChanged(ChangeEventArgs e)
    {
        SettingsService.Settings.CurseForgeApiKey = e.Value?.ToString() ?? "";
        SettingsService.Save();
        StateHasChanged();
    }

    private async Task PickWallpaper()
    {
        var path = await Task.Run(() =>
        {
            string? result = null;
            var thread = new System.Threading.Thread(() =>
            {
                var dlg = new Microsoft.Win32.OpenFileDialog
                {
                    Title  = "Choose a wallpaper",
                    Filter = "Image files|*.png;*.jpg;*.jpeg;*.bmp;*.gif;*.webp|All files|*.*"
                };
                if (dlg.ShowDialog() == true)
                    result = dlg.FileName;
            });
            thread.SetApartmentState(System.Threading.ApartmentState.STA);
            thread.Start();
            thread.Join();
            return result;
        });

        if (!string.IsNullOrEmpty(path))
            await OnWallpaperPicked(path);
    }

    [JSInvokable]
    public async Task OnWallpaperPicked(string path)
    {
        if (string.IsNullOrEmpty(path)) return;
        try
        {
            var fileInfo = new System.IO.FileInfo(path);
            if (!fileInfo.Exists) { _ = ShowToast("File not found.", "error"); return; }
            if (fileInfo.Length > 20 * 1024 * 1024)
            {
                _ = ShowToast("Image too large (max 20 MB). Try a smaller file.", "error");
                return;
            }

            var appFolder = System.IO.Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                "Glacier Launcher");
            System.IO.Directory.CreateDirectory(appFolder);
            var destName = "custom-bg" + System.IO.Path.GetExtension(path);
            var destPath = System.IO.Path.Combine(appFolder, destName);

            await Task.Run(() => System.IO.File.Copy(path, destPath, true));

            SettingsService.Settings.CustomBackgroundPath = destPath;
            SettingsService.Save();
            await JS.InvokeVoidAsync("setCustomBackground", destPath);
            await InvokeAsync(StateHasChanged);
        }
        catch (Exception ex)
        {
            _ = ShowToast("Failed to set wallpaper: " + ex.Message, "error");
        }
    }

    private async Task ResetWallpaper()
    {
        SettingsService.Settings.CustomBackgroundPath = "";
        SettingsService.Save();
        await JS.InvokeVoidAsync("setCustomBackground", "");
        StateHasChanged();
    }

    private async Task PickDllFile() => await JS.InvokeVoidAsync("pickDllFile", _selfRef);

    private async Task ExportSettings()
    {
        try
        {
            var path = await Task.Run(() =>
            {
                string? result = null;
                var thread = new System.Threading.Thread(() =>
                {
                    var dlg = new Microsoft.Win32.SaveFileDialog
                    {
                        Title    = "Export Glacier Launcher settings",
                        Filter   = "JSON|*.json",
                        FileName = $"glacier-settings-{DateTime.Now:yyyyMMdd-HHmm}.json"
                    };
                    if (dlg.ShowDialog() == true) result = dlg.FileName;
                });
                thread.SetApartmentState(System.Threading.ApartmentState.STA);
                thread.Start();
                thread.Join();
                return result;
            });
            if (string.IsNullOrEmpty(path)) return;

            var json = System.Text.Json.JsonSerializer.Serialize(SettingsService.Settings,
                new System.Text.Json.JsonSerializerOptions { WriteIndented = true });
            await System.IO.File.WriteAllTextAsync(path, json);
            _ = ShowToast("Settings exported.", "success");
        }
        catch (Exception ex) { _ = ShowToast("Export failed: " + ex.Message, "error"); }
    }

    private async Task ImportSettings()
    {
        try
        {
            var path = await Task.Run(() =>
            {
                string? result = null;
                var thread = new System.Threading.Thread(() =>
                {
                    var dlg = new Microsoft.Win32.OpenFileDialog
                    {
                        Title  = "Import Glacier Launcher settings",
                        Filter = "JSON|*.json"
                    };
                    if (dlg.ShowDialog() == true) result = dlg.FileName;
                });
                thread.SetApartmentState(System.Threading.ApartmentState.STA);
                thread.Start();
                thread.Join();
                return result;
            });
            if (string.IsNullOrEmpty(path)) return;

            var json = await System.IO.File.ReadAllTextAsync(path);
            var imported = System.Text.Json.JsonSerializer.Deserialize<LauncherSettings>(json);
            if (imported == null) { _ = ShowToast("Invalid settings file.", "error"); return; }

            SettingsService.Settings.SelectedClient        = imported.SelectedClient;
            SettingsService.Settings.DiscordRichPresence   = imported.DiscordRichPresence;
            SettingsService.Settings.LastUsedVersion       = imported.LastUsedVersion;
            SettingsService.Settings.Username              = imported.Username;
            SettingsService.Settings.UserHandle            = imported.UserHandle;
            SettingsService.Settings.CloseAfterLaunch      = imported.CloseAfterLaunch;
            SettingsService.Settings.AutoInject            = imported.AutoInject;
            SettingsService.Settings.InjectionDelayMs      = imported.InjectionDelayMs;
            SettingsService.Settings.AccentColor           = imported.AccentColor;
            SettingsService.Settings.ThemePreset           = imported.ThemePreset;
            SettingsService.Settings.BlurIntensity         = imported.BlurIntensity;
            SettingsService.Settings.BackgroundOpacity     = imported.BackgroundOpacity;
            SettingsService.Settings.CustomBackgroundPath  = imported.CustomBackgroundPath;
            SettingsService.Settings.CompactMode           = imported.CompactMode;
            SettingsService.Settings.AnimationsEnabled     = imported.AnimationsEnabled;
            SettingsService.Settings.RememberWindowSize    = imported.RememberWindowSize;
            SettingsService.Settings.MinimizeToTray        = imported.MinimizeToTray;
            SettingsService.Settings.ShowRecentlyLaunched  = imported.ShowRecentlyLaunched;
            SettingsService.Settings.RecentlyLaunched      = imported.RecentlyLaunched ?? new();
            SettingsService.Settings.PinnedVersions        = imported.PinnedVersions   ?? new();
            SettingsService.Settings.SavedServers          = imported.SavedServers     ?? new();
            SettingsService.Settings.VersionSortMode       = imported.VersionSortMode;
            SettingsService.Settings.ShowOnlyDownloaded    = imported.ShowOnlyDownloaded;
            SettingsService.Settings.ActiveVanillaVersion  = imported.ActiveVanillaVersion;
            SettingsService.Settings.CheckUpdatesOnStartup = imported.CheckUpdatesOnStartup;
            SettingsService.Save();

            await JS.InvokeVoidAsync("applyStoredSettings",
                SettingsService.Settings.AccentColor,
                SettingsService.Settings.ThemePreset,
                SettingsService.Settings.BlurIntensity,
                SettingsService.Settings.CustomBackgroundPath);
            await JS.InvokeVoidAsync("setCompactMode", SettingsService.Settings.CompactMode);
            await JS.InvokeVoidAsync("setAnimationsEnabled", SettingsService.Settings.AnimationsEnabled);

            _ = ShowToast("Settings imported.", "success");
            StateHasChanged();
        }
        catch (Exception ex) { _ = ShowToast("Import failed: " + ex.Message, "error"); }
    }

    private bool resetConfirmOpen = false;
    private void ConfirmResetSettings() { resetConfirmOpen = true; StateHasChanged(); }
    private void CancelResetSettings()  { resetConfirmOpen = false; StateHasChanged(); }

    private async Task DoResetSettings()
    {
        var fresh = new LauncherSettings();
        fresh.DiscordLoggedIn = SettingsService.Settings.DiscordLoggedIn;
        fresh.DiscordUsername = SettingsService.Settings.DiscordUsername;
        fresh.DiscordAvatar   = SettingsService.Settings.DiscordAvatar;
        fresh.DiscordToken    = SettingsService.Settings.DiscordToken;
        fresh.DiscordUserId   = SettingsService.Settings.DiscordUserId;
        fresh.XboxGamertag        = SettingsService.Settings.XboxGamertag;
        fresh.XboxXuid            = SettingsService.Settings.XboxXuid;
        fresh.XboxGamerPictureUrl = SettingsService.Settings.XboxGamerPictureUrl;
        fresh.XboxGamerscore      = SettingsService.Settings.XboxGamerscore;
        fresh.XboxAccountTier     = SettingsService.Settings.XboxAccountTier;
        fresh.XboxBio             = SettingsService.Settings.XboxBio;

        var json = System.Text.Json.JsonSerializer.Serialize(fresh);
        var copy = System.Text.Json.JsonSerializer.Deserialize<LauncherSettings>(json)!;
        var dst  = SettingsService.Settings;

        foreach (var p in typeof(LauncherSettings).GetProperties())
            if (p.CanWrite) p.SetValue(dst, p.GetValue(copy));

        SettingsService.Save();
        await JS.InvokeVoidAsync("applyStoredSettings",
            dst.AccentColor, dst.ThemePreset, dst.BlurIntensity, dst.CustomBackgroundPath);
        await JS.InvokeVoidAsync("setCompactMode", dst.CompactMode);
        await JS.InvokeVoidAsync("setAnimationsEnabled", dst.AnimationsEnabled);

        resetConfirmOpen = false;
        _ = ShowToast("Settings reset.", "success");
        StateHasChanged();
    }
}
