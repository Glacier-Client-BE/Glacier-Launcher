namespace GlacierLauncher.Models;

public class BedrockWorld
{
    public string  Name       { get; set; } = "";
    public string  FolderPath { get; set; } = "";
    public long    SizeBytes  { get; set; }
    public string  ModifiedAt { get; set; } = "";
    public string? IconPath   { get; set; } // full path to world_icon.jpeg, if present
}
