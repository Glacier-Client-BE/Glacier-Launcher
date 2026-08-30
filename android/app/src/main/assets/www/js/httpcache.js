// Lightweight localStorage-backed TTL cache for read-only remote fetches
// (CurseForge/Modrinth search, news/release feeds). Mirrors the intent of
// NewsService.cs's on-disk cache — serve a recent response instead of
// hitting the network (and CurseForge's/Modrinth's rate limits) again for
// every keystroke of a search or every panel re-open — plus the same
// "network failed, fall back to whatever we last had" behaviour as
// NewsService.GetNewsAsync()'s catch block, so a flaky connection degrades
// to stale data instead of an empty/error state. Deliberately NOT used for
// AnnouncementFeed (js/javaedition.js already documents why a stale
// "maintenance in progress" banner would be actively misleading) or for
// anything that must always reflect the very latest state (installs,
// account/session calls).
const HttpCache = {
    PREFIX: "glacier.httpcache.",

    _read(key) {
        try {
            const raw = localStorage.getItem(this.PREFIX + key);
            return raw ? JSON.parse(raw) : null;
        } catch (e) { return null; }
    },

    _write(key, value) {
        try { localStorage.setItem(this.PREFIX + key, JSON.stringify({ value, savedAt: Date.now() })); }
        catch (e) { /* storage full/unavailable — caching is a nicety, not a requirement */ }
    },

    // Runs `fetcher()` and caches its resolved value under `key` for
    // `ttlMs`. Within the TTL, returns the cached value without calling
    // `fetcher` at all. Past the TTL it re-fetches; if that throws, it
    // falls back to the stale cached value (any age) rather than an error,
    // and only re-throws when there is truly nothing cached yet.
    async fetch(key, ttlMs, fetcher) {
        const entry = this._read(key);
        if (entry && Date.now() - entry.savedAt < ttlMs) return entry.value;
        try {
            const value = await fetcher();
            this._write(key, value);
            return value;
        } catch (e) {
            if (entry) return entry.value;
            throw e;
        }
    },
};
