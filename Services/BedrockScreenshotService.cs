using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Bedrock (UWP) has no built-in screenshot key of its own — players capture
/// with Xbox Game Bar (Win+Alt+PrtScn), which saves everything to
/// Videos\Captures. This lists just the Minecraft-named ones from there.
/// </summary>
public class BedrockScreenshotService
{
    public static string CapturesDir =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyVideos), "Captures");

    public List<JavaInstanceFile> ListScreenshots()
    {
        var result = new List<JavaInstanceFile>();
        if (!Directory.Exists(CapturesDir)) return result;

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

        return result.OrderByDescending(f => f.ModifiedAt).ToList();
    }

    public void OpenFolder()
    {
        Directory.CreateDirectory(CapturesDir);
        System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
        {
            FileName = CapturesDir,
            UseShellExecute = true
        });
    }

    public void DeleteFile(string path)
    {
        if (File.Exists(path)) File.Delete(path);
    }
}
