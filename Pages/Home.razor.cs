using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using GlacierLauncher.Models;
using GlacierLauncher.Services;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.JSInterop;

namespace GlacierLauncher.Pages;

public partial class Home : IDisposable
{
    private string currentView   = "home";
    private bool   isLaunching   = false;
    private bool   isDownloading = false;
    private string downloadStage = "";
    private double downloadPct   = 0;
    private bool   isError       = false;
    private string statusMessage = "";
    private bool   statusVisible = false;
    private CancellationTokenSource? _statusCts;

    private bool   isLoadingVersions = false;
    private List<MinecraftVersion> versions = new();
    private string versionsFilter = "";
    private string versionsClient = "Latite";

    // Filter + sort results are memoized: Home re-renders on every state change
    // and recomputing LINQ chains over hundreds of versions each time is the
    // main render cost of the version panels. The key includes a cheap
    // downloaded/installed tally so flag flips (download finished) invalidate.
    private List<MinecraftVersion>? _filteredVersionsCache;
    private string _filteredVersionsKey = "";

    private IReadOnlyList<MinecraftVersion> FilteredVersions
    {
        get
        {
            var key = $"{versions.Count}|{versions.Count(v => v.IsDownloaded)}|{versionsFilter}|" +
                      $"{SettingsService.Settings.ShowOnlyDownloaded}|{SettingsService.Settings.VersionSortMode}";
            if (_filteredVersionsCache == null || key != _filteredVersionsKey)
            {
                IEnumerable<MinecraftVersion> q = versions;
                if (!string.IsNullOrWhiteSpace(versionsFilter))
                {
                    var filter = versionsFilter.Trim();
                    q = q.Where(v => v.DisplayName.Contains(filter, StringComparison.OrdinalIgnoreCase)
                                  || v.Tag.Contains(filter, StringComparison.OrdinalIgnoreCase));
                }
                if (SettingsService.Settings.ShowOnlyDownloaded)
                    q = q.Where(v => v.IsDownloaded);
                _filteredVersionsCache = SortedVersions(q).ToList();
                _filteredVersionsKey   = key;
            }
            return _filteredVersionsCache;
        }
    }

    private class OderSoDllState
    {
        public OderSoService.DllEntry Entry { get; }
        public bool   IsDownloaded  { get; set; }
        public bool   IsDownloading { get; set; }
        public double Progress      { get; set; }
        public string Error         { get; set; } = "";
        public bool   IsActive      { get; set; }
        public CancellationTokenSource? DownloadCts { get; set; }

        public string DisplayName =>
            System.IO.Path.GetFileNameWithoutExtension(Entry.Name);

        public string SizeLabel =>
            Entry.Size > 0
                ? (Entry.Size >= 1_048_576
                    ? $"{Entry.Size / 1_048_576.0:F1} MB"
                    : $"{Entry.Size / 1024.0:F0} KB")
                : "";

        public OderSoDllState(OderSoService.DllEntry entry) => Entry = entry;
    }
    private List<OderSoDllState>  oderSoVersions          = new();
    private bool                  isLoadingOderSoVersions = false;

    private bool   isLoadingMcVersions = false;
    private List<VanillaVersion> mcVersionsList = new();
    private string mcVersionsFilter = "";
    private string? mcVersionsError = null;

    private bool   storeInstallBusy   = false;
    private double storeInstallPct    = 0;
    private string storeInstallStage  = "";
    private string storeInstallDetail = "";

    private List<VanillaVersion>? _filteredMcCache;
    private string _filteredMcKey = "";

    private IReadOnlyList<VanillaVersion> FilteredMcVersions
    {
        get
        {
            var key = $"{mcVersionsList.Count}|{mcVersionsList.Count(v => v.IsDownloaded)}|{mcVersionsFilter}";
            if (_filteredMcCache == null || key != _filteredMcKey)
            {
                IEnumerable<VanillaVersion> q = mcVersionsList;
                if (!string.IsNullOrWhiteSpace(mcVersionsFilter))
                {
                    var filter = mcVersionsFilter.Trim();
                    q = q.Where(v => v.Version.Contains(filter, StringComparison.OrdinalIgnoreCase));
                }
                _filteredMcCache = q.ToList();
                _filteredMcKey   = key;
            }
            return _filteredMcCache;
        }
    }

