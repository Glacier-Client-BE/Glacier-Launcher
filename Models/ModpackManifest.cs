using System.Collections.Generic;

namespace GlacierLauncher.Models;

/// <summary>Parsed, source-agnostic description of a modpack to install.</summary>
public sealed class ModpackPlan
{
    public string Name            { get; set; } = "Modpack";
    public string MinecraftVersion{ get; set; } = "";
    public string Loader          { get; set; } = "";   // vanilla | fabric | quilt | forge | neoforge
    public string LoaderVersion   { get; set; } = "";
    public List<ModpackFile> Files { get; set; } = new();
    /// <summary>Human-facing links for files CurseForge won't let us download directly.</summary>
    public List<string> ManualDownloads { get; set; } = new();
}

/// <summary>A single file the pack pulls down, with its destination inside the instance.</summary>
public sealed class ModpackFile
{
    public string Url          { get; set; } = "";
    public string RelativePath { get; set; } = "";   // e.g. "mods/sodium.jar"
    public string Sha1         { get; set; } = "";
    public string Sha512       { get; set; } = "";
    public long   Size         { get; set; } = -1;
}
