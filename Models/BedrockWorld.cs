namespace GlacierLauncher.Models;

public class BedrockWorld
{
    public string Name       { get; set; } = "";
    public string FolderPath { get; set; } = "";
    public long   SizeBytes  { get; set; }
    public string ModifiedAt { get; set; } = "";
}