    private bool   isLoadingJavaVersions = false;
    private List<JavaVersion> javaVersionsList = new();
    private string javaVersionsFilter = "";
    private string? javaVersionsError = null;
    private List<JavaInstanceFile> javaMods = new();
    private List<JavaInstanceFile> javaScreenshots = new();
    private List<JavaInstanceFile> javaResourcePacks = new();
    private List<JavaInstanceFile> javaShaderPacks = new();
    private List<JavaInstanceFile> javaSchematics = new();
    private bool javaUtilityBusy = false;
    private string javaModsTab = "loaders";

    private bool IsBedrock => !IsJava;
    private bool IsJava    => string.Equals(SettingsService.Settings.Edition, "java", StringComparison.OrdinalIgnoreCase);

    private List<JavaVersion>? _filteredJavaCache;
    private string _filteredJavaKey = "";

    private IReadOnlyList<JavaVersion> FilteredJavaVersions
    {
        get
        {
            var key = $"{javaVersionsList.Count}|{javaVersionsList.Count(v => v.IsInstalled)}|{javaVersionsFilter}|" +
                      $"{SettingsService.Settings.JavaShowSnapshots}|{SettingsService.Settings.JavaShowHistorical}";
            if (_filteredJavaCache == null || key != _filteredJavaKey)
            {
                IEnumerable<JavaVersion> q = javaVersionsList;
                if (!SettingsService.Settings.JavaShowSnapshots)
                    q = q.Where(v => v.Type != "snapshot");
                if (!SettingsService.Settings.JavaShowHistorical)
                    q = q.Where(v => v.Type != "old_beta" && v.Type != "old_alpha");
                if (!string.IsNullOrWhiteSpace(javaVersionsFilter))
                {
                    var filter = javaVersionsFilter.Trim();
                    q = q.Where(v => v.Id.Contains(filter, StringComparison.OrdinalIgnoreCase));
                }
                _filteredJavaCache = q.ToList();
                _filteredJavaKey   = key;
            }
            return _filteredJavaCache;
        }
    }

    private string cfSearchQuery      = "";
    private string cfCategory         = "all";
    private bool   cfSearching        = false;
    private bool   cfHasSearched      = false;
    private string cfError            = "";
    private int    cfPage             = 0;
    private int    cfTotalCount       = 0;
    private int    cfDownloadingId    = 0;
    private double cfDownloadProgress = 0;
    private List<CurseForgeService.CfAddon> cfResults = new();

    private string mrSearchQuery      = "";
    private string mrCategory         = "mod";
    private bool   mrSearching        = false;
    private bool   mrHasSearched      = false;
    private string mrError            = "";
    private int    mrPage             = 0;
    private int    mrTotalCount       = 0;
    private string mrDownloadingId    = "";
    private double mrDownloadProgress = 0;
    private List<ModrinthService.MrProject> mrResults = new();

    private readonly ClientDownloadState _flarial = new();
    private readonly ClientDownloadState _oderso  = new();

    private string customDllPath { get => SettingsService.Settings.CustomDllPath; set { SettingsService.Settings.CustomDllPath = value; SettingsService.Save(); } }
    private bool   copiedDllPath = false;

    private string displayName   = Environment.UserName;
    private string displayHandle = "@" + Environment.UserName.ToLower();

    private bool   searchOpen   = false;
    private string searchQuery  = "";
    private int    searchSelIdx = 0;
    private List<SearchResult> searchResults = new();

    private string settingsFilter = "";
    private string settingsCategory = "all";

    private bool   discordModalOpen     = false;
    private string discordUsernameInput = "";

    private bool xboxModalOpen  = false;
    private bool xboxSigningIn  = false;

    private bool clientsHasBadge  = false;
    private bool versionsHasBadge = false;

    private bool                launcherUpdateAvailable   = false;
    private bool                launcherUpdateModalOpen   = false;
    private bool                launcherUpdating          = false;
    private double              launcherUpdateProgress    = 0;
    private LauncherUpdateInfo? launcherUpdateInfo        = null;

