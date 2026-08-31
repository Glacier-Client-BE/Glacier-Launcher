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
        return ApiClient.getJson(this.MANIFEST_URL); // { latest: {release, snapshot}, versions: [{id, type, url, releaseTime}] }
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
        const text = await ApiClient.getText(this.DB_URL);

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
        return ApiClient.getJson(this.MANIFEST_URL); // { latestRelease, versions: [{id, name, tag, url, sha256, fabric, forge, changelog}] }
    },
};

// Mirrors NewsService.cs / AutoUpdateService.cs — same public endpoints,
// no auth needed for either. Both go through HttpCache (js/httpcache.js)
// with a 10-minute TTL, same spirit as NewsService.cs's on-disk cache: a
// flaky connection or a re-open of the News panel falls back to the last
// good response instead of an empty list or a hard error.
const NewsFeed = {
    NEWS_URL: "https://glacierclient.xyz/news.json",
    RELEASES_URL: "https://api.github.com/repos/Glacier-Client-BE/Glacier-Launcher/releases?per_page=12",

    async fetchPosts() {
        return HttpCache.fetch("news.posts", 10 * 60 * 1000, () => ApiClient.getJson(this.NEWS_URL)); // [{title, subtitle, url, icon}]
    },

    async fetchReleases() {
        return HttpCache.fetch("news.releases", 10 * 60 * 1000, async () => {
            const data = await ApiClient.getJson(this.RELEASES_URL);
            return data.map(r => ({ tag: r.tag_name, publishedAt: r.published_at, body: r.body || "" }));
        });
    },
};

// Mirrors Services/AnnouncementService.cs — same optional remote banner
// file, same "absent/unreachable is normal, fail silently" contract (a
// stale "maintenance in progress" banner surviving after the outage ended
// would be actively misleading, so unlike news there's no cached fallback).
const AnnouncementFeed = {
    URL: "https://glacierclient.xyz/announcement.json",

    async fetch() {
        try {
            const res = await fetch(this.URL);
            if (!res.ok) return null;
            const a = await res.json();
            return a && a.id ? a : null;
        } catch (e) {
            return null;
        }
    },
};
