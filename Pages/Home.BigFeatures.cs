using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Services;
using Microsoft.AspNetCore.Components;
using Microsoft.JSInterop;

namespace GlacierLauncher.Pages;

public partial class Home
{
    // ── Mod conflict / dependency detection ──────────────────────────────────
    private ModReport _modReport = ModReport.Empty;

    private async Task AnalyzeModsAsync()
    {
        var paths = javaMods.Select(m => m.Path).ToList();
        if (paths.Count == 0) { _modReport = ModReport.Empty; return; }

        _modReport = await Task.Run(() => JavaModAnalyzer.Analyze(paths));
        await InvokeAsync(StateHasChanged);
    }

    // ── Java game-files location toggle ──────────────────────────────────────
    private void ToggleJavaUseDotMinecraft()
    {
        SettingsService.Settings.JavaUseDotMinecraft = !SettingsService.Settings.JavaUseDotMinecraft;
        SettingsService.Save();
        StateHasChanged();
    }

    // ── Shared Java panel tab bar ────────────────────────────────────────────
    // One definition drives every Java panel so Profile (skin/cape) and Photos
    // (screenshots) are first-class tabs everywhere, not buttons buried in Versions.
    // Sequence numbers are LITERAL per tab (base offset + fixed sub-index), never a
    // runtime counter. Blazor's diff relies on stable, compile-time-style sequence
    // numbers; a mutating counter can make the renderer add/drop nodes unpredictably.
    private RenderFragment JavaTabs(string active) => builder =>
    {
        void Tab(int seq, string view, string icon, string label, Action go)
        {
            builder.OpenElement(seq, "button");
            builder.AddAttribute(seq + 1, "class", active == view ? "panel-tab active" : "panel-tab");
            builder.AddAttribute(seq + 2, "onclick", EventCallback.Factory.Create(this, go));
            builder.OpenElement(seq + 3, "i");
            builder.AddAttribute(seq + 4, "class", icon);
            builder.CloseElement();
            builder.AddContent(seq + 5, label);
            builder.CloseElement();
        }
        Tab(0,  "settings",  "fa-solid fa-gear",         "Settings",  OpenSettings);
        Tab(10, "launchers", "fa-solid fa-rocket",       "Launchers", OpenJavaClients);
        Tab(20, "mods",      "fa-solid fa-puzzle-piece", "Mods",      OpenAddons);
        Tab(30, "versions",  "fa-solid fa-box-archive",  "Versions",  () => { _ = OpenJavaVersions(); });
        Tab(40, "profile",   "fa-solid fa-user",         "Profile",   OpenJavaProfile);
        Tab(50, "photos",    "fa-solid fa-images",       "Photos",    OpenJavaScreenshots);
        Tab(60, "credits",   "fa-solid fa-heart",        "Credits",   OpenCredits);
    };

    // ── News + changelog ─────────────────────────────────────────────────────
    private List<NewsPost> _newsPosts = new();
    private List<LauncherRelease> _releases = new();
    private bool _newsLoading;

    private async Task OpenNews()
    {
        await NavigateAsync(() => currentView = "news");
        if (_releases.Count == 0 && _newsPosts.Count == 0) await LoadNewsAsync();
    }

    private async Task LoadNewsAsync()
    {
        _newsLoading = true; StateHasChanged();
        var newsTask = NewsService.GetNewsAsync();
        var relTask  = AutoUpdate.GetRecentReleasesAsync();
        await Task.WhenAll(newsTask, relTask);
        _newsPosts = newsTask.Result;
        _releases  = relTask.Result;
        _newsLoading = false; StateHasChanged();
    }

    // ── Per-instance RAM ─────────────────────────────────────────────────────
    private int _instanceMaxRam;

    private void LoadInstanceRam() => _instanceMaxRam = JavaInstances.ActiveInstance.MaxRamMb;

