namespace GlacierLauncher.Models;

public class BedrockInstance
{
    public string Id          { get; set; } = "";
    public string Name        { get; set; } = "";
    // Informational only — Bedrock only has one installed package at a time,
    // so this doesn't drive an automatic reinstall; it just reminds the user
    // which version they were playing with this instance's data.
    public string VersionTag  { get; set; } = "";
    public string Directory   { get; set; } = "";
    public string CreatedAt   { get; set; } = "";
    public string UpdatedAt   { get; set; } = "";
}
