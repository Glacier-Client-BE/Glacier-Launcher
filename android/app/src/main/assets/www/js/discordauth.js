// Real Discord "identify" login — the same authorization-code OAuth2 flow
// the desktop app's OpenDiscordOAuth() (Pages/Home.Panels.cs) runs against
// a local HTTP listener, ported to fetch(). The interactive part (Discord's
// real login page) happens in a native Dialog+WebView (MainActivity.kt's
// signInDiscord()), which hands back only the final authorization "code" —
// the token exchange and profile fetch happen here, same pattern as
// js/xboxauth.js's Microsoft flow.
//
// This is only for the profile switcher's username/avatar (identify scope).
// It is unrelated to Discord Rich Presence, which the desktop app drives
// over a local IPC pipe to the Discord desktop client — there is no
// Android equivalent for that, see android/README.md.
const DiscordAuth = {
    CLIENT_ID: "1482726422094024779",
    CLIENT_SECRET: "zwvBIpvo18qHSLUzlvG1AKfJKAkMLujc",
    REDIRECT_URI: "http://localhost:5000/callback",

    _pending: null,

    begin() {
        if (!Bridge.signInDiscord) {
            return Promise.reject(new Error("Sign-in isn't available in this preview (no native bridge)."));
        }
        return new Promise((resolve, reject) => {
            this._pending = { resolve, reject };
            Bridge.signInDiscord();
        });
    },

    // Called from native (MainActivity.kt's notifyDiscordSignInResult) once
    // the Discord login WebView reaches the redirect URI.
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
        const accessToken = await this._exchangeCode(code);
        const profile = await this._fetchProfile(accessToken);
        return { profile, accessToken };
    },

    async _exchangeCode(code) {
        const body = new URLSearchParams({
            client_id: this.CLIENT_ID,
            client_secret: this.CLIENT_SECRET,
            grant_type: "authorization_code",
            code,
            redirect_uri: this.REDIRECT_URI,
        });
        const resp = await fetch("https://discord.com/api/oauth2/token", {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.toString(),
        });
        const data = await resp.json().catch(() => ({}));
        if (!resp.ok) throw new Error(data.error_description || `Token exchange failed (HTTP ${resp.status}).`);
        return data.access_token;
    },

    async _fetchProfile(accessToken) {
        const resp = await fetch("https://discord.com/api/users/@me", {
            headers: { Authorization: `Bearer ${accessToken}` },
        });
        if (!resp.ok) throw new Error(`Couldn't load your Discord profile (HTTP ${resp.status}).`);
        const data = await resp.json();
        const avatarUrl = data.avatar
            ? `https://cdn.discordapp.com/avatars/${data.id}/${data.avatar}.png`
            : "";
        return { userId: data.id, username: data.username, avatarUrl };
    },
};
// See xboxauth.js's window.MicrosoftAuth comment — same reason this needs
// an explicit assignment (MainActivity.kt calls back via window.DiscordAuth).
window.DiscordAuth = DiscordAuth;
