using System;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Drives the Xbox Live sign-in flow:
///   1. Acquires a Live OAuth access token via <see cref="LiveAuthService"/>.
///   2. Trades it for an XBL user token (xbl.signin).
///   3. Trades that for an XSTS token bound to xboxlive.com (xsts.authorize).
///   4. Calls profile.xboxlive.com with <c>XBL3.0 x=&lt;uhs&gt;;&lt;xstsToken&gt;</c>.
///
/// Uses the legacy MBI_SSL ticket form ("t=&lt;token&gt;"), matching the
/// public-client flow the community Minecraft auth libraries use.
/// </summary>
public class XboxProfileService
{
    private const string XblAuthEndpoint  = "https://user.auth.xboxlive.com/user/authenticate";
    private const string XstsAuthEndpoint = "https://xsts.auth.xboxlive.com/xsts/authorize";
    private const string ProfileEndpoint  = "https://profile.xboxlive.com/users/me/profile/settings";

    private readonly SettingsService  _settings;
    private readonly LiveAuthService  _liveAuth;
    private readonly HttpClient       _http;

    public XboxProfile? CurrentProfile { get; private set; }
    public bool         IsSignedIn     => CurrentProfile != null;
    public string?      LastError      { get; private set; }

    public XboxProfileService(SettingsService settings, LiveAuthService liveAuth)
    {
        _settings = settings;
        _liveAuth = liveAuth;
        _http     = HttpFactory.Shared;

        // Hydrate the cached profile so the UI shows the user as signed in on
        // launch, before any network roundtrip.
        var s = _settings.Settings;
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
            // 1. Live OAuth: try silent refresh first, then prompt.
            var liveToken = await _liveAuth.RefreshAsync()
                         ?? await _liveAuth.SignInInteractiveAsync();

            if (liveToken is null)
            {
                LastError = "Sign-in was cancelled.";
                return false;
            }

            // 2 + 3. XBL + XSTS exchange.
            var (xblToken, _)               = await ExchangeForXblTokenAsync(liveToken.AccessToken);
            var (xstsToken, userHash)       = await ExchangeForXstsTokenAsync(xblToken);

            // 4. Fetch the public Xbox profile.
            var profile = await FetchProfileAsync(xstsToken, userHash);
            if (profile is null)
            {
                LastError = "Could not load Xbox profile.";
                return false;
            }

            CurrentProfile = profile;
            PersistProfile(profile);
            return true;
        }
        catch (XboxAuthException ex)
        {
            LastError = ex.Message;
            return false;
        }
        catch (Exception ex)
        {
            LastError = $"Sign-in failed: {ex.Message}";
            return false;
        }
    }

    public void SignOut()
    {
        _liveAuth.SignOut();
        CurrentProfile = null;

        var s = _settings.Settings;
        s.XboxGamertag = s.XboxXuid = s.XboxGamerPictureUrl = "";
        s.XboxGamerscore = s.XboxAccountTier = s.XboxBio = "";
        _settings.Save();
    }

    public async Task RefreshProfileAsync()
    {
        if (!IsSignedIn) return;
        await SignInAsync();
    }

    // ── XBL / XSTS ────────────────────────────────────────────────

    private async Task<(string token, string userHash)> ExchangeForXblTokenAsync(string liveAccessToken)
    {
        // MBI_SSL ticket form: "t=<token>". Used by community Minecraft launchers
        // for years (vs. the v2.0 endpoint which would require "d=<token>").
        var payload = JsonSerializer.Serialize(new
        {
            RelyingParty = "http://auth.xboxlive.com",
            TokenType    = "JWT",
            Properties   = new
            {
                AuthMethod = "RPS",
                SiteName   = "user.auth.xboxlive.com",
                RpsTicket  = $"t={liveAccessToken}"
            }
        });

        return await PostXboxAuthAsync(XblAuthEndpoint, payload, contractVersion: "1");
    }

    private async Task<(string token, string userHash)> ExchangeForXstsTokenAsync(string xblToken)
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

        try
        {
            return await PostXboxAuthAsync(XstsAuthEndpoint, payload, contractVersion: "1");
        }
        catch (XboxAuthException ex) when (!string.IsNullOrEmpty(ex.XErr))
        {
            // Translate the common XErr codes to actionable messages.
            throw new XboxAuthException(MapXErr(ex.XErr), ex.XErr);
        }
    }

    private async Task<(string token, string userHash)> PostXboxAuthAsync(
        string endpoint, string body, string contractVersion)
    {
        using var content = new StringContent(body, Encoding.UTF8, "application/json");
        content.Headers.ContentType = new MediaTypeHeaderValue("application/json");

        using var req = new HttpRequestMessage(HttpMethod.Post, endpoint) { Content = content };
        req.Headers.Add("x-xbl-contract-version", contractVersion);

        using var resp = await _http.SendAsync(req);
        var json = await resp.Content.ReadAsStringAsync();

        if (!resp.IsSuccessStatusCode)
        {
            string? xErr = null;
            try
            {
                using var errDoc = JsonDocument.Parse(json);
                if (errDoc.RootElement.TryGetProperty("XErr", out var x))
                    xErr = x.ValueKind == JsonValueKind.Number ? x.GetInt64().ToString() : x.GetString();
            }
            catch { }

            throw new XboxAuthException(
                $"Xbox auth call failed ({(int)resp.StatusCode}).",
                xErr);
        }

        using var doc = JsonDocument.Parse(json);
        var token = doc.RootElement.GetProperty("Token").GetString() ?? "";
        var uhs   = doc.RootElement
                       .GetProperty("DisplayClaims")
                       .GetProperty("xui")[0]
                       .GetProperty("uhs").GetString() ?? "";
        return (token, uhs);
    }

    private async Task<XboxProfile?> FetchProfileAsync(string xstsToken, string userHash)
    {
        var url = $"{ProfileEndpoint}?settings=Gamertag,GameDisplayPicRaw,Gamerscore,AccountTier,XboxOneRep,PreferredColor,RealName,Bio";

        using var req = new HttpRequestMessage(HttpMethod.Get, url);
        req.Headers.Authorization = new AuthenticationHeaderValue("XBL3.0", $"x={userHash};{xstsToken}");
        req.Headers.Add("x-xbl-contract-version", "3");

        using var resp = await _http.SendAsync(req);
        resp.EnsureSuccessStatusCode();

        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());

        var profile = new XboxProfile();
        var user = doc.RootElement.GetProperty("profileUsers")[0];

        foreach (var setting in user.GetProperty("settings").EnumerateArray())
        {
            var id    = setting.GetProperty("id").GetString() ?? "";
            var value = setting.GetProperty("value").GetString() ?? "";

            switch (id)
            {
                case "Gamertag":          profile.Gamertag        = value; break;
                case "GameDisplayPicRaw": profile.GamerPictureUrl = value; break;
                case "Gamerscore":        profile.Gamerscore      = value; break;
                case "AccountTier":       profile.AccountTier     = value; break;
                case "Bio":               profile.Bio             = value; break;
            }
        }

        if (user.TryGetProperty("id", out var xuid))
            profile.Xuid = xuid.GetString() ?? "";

        return string.IsNullOrEmpty(profile.Gamertag) ? null : profile;
    }

    private void PersistProfile(XboxProfile profile)
    {
        var s = _settings.Settings;
        s.XboxGamertag        = profile.Gamertag;
        s.XboxXuid            = profile.Xuid;
        s.XboxGamerPictureUrl = profile.GamerPictureUrl;
        s.XboxGamerscore      = profile.Gamerscore;
        s.XboxAccountTier     = profile.AccountTier;
        s.XboxBio             = profile.Bio;
        _settings.Save();
    }

    private static string MapXErr(string xErr) => xErr switch
    {
        "2148916233" => "This Microsoft account doesn't have an Xbox profile yet. Create one at xbox.com.",
        "2148916235" => "Xbox Live isn't available in your country/region.",
        "2148916236" => "This account needs adult verification.",
        "2148916237" => "This account needs adult verification (age verification required in your region).",
        "2148916238" => "This is a child account and needs to be added to a Family group by an adult.",
        _            => $"Xbox couldn't authorize this account (XErr {xErr})."
    };

    private sealed class XboxAuthException : Exception
    {
        public string? XErr { get; }
        public XboxAuthException(string message, string? xErr = null) : base(message) => XErr = xErr;
    }
}
