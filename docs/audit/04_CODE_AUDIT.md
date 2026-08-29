# 04 — Code Quality Audit

## Windows (C#)

| Issue | Location | Severity | Explanation | Suggested Fix |
|---|---|---|---|---|
| God class via partial-class sprawl | `Pages/Home.razor.cs` (905 lines) + 20 `Home.*.cs` partials (17,100 combined lines in `Pages/`+`Services/`+`Models/`) | High | `Home` is a single logical class spanning ~20 files and 493 UI event handlers; any change to shared state (`currentView`, settings) risks touching many files with no enforced boundaries between concerns (Clients vs Panels vs Settings vs Search all mutate the same component state directly). | Extract per-feature state into small view-model classes injected into `Home`, communicating via events/callbacks instead of direct field access; keep `Home.razor.cs` as composition root only. |
| Large single-purpose files | `Pages/Home.Panels.cs` (1,055 lines), `Pages/Home.Clients.cs` (1,000 lines), `Services/JavaGameLauncher.cs` (811 lines), `Services/JavaInstanceService.cs` (744 lines) | Medium | Files exceeding ~700 lines with many responsibilities (panel routing + rendering + data-fetch triggers all interleaved) are hard to review/test in isolation. | Split by sub-responsibility (e.g., `JavaGameLauncher` → `JavaProcessBuilder` + `JavaLaunchArgsFactory` + `JavaLaunchMonitor`). |
| Repeated HTTP/service boilerplate | `Services/CurseForgeService.cs`, `ModrinthService.cs`, `GlacierClientService.cs`, `NewsService.cs`, `VanillaVersionService.cs`, `JavaVersionService.cs` all independently construct `HttpClient` calls, JSON deserialization, and error handling | Medium | `HttpFactory.cs` exists but each service still hand-rolls its own retry/error/caching pattern instead of a shared typed-client base. | Introduce a small `ApiClientBase<T>` (get-json + cache-if-stale + typed error) used by all six services — see `05_REFACTOR_PLAN.md`. |
| Settings persisted as one untyped-growing JSON blob | `Services/SettingsService.cs`/`JsonStore.cs`, `Models/LauncherSettings.cs` | Low | Works, but every new feature setting (Discord, injection, theme, onboarding flags) adds another field to one already-large model with no versioning/migration story visible. | Add a schema version int + migration step in `JsonStore`, or split `LauncherSettings` into per-feature sub-objects serialized together. |
| Injection/client-specific services not unified | `Services/InjectionService.cs`, `FlarialService.cs`, `LunarBadlionService.cs`, `LeviLaminaService.cs`, `OderSoServices.cs` | Low | Five services doing structurally similar "detect install → resolve exe/dll path → launch/inject" work with no shared interface. | Extract an `IThirdPartyClient` interface (`Detect()`, `Resolve()`, `Launch()`); each service implements it — enables the Clients panel to iterate a list instead of hand-wiring each card. |

## Android

| Issue | Location | Severity | Explanation | Suggested Fix |
|---|---|---|---|---|
| Duplicated fetch/error-handling logic across every API-calling JS file | `js/curseforge.js`, `js/modrinth.js`, `js/xboxauth.js`, `js/skinlibrary.js`, `js/javaedition.js` (5 independent `fetch()` call sites, no shared wrapper) | Medium | Each file reimplements its own `fetch().then().catch()` shape; no shared timeout, error-surface, or JSON-parse-failure handling — a CORS or network error can format differently panel to panel. | Add one `js/apiClient.js` (`getJson(url, opts)`) used by all five; matches the C#-side `HttpFactory` pattern this repo already established on desktop, so both platforms would share the same *shape* of fix even without shared code. |
| God file: `js/panels.js` (1,253 lines) | `android/app/src/main/assets/www/js/panels.js` | Medium | One file owns markup generation for essentially all 26 panels (mirrors `Pages/Home.Panels.cs`'s same problem, inherited by design since it's a port). | Split by panel family (`panels.bedrock.js`, `panels.java.js`, `panels.settings.js`) loaded as separate `<script>` tags, same functions, smaller review surface. |
| `js/app.js` (994 lines) mixes routing, state, and rendering | `android/app/src/main/assets/www/js/app.js` | Medium | Router (`currentView` switch), app state (`App.state.downloads`), and DOM rendering calls are not separated; consistent with the Windows-side god-class issue (inherited pattern, not introduced independently). | Separate `state.js` (pure state + getters/setters) from `router.js` (view switch) from render calls already in `panels.js`. |
| No shared settings schema between Kotlin and JS | `MainActivity.kt`'s `AndroidBridge` reads/writes `SharedPreferences` keys referenced only by string literals in both Kotlin and `js/app.js` | Medium | A typo'd key name in either language fails silently (no compile-time or runtime check ties them together); no equivalent of `LauncherSettings.cs`'s typed model exists on Android. | Define a single JSON-schema constant (key names + types) in one place — even a small generated `Settings.kt` object with `const val` keys referenced from a JS constants object populated via the bridge at load time — to remove the duplicated string literals. |
| No unit/instrumentation tests found | entire `android/app/` | Medium | Confirmed via file listing — `android/app/src/main/` has no `test`/`androidTest` source set. Given no physical device is available in CI either, regressions in bridge logic or JS panel rendering are undetected until manual QA. | At minimum, add JS unit tests for `panels.js` render functions (pure string-returning functions, easily testable with any JS test runner) and a Robolectric test for `AndroidBridge`'s settings read/write round-trip. |
| `ClientInjectionService.kt` doc comment (per README) previously asserted a technically incorrect claim ("needs root") that the README itself has now corrected in prose but the Kotlin doc comment/Settings UI copy may still lag | `android/app/src/main/java/xyz/glacierclient/launcher/service/ClientInjectionService.kt` | Low | README explicitly flags this ("`ClientInjectionService.kt`'s doc comment ... should stop asserting 'needs root'") as still-pending; verified the file is only 80 lines, a comment-text update is low-risk but was left as a documentation task rather than a code change per the "don't change behavior" audit constraint. | Update the doc comment (not behavior) to reflect the package-context/`dlopen` finding; tracked in `03_MISSING_FEATURES.md` item 4 rather than auto-fixed here since it touches in-app Settings copy shown to users, which is content, not indisputably-dead code. |

## Cross-cutting
- **No shared constants file for CurseForge game/class IDs.** Windows'
  `Services/CurseForgeService.cs` and Android's `js/curseforge.js` each hard-code
  the same numeric game/class IDs independently (confirmed by README's own
  "same game/class ids as `Services/CurseForgeService.cs`" note) — a value
  change on one side won't propagate. Not fixable without a shared
  cross-language constants mechanism; see `05_REFACTOR_PLAN.md`.
- **No dead code or unused-import findings confident enough to auto-fix.**
  A targeted grep across `android/app/src/main/java/**/*.kt` for unused
  imports found none (all imports in `MainActivity.kt`, `GlacierApp.kt`,
  `ClientInjectionService.kt`, `JavaEditionBridge.kt` are referenced in body
  code). No orphaned asset files were found under
  `android/app/src/main/assets/www/` — every file is referenced from
  `index.html` or another script. Per the audit's own safety constraint, no
  speculative deletions were made; see `08_IMPLEMENTED_CHANGES.md`.
</content>
