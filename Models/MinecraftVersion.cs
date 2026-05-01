namespace GlacierLauncher.Models;

public class MinecraftVersion
{
    public string  Tag           { get; set; } = "";
    public string  DisplayName   { get; set; } = "";
    public bool    IsDownloaded  { get; set; } = false;
    public bool    IsDownloading { get; set; } = false;
    public bool    IsLaunching   { get; set; } = false;
    public string? ErrorMessage  { get; set; }

    // GitHub release metadata (LatiteClient/Latite). DownloadUrl is required for
    // download since the new repo's asset name (LatiteNightly.dll) and tag do not
    // follow a constructable URL pattern.
    public string? DownloadUrl   { get; set; }
    public long    AssetSize     { get; set; }
    public string? PublishedAt   { get; set; }
    public string? Variant       { get; set; } // "Nightly" | "Debug" | "Release"
}
