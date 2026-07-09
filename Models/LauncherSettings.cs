namespace GlacierLauncher.Models;

public class LauncherSettings
{
    // ── Core ─────────────────────────────────────────────────────
    // Which Minecraft edition the launcher is currently driving.
    //   "bedrock" — Windows 10/UWP Minecraft + Latite/Flarial/OderSo client injection (original launcher behaviour)
    //   "java"    — Java Edition launches against %APPDATA%/.minecraft
    public string Edition              { get; set; } = "bedrock";
    public string SelectedClient       { get; set; } = "Latite Client";
    public bool   DiscordRichPresence  { get; set; } = true;
    public string LastUsedVersion      { get; set; } = "";
    public string Username             { get; set; } = "";
    public string UserHandle           { get; set; } = "";
    public bool   CloseAfterLaunch     { get; set; } = false;
    public bool   AutoInject           { get; set; } = false;
    public int    InjectionDelayMs     { get; set; } = 2000;

    // ── Appearance ───────────────────────────────────────────────
    public string AccentColor          { get; set; } = "#7289da";
    public string ThemePreset          { get; set; } = "dark";   // dark | darker | midnight | slate | ocean | forest | sunset | light
    public int    BlurIntensity        { get; set; } = 14;       // px, 0-30
    public double BackgroundOpacity    { get; set; } = 0.80;     // 0.0-1.0
    public string CustomBackgroundPath { get; set; } = "";       // empty = default bg
    public bool   CompactMode          { get; set; } = false;    // tighter spacing throughout
    public bool   AnimationsEnabled    { get; set; } = true;     // disables some animations on low-end
    public double AnimationSpeed       { get; set; } = 1.0;      // 0.25 slow-mo … 2.0 snappy; 1.0 default
    public string ActiveThemeId        { get; set; } = "";       // custom Theme Studio theme (empty = preset)
    public string FontFamily           { get; set; } = "";       // UI font when no theme is active (empty = Segoe UI)
    public int    UiScalePct           { get; set; } = 100;      // 75-150, multiplies the automatic window fit
    public string BackgroundFit        { get; set; } = "cover";  // cover | contain | tile | center

    // ── Window ───────────────────────────────────────────────────
    public double WindowWidth          { get; set; } = 740;
    public double WindowHeight         { get; set; } = 500;
    public bool   RememberWindowSize   { get; set; } = true;
    public bool   MinimizeToTray       { get; set; } = false;

    // ── Recently Launched ────────────────────────────────────────
    public bool         ShowRecentlyLaunched { get; set; } = true;
    public List<string> RecentlyLaunched     { get; set; } = new();

    // ── Playtime tracking ────────────────────────────────────────
    // Accumulated game time across all sessions, measured by the launcher's
    // running-state poll. LastPlayed is an ISO-8601 UTC timestamp.
    public long   TotalPlaytimeSeconds { get; set; } = 0;
    public string LastPlayed           { get; set; } = "";

    // ── Pinned Versions ──────────────────────────────────────────
    public List<string> PinnedVersions { get; set; } = new();

    // ── Versions QoL ─────────────────────────────────────────────
    public string VersionSortMode     { get; set; } = "newest"; // newest | oldest | name | downloaded
    public bool   ShowOnlyDownloaded  { get; set; } = false;

    // ── Vanilla Version Switcher ────────────────────────────────
    public string ActiveVanillaVersion  { get; set; } = "";

    // ── Java Edition ─────────────────────────────────────────────
    public string JavaActiveVersion     { get; set; } = "";
    public string JavaLastUsedVersion   { get; set; } = "";
    public string JavaActiveInstanceId  { get; set; } = "";
    // When true (default), Java game files live in the standard %APPDATA%\.minecraft.
    // When false, they live inside the Glacier Launcher instance folder. An explicit
    // JavaMinecraftDir below overrides both.
    public bool   JavaUseDotMinecraft   { get; set; } = true;
    // Override path to .minecraft (empty = use the toggle above)
    public string JavaMinecraftDir      { get; set; } = "";
    // Override path to javaw.exe (empty = auto-detect)
    public string JavaRuntimePath       { get; set; } = "";
    public int    JavaMinRamMb          { get; set; } = 512;
    public int    JavaMaxRamMb          { get; set; } = 2048;
    // Extra JVM args appended to every Java launch (e.g. -XX:+UseG1GC, -Dfile.encoding=UTF-8)
    public string JavaCustomJvmArgs     { get; set; } = "";
    public bool   JavaBackupSavesBeforeLaunch { get; set; } = false;
    public bool   JavaFullscreen        { get; set; } = false;
    public int    JavaWindowWidth       { get; set; } = 854;
    public int    JavaWindowHeight      { get; set; } = 480;
    public bool   JavaUseCustomResolution { get; set; } = false;
    public string JavaServerAddress     { get; set; } = "";
    public int    JavaServerPort        { get; set; } = 25565;
    // Pop the game console window on launch. Java captures real stdout/stderr;
    // Bedrock captures launcher lifecycle events (we can't read UWP stdout).
    public bool   ShowLaunchConsole     { get; set; } = true;
    // Versions filter — Mojang manifest types: "release" only by default, can opt-in to snapshots/old_beta/old_alpha
    public bool   JavaShowSnapshots     { get; set; } = false;
    public bool   JavaShowHistorical    { get; set; } = false;
    // Minecraft Services profile (Java's "real" user identity — distinct from Xbox Live's gamertag)
    public string JavaUsername          { get; set; } = "";
    public string JavaUuid              { get; set; } = "";
    public string JavaAccessToken       { get; set; } = "";
    // Offline mode: launch Java without Microsoft auth. Online-mode servers
    // reject offline sessions, but singleplayer + LAN + offline servers work.
    public bool   JavaOfflineMode       { get; set; } = false;
    public string JavaOfflineUsername   { get; set; } = "Player";
    public string JavaAccessTokenExpiry { get; set; } = "";
    public string JavaSkinUrl           { get; set; } = "";
    // Remembers the last "Upload as Slim (Alex) model" checkbox state so it
    // doesn't silently reset to Classic on every restart.
    public bool   JavaSkinSlimModel     { get; set; } = false;

