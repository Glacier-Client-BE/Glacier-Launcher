namespace GlacierLauncher.Models;

public class BedrockBackup
{
    public string Name      { get; set; } = "";
    public string FilePath  { get; set; } = "";
    public long   SizeBytes { get; set; }
    public string CreatedAt { get; set; } = "";
}
