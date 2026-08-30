// Mirrors Services/ModrinthService.cs — same base URL, same facet/query
// params, no API key required (Modrinth's search API is public).
const Modrinth = {
    BASE_URL: "https://api.modrinth.com/v2",

    javaCategories: [
        { label: "Mods", icon: "fa-solid fa-puzzle-piece", facet: "mod" },
        { label: "Modpacks", icon: "fa-solid fa-cubes", facet: "modpack" },
        { label: "Resource Packs", icon: "fa-solid fa-palette", facet: "resourcepack" },
        { label: "Shaders", icon: "fa-solid fa-droplet", facet: "shader" },
    ],

    // Same 5-minute TTL cache as CurseForge.search() — see HttpCache in
    // js/httpcache.js.
    async search(facetType, query, offset = 0, limit = 20) {
        const key = `mr.search.${facetType || ""}.${offset}.${limit}.${query || ""}`;
        return HttpCache.fetch(key, 5 * 60 * 1000, async () => {
            const url = new URL(`${this.BASE_URL}/search`);
            url.searchParams.set("limit", limit);
            url.searchParams.set("offset", offset);
            if (query) url.searchParams.set("query", query);
            if (facetType) url.searchParams.set("facets", `[["project_type:${facetType}"]]`);
            const res = await fetch(url);
            if (!res.ok) throw new Error(`Modrinth returned ${res.status}`);
            return res.json(); // { hits: [...], total_hits }
        });
    },

    // Mirrors ModrinthService.cs's GetLatestVersionAsync/GetModpackFileAsync
    // — the latest version's primary file (falling back to the first file
    // if none is flagged primary), used to resolve a project id to a
    // downloadable .mrpack URL for ModpackInstall (js/modpackinstall.js).
    async getLatestFile(projectId) {
        const res = await fetch(`${this.BASE_URL}/project/${projectId}/version?limit=1`);
        if (!res.ok) throw new Error(`Modrinth returned ${res.status}`);
        const versions = await res.json();
        for (const ver of versions) {
            if (!Array.isArray(ver.files) || ver.files.length === 0) continue;
            const primary = ver.files.find(f => f.primary) || ver.files[0];
            return { url: primary.url, fileName: primary.filename, size: primary.size || 0 };
        }
        return null;
    },
};
