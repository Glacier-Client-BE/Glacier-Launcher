using System.IO;

namespace GlacierLauncher.Models;

public sealed class GlacierClientVersion
{
    public string Id          { get; init; } = "";
    public string Name        { get; init; } = "";
    public string ReleaseDate { get; init; } = "";
    public string Tag         { get; init; } = "";
    public string Url         { get; init; } = "";
    public string Sha256      { get; init; } = "";
    public bool   Fabric      { get; init; }
    public bool   Forge       { get; init; }
    public int    JavaVersion  { get; init; }
    public string Changelog   { get; init; } = "";

    public string LoaderLabel  => Fabric ? "Fabric" : Forge ? "Forge" : "Unknown";
    public string LocalJarName => $"Glacier-{Id}.jar";

    public string InstallDir =>
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            ".glacier", "versions", Id);

    public string JarPath     => Path.Combine(InstallDir, LocalJarName);
    public bool   IsInstalled => File.Exists(JarPath);
}

public sealed class GlacierManifest
{
    public int    SchemaVersion { get; init; }
    public string LatestRelease { get; init; } = "";
    public string ReleaseDate   { get; init; } = "";
    public List<GlacierClientVersion> Versions { get; init; } = [];
    public GlacierLauncherMeta? Launcher { get; init; }
}

public sealed class GlacierLauncherMeta
{
    public string Version { get; init; } = "";
    public string Url     { get; init; } = "";
    public string Sha256  { get; init; } = "";
}
