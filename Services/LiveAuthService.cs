using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using System.Windows;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Microsoft Live OAuth 2.0 (legacy) flow used by community Minecraft launchers
/// for Xbox Live authentication. The client id <c>00000000402b5328</c> together
/// with the scope <c>service::user.auth.xboxlive.com::MBI_SSL</c> yields a
/// compact RPS ticket consumable directly by user.auth.xboxlive.com with the
/// <c>t=&lt;token&gt;</c> form (vs. the v2.0 <c>d=&lt;token&gt;</c> form).
/// </summary>
public sealed class LiveAuthService
{
    private const string TokenUrl = "https://login.live.com/oauth20_token.srf";

    private readonly SettingsService _settings;
    private readonly HttpClient      _http;

    public LiveAuthService(SettingsService settings)
    {
        _settings = settings;
        _http     = HttpFactory.Shared;
    }

    public sealed record LiveToken(string AccessToken, string? RefreshToken, DateTime ExpiresAtUtc);

    /// <summary>
    /// Show the WebView2 popup, exchange the resulting code for tokens.
    /// Returns null if the user cancels.
    /// </summary>
    public async Task<LiveToken?> SignInInteractiveAsync(bool promptSelectAccount = false)
    {
        var owner = Application.Current?.MainWindow;
        var window = new LiveAuthWindow { Owner = owner, PromptSelectAccount = promptSelectAccount };
        window.Show();

        var code = await window.GetAuthorizationCodeAsync();
        if (string.IsNullOrEmpty(code)) return null;

        return await ExchangeCodeAsync(code);
    }

    /// <summary>
    /// Silent refresh — call before each Xbox API call. Returns null if there
    /// is no usable refresh token or the refresh failed.
    /// </summary>
    public async Task<LiveToken?> RefreshAsync()
    {
        var refresh = _settings.Settings.XboxLiveRefreshToken;
        if (string.IsNullOrEmpty(refresh)) return null;

        try
        {
            var form = new FormUrlEncodedContent(new Dictionary<string, string>
            {
                ["client_id"]     = LiveAuthWindow.ClientId,
                ["grant_type"]    = "refresh_token",
                ["refresh_token"] = refresh,
                ["scope"]         = LiveAuthWindow.Scope,
                ["redirect_uri"]  = LiveAuthWindow.RedirectUri,
            });

            using var resp = await _http.PostAsync(TokenUrl, form);
            if (!resp.IsSuccessStatusCode) return null;

            var token = ParseTokenResponse(await resp.Content.ReadAsStringAsync());
            PersistRefreshToken(token);
            return token;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Wipe the stored refresh token and the WebView2 cookie/profile folder so
    /// the next sign-in shows a fresh account picker.
    /// </summary>
    public void SignOut()
    {
        _settings.Settings.XboxLiveRefreshToken = "";
        _settings.Save();

        try
        {
            var folder = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                "Glacier Launcher", "auth-webview");
            if (Directory.Exists(folder))
                Directory.Delete(folder, recursive: true);
        }
        catch { /* the WebView2 process may still be holding files; best effort */ }
    }

    private async Task<LiveToken?> ExchangeCodeAsync(string code)
    {
        var form = new FormUrlEncodedContent(new Dictionary<string, string>
        {
            ["client_id"]    = LiveAuthWindow.ClientId,
            ["grant_type"]   = "authorization_code",
            ["code"]         = code,
            ["redirect_uri"] = LiveAuthWindow.RedirectUri,
            ["scope"]        = LiveAuthWindow.Scope,
        });

        using var resp = await _http.PostAsync(TokenUrl, form);
        var body = await resp.Content.ReadAsStringAsync();

        if (!resp.IsSuccessStatusCode)
            throw new InvalidOperationException(ExtractErrorMessage(body) ?? $"Token exchange failed ({(int)resp.StatusCode}).");

        var token = ParseTokenResponse(body);
        PersistRefreshToken(token);
        return token;
    }

    private void PersistRefreshToken(LiveToken token)
    {
        if (string.IsNullOrEmpty(token.RefreshToken)) return;
        _settings.Settings.XboxLiveRefreshToken = token.RefreshToken;
        _settings.Save();
    }

    private static LiveToken ParseTokenResponse(string json)
    {
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;

        var access  = root.GetProperty("access_token").GetString() ?? "";
        var refresh = root.TryGetProperty("refresh_token", out var r) ? r.GetString() : null;
        var expiresIn = root.TryGetProperty("expires_in", out var e) ? e.GetInt32() : 3600;

        return new LiveToken(access, refresh, DateTime.UtcNow.AddSeconds(expiresIn - 60));
    }

    private static string? ExtractErrorMessage(string body)
    {
        try
        {
            using var doc = JsonDocument.Parse(body);
            return doc.RootElement.TryGetProperty("error_description", out var d) ? d.GetString()
                 : doc.RootElement.TryGetProperty("error",             out var e) ? e.GetString()
                 : null;
        }
        catch { return null; }
    }
}
