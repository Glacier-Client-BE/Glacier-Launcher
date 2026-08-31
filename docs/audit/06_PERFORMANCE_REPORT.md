# 06 — Performance Report

## Windows
- **No pagination on GitHub API cache invalidation path** — `Services/GitHubApiCache.cs`
  exists to avoid refetching, but `AutoUpdateService.cs` (305 lines) and
  `AnnouncementService.cs` should be checked for whether every panel open
  re-triggers a network call versus reading the cache; not confirmed as a bug
  here, flagged for verification (needs runtime profiling, not static read).
- **`Home.razor` single-component re-render surface** — a 4,896-line Razor
  file with 493 event handlers all mutating one component's state means
  Blazor's diffing has to reconcile a very large render tree on any state
  change that doesn't scope its `StateHasChanged()` calls narrowly. If any
  handler calls a bare `StateHasChanged()` (common Blazor anti-pattern) it
  forces a full-page re-render on every click. **Recommendation:** audit for
  `StateHasChanged()` call sites and scope updates to child components
  (`Components/*.razor`) wherever the same data doesn't need to propagate
  page-wide — measurable via Blazor's built-in render-count devtools.
- **Skin/image loading** — `SkinViewer.razor`/`SkinLibraryService.cs`: confirm
  a per-session in-memory cache exists for repeatedly-viewed skin textures
  rather than re-fetching from Mojang CDN on every panel reopen.

## Android
- **No native HTTP client, no shared cache** — every `fetch()` call in
  `js/curseforge.js`/`modrinth.js`/`xboxauth.js`/`skinlibrary.js`/`javaedition.js`
  hits the network fresh; there is no equivalent of desktop's `GitHubApiCache`.
  Reopening the News or Addons panel re-fetches every time. **Estimated
  improvement:** a simple `localStorage`-backed TTL cache (5-10 min for
  news/CurseForge listings) would eliminate the large majority of repeat
  fetches on panel re-open, at near-zero implementation cost given the
  `js/apiClient.js` extraction already proposed in `05_REFACTOR_PLAN.md`.
- **WebView `file://` origin fetches** — no HTTP/2 connection reuse tuning
  possible from JS; this is an inherent WebView constraint, not a code bug —
  documented, not actionable.
- **`panels.js` (1,253 lines) rebuilds full panel HTML strings on every
  navigation** — string-concatenation-based rendering (typical for this port
  style) means switching panels re-serializes large template strings instead
  of patching a DOM diff. For a WebView on modest Android hardware this is
  the most likely visible jank source (panel-switch stutter), more than any
  network cost. **Recommendation:** no need for a virtual-DOM library
  (over-engineering per this audit's constraints) — cache the rendered HTML
  string per panel in a `Map` and only rebuild when underlying data
  (`App.state`) actually changes, since most panels are largely static markup
  driven by rarely-changing data.
- **Immersive fullscreen re-apply on every focus change** — `MainActivity.kt`
  re-hides system bars "whenever the window regains focus" (per README);
  this is correct behavior, not a leak, but confirm it's guarded against
  redundant re-application when focus events fire in quick succession
  (e.g., during a system dialog dismiss) to avoid unnecessary
  `WindowInsetsControllerCompat` calls back-to-back.
- **No evidence of memory leaks in the 4 Kotlin files** — `MainActivity.kt`
  (186 lines), `GlacierApp.kt` (14), `ClientInjectionService.kt` (80),
  `JavaEditionBridge.kt` (71) are small enough that a manual read found no
  retained `Context` references, unclosed streams, or missing
  lifecycle-aware cleanup. This is a static read, not a heap-dump-verified
  claim — cannot be fully confirmed without a physical device (consistent
  with the whole Android side's testing constraint noted in the README).

## Both platforms
- **CurseForge/Modrinth search has no debounce confirmed** — verify search
  input handlers (`Home.Panels.cs` on Windows, `panels.js`/`curseforge.js` on
  Android) debounce keystroke-triggered searches; if not, each keystroke
  fires a full network request. Flagged for verification, not confirmed as
  present or absent from static reading alone.
</content>
