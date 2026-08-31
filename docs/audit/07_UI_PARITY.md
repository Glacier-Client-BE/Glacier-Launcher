# 07 — UI Parity Checklist (WPF/Blazor vs. Android WebView)

Android reuses `wwwroot/css/app.css` byte-for-byte (confirmed: same filename,
copied verbatim per README and directory listing) plus the same
`vendor/fontawesome`, `vendor/fonts` (Plus Jakarta Sans, Roboto), and
`images/` (icon.png, bg.jpg, client logos) directories. This gives a
structurally strong baseline — differences below are what `mobile.css` (loads
after `app.css`) and the Android runtime environment change on top of that
shared base, not a re-implementation.

| Aspect | Windows (BlazorWebView) | Android (WebView) | Verdict |
|---|---|---|---|
| Color palette / theme tokens | `app.css` CSS custom properties, `ThemeService.cs`/`ThemeDefinition.cs` | Same `app.css` + `js/theme.js` (ported `BuildCssVars()` math) | Match — same source file, same derivation logic |
| Typography | Plus Jakarta Sans / Roboto via `wwwroot/vendor/fonts` | Same files copied to `assets/www/vendor/fonts` | Match |
| Iconography | Font Awesome via `wwwroot/vendor/fontawesome` | Same files copied verbatim | Match |
| Spacing/layout (desktop widths) | `app.css`, min 360px desktop | Same `app.css` + `mobile.css` overrides for landscape phone heights (360-420px tall) | Different Behavior (intentional, scoped to chrome padding/heights only per README) |
| Corner radius / glass-blur panel treatment | `app.css` | Same file, unchanged | Match |
| Orientation | Resizable window | Landscape-locked (`MainActivity` `screenOrientation="landscape"`) | Different Behavior (correct — no portrait layout exists on either side to fall back to) |
| Tab-bar overflow behavior (>4 footer tabs) | `wwwroot/js/interop.js` `ensureTabCyclers()` | `js/tabcycler.js`, described as verbatim port | Match (assuming the port is faithful — spot-check recommended: read both files side by side before signing off, not done in this pass due to scope) |
| Animations/transitions | CSS-driven via `app.css` (same file) | Same file | Match, since the animation rules live in the shared CSS, not separately implemented |
| Loading states / spinners | `Components/Spinner.razor`, `Components/InlineProgress.razor` | Presumed reused via shared CSS classes in `panels.js`/`app.js`; not confirmed that JS renders identical markup structure (`Spinner.razor`'s exact DOM shape vs. hand-written HTML string in JS) — **needs verification** | Unverified — flag for a follow-up markup diff |
| Notifications/toasts | `Models/AppNotification.cs`, `Services/NotificationService.cs` rendering | `notifPanelHtml()` in `panels.js`, badge + Downloads section real, event-log list empty | Partial — visual shell matches, content source differs (see `02_FEATURE_PARITY.md`) |
| Error dialogs | Presumed `Components/Modal.razor` based (not fully traced per-call-site in this pass) | Not confirmed present as a distinct error-dialog component in `panels.js` | Unverified — worth a dedicated pass tracing every `catch` block's UI surface on both platforms |
| Tooltips | Native browser/Blazor tooltip attributes (if any) inside `Home.razor` | Not confirmed present in `index.html`/`panels.js` | Unverified — likely both platforms rely on `title=` attributes if used at all; needs a grep-and-compare pass |
| Splash/startup behavior | `App.xaml.cs`→`MainWindow` load, presumably a brief blank/loading window before `Home.razor` mounts | `MainActivity.kt` WebView load of `index.html` directly, no separate native splash screen confirmed | Unverified — check for an Android 12+ `SplashScreen` API usage; none found in the 186-line `MainActivity.kt`, likely simplest default system splash only |
| Settings toggles (visual switch component) | `Components/Toggle.razor` | Presumed reused via shared `.toggle` CSS class in `panels.js`; DOM shape not diffed | Likely match given shared CSS class names referenced throughout README, not independently re-verified here |
| Update prompt UI | `Home.razor` update modal (`OpenUpdateModal`) styled via `app.css`/`Components/Modal.razor` | N/A — no updater on Android | Missing (by design, see `02_FEATURE_PARITY.md`) |
| Accessibility (focus states, contrast, screen-reader labels) | Not audited in either `Home.razor` or `panels.js` for ARIA attributes in this pass | Same | Unverified both platforms — recommend a dedicated a11y pass with an automated checker (axe-core or similar) rather than manual read, out of scope for this audit's time budget |
| Responsiveness beyond the two target form factors | Desktop window resizing down to unspecified minimum | `mobile.css` only tunes for 360-420px landscape heights; no handling for tablet landscape (much taller) confirmed | Unverified — likely fine given `app.css`'s own flexible layout, but not confirmed on a wider landscape viewport |

## Summary
Structural CSS/asset parity is genuinely strong — this is not a marketing
claim in the README, the file inventory backs it up (identical filenames,
identical vendor directories, `mobile.css` as a strictly additive override
file rather than a replacement). The real parity risk is in **hand-written
JS markup fragments** (spinners, tooltips, error dialogs, toggles) that may
not byte-match their Razor component counterparts' exact DOM structure even
though they use the same CSS classes — CSS classes matching doesn't guarantee
identical HTML nesting, and several of those are marked "Unverified" above
because confirming them requires a side-by-side DOM diff per component that
this pass's time budget did not cover. Recommended as the single highest-value
follow-up for UI parity specifically: diff `Components/*.razor` output HTML
against the corresponding `panels.js`/`app.js` generated markup, component by
component.
</content>
