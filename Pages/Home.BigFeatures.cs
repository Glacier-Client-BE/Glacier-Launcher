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
    private RenderFragment JavaTabs(string active) => builder =>
    {
        int seq = 0;
        void Tab(string view, string icon, string label, Action go)
        {
            builder.OpenElement(seq++, "button");
            builder.AddAttribute(seq++, "class", active == view ? "panel-tab active" : "panel-tab");
            builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(this, go));
            builder.OpenElement(seq++, "i");
            builder.AddAttribute(seq++, "class", icon);
            builder.CloseElement();
            builder.AddContent(seq++, label);
            builder.CloseElement();
        }
        Tab("settings",  "fa-solid fa-gear",         "Settings",  OpenSettings);
        Tab("launchers", "fa-solid fa-rocket",       "Launchers", OpenJavaClients);
        Tab("mods",      "fa-solid fa-puzzle-piece", "Mods",      OpenAddons);
        Tab("versions",  "fa-solid fa-box-archive",  "Versions",  () => { _ = OpenJavaVersions(); });
        Tab("profile",   "fa-solid fa-user",         "Profile",   OpenJavaProfile);
        Tab("photos",    "fa-solid fa-images",       "Photos",    OpenJavaScreenshots);
    };

    // ── News + changelog ─────────────────────────────────────────────────────
    private List<NewsPost> _newsPosts = new();
    private List<LauncherRelease> _releases = new();
    private bool _newsLoading;

    private async Task OpenNews()
    {
        currentView = "news";
        if (_releases.Count == 0 && _newsPosts.Count == 0) await LoadNewsAsync();
        else StateHasChanged();
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
    private bool _changeSkinSlim;

    private async Task ChangeSkinAsync()
    {
        if (string.IsNullOrWhiteSpace(SettingsService.Settings.JavaAccessToken))
        {
            _ = ShowToast("Sign in with Microsoft first.", "error");
            return;
        }
        if (_selfRef != null) await JS.InvokeVoidAsync("pickSkinFile", _selfRef);
    }

    [JSInvokable]
    public async Task OnSkinPicked(string path)
    {
        if (string.IsNullOrWhiteSpace(path)) return;
        _ = ShowToast("Uploading skin…", "info");
        var err = await SkinService.UploadSkinAsync(SettingsService.Settings.JavaAccessToken, path, _changeSkinSlim);
        _ = ShowToast(err == null ? "Skin updated! Give it a minute to appear." : "Skin change failed: " + err,
                      err == null ? "success" : "error");
        await InvokeAsync(StateHasChanged);
    }

    private async Task ResetSkinAsync()
    {
        var err = await SkinService.ResetSkinAsync(SettingsService.Settings.JavaAccessToken);
        _ = ShowToast(err == null ? "Skin reset to default." : "Reset failed: " + err,
                      err == null ? "success" : "error");
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
