namespace MinecraftLauncher.Models;

public class LauncherSettings
{
    public string SelectedClient       { get; set; } = "Latite Client";
    public bool   DiscordRichPresence  { get; set; } = true;
    public string LastUsedVersion      { get; set; } = "";
    public string Username             { get; set; } = "";
    public string UserHandle           { get; set; } = "";
    public bool   CloseAfterLaunch     { get; set; } = false;
    public bool   AutoInject           { get; set; } = false;
    public int    InjectionDelayMs     { get; set; } = 4000;
    public string AccentColor          { get; set; } = "#7289da";
    public double WindowWidth          { get; set; } = 740;
    public double WindowHeight         { get; set; } = 500;
    public bool   RememberWindowSize   { get; set; } = true;
}
