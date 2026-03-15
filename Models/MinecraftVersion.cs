namespace MinecraftLauncher.Models;

public class MinecraftVersion
{
    public string Tag { get; set; } = "";
    public string DisplayName { get; set; } = "";
    public bool IsDownloaded { get; set; } = false;
    public bool IsDownloading { get; set; } = false;
    public bool IsLaunching { get; set; } = false;
    public string? ErrorMessage { get; set; }
}
