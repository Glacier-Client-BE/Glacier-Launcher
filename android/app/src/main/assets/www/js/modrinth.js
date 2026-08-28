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

    async search(facetType, query, offset = 0, limit = 20) {
        const url = new URL(`${this.BASE_URL}/search`);
        url.searchParams.set("limit", limit);
        url.searchParams.set("offset", offset);
        if (query) url.searchParams.set("query", query);
        if (facetType) url.searchParams.set("facets", `[["project_type:${facetType}"]]`);
        const res = await fetch(url);
        if (!res.ok) throw new Error(`Modrinth returned ${res.status}`);
        return res.json(); // { hits: [...], total_hits }
    },
};
