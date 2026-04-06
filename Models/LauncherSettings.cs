namespace GlacierLauncher.Models;

public class LauncherSettings
{
    // ── Core ─────────────────────────────────────────────────────
    public string SelectedClient       { get; set; } = "Latite Client";
    public bool   DiscordRichPresence  { get; set; } = true;
    public string LastUsedVersion      { get; set; } = "";
    public string Username             { get; set; } = "";
    public string UserHandle           { get; set; } = "";
    public bool   CloseAfterLaunch     { get; set; } = false;
    public bool   AutoInject           { get; set; } = false;
    public int    InjectionDelayMs     { get; set; } = 4000;

    // ── Appearance ───────────────────────────────────────────────
    public string AccentColor          { get; set; } = "#7289da";
    public string ThemePreset          { get; set; } = "dark";   // dark | darker | midnight | slate
    public int    BlurIntensity        { get; set; } = 14;       // px, 0-30
    public double BackgroundOpacity    { get; set; } = 0.80;     // 0.0-1.0

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

    // ── Updates ──────────────────────────────────────────────────
    public bool   CheckUpdatesOnStartup   { get; set; } = true;
    public string SkippedLauncherVersion  { get; set; } = "";
    public string LastUpdateCheck         { get; set; } = "";

    // ── Discord ──────────────────────────────────────────────────
    public bool   DiscordLoggedIn  { get; set; } = false;
    public string DiscordUserId    { get; set; } = "";
    public string DiscordUsername  { get; set; } = "";
    public string DiscordAvatar    { get; set; } = "";
    public string DiscordToken     { get; set; } = "";
}
