# 02 — Feature Parity Matrix

Source of truth for Windows: `Pages/Home.razor` (26 panel views, 493 `@onclick`
handlers) + partial `Home.*.cs` files + `Services/*.cs`. Source of truth for
Android: `android/README.md`'s "UI parity status" + "Deep gap analysis"
sections (an existing systematic 135-handler pass), cross-checked against
`android/app/src/main/assets/www/js/*.js` and `android/.../*.kt`.

Status legend: **Complete** (same behavior+data), **Partial** (UI/markup
present, backing logic reduced or stubbed), **Missing** (no Android artifact
at all), **Different Behavior** (intentionally re-scoped for the platform),
**Bugged** (present on Android but incorrect/inconsistent — see `04_CODE_AUDIT.md`).

| Feature | Windows (location) | Android (location) | Status | Missing Parts |
|---|---|---|---|---|
| Top bar (branding/edition switch/client chip) | `Home.razor` top-bar | `index.html` + `app.js` | Complete | — |
| Quick-actions dock | `Home.QuickActions.cs` | `app.js` | Complete | — |
| Footer (profile/RPC toggle/Xbox/Discord) | `Home.razor` footer | `app.js`/`index.html` | Partial | RPC toggle persists pref only, no live presence (see below) |
| News ticker | `Services/NewsService.cs` | `js/javaedition.js` NewsFeed | Complete | — |
| Clients panel (Vanilla) | `Home.Clients.cs`, `Services/GameLauncher.cs` | `panels.js` | Complete | — |
| Clients panel (Flarial/Latite/OderSo/LeviLamina DLL injection) | `Services/InjectionService.cs`, `Services/FlarialService.cs`, `Services/LunarBadlionService.cs`, `Services/LeviLaminaService.cs` | `ClientInjectionService.kt` (root-only stub) | Different Behavior | Cards removed from UI entirely; non-root package-context+`dlopen` path researched (README) but not implemented |
| Servers panel | `Home.razor`/`Home.Panels.cs` | `panels.js` | Complete | — |
| Credits panel | `Home.razor` | `panels.js` | Complete | — |
| Settings — general categories | `Home.Settings.cs` (492 lines) | `panels.js`/`app.js` | Complete | Java Edition settings link out to Pojav's own instead of duplicating (intentional) |
| Settings — Injection category | `Home.Settings.cs` | removed, replaced with "Clients" explainer | Different Behavior | — |
| Addons (Bedrock, CurseForge) | `Services/CurseForgeService.cs` | `js/curseforge.js` | Complete | — |
| MC Versions (Bedrock) | `Home.Panels.cs` | `panels.js` | Partial | "Install from MS Store" row replaced with note (correct); version list empty pending real data source |
| Bedrock Worlds | `Services/BedrockWorldService.cs` | `panels.js` empty-state | Partial | No LevelDB reader wired (README names a concrete path) |
| Bedrock Packs | `Services/BedrockPackService.cs` | `panels.js` empty-state | Partial | No file listing |
| Bedrock Backups | `Services/BedrockBackupService.cs`, `Components/BackupsPanel.razor` | `panels.js` empty-state | Partial | No file listing |
| Bedrock Instances | `Services/BedrockInstanceService.cs` (318 lines) | `panels.js` empty-state | Partial | No instance-management model on Android at all |
| Bedrock Screenshots | `Services/BedrockScreenshotService.cs` | `panels.js` empty-state | Partial | No shared-storage wiring |
| Java Addons — Loaders/Mods/Assets/Tools | `Services/JavaModLoaderService.cs`, `Services/JavaModAnalyzer.cs` | `panels.js` | Partial | Actions render but disabled (no active-version tracking, no backing service) |
| Java Addons — Datapacks | `Home.Datapacks.cs` | `panels.js` | Missing (queued) | Needs world picker |
| Java Addons — CurseForge/Modrinth search | `CurseForgeService.cs`/`ModrinthService.cs` | `js/curseforge.js`/`js/modrinth.js` | Complete | — |
| Java Launchers (Vanilla/Glacier/Lunar/Badlion) | `Home.Clients.cs`, `Services/GlacierClientService.cs`, `Services/LunarBadlionService.cs` | `panels.js` | Partial | Lunar/Badlion honestly marked unavailable (no Android builds exist) |
| Java Versions | `Services/JavaVersionService.cs` (653 lines: `VanillaVersionService.cs`) | `panels.js` | Complete | Install/Launch delegate to Pojav runtime rather than own management |
| Java Profile (skin viewer/cape wardrobe) | `Components/SkinViewer.razor` | `panels.js` `.skin-empty` branch | Partial | Sign-in works (see below) but skin viewer UI not wired to it yet |
| Java Screenshots | — | `panels.js` empty-state | Missing | Needs shared-storage wiring |
| News panel (dedicated) | reachable via `Home.Search.cs` palette | reachable via `panels.js` search overlay | Complete | Neither app has a persistent nav entry (parity, not a gap) |
| Downloads panel | `Home.Downloads.cs` | `panels.js`, `App.state.downloads` | Complete | — |
| Stats panel | `Components/StatsPanel.razor`, `Services/StatsService.cs` | `panels.js` | Different Behavior | Android has no session tracking to ever populate it (Windows does, once used) |
| Logs panel | `Components/LogsPanel.razor`, `Services/LogService.cs`, `Services/GameConsoleService.cs` | `panels.js` empty-state | Missing | No log capture pipeline on Android |
| Modpacks panel | `Components/ModpacksPanel.razor`, `Services/ModpackInstallService.cs` | `panels.js` | Partial | Search works; Install disabled (no instance model) |
| Theme Studio | `Components/ThemeStudioPanel.razor`, `Components/ColorPicker.razor`, `Services/ThemeService.cs` | `js/theme.js` | Complete | Native `<input type=color>` swatch instead of custom hue/sat picker (cosmetic diff); wallpaper picking disabled |
| Skin Library | `Components/SkinLibraryPanel.razor`, `Services/SkinLibraryService.cs`, `Services/SkinService.cs` | `js/skinlibrary.js` | Partial | "Add by username" works; "Save current"/Apply/"Add PNG" disabled (no file picker bridge) |
| Onboarding wizard | `Home.Onboarding.cs` | none | Missing | No first-run wizard or Minecraft-install import on Android |
| Announcement banner | `Home.Announcement.cs`, `Services/AnnouncementService.cs` | none found | Missing | Not ported |
| Global search / command palette | `Home.Search.cs` `BuildDefaultSearchResults()` | `panels.js` `searchOverlayHtml()` | Partial | Curated subset; Windows-only entries correctly excluded |
| Notification bell | `Services/NotificationService.cs` | `panels.js` `notifPanelHtml()` | Partial | Badge/Downloads real; event-log list is honestly empty (no NotificationService equivalent) |
| Microsoft/Xbox sign-in | `LiveAuthWindow.xaml.cs`, `LiveAuthService.cs`, `XboxProfileService.cs` | native `Dialog`+`WebView` in `MainActivity.kt`, `js/xboxauth.js` | Complete | — |
| Discord Rich Presence | `Services/DiscordRpcService.cs` (native IPC) | none (toggle only) | Missing | No local IPC channel exists on Android; OAuth-only path also not implemented (`openUrl()` stub) |
| Discord OAuth manual connect | `Home.razor` `OpenDiscordOAuth`/`SaveDiscordManual`/`DisconnectDiscord` | `openUrl()` only | Missing | Real redirect-interception OAuth (same technique as Xbox) not yet built |
| Client injection settings (delay/auto-inject) | `Home.Settings.cs` | removed | Different Behavior | Intentional, since injection itself is gated |
| Auto-update (in-app) | `Services/AutoUpdateService.cs`, update modal in `Home.razor` | none | Missing | Play Store/new-APK model instead (platform-appropriate) |
| System tray | `TrayIcon.cs` | none | Missing | No desktop-shell concept on Android (correct omission) |
| Window chrome (min/max/close/F11 fullscreen) | `MainWindow.xaml.cs` | immersive fullscreen only | Different Behavior | No windowing on a phone (correct) |
| Folder shortcuts (Open Launcher/Minecraft Folder) | `Home.razor` `Open*Folder` handlers | none | Missing | SAF folder-open intent is the closest analogue, not yet wired |
| Wallpaper picker | `Home.razor` `PickWallpaper`/`ResetWallpaper` | disabled in Theme Studio | Missing | No file-picker bridge |
| Custom client DLL/.so picker | `Home.razor` `PickDllFile`/`CopyDllPath`/`ClearCustomDll` | none | Missing | SAF `ACTION_OPEN_DOCUMENT` path identified but not implemented |
| LevelDat editor | `Services/LevelDatEditorService.cs`, `Models/LevelDatSummary.cs` | none | Missing | Needs a world to be reachable first |
| Java multi-instance management | `Services/JavaInstanceService.cs` (744 lines), `Models/JavaInstance.cs` | none | Missing | Blocks Modpack Install and several Java Addons actions |
| Localization | `Services/LocalizationService.cs` | none apparent in `android/app/src/main/assets/www/js/` | Missing | Android UI appears hard-coded English only |
| Server ping (latency/MOTD) | `Services/ServerPingService.cs` | not found in `panels.js`/`app.js` grep scope | Missing/Unverified | Confirm during implementation pass; likely absent |
| Loading/transition animations, tab-cycler paging | `wwwroot/js/interop.js` `ensureTabCyclers()` | `js/tabcycler.js` (verbatim port) | Complete | — |
| Mobile-only: landscape lock, `mobile.css` chrome scaling | N/A (desktop) | `MainActivity.kt`, `css/mobile.css` | Different Behavior (Android-only, correct) | — |

**Overall parity read:** UI markup/CSS parity is very high (README's claim of
byte-for-byte CSS/asset reuse checks out structurally — same file names, same
class names referenced from `panels.js`). The gap is almost entirely in the
**service/backing-logic layer**: anything requiring native file I/O (SAF),
an instance-management model, or a platform IPC channel (Discord RPC, DLL
injection) is Partial/Missing, exactly as the README's own gap analysis
already states. No evidence found of the README's self-assessment being
overstated; if anything it under-claims (e.g., Localization and Announcement
banner aren't mentioned there but appear absent).
</content>
