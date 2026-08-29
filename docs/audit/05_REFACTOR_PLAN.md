# 05 — Refactor Plan (DRY / SOLID, no over-engineering)

Scope kept deliberately small: each item below is a targeted extraction of
existing duplicated logic into one reusable unit, not a rearchitecture.

## Windows
1. **`ApiClientBase`** — one small typed-GET-JSON helper (url, optional cache
   key/TTL) consumed by `CurseForgeService`, `ModrinthService`,
   `GlacierClientService`, `NewsService`, `VanillaVersionService`,
   `JavaVersionService`. Removes ~6x duplicated try/catch/deserialize
   boilerplate. Built on top of existing `HttpFactory.cs`, not replacing it.
2. **`IThirdPartyClient` interface** — `Detect()`/`ResolvePath()`/`Launch()`
   implemented by `FlarialService`, `LunarBadlionService`, `LeviLaminaService`,
   `OderSoServices`. Lets `Home.Clients.cs` iterate a `List<IThirdPartyClient>`
   instead of one hand-written branch per client, and is the natural seam
   for Android's future package-context-based clients to plug into if the
   two codebases ever want to share an interface *shape* (not code) — see
   the "Custom `.so` picker" item in `03_MISSING_FEATURES.md`.
3. **Settings schema versioning** — add `SchemaVersion` to `LauncherSettings`
   + a migration switch in `JsonStore.Load()`. Currently any settings-shape
   change is a silent breaking change for existing users' `settings.json`.
4. **Shared dialog/modal component reuse** — `Components/Modal.razor` already
   exists; audit found several `Home.*.cs` files building bespoke inline
   confirm/alert markup instead of reusing it (e.g., some Bedrock instance
   rename flows in `Home.BedrockInstances.cs`). Route all confirm/alert UI
   through `Modal.razor` for consistent animation/styling and one place to
   fix accessibility issues.

## Android
1. **`js/apiClient.js`** — mirrors refactor #1 above conceptually (not
   code-shared, different language): one `getJson(url)` used by
   `curseforge.js`, `modrinth.js`, `xboxauth.js`, `skinlibrary.js`,
   `javaedition.js`. Immediate win: consistent error surface for the
   "CORS restriction shows as a visible error" behavior the README commits to.
2. **Split `panels.js` and `app.js` by concern** (see `04_CODE_AUDIT.md`) —
   `panels.bedrock.js` / `panels.java.js` / `panels.settings.js` /
   `panels.shared.js` (empty-state helper, shared card renderers); `state.js`
   extracted from `app.js`. Pure refactor, no behavior change — verify by
   diffing rendered HTML output before/after for each panel.
3. **Shared empty-state helper** — `.empty-state`/`.stats-empty`/`.skin-empty`
   markup is currently hand-written per panel in `panels.js`; extract one
   `emptyStateHtml({icon, title, body})` used everywhere, matching how
   `app.css` already treats them as one visual class family.
4. **Settings key constants** — one `SETTINGS_KEYS` object literal in JS and
   a matching Kotlin `object SettingsKeys` with the same string values,
   both hand-kept-in-sync (no cross-language codegen needed for ~15-20 keys)
   to eliminate the string-literal-typo risk noted in `04_CODE_AUDIT.md`.
5. **Generic file-picker bridge method** — `AndroidBridge.pickFile(mimeType,
   callback)` used by both the future custom `.so` picker (`03` item 5) and
   Skin Library's "Add PNG" (`03` item 7) and Theme Studio's wallpaper picker,
   instead of three bespoke SAF flows built separately over time.

## Explicitly not recommended
- **Do not** try to share actual code (not just interface shape) between the
  C# and Kotlin/JS codebases — different runtimes, no realistic shared
  compilation unit; the CurseForge ID duplication is best solved by a
  checked-in JSON/constants file both build steps read from, not a shared
  library, if it's worth solving at all given how rarely those IDs change.
- **Do not** introduce a DI container on either platform for this codebase's
  current size — the manual construction in `Home.razor.cs` and `MainActivity.kt`
  is small enough that a DI framework would add more ceremony than it removes.
- **Do not** merge `panels.js` back into `app.js` or vice versa — the current
  file split (state/router vs. render) is basically correct in principle;
  the fix is finishing that split more granularly, not undoing it.
</content>