    private void OnInstanceMaxRamChange(ChangeEventArgs e)
    {
        int.TryParse(e.Value?.ToString(), out var v);
        if (v != 0) v = Math.Clamp(v, 512, 16384);
        _instanceMaxRam = v;
        JavaInstances.SetActiveRam(0, v);            // min stays global, max overridden
        _ = ShowToast(v > 0 ? $"This instance: {v} MB" : "Using global memory", "info");
        StateHasChanged();
    }

    // ── Player profile helpers ───────────────────────────────────────────────
    private string PlayerCleanUuid => (SettingsService.Settings.JavaUuid ?? "").Replace("-", "");

    private string PlayerName =>
        !string.IsNullOrEmpty(SettingsService.Settings.JavaUsername) ? SettingsService.Settings.JavaUsername
      : !string.IsNullOrEmpty(SettingsService.Settings.XboxGamertag) ? SettingsService.Settings.XboxGamertag
      : "Player";

    private void OpenPlayerNameMc()   => OpenUrl($"https://namemc.com/profile/{PlayerCleanUuid}");
    private void DownloadPlayerSkin() => OpenUrl($"https://crafatar.com/skins/{PlayerCleanUuid}");

    private async Task CopyPlayerUuid()
    {
        if (string.IsNullOrEmpty(PlayerCleanUuid)) return;
        await JS.InvokeVoidAsync("copyToClipboard", PlayerCleanUuid);
        _ = ShowToast("UUID copied", "success");
    }

    // ── Change skin (Minecraft Services API) ─────────────────────────────────
    private bool _changeSkinSlim
    {
        get => SettingsService.Settings.JavaSkinSlimModel;
        set { SettingsService.Settings.JavaSkinSlimModel = value; SettingsService.Save(); }
    }

    private async Task ChangeSkinAsync()
    {
        if (string.IsNullOrWhiteSpace(SettingsService.Settings.JavaAccessToken))
        {
            _ = ShowToast("Sign in with Microsoft first.", "error");
            return;
        }
        // Native picker: WebView2's <input type=file> can't hand back a real path,
        // which was the "skin file not found" bug.
        var path = await LauncherUtilityService.PickFileAsync(
            "Choose a skin PNG", "Minecraft skin (PNG)|*.png|All files|*.*");
        if (string.IsNullOrEmpty(path)) return;
        await ApplyPickedSkinAsync(path);
    }

    [JSInvokable]
    public Task OnSkinPicked(string path) => ApplyPickedSkinAsync(path);

    private async Task ApplyPickedSkinAsync(string path)
    {
        if (string.IsNullOrWhiteSpace(path)) return;
        if (!System.IO.File.Exists(path)) { _ = ShowToast("That skin file no longer exists.", "error"); return; }

        // Optimistic preview: the picked PNG IS the new skin, so show it in the
        // viewer/avatar immediately and let the upload run behind it. The wait
        // here is pure network (token refresh chain + Mojang POST can be several
        // seconds); there is nothing visual to wait for. Rolls back on failure.
        var prevPath  = SettingsService.Settings.LastAppliedSkinPath;
        var prevTicks = SettingsService.Settings.SkinChangedTicks;
        string stored = "";
        try { stored = await SkinLibrary.AddFromFileAsync(path, _changeSkinSlim); } catch { /* library copy is best-effort */ }
        MarkSkinChanged(string.IsNullOrEmpty(stored) ? path : stored);
        _ = ShowToast("Applying skin…", "info");
        await InvokeAsync(StateHasChanged);

        // Refresh the Minecraft token first — a stale one makes the upload no-op/401.
        await Xbox.ValidateSessionAsync();
        var err = await SkinService.UploadSkinAsync(SettingsService.Settings.JavaAccessToken, path, _changeSkinSlim);
        if (err == null)
        {
            _ = ShowToast("Skin updated! Saved to your library.", "success");
        }
        else
        {
            // Mojang said no — the account still wears the old skin, so put the
            // preview back and drop the library copy we just made.
            SettingsService.Settings.LastAppliedSkinPath = prevPath;
            SettingsService.Settings.SkinChangedTicks    = prevTicks;
            SettingsService.Save();
            if (!string.IsNullOrEmpty(stored)) SkinLibrary.Delete(stored);
            _ = ShowToast("Skin change failed: " + err, "error");
        }
        await InvokeAsync(StateHasChanged);
    }

