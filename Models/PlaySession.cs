namespace GlacierLauncher.Models;

/// <summary>One recorded play session — the unit behind the stats dashboard.</summary>
public class PlaySession
{
    public string StartedAt    { get; set; } = "";  // ISO-8601 UTC
    public long   Seconds      { get; set; }
    public string Edition      { get; set; } = "";  // bedrock | java
    public string Label        { get; set; } = "";  // client / version launched
}