    private bool   checkingUpdatesManual = false;
    private string lastUpdateCheckLabel  = "Never checked";

    private record ToastItem(string Message, string Kind, string Icon)
    {
        public bool Closing { get; set; }
    }
    private List<ToastItem> _toasts = new();

    private bool mcRunning = false;
    private DateTime? _sessionStart;
    private string _sessionLabel = "";
    private string _sessionEdition = "";
    private CancellationTokenSource? _mcPollCts;

    private bool _isFullscreen = false;

    private DotNetObjectReference<Home>? _selfRef;

    // ── View-transition navigation ──────────────────────────────
    // Runs a view-state mutation inside a browser View Transition so the old
    // and new screens animate (crossfade / panel slide) instead of swapping
    // instantly. Falls back to an immediate swap when JS isn't ready yet,
    // animations are disabled, or a transition is already in flight.
    private Action? _pendingNav;
    private bool    _navBusy;

    private async Task NavigateAsync(Action mutate)
    {
        if (_selfRef == null || _navBusy)
        {
            mutate();
            StateHasChanged();
            return;
        }
        _navBusy    = true;
        _pendingNav = mutate;
        try { await JS.InvokeVoidAsync("glacierViewTransition", _selfRef); }
        catch { }
        finally
        {
            _navBusy = false;
            // JS never called back (old WebView2 runtime, interop failure) —
            // apply the pending mutation directly so navigation always works.
            if (_pendingNav != null)
            {
                var m = _pendingNav;
                _pendingNav = null;
                m();
                StateHasChanged();
            }
        }
    }

    [JSInvokable]
    public async Task ApplyPendingNav()
    {
        await InvokeAsync(() =>
        {
            var m = _pendingNav;
            _pendingNav = null;
            m?.Invoke();
            StateHasChanged();
        });
    }

    private static readonly string[] _accentSwatches =
        { "#7289da", "#5865f2", "#3ba55d", "#faa61a", "#f04747", "#eb459e", "#00d8ff", "#ff7043",
          "#a855f7", "#22d3ee", "#84cc16", "#f59e0b" };

    private record NewsItem(string Icon, string Title, string Subtitle, string Url);
    private static readonly NewsItem[] _newsItems =
    {
        new("fa-brands fa-discord",     "Join Our Discord",       "discord.glacierclient.xyz",  "https://discord.glacierclient.xyz"),
        new("fa-solid fa-globe",        "Visit Our Website",      "glacierclient.xyz",          "https://glacierclient.xyz"),
        new("fa-solid fa-layer-group",  "Glacier Texture Pack",   "Download for free",          "https://glacierclient.xyz/#downloads"),
        new("fa-solid fa-star",         "Star Us on GitHub",      "github.com/Glacier-Client-BE","https://github.com/Glacier-Client-BE/Glacier-Launcher"),
        new("fa-solid fa-newspaper",    "Latest Release",         "Check the Versions tab",    ""),
        new("fa-solid fa-shield-halved","About Glacier",          "Made with ❤ by Glacier",   ""),
    };

    private record SearchResult(string Icon, string Label, string Category, string Sub, string Shortcut, Action OnSelect)
    {
        public SearchResult(string Icon, string Label, string Category, string Sub, Action OnSelect)
            : this(Icon, Label, Category, Sub, "", OnSelect) {}
    }

    private sealed record SearchGroup(string Category, List<SearchResult> Results);

    private List<SearchResult> _defaultSearchResults = new();
    private List<SearchGroup> _searchGroups = new();

    protected override void OnInitialized()
    {
        if (!string.IsNullOrEmpty(SettingsService.Settings.Username))
            displayName = SettingsService.Settings.Username;
        if (!string.IsNullOrEmpty(SettingsService.Settings.UserHandle))
            displayHandle = SettingsService.Settings.UserHandle;
        BuildDefaultSearchResults();
    }

