using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace GlacierLauncher.Services;

public sealed record ModInfo(
    string FileName, string ModId, string Name,
    IReadOnlyList<string> Provides, IReadOnlyList<string> RequiredDeps,
    string Loader, bool Disabled);

public sealed record ModReport(IReadOnlyList<ModInfo> Mods, IReadOnlyList<string> Warnings)
{
    public static readonly ModReport Empty = new(Array.Empty<ModInfo>(), Array.Empty<string>());
}

/// <summary>
/// Reads installed Java mod jars and flags problems: duplicate mod ids and
/// missing required dependencies. Understands Fabric/Quilt (fabric.mod.json /
/// quilt.mod.json) and Forge/NeoForge (META-INF/mods.toml). Loader-provided ids
/// (minecraft, fabric, forge, …) are treated as always satisfied.
/// </summary>
public static class JavaModAnalyzer
{
    private static readonly HashSet<string> BuiltIns = new(StringComparer.OrdinalIgnoreCase)
    {
        "minecraft", "java", "fabricloader", "fabric", "fabric-api", "fabric_api",
        "forge", "neoforge", "quilt_loader", "quilt_base", "quilted_fabric_api", "mcp"
    };

    public static ModReport Analyze(IEnumerable<string> jarPaths)
    {
        var mods = jarPaths.Select(TryRead).Where(m => m != null).Select(m => m!).ToList();
        var enabled = mods.Where(m => !m.Disabled).ToList();

        var provided = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var m in enabled)
        {
            if (!string.IsNullOrEmpty(m.ModId)) provided.Add(m.ModId);
            foreach (var p in m.Provides) provided.Add(p);
        }

        var warnings = new List<string>();

        foreach (var grp in enabled.Where(m => !string.IsNullOrEmpty(m.ModId))
                                   .GroupBy(m => m.ModId, StringComparer.OrdinalIgnoreCase)
                                   .Where(g => g.Count() > 1))
        {
            warnings.Add($"Duplicate mod \"{grp.Key}\": {string.Join(", ", grp.Select(g => g.FileName))}");
        }

        foreach (var m in enabled)
        {
            foreach (var dep in m.RequiredDeps)
            {
                if (BuiltIns.Contains(dep) || provided.Contains(dep)) continue;
                warnings.Add($"\"{m.Name}\" needs \"{dep}\", which isn't installed");
            }
        }

        return new ModReport(mods, warnings);
    }

    private static ModInfo? TryRead(string path)
    {
        var disabled = path.EndsWith(".disabled", StringComparison.OrdinalIgnoreCase);
        var fileName = Path.GetFileName(path);
        try
        {
            using var zip = ZipFile.OpenRead(path);

            var fabric = zip.GetEntry("fabric.mod.json") ?? zip.GetEntry("quilt.mod.json");
            if (fabric != null) return ReadFabric(fabric, fileName, disabled);

            var toml = zip.GetEntry("META-INF/mods.toml") ?? zip.GetEntry("META-INF/neoforge.mods.toml");
            if (toml != null) return ReadForge(toml, fileName, disabled);
        }
        catch { /* unreadable jar — fall through to a bare entry */ }

        return new ModInfo(fileName, "", Path.GetFileNameWithoutExtension(fileName),
            Array.Empty<string>(), Array.Empty<string>(), "unknown", disabled);
    }

    private static ModInfo ReadFabric(ZipArchiveEntry entry, string fileName, bool disabled)
    {
        try
        {
            using var s = entry.Open();
            using var doc = JsonDocument.Parse(ReadAll(s));
            var root = doc.RootElement;
            if (root.TryGetProperty("quilt_loader", out var ql)) root = ql;   // Quilt nests metadata

            var id   = Str(root, "id");
            var name = Str(root, "name");

            var provides = new List<string>();
            if (root.TryGetProperty("provides", out var pv) && pv.ValueKind == JsonValueKind.Array)
                foreach (var p in pv.EnumerateArray())
                    if (p.GetString() is { } ps) provides.Add(ps);

            var deps = new List<string>();
            if (root.TryGetProperty("depends", out var dp))
            {
                if (dp.ValueKind == JsonValueKind.Object)
                    foreach (var d in dp.EnumerateObject()) deps.Add(d.Name);
                else if (dp.ValueKind == JsonValueKind.Array)
                    foreach (var d in dp.EnumerateArray())
                    {
                        if (d.ValueKind == JsonValueKind.String && d.GetString() is { } ds) deps.Add(ds);
                        else if (d.ValueKind == JsonValueKind.Object && d.TryGetProperty("id", out var did)
                                 && did.GetString() is { } ds2) deps.Add(ds2);
                    }
            }

            return new ModInfo(fileName, id, string.IsNullOrEmpty(name) ? id : name, provides, deps, "fabric", disabled);
        }
        catch
        {
            return new ModInfo(fileName, "", Path.GetFileNameWithoutExtension(fileName),
                Array.Empty<string>(), Array.Empty<string>(), "fabric", disabled);
        }
    }

    private static ModInfo ReadForge(ZipArchiveEntry entry, string fileName, bool disabled)
    {
        try
        {
            using var s = entry.Open();
            var text = ReadAll(s);
            // First modId belongs to the [[mods]] block (it precedes any dependencies).
            var id   = Regex.Match(text, @"modId\s*=\s*""([^""]+)""").Groups[1].Value;
            var name = Regex.Match(text, @"displayName\s*=\s*""([^""]+)""").Groups[1].Value;

            // Parse each [[dependencies.*]] block and keep the mandatory/required ones.
            var deps = new List<string>();
            foreach (Match block in Regex.Matches(text, @"\[\[dependencies[^\]]*\]\]([\s\S]*?)(?=\[\[|\z)"))
            {
                var body  = block.Groups[1].Value;
                var depId = Regex.Match(body, @"modId\s*=\s*""([^""]+)""").Groups[1].Value;
                if (string.IsNullOrEmpty(depId)) continue;

                var required = Regex.IsMatch(body, @"mandatory\s*=\s*true",  RegexOptions.IgnoreCase)
                            || Regex.IsMatch(body, @"type\s*=\s*""required""", RegexOptions.IgnoreCase);
                if (required
                    && !string.Equals(depId, id, StringComparison.OrdinalIgnoreCase)
                    && !BuiltIns.Contains(depId))
                    deps.Add(depId);
            }

            var display = !string.IsNullOrEmpty(name) ? name
                        : !string.IsNullOrEmpty(id)   ? id
                        : Path.GetFileNameWithoutExtension(fileName);
            return new ModInfo(fileName, id, display, Array.Empty<string>(), deps, "forge", disabled);
        }
        catch
        {
            return new ModInfo(fileName, "", Path.GetFileNameWithoutExtension(fileName),
                Array.Empty<string>(), Array.Empty<string>(), "forge", disabled);
        }
    }

    private static string Str(JsonElement e, string prop) =>
        e.TryGetProperty(prop, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() ?? "" : "";

    private static string ReadAll(Stream s)
    {
        using var r = new StreamReader(s);
        return r.ReadToEnd();
    }
}
