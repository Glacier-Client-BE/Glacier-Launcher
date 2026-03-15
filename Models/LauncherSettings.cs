namespace MinecraftLauncher.Models;

public class LauncherSettings
{
    public string SelectedClient { get; set; } = "Latite Client";
    public bool DiscordRichPresence { get; set; } = true;
    public string LastUsedVersion { get; set; } = "";
    public string Username { get; set; } = "";
    public string UserHandle { get; set; } = "";
}