    protected override async Task OnAfterRenderAsync(bool firstRender)
    {
        if (!firstRender) return;
        _selfRef = DotNetObjectReference.Create(this);
        await JS.InvokeVoidAsync("glacierRegisterDropHandler", _selfRef);
        await JS.InvokeVoidAsync("applyStoredSettings",
            SettingsService.Settings.AccentColor,
            SettingsService.Settings.ThemePreset,
            SettingsService.Settings.BlurIntensity,
            SettingsService.Settings.CustomBackgroundPath,
            SettingsService.Settings.CompactMode,
            SettingsService.Settings.AnimationsEnabled,
            SettingsService.Settings.AnimationSpeed);
        if (SettingsService.Settings.UiScalePct != 100)
            await JS.InvokeVoidAsync("setUiScale", SettingsService.Settings.UiScalePct);

        // A custom theme overrides the plain preset look entirely; otherwise
        // the standalone font / background-fit settings apply.
        var activeTheme = ThemeSvc.Get(SettingsService.Settings.ActiveThemeId);
        if (activeTheme != null)
        {
            await ThemeSvc.ApplyAsync(JS, activeTheme);
        }
        else
        {
            if (!string.IsNullOrEmpty(SettingsService.Settings.FontFamily))
                await JS.InvokeVoidAsync("setFont", SettingsService.Settings.FontFamily);
            if (SettingsService.Settings.BackgroundFit != "cover")
                await JS.InvokeVoidAsync("setBackgroundFit", SettingsService.Settings.BackgroundFit, 1.0);
        }
        clientsHasBadge = !Flarial.IsDownloaded || !OderSo.IsDownloaded;

        if (!string.IsNullOrEmpty(SettingsService.Settings.LastUpdateCheck))
            lastUpdateCheckLabel = "Last checked " + FormatRelativeTime(SettingsService.Settings.LastUpdateCheck);

        StateHasChanged();

        _ = RunStartupChecksAsync();
        _ = StartMcPollAsync();
    }

    private async Task StartMcPollAsync()
    {
        _mcPollCts?.Cancel();
        _mcPollCts = new CancellationTokenSource();
        var token = _mcPollCts.Token;
        try
        {
            while (!token.IsCancellationRequested)
            {
                bool running = false;
                try
                {
                    running = IsBedrock
                        ? GameLauncher.IsMinecraftRunning()
                        : JavaGameLauncher.IsRunning();
                }
                catch { }
                if (running != mcRunning)
                {
                    bool justExited = mcRunning && !running;
                    var exitedEdition = _sessionEdition;
                    mcRunning = running;
                    AccumulatePlaytime(running);
                    await InvokeAsync(StateHasChanged);
                    // Surface a crash toast if Java left a fresh crash report behind.
                    if (justExited && exitedEdition == "java")
                        _ = CheckForCrashAsync();
                }
                await Task.Delay(running ? 2500 : 4000, token);
            }
        }
        catch (TaskCanceledException) { }
    }

    [JSInvokable]
    public void OnDllDropped(string path)
    {
        customDllPath = path;
        SelectClient("Custom DLL");
        currentView = "clients";
        InvokeAsync(StateHasChanged);
    }

    [JSInvokable]
    public async Task OnFileDropped(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return;
        var ext = System.IO.Path.GetExtension(path).ToLowerInvariant();
        if (ext == ".dll")
        {
            OnDllDropped(path);
            return;
        }
        if (ext == ".mrpack")
        {
            SetEditionCore("java");
            _ = ShowToast("Installing modpack…", "info");
            try
            {
                var cts = new CancellationTokenSource();
                var instance = await Modpacks.InstallMrpackFileAsync(
                    path, System.IO.Path.GetFileNameWithoutExtension(path), null, cts.Token);
                OnModpackInstalled(instance.Id);
            }
            catch (Exception ex) { _ = ShowToast("Modpack install failed: " + ex.Message, "error"); }
            await InvokeAsync(StateHasChanged);
            return;
        }
        if (IsJava && ext == ".zip")
        {
            try
            {
                var instance = await JavaInstances.ImportModpackAsync(path);
                JavaInstances.SetActive(instance.Id);
                currentView = "addons";
                RefreshJavaInstanceFiles();
                _ = ShowToast("Modpack imported.", "success");
            }
            catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
            await InvokeAsync(StateHasChanged);
            return;
        }
        if (IsJava && ext == ".jar")
        {
            try
            {
                var dest = System.IO.Path.Combine(JavaInstances.PathFor("mods"), System.IO.Path.GetFileName(path));
                System.IO.Directory.CreateDirectory(System.IO.Path.GetDirectoryName(dest)!);
                System.IO.File.Copy(path, dest, true);
                currentView = "addons";
                RefreshJavaInstanceFiles();
                _ = ShowToast("Mod installed.", "success");
            }
            catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
            await InvokeAsync(StateHasChanged);
        }
    }

