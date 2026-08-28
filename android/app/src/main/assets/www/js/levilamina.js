// Mirrors Services/LeviLaminaModsService.cs exactly: same registry (the
// real public "lip" package manager index LiteLDev/lipr publishes), same
// package-key parsing ("github.com/Owner/Repo"), same levilamina+mod tag
// filter, same latest-version resolution from the variants map.
const LeviLaminaMods = {
    REGISTRY_URL: "https://raw.githubusercontent.com/LiteLDev/lipr/main/index.json",
    _cache: null,

    async loadAll() {
        if (this._cache) return this._cache;
        const res = await fetch(this.REGISTRY_URL);
        if (!res.ok) throw new Error(`Registry returned ${res.status}`);
        const data = await res.json();
        const list = [];
        for (const [key, pkg] of Object.entries(data.packages || {})) {
            const parts = key.split("/");
            if (parts.length < 3 || parts[0].toLowerCase() !== "github.com") continue;
            const [, owner, repo] = parts;

            const info = pkg.info;
            if (!info) continue;
            const tags = (info.tags || []).map(t => t.toLowerCase());
            const isLevilaminaMod = tags.some(t => t.includes("levilamina")) && tags.some(t => t.includes("mod"));
            if (!isLevilaminaMod) continue;

            let latestVersion = "";
            for (const variant of Object.values(pkg.variants || {})) {
                if (Array.isArray(variant.versions) && variant.versions.length > 0) {
                    latestVersion = variant.versions[variant.versions.length - 1];
                    if (latestVersion) break;
                }
            }
            if (!latestVersion) continue;

            list.push({
                repoOwner: owner,
                repoName: repo,
                name: info.name || repo,
                description: info.description || "",
                avatarUrl: info.avatar_url || "",
                latestVersion,
                stars: pkg.stargazer_count || 0,
            });
        }
        list.sort((a, b) => b.stars - a.stars);
        this._cache = list;
        return list;
    },

    async search(query) {
        const all = await this.loadAll();
        if (!query) return all;
        const q = query.toLowerCase();
        return all.filter(m => m.name.toLowerCase().includes(q) || m.description.toLowerCase().includes(q));
    },
};
