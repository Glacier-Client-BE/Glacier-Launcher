using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Lists / deletes / exports the local Bedrock worlds under com.mojang's
/// minecraftWorlds folder — the same tree CurseForgeService installs .mcworld
/// files into. Kept separate since world management isn't a CurseForge concern.
/// </summary>
public class BedrockWorldService
{
    public static string WorldsDir => CurseForgeService.WorldsDir;

    private static readonly string[] IconNames = { "world_icon.jpeg", "world_icon.jpg", "world_icon.png" };

    public List<BedrockWorld> ListWorlds()
    {
        var result = new List<BedrockWorld>();
        if (!Directory.Exists(WorldsDir)) return result;

        foreach (var dir in Directory.EnumerateDirectories(WorldsDir))
        {
            try
            {
                var dbPath = Path.Combine(dir, "db");
                var levelDatPath = Path.Combine(dir, "level.dat");
                // Not a real world folder (no db/level.dat) — skip stray junk.
                if (!Directory.Exists(dbPath) && !File.Exists(levelDatPath)) continue;

                var name = ReadWorldName(dir) ?? Path.GetFileName(dir);
                var size = DirectorySize(dir);
                var modified = Directory.GetLastWriteTimeUtc(dir).ToString("o");

                result.Add(new BedrockWorld
                {
                    Name       = name,
                    FolderPath = dir,
                    SizeBytes  = size,
                    ModifiedAt = modified,
                    IconPath   = FindIcon(dir)
                });
            }
            catch { /* skip unreadable folder, keep listing the rest */ }
        }

        return result.OrderByDescending(w => w.ModifiedAt).ToList();
    }

    private static string? FindIcon(string worldDir)
    {
        foreach (var iconName in IconNames)
        {
            var path = Path.Combine(worldDir, iconName);
            if (File.Exists(path)) return path;
        }
        return null;
    }

    private static string? ReadWorldName(string worldDir)
    {
        try
        {
            var levelNamePath = Path.Combine(worldDir, "levelname.txt");
            if (!File.Exists(levelNamePath)) return null;
            var name = File.ReadAllText(levelNamePath).Trim();
            return string.IsNullOrEmpty(name) ? null : name;
        }
        catch { return null; }
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

    public void OpenFolder(string worldPath)
    {
        if (!Directory.Exists(worldPath)) throw new DirectoryNotFoundException(worldPath);
        System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
        {
            FileName = worldPath,
            UseShellExecute = true
        });
    }

    public void DeleteWorld(string worldPath)
    {
        if (!Directory.Exists(worldPath)) return;
        Directory.Delete(worldPath, recursive: true);
    }

    /// <summary>Zips the world folder into a .mcworld file under the export folder, returning the output path.</summary>
    public async Task<string> ExportWorldAsync(string worldPath)
    {
        if (!Directory.Exists(worldPath)) throw new DirectoryNotFoundException(worldPath);

        var exportDir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "exported-worlds");
        Directory.CreateDirectory(exportDir);

        var folderName = Path.GetFileName(worldPath);
        var worldName  = ReadWorldName(worldPath) ?? folderName;
        var safeName   = string.Join("_", worldName.Split(Path.GetInvalidFileNameChars()));
        var destPath   = Path.Combine(exportDir, safeName + ".mcworld");

        // Avoid clobbering a previous export of the same name.
        var counter = 1;
        while (File.Exists(destPath))
        {
            destPath = Path.Combine(exportDir, $"{safeName} ({counter}).mcworld");
            counter++;
        }

        await Task.Run(() => ZipFile.CreateFromDirectory(worldPath, destPath, CompressionLevel.Optimal, includeBaseDirectory: false));
        return destPath;
    }
}
