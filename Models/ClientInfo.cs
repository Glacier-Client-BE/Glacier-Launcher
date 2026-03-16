namespace MinecraftLauncher.Models;

public enum ClientType { Latite, Flarial, OderSo }

public class ClientInfo
{
    public ClientType Type        { get; set; }
    public string     Name        { get; set; } = "";
    public string?    Version     { get; set; }
    public bool       IsDownloaded { get; set; }
    public bool       IsDownloading { get; set; }
    public bool       IsUpToDate  { get; set; }
    public string?    ErrorMessage { get; set; }
    public double     Progress    { get; set; }
}
