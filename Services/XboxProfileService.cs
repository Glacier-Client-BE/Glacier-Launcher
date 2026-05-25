using System;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

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

    public async Task<bool> SignInAsync(bool promptAccountPicker = false)
    {
        LastError = null;
        try
        {
            var liveToken = promptAccountPicker
                ? await _liveAuth.SignInInteractiveAsync(true).ConfigureAwait(false)
                : await _liveAuth.RefreshAsync().ConfigureAwait(false) ?? await _liveAuth.SignInInteractiveAsync().ConfigureAwait(false);

            if (liveToken is null)
            {
                LastError = "Sign-in was cancelled.";
                return false;
            }

            var (xblToken, _) = await ExchangeForXblTokenAsync(liveToken.AccessToken).ConfigureAwait(false);

            var (xstsToken, userHash) = await ExchangeForXstsTokenAsync(xblToken).ConfigureAwait(false);

            var (mcXstsToken, mcUserHash) = await ExchangeForXstsTokenAsync(xblToken, "rp://api.minecraftservices.com/").ConfigureAwait(false);

            var profile = await FetchProfileAsync(xstsToken, userHash).ConfigureAwait(false);
            if (profile is null)
            {
                LastError = "Could not load Xbox profile.";
                return false;
            }

            CurrentProfile = profile;
            PersistProfile(profile);

            var mcToken = await AuthenticateWithMinecraftAsync(mcXstsToken, mcUserHash).ConfigureAwait(false);
            if (mcToken is null)
            {
                LastError = "Xbox sign-in succeeded but Minecraft authentication failed.";
                return true;
            }

            var mcProfile = await FetchMinecraftProfileAsync(mcToken).ConfigureAwait(false);
            if (mcProfile is not null)
            {
                var s = _settings.Settings;
                s.JavaUsername          = mcProfile.Value.Name;
                s.JavaUuid             = mcProfile.Value.Uuid;
                s.JavaAccessToken      = mcToken;
                s.JavaAccessTokenExpiry = DateTime.UtcNow.AddHours(23).ToString("o");
                s.JavaSkinUrl          = mcProfile.Value.SkinUrl;
                PersistAccount(profile, mcProfile.Value, mcToken, liveToken.RefreshToken);
                _settings.Save();
            }

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
        s.JavaUsername = s.JavaUuid = s.JavaAccessToken = s.JavaAccessTokenExpiry = "";
        _settings.Save();
    }

    public async Task RefreshProfileAsync()
    {
        if (!IsSignedIn) return;
        await SignInAsync().ConfigureAwait(false);
    }

    public async Task ValidateSessionAsync()
    {
        var s = _settings.Settings;
        if (string.IsNullOrWhiteSpace(s.JavaAccessToken) || string.IsNullOrWhiteSpace(s.JavaAccessTokenExpiry))
            return;
        if (!DateTime.TryParse(s.JavaAccessTokenExpiry, null, System.Globalization.DateTimeStyles.RoundtripKind, out var expiry))
            return;
        if (DateTime.UtcNow < expiry.AddMinutes(-10))
            return;
        await SignInAsync().ConfigureAwait(false);
    }

    public bool SwitchAccount(string id)
    {
        var s = _settings.Settings;
        var account = s.JavaAccounts.FirstOrDefault(a => string.Equals(a.Id, id, StringComparison.OrdinalIgnoreCase));
        if (account == null)
            return false;
        s.ActiveJavaAccountId = account.Id;
        s.XboxLiveRefreshToken = account.RefreshToken;
        s.XboxGamertag = account.Gamertag;
        s.XboxXuid = account.Xuid;
        s.XboxGamerPictureUrl = account.GamerPictureUrl;
        s.JavaUsername = account.MinecraftUsername;
        s.JavaUuid = account.MinecraftUuid;
        s.JavaAccessToken = account.MinecraftAccessToken;
        s.JavaAccessTokenExpiry = account.MinecraftAccessTokenExpiry;
        s.JavaSkinUrl = account.MinecraftSkinUrl;
        account.LastUsedAt = DateTime.UtcNow.ToString("o");
        CurrentProfile = new XboxProfile { Gamertag = account.Gamertag, Xuid = account.Xuid, GamerPictureUrl = account.GamerPictureUrl };
        _settings.Save();
        return true;
    }

    public void RemoveAccount(string id)
    {
        var s = _settings.Settings;
        var account = s.JavaAccounts.FirstOrDefault(a => string.Equals(a.Id, id, StringComparison.OrdinalIgnoreCase));
        if (account == null)
            return;
        s.JavaAccounts.Remove(account);
        if (s.ActiveJavaAccountId == id)
            SignOut();
        _settings.Save();
    }

    private async Task<(string token, string userHash)> ExchangeForXblTokenAsync(string liveAccessToken)
    {
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

        return await PostXboxAuthAsync(XblAuthEndpoint, payload, contractVersion: "1").ConfigureAwait(false);
    }

    private Task<(string token, string userHash)> ExchangeForXstsTokenAsync(string xblToken,
        string relyingParty = "http://xboxlive.com")
    {
        var payload = JsonSerializer.Serialize(new
        {
            RelyingParty = relyingParty,
            TokenType    = "JWT",
            Properties   = new
            {
                SandboxId  = "RETAIL",
                UserTokens = new[] { xblToken }
            }
        });

        try
        {
            return PostXboxAuthAsync(XstsAuthEndpoint, payload, contractVersion: "1");
        }
        catch (XboxAuthException ex) when (!string.IsNullOrEmpty(ex.XErr))
        {
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

        using var resp = await _http.SendAsync(req).ConfigureAwait(false);

        if (!resp.IsSuccessStatusCode)
        {
            string? xErr = null;
            try
            {
                await using var errStream = await resp.Content.ReadAsStreamAsync().ConfigureAwait(false);
                using var errDoc = await JsonDocument.ParseAsync(errStream).ConfigureAwait(false);
                if (errDoc.RootElement.TryGetProperty("XErr", out var x))
                    xErr = x.ValueKind == JsonValueKind.Number ? x.GetInt64().ToString() : x.GetString();
            }
            catch { }

            throw new XboxAuthException(
                $"Xbox auth call failed ({(int)resp.StatusCode}).",
                xErr);
        }

        await using var stream = await resp.Content.ReadAsStreamAsync().ConfigureAwait(false);
        using var doc = await JsonDocument.ParseAsync(stream).ConfigureAwait(false);
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

        using var resp = await _http.SendAsync(req).ConfigureAwait(false);
        resp.EnsureSuccessStatusCode();

        await using var stream = await resp.Content.ReadAsStreamAsync().ConfigureAwait(false);
        using var doc = await JsonDocument.ParseAsync(stream).ConfigureAwait(false);

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

    private const string McAuthEndpoint    = "https://api.minecraftservices.com/authentication/login_with_xbox";
    private const string McProfileEndpoint = "https://api.minecraftservices.com/minecraft/profile";

    private async Task<string?> AuthenticateWithMinecraftAsync(string xstsToken, string userHash)
    {
        var payload = JsonSerializer.Serialize(new
        {
            identityToken = $"XBL3.0 x={userHash};{xstsToken}"
        });

        using var content = new StringContent(payload, Encoding.UTF8, "application/json");
        using var resp = await _http.PostAsync(McAuthEndpoint, content).ConfigureAwait(false);
        if (!resp.IsSuccessStatusCode) return null;

        await using var stream = await resp.Content.ReadAsStreamAsync().ConfigureAwait(false);
        using var doc = await JsonDocument.ParseAsync(stream).ConfigureAwait(false);
        return doc.RootElement.TryGetProperty("access_token", out var t) ? t.GetString() : null;
    }

    private readonly record struct MinecraftProfile(string Uuid, string Name, string SkinUrl);

    private async Task<MinecraftProfile?> FetchMinecraftProfileAsync(string mcAccessToken)
    {
        using var req = new HttpRequestMessage(HttpMethod.Get, McProfileEndpoint);
        req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", mcAccessToken);

        using var resp = await _http.SendAsync(req).ConfigureAwait(false);
        if (!resp.IsSuccessStatusCode) return null;

        await using var stream = await resp.Content.ReadAsStreamAsync().ConfigureAwait(false);
        using var doc = await JsonDocument.ParseAsync(stream).ConfigureAwait(false);
        var root = doc.RootElement;

        var id   = root.TryGetProperty("id",   out var i) ? i.GetString() : null;
        var name = root.TryGetProperty("name", out var n) ? n.GetString() : null;
        var skin = "";
        if (root.TryGetProperty("skins", out var skins) && skins.ValueKind == JsonValueKind.Array)
        {
            foreach (var item in skins.EnumerateArray())
            {
                if (item.TryGetProperty("url", out var u))
                {
                    skin = u.GetString() ?? "";
                    break;
                }
            }
        }

        if (string.IsNullOrEmpty(id) || string.IsNullOrEmpty(name)) return null;

        if (id.Length == 32 && !id.Contains('-'))
            id = $"{id[..8]}-{id[8..12]}-{id[12..16]}-{id[16..20]}-{id[20..]}";

        return new MinecraftProfile(id, name, skin);
    }

    private void PersistAccount(XboxProfile profile, MinecraftProfile mcProfile, string accessToken, string? refreshToken)
    {
        var s = _settings.Settings;
        var id = string.IsNullOrWhiteSpace(mcProfile.Uuid) ? profile.Xuid : mcProfile.Uuid;
        var account = s.JavaAccounts.FirstOrDefault(a => string.Equals(a.Id, id, StringComparison.OrdinalIgnoreCase));
        if (account == null)
        {
            account = new JavaAccount { Id = id };
            s.JavaAccounts.Add(account);
        }
        account.Gamertag = profile.Gamertag;
        account.Xuid = profile.Xuid;
        account.GamerPictureUrl = profile.GamerPictureUrl;
        account.MinecraftUsername = mcProfile.Name;
        account.MinecraftUuid = mcProfile.Uuid;
        account.MinecraftAccessToken = accessToken;
        account.MinecraftAccessTokenExpiry = DateTime.UtcNow.AddHours(23).ToString("o");
        account.MinecraftSkinUrl = mcProfile.SkinUrl;
        if (!string.IsNullOrWhiteSpace(refreshToken))
            account.RefreshToken = refreshToken;
        account.LastUsedAt = DateTime.UtcNow.ToString("o");
        s.ActiveJavaAccountId = account.Id;
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
