namespace GlacierLauncher.Models;

public class JavaVersion
{
    public string Id           { get; set; } = "";   // e.g. "1.21.4" or "24w36a"
    public string Type         { get; set; } = "";   // release | snapshot | old_beta | old_alpha
    public string Url          { get; set; } = "";   // version JSON URL on piston-meta
    public string Sha1         { get; set; } = "";   // SHA-1 of the version JSON
    public string ReleaseTime  { get; set; } = "";   // ISO-8601
    public string Time         { get; set; } = "";   // ISO-8601 (last update of manifest entry)
    public int    ComplianceLevel { get; set; }

    // ── Local state ──────────────────────────────────────────────
    public bool   IsInstalled  { get; set; }         // versions/<id>/<id>.json exists
    public bool   HasJar       { get; set; }         // <id>.jar exists too — required to launch without a full download
    public bool   IsActive     { get; set; }         // user-selected default
    public bool   IsLaunching  { get; set; }
    public bool   IsInstalling { get; set; }
    public double InstallPercent { get; set; }
    public string InstallStage  { get; set; } = "";
    public string? ErrorMessage { get; set; }

    public string TypeLabel => Type switch
    {
        "release"   => "Release",
        "snapshot"  => "Snapshot",
        "old_beta"  => "Beta",
        "old_alpha" => "Alpha",
        _           => Type
    };
}