    // ── Skin viewer (Profile tab 3D preview) ─────────────────────
    public string SkinViewerMode        { get; set; } = "2d";      // 2d (static) | 3d (animated)
    public string SkinViewerModel       { get; set; } = "default"; // default = Steve | slim = Alex
    public string SkinViewerCapeMode    { get; set; } = "cape";    // cape | elytra | off

    // Bumped every time the account skin is changed through the launcher.
    // Third-party render URLs (mc-heads/crafatar) carry this as a ?v= token so
    // the WebView's HTTP cache can't keep serving the pre-change render.
    // Persisted: the browser cache survives launcher restarts. 0 = never changed.
    public long   SkinChangedTicks      { get; set; } = 0;
    // Library copy of the last skin we uploaded — the authoritative texture for
    // the 3D preview until the render CDNs catch up (they cache for minutes).
    public string LastAppliedSkinPath   { get; set; } = "";
    public string ActiveJavaAccountId   { get; set; } = "";
    public List<JavaAccount> JavaAccounts { get; set; } = new();

    // ── Custom DLL ──────────────────────────────────────────────
    public string CustomDllPath         { get; set; } = "";

    // ── CurseForge ──────────────────────────────────────────────
    public string CurseForgeApiKey        { get; set; } = "";

    // ── Updates ──────────────────────────────────────────────────
    public bool   CheckUpdatesOnStartup   { get; set; } = true;
    public string SkippedLauncherVersion  { get; set; } = "";
    public string LastUpdateCheck         { get; set; } = "";

    // ── Xbox Profile ────────────────────────────────────────────────
    public string XboxGamertag         { get; set; } = "";
    public string XboxXuid             { get; set; } = "";
    public string XboxGamerPictureUrl  { get; set; } = "";
    public string XboxGamerscore       { get; set; } = "";
    public string XboxAccountTier      { get; set; } = "";
    public string XboxBio              { get; set; } = "";
    // Live OAuth refresh token (00000000402b5328 / MBI_SSL) — enables silent
    // re-auth without prompting the user again.
    public string XboxLiveRefreshToken { get; set; } = "";

    // ── Discord ──────────────────────────────────────────────────
    public bool   DiscordLoggedIn  { get; set; } = false;
    public string DiscordUserId    { get; set; } = "";
    public string DiscordUsername  { get; set; } = "";
    public string DiscordAvatar    { get; set; } = "";
    public string DiscordToken     { get; set; } = "";

    // ── Footer Profile Display ───────────────────────────────────
    // Which account to show in the bottom-left profile area.
    //   "auto"    — Xbox if signed in, otherwise Discord, otherwise local user
    //   "xbox"    — always prefer Xbox (falls back to none if not signed in)
    //   "discord" — always prefer Discord (falls back to none if not signed in)
    public string ProfileDisplayMode { get; set; } = "auto";

    // ── Saved Servers ────────────────────────────────────────────
    public List<SavedServer> SavedServers { get; set; } = new();

    // ── Bedrock instances (copy-sync isolation) ──────────────────
    public string BedrockActiveInstanceId { get; set; } = "";

    // ── First-run onboarding ──────────────────────────────────────
    public bool   OnboardingCompleted { get; set; } = false;

    // ── Localization ───────────────────────────────────────────────
    public string Language { get; set; } = "en";

    // ── Announcements ──────────────────────────────────────────────
    // Id of the last announcement the user dismissed, so the same one
    // doesn't reappear after being closed once.
    public string LastDismissedAnnouncementId { get; set; } = "";
}

public class SavedServer
{
    public string Name      { get; set; } = "";
    public string Address   { get; set; } = "";
    public int    Port      { get; set; } = 19132; // Bedrock default
    public string Icon      { get; set; } = "";    // Font Awesome class, e.g. "fa-solid fa-cube". Empty = generic server icon.
    public string IconColor { get; set; } = "";    // Hex color for the icon background tile. Empty = accent.
}
