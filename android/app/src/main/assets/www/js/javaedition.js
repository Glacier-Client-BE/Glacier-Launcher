// Real data sources for the Java-edition panels — same endpoints the desktop
// services use, called directly via fetch() since there's nothing Windows-
// specific about the network calls themselves (unlike installing/launching,
// which the Pojav companion app owns on Android — see JavaEditionBridge.kt).

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
