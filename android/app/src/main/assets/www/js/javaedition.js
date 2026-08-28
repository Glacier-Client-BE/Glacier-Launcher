// Real data sources for the Java-edition panels — same endpoints the desktop
// services use, called directly via fetch() since there's nothing Windows-
// specific about the network calls themselves (unlike launching, which the
// built-in Java Edition runtime owns on Android — see JavaEditionBridge.kt).

const MojangVersions = {
    MANIFEST_URL: "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",

    typeLabel(type) {
        switch (type) {
            case "release": return "Release";
            case "snapshot": return "Snapshot";
            case "old_beta": return "Beta";
            case "old_alpha": return "Alpha";
            default: return type;
        }
    },

    async fetchManifest() {
        const res = await fetch(this.MANIFEST_URL);
        if (!res.ok) throw new Error(`Mojang manifest returned ${res.status}`);
        return res.json(); // { latest: {release, snapshot}, versions: [{id, type, url, releaseTime}] }
    },
};

const GlacierClient = {
    MANIFEST_URL: "https://cdn.glacierclient.xyz/versions.json",

    async fetchManifest() {
        const res = await fetch(this.MANIFEST_URL);
        if (!res.ok) throw new Error(`Glacier manifest returned ${res.status}`);
        return res.json(); // { latestRelease, versions: [{id, name, tag, url, sha256, fabric, forge, changelog}] }
    },
};

// Mirrors NewsService.cs / AutoUpdateService.cs — same public endpoints,
// no auth needed for either.
const NewsFeed = {
    NEWS_URL: "https://glacierclient.xyz/news.json",
    RELEASES_URL: "https://api.github.com/repos/Glacier-Client-BE/Glacier-Launcher/releases?per_page=12",

    async fetchPosts() {
        const res = await fetch(this.NEWS_URL);
        if (!res.ok) throw new Error(`news.json returned ${res.status}`);
        return res.json(); // [{title, subtitle, url, icon}]
    },

    async fetchReleases() {
        const res = await fetch(this.RELEASES_URL);
        if (!res.ok) throw new Error(`GitHub releases returned ${res.status}`);
        const data = await res.json();
        return data.map(r => ({ tag: r.tag_name, publishedAt: r.published_at, body: r.body || "" }));
    },
};
