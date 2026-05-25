namespace GlacierLauncher.Models;

public class JavaInstance
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
    public string VersionId { get; set; } = "";
    public string Directory { get; set; } = "";
    public string CreatedAt { get; set; } = "";
    public string UpdatedAt { get; set; } = "";
}

public class JavaInstanceFile
{
    public string Name { get; set; } = "";
    public string Path { get; set; } = "";
    public string Kind { get; set; } = "";
    public long SizeBytes { get; set; }
    public string ModifiedAt { get; set; } = "";
    public bool IsDisabled { get; set; }
    public string DependencyHint { get; set; } = "";
}
