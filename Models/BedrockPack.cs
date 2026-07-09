namespace GlacierLauncher.Models;

public class BedrockPack
{
    public string Name       { get; set; } = "";
    public string FolderPath { get; set; } = "";
    public string Kind       { get; set; } = ""; // resource | behavior | skin
    public long   SizeBytes  { get; set; }
    public string ModifiedAt { get; set; } = "";
}
