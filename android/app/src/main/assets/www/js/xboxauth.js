// Real Microsoft/Xbox Live/Minecraft sign-in — the same legacy OAuth flow
// the desktop app's LiveAuthWindow.xaml.cs/LiveAuthService.cs and
// XboxProfileService.cs use, ported to fetch(). The interactive part (the
// Microsoft login pages, which need real user password entry the WebView
// isn't allowed to script) happens in a native Dialog+WebView
// (MainActivity.kt's signInMicrosoft()), which hands back only the final
// authorization "code" — everything after that (token exchange, Xbox Live
// user/XSTS auth, Minecraft auth, Minecraft profile) is real REST calls
// made here, no different from the CurseForge/Mojang/LeviLamina integrations
// elsewhere in this app.
//
// Same caveat as the Skin Library's Mojang lookups: these are browser
// fetch() calls from the WebView's file:// origin rather than a native
// HttpClient, so if any of these endpoints ever enforce CORS against
// browser requests, that surfaces as a real, visible error here rather
// than being silently worked around.
const MicrosoftAuth = {
    CLIENT_ID: "00000000402b5328",
    SCOPE: "service::user.auth.xboxlive.com::MBI_SSL",
    REDIRECT_URI: "https://login.live.com/oauth20_desktop.srf",

    _pending: null,

    begin() {
        if (!Bridge.signInMicrosoft) {
            return Promise.reject(new Error("Sign-in isn't available in this preview (no native bridge)."));
        }
        return new Promise((resolve, reject) => {
            this._pending = { resolve, reject };
            Bridge.signInMicrosoft();
        });
    },

    // Called from native (MainActivity.kt's notifySignInResult) once the
    // Microsoft login WebView reaches the redirect URI.
    _onCode(code) {
        if (!this._pending) return;
        const { resolve, reject } = this._pending;
        this._pending = null;
        this._completeSignIn(code).then(resolve, reject);
    },

    _onError(message) {
        if (!this._pending) return;
        const { reject } = this._pending;
        this._pending = null;
        reject(new Error(message));
    },

    async _completeSignIn(code) {
        const liveToken = await this._exchangeCode(code);
        const { token: xblToken } = await this._xboxAuth(
            "https://user.auth.xboxlive.com/user/authenticate",
            { RelyingParty: "http://auth.xboxlive.com", TokenType: "JWT", Properties: { AuthMethod: "RPS", SiteName: "user.auth.xboxlive.com", RpsTicket: `t=${liveToken.access_token}` } });

        const xboxXsts = await this._xboxAuth(
            "https://xsts.auth.xboxlive.com/xsts/authorize",
            { RelyingParty: "http://xboxlive.com", TokenType: "JWT", Properties: { SandboxId: "RETAIL", UserTokens: [xblToken] } });
        const mcXsts = await this._xboxAuth(
            "https://xsts.auth.xboxlive.com/xsts/authorize",
            { RelyingParty: "rp://api.minecraftservices.com/", TokenType: "JWT", Properties: { SandboxId: "RETAIL", UserTokens: [xblToken] } });

        const profile = await this._fetchXboxProfile(xboxXsts.token, xboxXsts.userHash);
        const mcAccessToken = await this._authenticateMinecraft(mcXsts.token, mcXsts.userHash);
        const mcProfile = mcAccessToken ? await this._fetchMinecraftProfile(mcAccessToken) : null;

        return { profile, mcProfile, mcAccessToken };
    },

    async _exchangeCode(code) {
        const body = new URLSearchParams({
            client_id: this.CLIENT_ID,
            grant_type: "authorization_code",
            code,
            redirect_uri: this.REDIRECT_URI,
            scope: this.SCOPE,
        });
        const resp = await fetch("https://login.live.com/oauth20_token.srf", {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.toString(),
        });
        const data = await resp.json().catch(() => ({}));
        if (!resp.ok) throw new Error(data.error_description || `Token exchange failed (HTTP ${resp.status}).`);
        return data;
    },

    async _xboxAuth(endpoint, payload) {
        const resp = await fetch(endpoint, {
            method: "POST",
            headers: { "Content-Type": "application/json", "x-xbl-contract-version": "1" },
            body: JSON.stringify(payload),
        });
        const data = await resp.json().catch(() => ({}));
        if (!resp.ok) {
            const xErr = data.XErr ? String(data.XErr) : null;
            throw new Error(xErr ? this._mapXErr(xErr) : `Xbox auth call failed (HTTP ${resp.status}).`);
        }
        return { token: data.Token, userHash: data.DisplayClaims.xui[0].uhs };
    },

    _mapXErr(xErr) {
        if (xErr === "2148916233") return "This Microsoft account has no Xbox Live profile — create one at xbox.com first.";
        if (xErr === "2148916238") return "This account is a child account and needs a family group to sign in.";
        return `Xbox authorization failed (${xErr}).`;
    },

    async _fetchXboxProfile(xstsToken, userHash) {
        const url = "https://profile.xboxlive.com/users/me/profile/settings?settings=Gamertag,GameDisplayPicRaw,Gamerscore,AccountTier,Bio";
        const resp = await fetch(url, {
            headers: { Authorization: `XBL3.0 x=${userHash};${xstsToken}`, "x-xbl-contract-version": "3" },
        });
        if (!resp.ok) throw new Error(`Couldn't load your Xbox profile (HTTP ${resp.status}).`);
        const data = await resp.json();
        const user = data.profileUsers[0];
        const profile = { xuid: user.id, gamertag: "", gamerPictureUrl: "", gamerscore: "", accountTier: "", bio: "" };
        for (const s of user.settings) {
            if (s.id === "Gamertag") profile.gamertag = s.value;
            else if (s.id === "GameDisplayPicRaw") profile.gamerPictureUrl = s.value;
            else if (s.id === "Gamerscore") profile.gamerscore = s.value;
            else if (s.id === "AccountTier") profile.accountTier = s.value;
            else if (s.id === "Bio") profile.bio = s.value;
        }
        return profile;
    },

    async _authenticateMinecraft(xstsToken, userHash) {
        const resp = await fetch("https://api.minecraftservices.com/authentication/login_with_xbox", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ identityToken: `XBL3.0 x=${userHash};${xstsToken}` }),
        });
        if (!resp.ok) return null;
        const data = await resp.json();
        return data.access_token || null;
    },

    async _fetchMinecraftProfile(mcAccessToken) {
        const resp = await fetch("https://api.minecraftservices.com/minecraft/profile", {
            headers: { Authorization: `Bearer ${mcAccessToken}` },
        });
        if (!resp.ok) return null;
        const data = await resp.json();
        const skin = Array.isArray(data.skins) ? data.skins.find(s => s.state === "ACTIVE") : null;
        return { uuid: data.id, name: data.name, skinUrl: skin ? skin.url : "" };
    },
};
