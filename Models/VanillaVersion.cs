namespace GlacierLauncher.Models;

public class VanillaVersion
{
    public string Version      { get; set; } = "";
    public string UpdateId     { get; set; } = "";
    public string PackageName  { get; set; } = "";
    public bool   IsInstalled  { get; set; }
    public bool   IsDownloaded { get; set; }
    public bool   IsDownloading { get; set; }
    public bool   IsSwitching  { get; set; }
    public bool   IsActive     { get; set; }
    public double Progress     { get; set; }
    public string? ErrorMessage { get; set; }
    public long   SizeBytes    { get; set; }
    public System.Threading.CancellationTokenSource? DownloadCts { get; set; }

    public string SizeLabel => SizeBytes switch
    {
        >= 1_073_741_824 => $"{SizeBytes / 1_073_741_824.0:F1} GB",
        >= 1_048_576     => $"{SizeBytes / 1_048_576.0:F0} MB",
        > 0              => $"{SizeBytes / 1024.0:F0} KB",
        _                => ""
    };
}