    private async Task ResetSkinAsync()
    {
        var err = await SkinService.ResetSkinAsync(SettingsService.Settings.JavaAccessToken);
        if (err == null) MarkSkinChanged("");
        _ = ShowToast(err == null ? "Skin reset to default." : "Reset failed: " + err,
                      err == null ? "success" : "error");
    }

    /// <summary>
    /// Records that the account skin just changed: bumps the persisted
    /// cache-bust token (render CDNs / the WebView cache would otherwise keep
    /// showing the old skin) and remembers the local PNG so the 3D preview can
    /// show the new skin immediately.
    /// </summary>
    private void MarkSkinChanged(string localPngPath)
    {
        SettingsService.Settings.LastAppliedSkinPath = localPngPath;
        SettingsService.Settings.SkinChangedTicks    = DateTime.UtcNow.Ticks;
        SettingsService.Save();
    }

    // Memoized: this runs on every render of the profile view and hits the
    // filesystem to validate the path — only recompute when the skin changes.
    private string _skinUrlKey = "";
    private string _skinUrlCached = "";

    private string LastAppliedSkinUrl
    {
        get
        {
            var p     = SettingsService.Settings.LastAppliedSkinPath;
            var ticks = SettingsService.Settings.SkinChangedTicks;
            var key   = p + "|" + ticks;
            if (key == _skinUrlKey) return _skinUrlCached;

            // The local PNG is only authoritative while the render CDNs may
            // still be stale (minutes). Past that, defer to mc-heads — the
            // user may have changed their skin outside the launcher since.
            bool recent = ticks > 0 && ticks <= DateTime.MaxValue.Ticks
                && (DateTime.UtcNow - new DateTime(Math.Min(ticks, DateTime.MaxValue.Ticks), DateTimeKind.Utc)) < TimeSpan.FromHours(1);

            string url = "";
            if (recent && !string.IsNullOrEmpty(p) && System.IO.File.Exists(p))
            {
                url = FileToLocalUrl(p);
                if (!string.IsNullOrEmpty(url))
                    url += "?v=" + ticks;
            }
            _skinUrlKey = key;
            _skinUrlCached = url;
            return url;
        }
    }

    // ── Cape wardrobe ────────────────────────────────────────────────────────
    private List<CapeInfo> _capes = new();
    private bool _capesLoading;

    private bool NoCapeActive => _capes.Count > 0 && _capes.All(c => !c.Active);

    /// <summary>Texture URL of the worn cape (empty if none) — fed to the 3D skin viewer.</summary>
    private string ActiveCapeUrl => _capes.FirstOrDefault(c => c.Active)?.Url ?? "";

    private async Task LoadCapesAsync()
    {
        if (string.IsNullOrWhiteSpace(SettingsService.Settings.JavaAccessToken)) { _capes = new(); return; }
        _capesLoading = true; StateHasChanged();
        _capes = await SkinService.GetCapesAsync(SettingsService.Settings.JavaAccessToken);
        _capesLoading = false;
        await InvokeAsync(StateHasChanged);
    }

    private async Task SelectCapeAsync(CapeInfo? cape)
    {
        if (cape is { Active: true }) return;   // already worn
        var token = SettingsService.Settings.JavaAccessToken;
        var err = cape == null
            ? await SkinService.HideCapeAsync(token)
            : await SkinService.SetCapeAsync(token, cape.Id);

        if (err != null) { _ = ShowToast("Cape change failed: " + err, "error"); return; }
        _ = ShowToast(cape == null ? "Cape hidden." : $"Now wearing: {cape.Alias}", "success");
        await LoadCapesAsync();
    }
}
