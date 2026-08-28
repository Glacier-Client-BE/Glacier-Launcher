// Mirrors Services/CurseForgeService.cs — same base URL and game/class ids.
// The WebView's fetch() hits the same public API the desktop app's
// HttpClient does; no native code needed for this part.
const CurseForge = {
    BASE_URL: "https://api.curseforge.com",
    GAME_ID_BEDROCK: 78022,
    GAME_ID_JAVA: 432,

    BEDROCK_CLASS_ADDONS: 4984,
    BEDROCK_CLASS_MAPS: 6913,
    BEDROCK_CLASS_SKINS: 6925,
    BEDROCK_CLASS_TEXTURE_PACKS: 6929,
    BEDROCK_CLASS_SCRIPTS: 6940,

    JAVA_CLASS_MODS: 6,
    JAVA_CLASS_MODPACKS: 4471,
    JAVA_CLASS_RESOURCE_PACKS: 12,
    JAVA_CLASS_WORLDS: 17,
    JAVA_CLASS_SHADER_PACKS: 6552,

    bedrockCategories: [
        { label: "Addons", icon: "fa-solid fa-cube", classId: 4984 },
        { label: "Maps", icon: "fa-solid fa-map", classId: 6913 },
        { label: "Skins", icon: "fa-solid fa-shirt", classId: 6925 },
        { label: "Texture Packs", icon: "fa-solid fa-swatchbook", classId: 6929 },
        { label: "Scripts", icon: "fa-solid fa-code", classId: 6940 },
    ],

    javaCategories: [
        { label: "Mods", icon: "fa-solid fa-puzzle-piece", classId: 6 },
        { label: "Modpacks", icon: "fa-solid fa-cubes", classId: 4471 },
        { label: "Resource Packs", icon: "fa-solid fa-image", classId: 12 },
        { label: "Worlds", icon: "fa-solid fa-globe", classId: 17 },
        { label: "Shaders", icon: "fa-solid fa-sun", classId: 6552 },
    ],

    isAvailable() {
        return !!Bridge.curseForgeApiKey();
    },

    async search(gameId, classId, query, index = 0, pageSize = 20) {
        const url = new URL(`${this.BASE_URL}/v1/mods/search`);
        url.searchParams.set("gameId", gameId);
        url.searchParams.set("classId", classId);
        url.searchParams.set("searchFilter", query || "");
        url.searchParams.set("index", index);
        url.searchParams.set("pageSize", pageSize);
        url.searchParams.set("sortField", 2);
        url.searchParams.set("sortOrder", "desc");
        const res = await fetch(url, { headers: { "x-api-key": Bridge.curseForgeApiKey() } });
        if (!res.ok) throw new Error(`CurseForge returned ${res.status}`);
        return res.json();
    },
};
