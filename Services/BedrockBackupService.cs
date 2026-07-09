using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Snapshots and restores the Bedrock game-data folders Glacier manages —
/// worlds, resource/behavior/skin packs, and Minecraft's own local settings —
/// into a single zip. Glacier doesn't isolate Bedrock into per-version
/// instances the way it does for Java, so a "backup" here is a full snapshot
/// of the shared com.mojang state rather than a single instance's data; still
/// useful before a risky pack install or Windows reset.
/// </summary>
public class BedrockBackupService
{
    // Folder names under com.mojang that make up "the user's stuff" worth
    // snapshotting. Deliberately excludes caches/logs (resource_packs_temp,
    // minecraftpe/logs, etc.) to keep backups small and fast. Also used by
    // BedrockInstanceService to know what to copy-sync between instances.
    public static readonly string[] ManagedFolders =
        { "minecraftWorlds", "resource_packs", "behaviour_packs", "skin_packs", "minecraftpe" };

    public static string ComMojangRoot => Path.GetDirectoryName(CurseForgeService.WorldsDir)!;

    public static string BackupsDir =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "bedrock-backups");

    public List<BedrockBackup> ListBackups()
    {
        var result = new List<BedrockBackup>();
        if (!Directory.Exists(BackupsDir)) return result;

        foreach (var file in Directory.EnumerateFiles(BackupsDir, "*.zip"))
        {
            try
            {
                var fi = new FileInfo(file);
                result.Add(new BedrockBackup
                {
                    Name      = Path.GetFileNameWithoutExtension(file),
                    FilePath  = file,
                    SizeBytes = fi.Length,
                    CreatedAt = fi.CreationTimeUtc.ToString("o")
                });
            }
            catch { /* skip unreadable entry, keep listing the rest */ }
        }

        return result.OrderByDescending(b => b.CreatedAt).ToList();
    }

    public Task<string> CreateBackupAsync(string? label = null) => CreateBackupFromFolderAsync(ComMojangRoot, label);

    public Task RestoreBackupAsync(string backupPath) => RestoreBackupIntoFolderAsync(backupPath, ComMojangRoot);

    /// <summary>Zips the managed subfolders of <paramref name="sourceRoot"/> (either the live
    /// com.mojang folder or a stored instance folder) into a backup under <see cref="BackupsDir"/>.</summary>
    public async Task<string> CreateBackupFromFolderAsync(string sourceRoot, string? label = null)
    {
        Directory.CreateDirectory(BackupsDir);

        var stamp = DateTime.Now.ToString("yyyy-MM-dd_HH-mm-ss");
        var safeLabel = string.IsNullOrWhiteSpace(label)
            ? ""
            : "_" + string.Join("_", label.Split(Path.GetInvalidFileNameChars(), StringSplitOptions.RemoveEmptyEntries));
        var destPath = Path.Combine(BackupsDir, $"backup_{stamp}{safeLabel}.zip");

        await Task.Run(() =>
        {
            using var archive = ZipFile.Open(destPath, ZipArchiveMode.Create);
            var foundAny = false;

            foreach (var folder in ManagedFolders)
            {
                var srcDir = Path.Combine(sourceRoot, folder);
                if (!Directory.Exists(srcDir)) continue;
                foundAny = true;

                foreach (var file in Directory.EnumerateFiles(srcDir, "*", SearchOption.AllDirectories))
                {
                    var relative = Path.GetRelativePath(sourceRoot, file).Replace('\\', '/');
                    try { archive.CreateEntryFromFile(file, relative, CompressionLevel.Optimal); }
                    catch { /* skip locked/unreadable file, keep the rest of the backup */ }
                }
            }

            if (!foundAny)
                throw new InvalidOperationException("Nothing to back up yet — no worlds or packs found.");
        });

        return destPath;
    }

    /// <summary>Restores a backup zip into <paramref name="targetRoot"/>, wiping the top-level
    /// folders the backup contains first so no stale data survives the restore.</summary>
    public async Task RestoreBackupIntoFolderAsync(string backupPath, string targetRoot)
    {
        if (!File.Exists(backupPath))
            throw new FileNotFoundException("Backup file not found.", backupPath);

        await Task.Run(() =>
        {
            using var archive = ZipFile.OpenRead(backupPath);

            var topLevelFolders = archive.Entries
                .Select(e => e.FullName.Split('/')[0])
                .Where(f => !string.IsNullOrEmpty(f))
                .Distinct()
                .ToList();

            foreach (var folder in topLevelFolders)
            {
                var target = Path.Combine(targetRoot, folder);
                if (Directory.Exists(target))
                {
                    try { Directory.Delete(target, recursive: true); } catch { /* best effort */ }
                }
            }

            foreach (var entry in archive.Entries)
            {
                if (string.IsNullOrEmpty(entry.Name)) continue; // directory entry
                var destPath = Path.Combine(targetRoot, entry.FullName.Replace('/', Path.DirectorySeparatorChar));
                Directory.CreateDirectory(Path.GetDirectoryName(destPath)!);
                entry.ExtractToFile(destPath, overwrite: true);
            }
        });
    }

    public void DeleteBackup(string backupPath)
    {
        if (File.Exists(backupPath)) File.Delete(backupPath);
    }

    public void OpenBackupsFolder()
    {
        Directory.CreateDirectory(BackupsDir);
        System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
        {
            FileName = BackupsDir,
            UseShellExecute = true
        });
    }
}
