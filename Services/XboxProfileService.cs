using System;
using System.Linq;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text.Json;
using System.Threading.Tasks;
using Microsoft.Identity.Client;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public class XboxProfileService
{
    // Register at https://portal.azure.com → Entra ID → App registrations
    //   • Account types: Personal Microsoft accounts only
    //   • Platform: Mobile/Desktop → redirect URI  http://localhost
    //   • API permissions → Add → Xbox Live → XboxLive.signin (delegated)
    //   • Authentication → Advanced → Allow public client flows → Yes
    private const string ClientId  = "5e4740f0-43be-4e14-8054-078c9a13a76a";
    private const string Authority = "https://login.microsoftonline.com/consumers";
    private static readonly string[] Scopes = { "XboxLive.signin", "XboxLive.offline_access" };

    private const string XboxLiveAuthEndpoint = "https://user.auth.xboxlive.com/user/authenticate";
    private const string XstsAuthEndpoint     = "https://xsts.auth.xboxlive.com/xsts/authorize";
    private const string ProfileEndpoint      = "https://profile.xboxlive.com/users/me/profile/settings";

    private readonly SettingsService _settingsService;
    private readonly HttpClient      _httpClient;
    private readonly IPublicClientApplication _msalApp;

    public XboxProfile? CurrentProfile { get; private set; }
    public bool IsSignedIn => CurrentProfile != null;
    public string? LastError { get; private set; }

    public XboxProfileService(SettingsService settingsService)
    {
        _settingsService = settingsService;
        _httpClient      = HttpFactory.Shared;

        _msalApp = PublicClientApplicationBuilder
            .Create(ClientId)
            .WithAuthority(Authority)
            .WithRedirectUri("http://localhost")
            .Build();

        var s = _settingsService.Settings;
        if (!string.IsNullOrEmpty(s.XboxGamertag))
        {
            CurrentProfile = new XboxProfile
            {
                Gamertag        = s.XboxGamertag,
                Xuid            = s.XboxXuid,
                GamerPictureUrl = s.XboxGamerPictureUrl,
                Gamerscore      = s.XboxGamerscore,
                AccountTier     = s.XboxAccountTier,
                Bio             = s.XboxBio
            };
        }
    }

    public async Task<bool> SignInAsync()
    {
        LastError = null;
        try
        {
            var msaToken = await GetMsaTokenAsync();
            if (string.IsNullOrEmpty(msaToken))
            {
                LastError = "Could not obtain a Microsoft account token.";
                return false;
            }

            var (xblToken, userHash) = await AuthenticateXboxLiveAsync(msaToken);
            if (string.IsNullOrEmpty(xblToken))
            {
                LastError = "Xbox Live authentication failed.";
                return false;
            }

            var (xstsToken, xstsHash) = await GetXstsTokenAsync(xblToken);
            if (string.IsNullOrEmpty(xstsToken) || string.IsNullOrEmpty(xstsHash))
            {
                LastError = "XSTS authorization failed.";
                return false;
            }

            var profile = await FetchProfileAsync(xstsToken, xstsHash);
            if (profile == null)
            {
                LastError = "Could not fetch Xbox profile.";
                return false;
            }

            CurrentProfile = profile;
            _settingsService.Settings.XboxGamertag        = profile.Gamertag;
            _settingsService.Settings.XboxXuid            = profile.Xuid;
            _settingsService.Settings.XboxGamerPictureUrl = profile.GamerPictureUrl;
            _settingsService.Settings.XboxGamerscore      = profile.Gamerscore;
            _settingsService.Settings.XboxAccountTier     = profile.AccountTier;
            _settingsService.Settings.XboxBio             = profile.Bio;
            _settingsService.Save();

            return true;
        }
        catch (MsalServiceException ex) when (ex.ErrorCode == "invalid_client")
        {
            LastError = "Xbox sign-in not configured — the app needs a valid Azure AD client ID.";
            return false;
        }
        catch (MsalClientException ex) when (ex.ErrorCode == "authentication_canceled")
        {
            LastError = "Sign-in was cancelled.";
            return false;
        }
        catch (Exception ex)
        {
            LastError = ex.Message;
            return false;
        }
    }

    public void SignOut()
    {
        _ = Task.Run(async () =>
        {
            try
            {
                foreach (var account in await _msalApp.GetAccountsAsync())
                    await _msalApp.RemoveAsync(account);
            }
            catch { }
        });

        CurrentProfile = null;

        _settingsService.Settings.XboxGamertag        = "";
        _settingsService.Settings.XboxXuid            = "";
        _settingsService.Settings.XboxGamerPictureUrl = "";
        _settingsService.Settings.XboxGamerscore      = "";
        _settingsService.Settings.XboxAccountTier     = "";
        _settingsService.Settings.XboxBio             = "";
        _settingsService.Save();
    }

    public async Task RefreshProfileAsync()
    {
        if (!IsSignedIn) return;
        await SignInAsync();
    }

    private async Task<string?> GetMsaTokenAsync()
    {
        var accounts = await _msalApp.GetAccountsAsync();
        var first    = accounts.FirstOrDefault();

        if (first != null)
        {
            try
            {
                var silent = await _msalApp
                    .AcquireTokenSilent(Scopes, first)
                    .ExecuteAsync();
                return silent.AccessToken;
            }
            catch (MsalUiRequiredException) { }
        }

        var result = await _msalApp
            .AcquireTokenInteractive(Scopes)
            .WithPrompt(Prompt.SelectAccount)
            .ExecuteAsync();

        return result.AccessToken;
    }

    private async Task<(string? token, string? userHash)> AuthenticateXboxLiveAsync(string msaToken)
    {
        var payload = JsonSerializer.Serialize(new
        {
            RelyingParty = "http://auth.xboxlive.com",
            TokenType    = "JWT",
            Properties   = new
            {
                AuthMethod = "RPS",
                SiteName   = "user.auth.xboxlive.com",
                RpsTicket  = $"d={msaToken}"
            }
        });

        using var content = new StringContent(payload, System.Text.Encoding.UTF8, "application/json");
        content.Headers.ContentType = new MediaTypeHeaderValue("application/json");

        using var request = new HttpRequestMessage(HttpMethod.Post, XboxLiveAuthEndpoint) { Content = content };
        request.Headers.Add("x-xbl-contract-version", "1");

        using var resp = await _httpClient.SendAsync(request);
        resp.EnsureSuccessStatusCode();

        var json = await resp.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(json);

        var token    = doc.RootElement.GetProperty("Token").GetString();
        var userHash = doc.RootElement
            .GetProperty("DisplayClaims")
            .GetProperty("xui")[0]
            .GetProperty("uhs").GetString();

        return (token, userHash);
    }

    private async Task<(string? token, string? userHash)> GetXstsTokenAsync(string xblToken)
    {
        var payload = JsonSerializer.Serialize(new
        {
            RelyingParty = "http://xboxlive.com",
            TokenType    = "JWT",
            Properties   = new
            {
                SandboxId  = "RETAIL",
                UserTokens = new[] { xblToken }
            }
        });

        using var content = new StringContent(payload, System.Text.Encoding.UTF8, "application/json");
        content.Headers.ContentType = new MediaTypeHeaderValue("application/json");

        using var request = new HttpRequestMessage(HttpMethod.Post, XstsAuthEndpoint) { Content = content };
        request.Headers.Add("x-xbl-contract-version", "1");

        using var resp = await _httpClient.SendAsync(request);
        resp.EnsureSuccessStatusCode();

        var json = await resp.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(json);

        var token    = doc.RootElement.GetProperty("Token").GetString();
        var userHash = doc.RootElement
            .GetProperty("DisplayClaims")
            .GetProperty("xui")[0]
            .GetProperty("uhs").GetString();

        return (token, userHash);
    }

    private async Task<XboxProfile?> FetchProfileAsync(string xstsToken, string userHash)
    {
        var url = $"{ProfileEndpoint}?settings=Gamertag,GameDisplayPicRaw,Gamerscore,AccountTier,XboxOneRep,PreferredColor,RealName,Bio";

        using var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.Authorization = new AuthenticationHeaderValue("XBL3.0", $"x={userHash};{xstsToken}");
        request.Headers.Add("x-xbl-contract-version", "3");

        using var resp = await _httpClient.SendAsync(request);
        resp.EnsureSuccessStatusCode();

        var json = await resp.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(json);

        var profile = new XboxProfile();

        foreach (var setting in doc.RootElement.GetProperty("profileUsers")[0].GetProperty("settings").EnumerateArray())
        {
            var id    = setting.GetProperty("id").GetString() ?? "";
            var value = setting.GetProperty("value").GetString() ?? "";

            switch (id)
            {
                case "Gamertag":           profile.Gamertag        = value; break;
                case "GameDisplayPicRaw":  profile.GamerPictureUrl = value; break;
                case "Gamerscore":         profile.Gamerscore      = value; break;
                case "AccountTier":        profile.AccountTier     = value; break;
                case "Bio":                profile.Bio             = value; break;
            }
        }

        if (doc.RootElement.GetProperty("profileUsers")[0].TryGetProperty("id", out var xuidEl))
            profile.Xuid = xuidEl.GetString() ?? "";

        return string.IsNullOrEmpty(profile.Gamertag) ? null : profile;
    }
}
