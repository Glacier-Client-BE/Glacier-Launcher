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

// Same public community version database desktop's VanillaVersionService.cs
// reads (FetchFromCommunityDbAsync) — a plain text dump, no auth needed.
// Desktop also queries the Microsoft Store's Windows-Update SOAP API and can
// download/register/switch AppX packages; none of that has an Android
// equivalent (Bedrock here is a single always-current Play Store APK with
// no side-loadable version history), so this only sources the version
// *list* itself — see mcVersionInfoRowHtml (panels.js) for the read-only
// row this feeds.
const BedrockVersions = {
    DB_URL: "https://raw.githubusercontent.com/ddf8196/mc-w10-versiondb-auto-update/master/versions.txt",

    NAMED_SCHEME_RE: /[_-]v(\d{1,3}\.\d{1,3}(?:\.\d{1,3})?)(?=[_-]|$)/i,
    VERSION_RE: /(\d+\.\d+\.\d+(?:\.\d+)?)/,

    parseVersion(packageName) {
        const named = packageName.match(this.NAMED_SCHEME_RE);
        if (named) return named[1];
        const m = packageName.match(this.VERSION_RE);
        return m ? m[1] : null;
    },

    async fetch() {
        const res = await fetch(this.DB_URL);
        if (!res.ok) throw new Error(`Version database returned ${res.status}`);
        const text = await res.text();

        const seen = new Map();
        for (const rawLine of text.split("\n")) {
            const line = rawLine.trim();
            if (!line) continue;
            if (/^(Releases|Betas|Previews)/i.test(line)) continue;

            const spaceIdx = line.indexOf(" ");
            if (spaceIdx <= 0) continue;
            const packageName = line.slice(spaceIdx + 1).trim();

            if (!/x64/i.test(packageName)) continue;
            if (/\.EAppx/i.test(packageName)) continue;

            const id = this.parseVersion(packageName);
            if (!id) continue;

            const channel = /WindowsBeta/i.test(packageName) ? "preview" : "release";
            const key = `${id}|${channel}`;
            if (!seen.has(key)) seen.set(key, { id, channel });
        }

        return Array.from(seen.values()).sort((a, b) =>
            b.id.localeCompare(a.id, undefined, { numeric: true, sensitivity: "base" }));
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
