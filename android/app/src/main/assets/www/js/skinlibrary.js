// Real port of Services/SkinLibraryService.cs's AddFromUsernameAsync flow:
// username -> UUID (api.mojang.com) -> signed texture URL + slim flag
// (sessionserver.mojang.com) -> the PNG itself, downloaded straight from
// Mojang's texture CDN. No third-party proxy (e.g. crafatar) is used, same
// as the desktop app.
//
// Unlike the desktop's native HttpClient, these fetch() calls run from the
// WebView's page origin, so if Mojang's endpoints ever enforce CORS for
// browser requests, this will surface as a real network error here rather
// than being silently proxied around - that failure is left visible to the
// user instead of being faked away.
const SkinLibrary = {
    async resolveUuid(name) {
        let resp;
        try {
            resp = await fetch(`https://api.mojang.com/users/profiles/minecraft/${encodeURIComponent(name)}`);
        } catch (e) {
            throw new Error("No internet connection, or Mojang is unreachable.");
        }
        if (resp.status === 204 || resp.status === 404) throw new Error(`No Minecraft player named '${name}'.`);
        if (!resp.ok) throw new Error(`Mojang is unavailable (HTTP ${resp.status}) - try again shortly.`);
        const data = await resp.json();
        if (!data || !data.id) throw new Error(`No Minecraft player named '${name}'.`);
        return data.id;
    },

    async resolveSkinTexture(uuid, name) {
        let resp;
        try {
            resp = await fetch(`https://sessionserver.mojang.com/session/minecraft/profile/${uuid}`);
        } catch (e) {
            throw new Error("No internet connection, or Mojang is unreachable.");
        }
        if (!resp.ok) throw new Error(`Mojang profile lookup failed (HTTP ${resp.status}).`);
        const data = await resp.json();
        const props = Array.isArray(data.properties) ? data.properties : [];
        const texturesProp = props.find(p => p.name === "textures");
        if (!texturesProp || !texturesProp.value) throw new Error(`Couldn't read the textures for '${name}'.`);

        let decoded;
        try {
            decoded = JSON.parse(atob(texturesProp.value));
        } catch (e) {
            throw new Error(`Couldn't read the textures for '${name}'.`);
        }
        const skin = decoded.textures && decoded.textures.SKIN;
        if (!skin || !skin.url) throw new Error(`'${name}' is using the default skin.`);
        const slim = !!(skin.metadata && String(skin.metadata.model).toLowerCase() === "slim");
        return { url: skin.url.replace("http://", "https://"), slim };
    },

    // Returns { name, url, slim } - the caller stores this in localStorage;
    // there's no filesystem library on Android, so the resolved texture URL
    // (Mojang's own CDN, stable/signed) is kept and re-fetched for preview
    // instead of downloaded bytes.
    async addFromUsername(username) {
        const name = (username || "").trim();
        if (!name) throw new Error("Enter a username.");
        const uuid = await this.resolveUuid(name);
        const { url, slim } = await this.resolveSkinTexture(uuid, name);
        return { name, url, slim, id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}` };
    },

    // Real port of Services/SkinService.cs's UploadSkinAsync — same endpoint,
    // same multipart shape. The desktop version reads PNG bytes off disk;
    // this app only ever has a texture URL (Mojang's own CDN), so it fetches
    // that URL into a Blob first instead of downloading to a local file.
    async applySkin(mcAccessToken, textureUrl, slim) {
        if (!mcAccessToken) return "Not signed in — sign in with Microsoft first.";
        let blob;
        try {
            const imgResp = await fetch(textureUrl);
            if (!imgResp.ok) return `Couldn't fetch the skin texture (HTTP ${imgResp.status}).`;
            blob = await imgResp.blob();
        } catch (e) {
            return "Couldn't download the skin texture - try again shortly.";
        }

        const form = new FormData();
        form.append("variant", slim ? "slim" : "classic");
        form.append("file", blob, "skin.png");

        try {
            const resp = await fetch("https://api.minecraftservices.com/minecraft/profile/skins", {
                method: "POST",
                headers: { Authorization: `Bearer ${mcAccessToken}` },
                body: form,
            });
            if (resp.ok) return null;
            if (resp.status === 401) return "Your session expired - re-sign in with Microsoft.";
            const body = await resp.text().catch(() => "");
            return `Minecraft rejected the skin (HTTP ${resp.status})${body ? `: ${body}` : "."}`;
        } catch (e) {
            return "No internet connection, or Minecraft services are unreachable.";
        }
    },
};
