namespace GlacierLauncher.Models;

public class VanillaVersion
{
    public string Version      { get; set; } = "";
    public string UpdateId     { get; set; } = "";
    public string PackageName  { get; set; } = "";
    public string Channel      { get; set; } = "Release";
    public bool   IsInstalled  { get; set; }
    public bool   IsDownloaded { get; set; }
    public bool   IsDownloading { get; set; }
    public bool   IsSwitching  { get; set; }
    public bool   IsActive     { get; set; }
    public double Progress     { get; set; }
    public string? ErrorMessage { get; set; }
    public long   SizeBytes    { get; set; }
    public System.Threading.CancellationTokenSource? DownloadCts { get; set; }

    // Legacy Bedrock versions are all "1.x.y.z"; the newer calendar-based scheme (e.g. "26.1")
    // uses a non-1 major, so we prefix those with "v" to match Mojang's own marketing naming.
    private bool IsCalendarScheme => !Version.StartsWith("1.");

    public string DisplayName
    {
        get
        {
            var baseName = IsCalendarScheme ? $"v{Version}" : Version;
            return Channel == "Preview" ? $"{baseName} Preview" : baseName;
        }
    }

    public string SizeLabel => SizeBytes switch
    {
        >= 1_073_741_824 => $"{SizeBytes / 1_073_741_824.0:F1} GB",
        >= 1_048_576     => $"{SizeBytes / 1_048_576.0:F0} MB",
        > 0              => $"{SizeBytes / 1024.0:F0} KB",
        _                => ""
    };
}