    [JSInvokable]
    public void OnFullscreenChanged(bool isFullscreen)
    {
        _isFullscreen = isFullscreen;
        InvokeAsync(StateHasChanged);
    }

    [JSInvokable]
    public void KbShortcut(string key)
    {
        InvokeAsync(async () =>
        {
            switch (key)
            {
                case "search":  if (!searchOpen) await OpenSearch(); else CloseSearch(); break;
                case "launch":  if (currentView == "home" && !isLaunching) await HandleLaunch(); break;
                case "settings": if (!searchOpen && currentView != "settings") OpenSettings(); break;
                case "clients":  if (!searchOpen && currentView != "clients")  await OpenClients(); break;
                case "addons":   if (!searchOpen && currentView != "addons")   OpenAddons(); break;
                case "servers":  if (!searchOpen && currentView != "servers")  OpenServers(); break;
                case "versions": if (!searchOpen && currentView != "versions") await OpenVersions(); break;
                case "mcversions": if (!searchOpen && currentView != "mcversions") await OpenMcVersions(); break;
                case "fullscreen": await ToggleFullscreen(); break;
                case "cycle":    if (!searchOpen) CycleClient(); break;
                case "cycletheme":  await CycleTheme();      break;
                case "cycleaccent": await CycleAccent();     break;
                case "diagnostics": await CopyDiagnostics(); break;
                case "refresh":
                    if (currentView == "versions") await RefreshVersionsAsync();
                    else if (currentView == "clients") await OpenClients();
                    else if (currentView == "addons" && CurseForge.IsAvailable) await CfSearchAsync();
                    break;
                case "escape":
                    if      (resetConfirmOpen) CancelResetSettings();
                    else if (serverModalOpen)  CloseServerModal();
                    else if (searchOpen)       CloseSearch();
                    else if (launcherUpdateModalOpen) CloseUpdateModal();
                    else if (xboxModalOpen)    CloseXboxModal();
                    else if (discordModalOpen) CloseDiscordModal();
                    else if (currentView == "versions") await BackFromVersions();
                    else if (currentView != "home") GoHome();
                    break;
                case "down":  if (searchOpen) searchSelIdx = Math.Min(searchSelIdx + 1, GetCurrentSearchList().Count - 1); break;
                case "up":    if (searchOpen) searchSelIdx = Math.Max(searchSelIdx - 1, 0); break;
                case "enter":
                    if (searchOpen)
                    {
                        var list = GetCurrentSearchList();
                        if (searchSelIdx >= 0 && searchSelIdx < list.Count) list[searchSelIdx].OnSelect();
                    }
                    break;
            }
            StateHasChanged();
        });
    }

    private List<SearchResult> GetCurrentSearchList() =>
        searchResults.Count > 0 ? searchResults : _defaultSearchResults;

    public void Dispose()
    {
        _selfRef?.Dispose();
        _statusCts?.Cancel();
        _statusCts?.Dispose();
        _mcPollCts?.Cancel();
        _mcPollCts?.Dispose();
        _cfDebounceCts?.Cancel();
        _cfDebounceCts?.Dispose();
        _mrDebounceCts?.Cancel();
        _mrDebounceCts?.Dispose();
        _launchCts?.Cancel();
        _launchCts?.Dispose();
        _serverPingCts?.Cancel();
        _serverPingCts?.Dispose();
    }

    private async Task OnDragStart()    => await JS.InvokeVoidAsync("startDrag");
    private async Task CloseWindow()    => await JS.InvokeVoidAsync("closeWindow");
    private async Task MaximizeWindow() => await JS.InvokeVoidAsync("maximizeWindow");
    private async Task ToggleFullscreen() => await JS.InvokeVoidAsync("toggleFullscreen");
    private async Task MinimizeWindow()
    {
        if (SettingsService.Settings.MinimizeToTray)
            await JS.InvokeVoidAsync("minimizeToTray");
        else
            await JS.InvokeVoidAsync("minimizeWindow");
    }

