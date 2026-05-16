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

    // ── Window ───────────────────────────────────────────────────
    public double WindowWidth          { get; set; } = 740;
    public double WindowHeight         { get; set; } = 500;
    public bool   RememberWindowSize   { get; set; } = true;
    public bool   MinimizeToTray       { get; set; } = false;

    // ── Recently Launched ────────────────────────────────────────
    public bool         ShowRecentlyLaunched { get; set; } = true;
    public List<string> RecentlyLaunched     { get; set; } = new();

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
    // Override path to .minecraft (empty = %APPDATA%\.minecraft)
    public string JavaMinecraftDir      { get; set; } = "";
    // Override path to javaw.exe (empty = auto-detect)
    public string JavaRuntimePath       { get; set; } = "";
    public int    JavaMaxRamMb          { get; set; } = 2048;
    // Extra JVM args appended to every Java launch (e.g. -XX:+UseG1GC, -Dfile.encoding=UTF-8)
    public string JavaCustomJvmArgs     { get; set; } = "";
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
    public string JavaAccessTokenExpiry { get; set; } = "";
    public string JavaSkinUrl           { get; set; } = "";

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
}

public class SavedServer
{
    public string Name      { get; set; } = "";
    public string Address   { get; set; } = "";
    public int    Port      { get; set; } = 19132; // Bedrock default
    public string Icon      { get; set; } = "";    // Font Awesome class, e.g. "fa-solid fa-cube". Empty = generic server icon.
    public string IconColor { get; set; } = "";    // Hex color for the icon background tile. Empty = accent.
}
