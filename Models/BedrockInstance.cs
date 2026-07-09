namespace GlacierLauncher.Models;

public class BedrockInstance
{
    public string Id          { get; set; } = "";
    public string Name        { get; set; } = "";
    // Which downloaded vanilla version (VanillaVersionService.VersionsDirectory)
    // this instance should register when switched to. Empty = leave whatever
    // version is currently registered alone. Bedrock only has one installed
    // package at a time, so switching TO this instance also swaps the
    // machine-wide registered appx to match.
    public string VersionTag  { get; set; } = "";
    public string Directory   { get; set; } = "";
    public string CreatedAt   { get; set; } = "";
    public string UpdatedAt   { get; set; } = "";
}
