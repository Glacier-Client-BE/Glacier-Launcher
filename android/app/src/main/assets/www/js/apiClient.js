// Shared fetch-JSON/text helper — Android counterpart to the `ApiClientBase`
// refactor item in docs/audit/05_REFACTOR_PLAN.md ("mirrors refactor #1
// above conceptually, not code-shared, different language"). Removes the
// repeated `fetch(url) -> if (!res.ok) throw -> res.json()` boilerplate that
// was duplicated across curseforge.js, modrinth.js, xboxauth.js,
// skinlibrary.js and javaedition.js, and gives every one of those call
// sites the same error surface (a real thrown Error naming the endpoint and
// status, since these WebView fetch()es have no native HttpClient underneath
// to normalize it for them).
const ApiClient = {
    // GET `url` and parse the response body as JSON. `opts` is passed straight
    // through to fetch() (headers/method/body/etc) for call sites that need
    // auth headers or POST bodies, same shape fetch() itself takes.
    async getJson(url, opts) {
        const res = await fetch(url, opts);
        if (!res.ok) throw new Error(`${describeUrl(url)} returned ${res.status}`);
        return res.json();
    },

    // GET `url` and return the response body as plain text (e.g. the Bedrock
    // version-database dump, which is a flat text file, not JSON).
    async getText(url, opts) {
        const res = await fetch(url, opts);
        if (!res.ok) throw new Error(`${describeUrl(url)} returned ${res.status}`);
        return res.text();
    },
};

function describeUrl(url) {
    try { return new URL(url, location.href).hostname; }
    catch (e) { return "Request"; }
}
