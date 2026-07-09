using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Lists / deletes the local Bedrock resource, behavior, and skin packs under
/// com.mojang. Installation itself already happens via CurseForgeService
/// (CurseForge downloads, drag-and-drop import) — this service is read/manage only.
/// </summary>
public class BedrockPackService
{
    public static string ResourcePacksDir    => CurseForgeService.ResourcePacksDir;
    public static string BehaviorPacksDir    => CurseForgeService.BehaviorPacksDir;
    public static string SkinPacksDir        => CurseForgeService.SkinPacksDir;
    public static string DevResourcePacksDir => CurseForgeService.DevResourcePacksDir;
    public static string DevBehaviorPacksDir => CurseForgeService.DevBehaviorPacksDir;
    public static string DevSkinPacksDir     => CurseForgeService.DevSkinPacksDir;

    public static string DirFor(string kind) => kind switch
    {
        "behavior"     => BehaviorPacksDir,
        "skin"         => SkinPacksDir,
        "resource-dev" => DevResourcePacksDir,
        "behavior-dev" => DevBehaviorPacksDir,
        "skin-dev"     => DevSkinPacksDir,
        _              => ResourcePacksDir
    };

    private static readonly string[] IconNames = { "pack_icon.png", "pack_icon.jpg", "pack_icon.jpeg" };

    public List<BedrockPack> ListPacks(string kind)
    {
        var dir = DirFor(kind);
        var result = new List<BedrockPack>();
        if (!Directory.Exists(dir)) return result;

        foreach (var packDir in Directory.EnumerateDirectories(dir))
        {
            try
            {
                var name = ReadPackName(packDir) ?? Path.GetFileName(packDir);
                result.Add(new BedrockPack
                {
                    Name       = name,
                    FolderPath = packDir,
                    Kind       = kind,
                    SizeBytes  = DirectorySize(packDir),
                    ModifiedAt = Directory.GetLastWriteTimeUtc(packDir).ToString("o"),
                    IconPath   = FindIcon(packDir)
                });
            }
            catch { /* skip unreadable pack, keep listing the rest */ }
        }

        return result.OrderBy(p => p.Name, StringComparer.OrdinalIgnoreCase).ToList();
    }

    private static string? FindIcon(string packDir)
    {
        foreach (var iconName in IconNames)
        {
            var path = Path.Combine(packDir, iconName);
            if (File.Exists(path)) return path;
        }
        return null;
    }

    private static string? ReadPackName(string packDir)
    {
        try
        {
            var manifestPath = Path.Combine(packDir, "manifest.json");
            if (!File.Exists(manifestPath)) return null;
            using var doc = JsonDocument.Parse(File.ReadAllText(manifestPath));
            if (doc.RootElement.TryGetProperty("header", out var header) &&
                header.TryGetProperty("name", out var nameEl))
            {
                var name = nameEl.GetString();
                if (!string.IsNullOrWhiteSpace(name))
                {
                    // Bedrock manifests sometimes reference a language pack key
                    // (e.g. "pack.name") instead of a literal string — those aren't
                    // resolvable without loading the pack's texts/en_US.lang, so just
                    // fall back to the folder name rather than showing the raw key.
                    return name.StartsWith("pack.", StringComparison.OrdinalIgnoreCase) ? null : name;
                }
            }
        }
        catch { }
        return null;
    }

    private static long DirectorySize(string dir)
    {
        long total = 0;
        try
        {
            foreach (var f in Directory.EnumerateFiles(dir, "*", SearchOption.AllDirectories))
            {
                try { total += new FileInfo(f).Length; } catch { }
            }
        }
        catch { }
        return total;
    }

    public void OpenFolder(string path)
    {
        if (!Directory.Exists(path)) throw new DirectoryNotFoundException(path);
        System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
        {
            FileName = path,
            UseShellExecute = true
        });
    }

    public void DeletePack(string path)
    {
        if (!Directory.Exists(path)) return;
        Directory.Delete(path, recursive: true);
    }
}
