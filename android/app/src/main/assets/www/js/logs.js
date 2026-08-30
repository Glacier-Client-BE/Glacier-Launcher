// Mirrors Services/LogService.cs's Redact()/ShareAsync() — same secret
// patterns, same public mclo.gs endpoint, no auth needed. Listing/reading
// the actual log files needs real file access (LogService.kt via
// Bridge.listJavaLogs/readJavaLog); this only needs fetch(), same as every
// other read-only API integration in this app.
const LogSharing = {
    UPLOAD_URL: "https://api.mclo.gs/1/log",

    SECRET_PATTERNS: [
        [/(--accessToken\s+)\S+/g, "$1<redacted>"],
        [/(accessToken["']?\s*[:=]\s*["']?)[A-Za-z0-9.\-_]+/gi, "$1<redacted>"],
        [/(--session\s+)\S+/g, "$1<redacted>"],
        [/eyJ[A-Za-z0-9_-]{6,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g, "<redacted-token>"],
    ],

    redact(text) {
        return this.SECRET_PATTERNS.reduce((acc, [pattern, replacement]) => acc.replace(pattern, replacement), text);
    },

    async share(content) {
        const body = new URLSearchParams({ content: this.redact(content) });
        const resp = await fetch(this.UPLOAD_URL, {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.toString(),
        });
        if (!resp.ok) throw new Error(`mclo.gs returned HTTP ${resp.status}`);
        const data = await resp.json();
        if (data.success && data.url) return data.url;
        throw new Error(`mclo.gs rejected the log: ${data.error || "unknown error"}`);
    },
};
// A top-level `const` doesn't attach to `window` the way `var`/function
// declarations do, and app.js calls this back via a plain `LogSharing`
// reference loaded as a separate <script> before it.
window.LogSharing = LogSharing;
