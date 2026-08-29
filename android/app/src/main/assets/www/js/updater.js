// Android analogue of AutoUpdateService.CheckLauncherUpdateAsync — same
// public GitHub releases endpoint (NewsFeed.RELEASES_URL, javaedition.js),
// walked the same way: skip drafts, keep the highest semver tag, and only
// report an update if it's newer than the running build (Bridge.appVersionName()).
// The actual download + install-intent is native (LauncherUpdateService.kt)
// since a WebView can't hand a file to the system package installer itself.
const LauncherUpdate = {
    _pending: null,

    isNewerVersion(remote, local) {
        const rp = remote.split(".").map(n => parseInt(n, 10) || 0);
        const lp = local.split(".").map(n => parseInt(n, 10) || 0);
        for (let i = 0; i < Math.max(rp.length, lp.length); i++) {
            const r = rp[i] || 0, l = lp[i] || 0;
            if (r !== l) return r > l;
        }
        return false;
    },

    // Returns { tag, downloadUrl, changelog } or null if already up to date /
    // no .apk asset found / offline.
    async check() {
        const res = await fetch(NewsFeed.RELEASES_URL);
        if (!res.ok) throw new Error(`GitHub releases returned ${res.status}`);
        const releases = await res.json();

        let best = null;
        for (const r of releases) {
            if (r.draft) continue;
            const tag = (r.tag_name || "").replace(/^v/i, "");
            if (!tag) continue;
            if (!best || this.isNewerVersion(tag, best.tag)) best = { tag, release: r };
        }
        if (!best) return null;

        const current = Bridge.appVersionName();
        if (!this.isNewerVersion(best.tag, current)) return null;

        const assets = best.release.assets || [];
        const asset = assets.find(a => (a.name || "").toLowerCase().endsWith(".apk")) || assets[0];
        if (!asset) return null;

        return { tag: best.tag, downloadUrl: asset.browser_download_url, changelog: best.release.body || "" };
    },

    // Kicks off the native download+install and resolves/rejects once it's
    // done (install-intent shown) or failed — progress is reported via the
    // onProgress callback as the native side polls DownloadManager.
    install(info, onProgress) {
        if (!Bridge.downloadAndInstallUpdate) {
            return Promise.reject(new Error("Updates aren't available in this preview (no native bridge)."));
        }
        return new Promise((resolve, reject) => {
            this._pending = { resolve, reject, onProgress };
            Bridge.downloadAndInstallUpdate(info.downloadUrl, info.tag);
        });
    },

    // Called from native (MainActivity.kt's downloadAndInstallUpdate) as
    // DownloadManager reports progress.
    _onProgress(pct) {
        if (this._pending?.onProgress) this._pending.onProgress(pct);
        if (pct >= 100 && this._pending) {
            const { resolve } = this._pending;
            this._pending = null;
            resolve();
        }
    },

    _onError(message) {
        if (!this._pending) return;
        const { reject } = this._pending;
        this._pending = null;
        reject(new Error(message));
    },
};
// See js/xboxauth.js's window.MicrosoftAuth comment — MainActivity.kt calls
// back via window.LauncherUpdate, which needs an explicit assignment.
window.LauncherUpdate = LauncherUpdate;
