using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Bedrock screenshots come from two places: the game's own in-game capture
/// (F1 by default, or bound in controls), saved as .jpeg files under
/// com.mojang\Screenshots\&lt;xbox-user-id&gt;\, and Xbox Game Bar
/// (Win+Alt+PrtScn), which saves everything to Videos\Captures. Both are
/// listed here, merged and sorted by most recent.
/// </summary>
public class BedrockScreenshotService
{
    public static string CapturesDir =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyVideos), "Captures");

    public static string ComMojangScreenshotsDir =>
        Path.Combine(CurseForgeService.ComMojangRoot, "Screenshots");

    public List<JavaInstanceFile> ListScreenshots()
    {
        var result = new List<JavaInstanceFile>();

        if (Directory.Exists(ComMojangScreenshotsDir))
        {
            foreach (var file in Directory.EnumerateFiles(ComMojangScreenshotsDir, "*.jpeg", SearchOption.AllDirectories))
            {
                try
                {
                    var info = new FileInfo(file);
                    result.Add(new JavaInstanceFile
                    {
                        Name = Path.GetFileName(file),
                        Path = info.FullName,
                        Kind = "screenshot",
                        SizeBytes = info.Length,
                        ModifiedAt = info.LastWriteTimeUtc.ToString("o")
                    });
                }
                catch { /* skip unreadable file, keep listing the rest */ }
            }
        }

        if (Directory.Exists(CapturesDir))
        {
            foreach (var file in Directory.EnumerateFiles(CapturesDir, "*.png", SearchOption.TopDirectoryOnly))
            {
                var name = Path.GetFileName(file);
                if (!name.Contains("minecraft", StringComparison.OrdinalIgnoreCase)) continue;

                try
                {
                    var info = new FileInfo(file);
                    result.Add(new JavaInstanceFile
                    {
                        Name = name,
                        Path = info.FullName,
                        Kind = "screenshot",
                        SizeBytes = info.Length,
                        ModifiedAt = info.LastWriteTimeUtc.ToString("o")
                    });
                }
                catch { /* skip unreadable file, keep listing the rest */ }
            }
        }

        return result.OrderByDescending(f => f.ModifiedAt).ToList();
    }

    public void OpenFolder()
    {
        // Prefer the real in-game screenshots folder; Game Bar's Captures is the
        // fallback when the player hasn't taken any in-game screenshots yet.
        var dir = Directory.Exists(ComMojangScreenshotsDir) ? ComMojangScreenshotsDir : CapturesDir;
        Directory.CreateDirectory(dir);
        System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
        {
            FileName = dir,
            UseShellExecute = true
        });
    }

    public void DeleteFile(string path)
    {
        if (File.Exists(path)) File.Delete(path);
    }
}