    private void GoHome() =>
        _ = NavigateAsync(() => { currentView = "home"; Discord.SetIdlePresence(); });

    private async Task OpenSearch()
    {
        searchOpen = true; searchQuery = ""; searchResults.Clear(); searchSelIdx = 0;
        StateHasChanged();
        await JS.InvokeVoidAsync("focusSearchInput");
    }

    private void CloseSearch()
    {
        searchOpen = false; searchQuery = ""; searchResults.Clear(); _searchGroups.Clear(); searchSelIdx = 0;
    }

    private void OnSearchInput(ChangeEventArgs e)
    {
        searchQuery = e.Value?.ToString() ?? "";
        searchSelIdx = 0;
        if (string.IsNullOrWhiteSpace(searchQuery)) { searchResults.Clear(); _searchGroups.Clear(); return; }

        var q = searchQuery.Trim();
        searchResults = BuildDynamicSearchPool()
            .Where(r => r.Label.Contains(q, StringComparison.OrdinalIgnoreCase)
                     || r.Sub.Contains(q, StringComparison.OrdinalIgnoreCase)
                     || r.Category.Contains(q, StringComparison.OrdinalIgnoreCase))
            .Take(10).ToList();
        _searchGroups = BuildSearchGroups(searchResults);
    }

    private static List<SearchGroup> BuildSearchGroups(List<SearchResult> results)
    {
        var groups = new List<SearchGroup>();
        foreach (var result in results)
        {
            var group = groups.FirstOrDefault(g => g.Category == result.Category);
            if (group == null)
            {
                group = new SearchGroup(result.Category, new List<SearchResult>());
                groups.Add(group);
            }
            group.Results.Add(result);
        }
        return groups;
    }

    private async Task ShowStatus(string message, bool error = false, int clearAfterMs = 4000)
    {
        _statusCts?.Cancel();
        _statusCts = new CancellationTokenSource();
        var token = _statusCts.Token;
        isError = error; statusMessage = message; statusVisible = true;
        StateHasChanged();
        try { await Task.Delay(clearAfterMs, token); statusVisible = false; StateHasChanged(); }
        catch (TaskCanceledException) { }
    }

    private async Task ShowToast(string message, string kind = "info")
    {
        var icon = kind switch { "success" => "fa-solid fa-circle-check", "error" => "fa-solid fa-circle-xmark", _ => "fa-solid fa-circle-info" };
        var t = new ToastItem(message, kind, icon);
        _toasts.Add(t);
        StateHasChanged();
        await Task.Delay(2800);
        t.Closing = true;
        StateHasChanged();
        await Task.Delay(240);
        _toasts.Remove(t);
        StateHasChanged();
    }

    private void AddToRecentlyLaunched(string label)
    {
        if (string.IsNullOrEmpty(label)) return;
        var rec = SettingsService.Settings.RecentlyLaunched;
        rec.Remove(label);
        rec.Insert(0, label);
        while (rec.Count > 5) rec.RemoveAt(rec.Count - 1);
        SettingsService.Save();
    }

    private List<string> RecentsForCurrentEdition()
    {
        var all = SettingsService.Settings.RecentlyLaunched;
        if (IsJava)
            return all.Where(r => r.StartsWith("Java ", StringComparison.Ordinal)).ToList();
        return all.Where(r => !r.StartsWith("Java ", StringComparison.Ordinal)).ToList();
    }

    private string RecentDisplayLabel(string raw) =>
        raw.StartsWith("Java ", StringComparison.Ordinal) ? raw[5..] : raw;

