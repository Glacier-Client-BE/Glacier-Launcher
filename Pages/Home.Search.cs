using System;
using System.Collections.Generic;
using System.Linq;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private void BuildDefaultSearchResults()
    {
        _defaultSearchResults = new List<SearchResult>
        {
            new("fa-solid fa-play", "Launch Game", "Quick Actions", "with " + SettingsService.Settings.SelectedClient, "Ctrl+L", () => { CloseSearch(); _ = HandleLaunch(); }),
            new("fa-solid fa-gear", "Open Settings", "Quick Actions", "Injection, Appearance, Account", "Ctrl+1", () => { CloseSearch(); OpenSettings(); }),
            new("fa-solid fa-puzzle-piece", "Manage Clients", "Quick Actions", "Flarial, Latite, OderSo, Custom DLL", "Ctrl+2", () => { CloseSearch(); _ = OpenClients(); }),
            new("fa-solid fa-cube", "Browse Addons", "Quick Actions", "CurseForge mods, resource packs and worlds", "Ctrl+4", () => { CloseSearch(); OpenAddons(); }),
            new("fa-solid fa-server", "Servers", "Quick Actions", "Quick-launch into saved servers", "Ctrl+5", () => { CloseSearch(); OpenServers(); }),
            new("fa-solid fa-clock-rotate-left", "Browse Versions", "Quick Actions", "Download and manage client versions", "", () => { CloseSearch(); _ = OpenVersions(); }),
            new("fa-solid fa-box-archive", "MC Versions", "Quick Actions", "Switch Minecraft Bedrock versions", "", () => { CloseSearch(); _ = OpenMcVersions(); }),
            new("fa-brands fa-java", "Java Versions", "Java", "Install or launch Java Edition", "", () => { CloseSearch(); _ = OpenJavaVersions(); }),
            new("fa-solid fa-cubes", "Browse Modpacks", "Java", "Install CurseForge / Modrinth modpacks into an instance", "", () => { CloseSearch(); OpenModpacks(); }),
            new("fa-solid fa-chart-simple", "Statistics", "Navigation", "Playtime, sessions and launch history", "", () => { CloseSearch(); OpenStats(); }),
            new("fa-solid fa-file-lines", "Logs & Crashes", "Java", "View logs, detect crashes, share to mclo.gs", "", () => { CloseSearch(); OpenLogs(); }),
            new("fa-solid fa-shirt", "Skin Library", "Account", "Saved skins — apply to your account", "", () => { CloseSearch(); OpenSkinLibrary(); }),
            new("fa-solid fa-box-archive", "World Backups", "Java", "Back up, restore and manage world saves", "", () => { CloseSearch(); OpenBackups(); }),
            new("fa-solid fa-gamepad", "Xbox Profile", "Account", "Sign in with Microsoft", "", () => { CloseSearch(); OpenXboxModal(); }),
            new("fa-brands fa-discord", "Discord Account", "Account", "Connect or manage Discord", "", () => { CloseSearch(); OpenDiscordModal(); }),
            new("images/clients/flarial.svg", "Select Flarial", "Clients", "Switch active client to Flarial", "", () => { CloseSearch(); SelectClient("Flarial Client"); }),
            new("images/clients/latite.png", "Select Latite", "Clients", "Switch active client to Latite", "", () => { CloseSearch(); SelectClient("Latite Client"); }),
            new("images/clients/oderso.png", "Select OderSo", "Clients", "Switch active client to OderSo", "", () => { CloseSearch(); SelectClient("OderSo Client"); }),
            new("fa-solid fa-cube", "Select Vanilla", "Clients", "Launch Minecraft without a client", "", () => { CloseSearch(); SelectClient("Vanilla"); }),
            new("fa-solid fa-folder-open", "Open Launcher Folder", "Quick Actions", "Settings, downloads and clients folder", "", () => { CloseSearch(); OpenLauncherFolder(); }),
            new("fa-solid fa-cubes", "Open Minecraft Folder", "Quick Actions", "Minecraft data folder", "", () => { CloseSearch(); OpenMinecraftFolder(); }),
            new("fa-solid fa-file-export", "Export Settings", "Settings", "Save settings as JSON", "", () => { CloseSearch(); _ = ExportSettings(); }),
            new("fa-solid fa-file-import", "Import Settings", "Settings", "Restore from JSON", "", () => { CloseSearch(); _ = ImportSettings(); }),
            new("fa-solid fa-syringe", "Auto-inject", "Settings", "Inject automatically when game is detected", "", () => { CloseSearch(); OpenSettings("auto-inject"); }),
            new("fa-solid fa-clock", "Injection Delay", "Settings", "Time to wait before injecting", "", () => { CloseSearch(); OpenSettings("injection delay"); }),
            new("fa-solid fa-door-closed", "Close After Launch", "Settings", "Close launcher after launch", "", () => { CloseSearch(); OpenSettings("close after"); }),
            new("fa-solid fa-palette", "Accent Colour", "Settings", "Change highlight colour", "", () => { CloseSearch(); OpenSettings("accent"); }),
            new("fa-solid fa-moon", "Theme Preset", "Settings", "Dark, light and colour themes", "", () => { CloseSearch(); OpenSettings("theme"); }),
            new("fa-solid fa-palette", "Theme Studio", "Settings", "Create fully custom themes — colors, fonts, radii, CSS", "", () => { CloseSearch(); OpenThemeStudio(); }),
            new("fa-solid fa-image", "Background Wallpaper", "Settings", "Change custom background image", "", () => { CloseSearch(); OpenSettings("wallpaper"); }),
            new("fa-solid fa-memory", "Java RAM", "Settings", "Maximum Java memory allocation", "", () => { CloseSearch(); OpenSettings("ram"); }),
            new("fa-solid fa-terminal", "JVM Arguments", "Settings", "Advanced Java flags", "", () => { CloseSearch(); OpenSettings("jvm"); }),
            new("fa-solid fa-window-minimize", "Minimize to Tray", "Settings", "Send launcher to system tray", "", () => { CloseSearch(); OpenSettings("tray"); }),
            new("fa-solid fa-trash", "Clear Recent History", "Settings", "Clear recently launched list", "", () => { CloseSearch(); ClearRecentHistory(); }),
            new("fa-solid fa-rotate", "Check for Updates", "Settings", "Check launcher and client updates", "", () => { CloseSearch(); OpenSettings("update"); }),
            new("fa-solid fa-expand", "Toggle Fullscreen", "Quick Actions", "F11 fullscreen mode", "F11", () => { CloseSearch(); _ = ToggleFullscreen(); }),
            new("fa-solid fa-heart", "Credits", "About", "View launcher credits", "", () => { CloseSearch(); OpenCredits(); }),
        };

        _defaultSearchResults.AddRange(ExtraQuickActions());
    }

    private IEnumerable<SearchResult> BuildDynamicSearchPool()
    {
        foreach (var s in _defaultSearchResults)
            yield return s;

        foreach (var recent in RecentsForCurrentEdition())
        {
            var raw = recent;
            yield return new SearchResult("fa-solid fa-clock-rotate-left", RecentDisplayLabel(raw), "Recent", "Quick-launch", () => { CloseSearch(); _ = QuickLaunchRecent(raw); });
        }

        if (IsBedrock)
        {
            foreach (var version in versions)
            {
                var local = version;
                yield return new SearchResult("fa-solid fa-download", local.DisplayName, "Versions", local.IsDownloaded ? "Downloaded" : "Not downloaded", () => { CloseSearch(); _ = local.IsDownloaded ? HandleVersionLaunch(local) : HandleVersionDownload(local); });
            }

            foreach (var version in mcVersionsList)
            {
                var local = version;
                yield return new SearchResult("fa-solid fa-box-archive", "Minecraft " + local.Version, "MC Versions", local.IsDownloaded ? "Downloaded" : "Not downloaded", () => { CloseSearch(); _ = OpenMcVersions(); });
            }

            foreach (var server in SettingsService.Settings.SavedServers)
            {
                var local = server;
                yield return new SearchResult("fa-solid fa-server", local.Name, "Servers", $"{local.Address}:{local.Port}", () => { CloseSearch(); _ = LaunchServer(local); });
            }
        }
        else
        {
            foreach (var version in javaVersionsList)
            {
                var local = version;
                yield return new SearchResult("fa-brands fa-java", "Minecraft " + local.Id, local.IsInstalled ? "Java Installed" : "Java " + local.TypeLabel, local.IsInstalled ? "Launch" : "Install", () => { CloseSearch(); _ = local.IsInstalled ? HandleJavaLaunch(local) : HandleJavaInstall(local); });
            }

            foreach (var installed in LunarBadlion.LunarInstalledVersions)
            {
                var local = installed;
                yield return new SearchResult("fa-solid fa-moon", "Lunar " + local, "Launchers", "Direct game launch", () => { CloseSearch(); lunarSelectedVersion = local; LaunchLunarDirect(); });
            }

            yield return new SearchResult("fa-solid fa-moon", "Lunar Client", "Launchers", LunarBadlion.LunarInstalled ? "Detected" : "Not installed", () => { CloseSearch(); LaunchLunar(); });

            if (LunarBadlion.BadlionInstalled)
                yield return new SearchResult("fa-solid fa-shield", "Badlion Client", "Launchers", "Detected", () => { CloseSearch(); LaunchBadlion(); });
        }
    }

    private void ClearRecentHistory()
    {
        SettingsService.Settings.RecentlyLaunched.Clear();
        SettingsService.Save();
        StateHasChanged();
    }
}