    private void OpenUrl(string url)
    {
        if (!string.IsNullOrEmpty(url))
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            { FileName = url, UseShellExecute = true });
    }

    private static string FormatRelativeTime(string isoTimestamp)
    {
        try
        {
            var dt   = DateTime.Parse(isoTimestamp, null, System.Globalization.DateTimeStyles.RoundtripKind);
            var diff = DateTime.UtcNow - dt;
            if (diff.TotalMinutes < 2)  return "just now";
            if (diff.TotalHours   < 1)  return $"{(int)diff.TotalMinutes}m ago";
            if (diff.TotalDays    < 1)  return $"{(int)diff.TotalHours}h ago";
            if (diff.TotalDays    < 7)  return $"{(int)diff.TotalDays}d ago";
            return dt.ToString("MMM d");
        }
        catch { return ""; }
    }

    private static string FormatSize(long bytes)
    {
        if (bytes <= 0) return "0 B";
        string[] units = { "B", "KB", "MB", "GB" };
        double value = bytes;
        int unit = 0;
        while (value >= 1024 && unit < units.Length - 1) { value /= 1024; unit++; }
        return $"{value:0.##} {units[unit]}";
    }

    private static string GetClientIcon(string client) => client switch
    {
        "Custom DLL"     => "fa-solid fa-file-code",
        "Vanilla"        => "fa-solid fa-cube",
        _                => "",
    };

    private static string GetClientImagePath(string client) => client switch
    {
        "Flarial Client" => "images/clients/flarial.svg",
        "OderSo Client"  => "images/clients/oderso.png",
        _                => "images/clients/latite.png",
    };

    private static bool IsImageClient(string client) =>
        client is "Latite Client" or "Flarial Client" or "OderSo Client";

    private static string FormatChangelog(string md)
    {
        var html = System.Net.WebUtility.HtmlEncode(md);
        html = System.Text.RegularExpressions.Regex.Replace(html, @"\*\*(.+?)\*\*", "<strong>$1</strong>");
        html = System.Text.RegularExpressions.Regex.Replace(html, @"\[([^\]]+)\]\((https?://[^\)]+)\)", "<a href=\"$2\" target=\"_blank\" rel=\"noopener\" style=\"color:var(--accent);text-decoration:underline;cursor:pointer;\">$1</a>");
        html = html.Replace("\r\n", "<br/>").Replace("\n", "<br/>");
        return html;
    }

    private static string FormatDownloads(long count) => count switch
    {
        >= 1_000_000 => $"{count / 1_000_000.0:F1}M downloads",
        >= 1_000     => $"{count / 1_000.0:F1}K downloads",
        _            => $"{count} downloads"
    };

    private string GetFooterVersionLabel()
    {
        var client = SettingsService.Settings.SelectedClient;
        return client switch
        {
            "Flarial Client" => "Flarial Client",
            "OderSo Client"  => "OderSo Client",
            "Vanilla"        => "Vanilla",
            "Custom DLL"     => !string.IsNullOrEmpty(customDllPath)
                                ? System.IO.Path.GetFileNameWithoutExtension(customDllPath)
                                : "Custom DLL",
            _ => !string.IsNullOrEmpty(SettingsService.Settings.LastUsedVersion)
                 ? SettingsService.Settings.LastUsedVersion
                 : "Latite Client",
        };
    }

    private string GetAvatarSrc()
    {
        if (IsJava && !string.IsNullOrEmpty(SettingsService.Settings.JavaUuid))
        {
            // ?v= keeps the avatar in step with skin changes made through the
            // launcher — without it the WebView cache pins the old head.
            var ticks = SettingsService.Settings.SkinChangedTicks;
            return "https://mc-heads.net/avatar/" + SettingsService.Settings.JavaUuid.Replace("-", "")
                 + (ticks > 0 ? "?v=" + ticks : "");
        }
        var which = EffectiveProfile();
        if (which == "xbox" && !string.IsNullOrEmpty(Xbox.CurrentProfile?.GamerPictureUrl))
            return Xbox.CurrentProfile.GamerPictureUrl;
        if (which == "discord" && !string.IsNullOrEmpty(SettingsService.Settings.DiscordAvatar))
            return SettingsService.Settings.DiscordAvatar;
        return "images/icon.png";
    }

    private string EffectiveProfile()
    {
        var mode = SettingsService.Settings.ProfileDisplayMode;
        switch (mode)
        {
            case "xbox":    return Xbox.IsSignedIn ? "xbox" : "none";
            case "discord": return SettingsService.Settings.DiscordLoggedIn ? "discord" : "none";
            default:
                if (Xbox.IsSignedIn) return "xbox";
                if (SettingsService.Settings.DiscordLoggedIn) return "discord";
                return "none";
        }
    }

    private void CycleProfileDisplay()
    {
        var next = SettingsService.Settings.ProfileDisplayMode switch
        {
            "auto"    => "xbox",
            "xbox"    => "discord",
            "discord" => "auto",
            _          => "auto"
        };
        SettingsService.Settings.ProfileDisplayMode = next;
        SettingsService.Save();
        _ = ShowToast(next switch
        {
            "xbox"    => "Footer profile: Xbox",
            "discord" => "Footer profile: Discord",
            _          => "Footer profile: Auto"
        }, "info");
        StateHasChanged();
    }

    private string ProfileSwitchTooltip() => SettingsService.Settings.ProfileDisplayMode switch
    {
        "xbox"    => "Showing Xbox — click to switch to Discord",
        "discord" => "Showing Discord — click to switch to Auto",
        _          => "Auto (prefers Xbox) — click to switch to Xbox"
    };

    private void OpenActiveProfileModal()
    {
        switch (EffectiveProfile())
        {
            case "discord": OpenDiscordModal(); break;
            case "xbox":
            default:        OpenXboxModal();    break;
        }
    }

    private void SetEdition(string ed)
    {
        if (string.Equals(SettingsService.Settings.Edition, ed, StringComparison.OrdinalIgnoreCase))
            return;

        _ = NavigateAsync(() =>
        {
            SetEditionCore(ed);
            currentView = "home";
        });
    }

    // Edition switch without the navigation reset — used by flows that land on
    // a specific Java view (profile, screenshots) instead of home.
    private void SetEditionCore(string ed)
    {
        if (string.Equals(SettingsService.Settings.Edition, ed, StringComparison.OrdinalIgnoreCase))
            return;

        SettingsService.Settings.Edition = ed;
        SettingsService.Save();

        cfCategory = "all";
        cfResults.Clear();
        cfHasSearched = false;
        settingsCategory = "all";
    }

    private string FormatMarkdown(string text)
    {
        if (string.IsNullOrWhiteSpace(text)) return "";
        try
        {
            // Escape HTML first
            text = System.Net.WebUtility.HtmlEncode(text);

            // Bold: **text**
            text = System.Text.RegularExpressions.Regex.Replace(text, @"\*\*(.*?)\*\*", "<strong>$1</strong>");
            
            // Links: [text](url)
            text = System.Text.RegularExpressions.Regex.Replace(text, @"\[(.*?)\]\((.*?)\)", "<a href=\"$2\" target=\"_blank\">$1</a>");

            // Lists: - item
            text = System.Text.RegularExpressions.Regex.Replace(text, @"^\s*-\s+(.*)$", "<li>$1</li>", System.Text.RegularExpressions.RegexOptions.Multiline);
            text = System.Text.RegularExpressions.Regex.Replace(text, @"(<li>.*</li>)", "<ul>$1</ul>", System.Text.RegularExpressions.RegexOptions.Singleline);

            // Line breaks
            text = text.Replace("\n", "<br/>");

            return text;
        }
        catch { return text; }
    }

    private void ToggleDiscordRpc()
    {
        SettingsService.Settings.DiscordRichPresence = !SettingsService.Settings.DiscordRichPresence;
        SettingsService.Save();
        _ = ShowToast(SettingsService.Settings.DiscordRichPresence ? "Discord RPC enabled" : "Discord RPC disabled", "success");
        StateHasChanged();
    }

    private string Highlight(string text, string query)
    {
        if (string.IsNullOrWhiteSpace(query)) return text;
        try
        {
            var parts = System.Text.RegularExpressions.Regex.Split(text, $"({System.Text.RegularExpressions.Regex.Escape(query)})", System.Text.RegularExpressions.RegexOptions.IgnoreCase);
            var sb = new System.Text.StringBuilder();
            foreach (var p in parts)
            {
                if (p.Equals(query, StringComparison.OrdinalIgnoreCase))
                    sb.Append("<span class=\"search-highlight\">").Append(p).Append("</span>");
                else
                    sb.Append(p);
            }
            return sb.ToString();
        }
        catch { return text; }
    }
}
