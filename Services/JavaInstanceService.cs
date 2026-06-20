using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public sealed class JavaInstanceService
{
    private readonly SettingsService _settings;
    private readonly string _indexPath;
    private readonly object _sync = new();
    private List<JavaInstance> _instances = new();

    public JavaInstanceService(SettingsService settings)
    {
        _settings = settings;
        _indexPath = Path.Combine(RootDir, "instances.json");
        Directory.CreateDirectory(RootDir);
        Directory.CreateDirectory(InstancesDir);
        Directory.CreateDirectory(CacheDir);
        Directory.CreateDirectory(SharedAssetsDir);
        Directory.CreateDirectory(SharedLibrariesDir);
        Load();
        EnsureDefaultInstance();
    }

    public static string RootDir => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "Glacier Launcher", "java");
    public static string InstancesDir => Path.Combine(RootDir, "instances");
    public static string CacheDir => Path.Combine(RootDir, "cache");
    public static string SharedAssetsDir => Path.Combine(CacheDir, "assets");
    public static string SharedLibrariesDir => Path.Combine(CacheDir, "libraries");

    public IReadOnlyList<JavaInstance> Instances
    {
        get
        {
            lock (_sync)
                return _instances.Select(Clone).ToList();
        }
    }

    public JavaInstance ActiveInstance
    {
        get
        {
            lock (_sync)
            {
                EnsureDefaultInstance();
                var id = _settings.Settings.JavaActiveInstanceId;
                var instance = _instances.FirstOrDefault(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase)) ?? _instances.First();
                EnsureLayout(instance);
                return Clone(instance);
            }
        }
    }

    public string ActiveMinecraftDir
    {
        get
        {
            var custom = _settings.Settings.JavaMinecraftDir;
            if (!string.IsNullOrWhiteSpace(custom))
            {
                Directory.CreateDirectory(custom);
                return custom;
            }
            return ActiveInstance.Directory;
        }
    }

    public JavaInstance Create(string name, string versionId = "")
    {
        lock (_sync)
        {
            var now = DateTime.UtcNow.ToString("o");
            var id = Slug(name);
            var baseId = id;
            var n = 2;
            while (_instances.Any(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase)))
                id = baseId + "-" + n++;
            var instance = new JavaInstance
            {
                Id = id,
                Name = string.IsNullOrWhiteSpace(name) ? "Default" : name.Trim(),
                VersionId = versionId,
                Directory = Path.Combine(InstancesDir, id),
                CreatedAt = now,
                UpdatedAt = now
            };
            EnsureLayout(instance);
            _instances.Add(instance);
            Save();
            return Clone(instance);
        }
    }

    public JavaInstance DuplicateActive()
    {
        lock (_sync)
        {
            var source = ActiveInstance;
            var copy = Create(source.Name + " Copy", source.VersionId);
            CopyDirectory(source.Directory, copy.Directory);
            Save();
            return copy;
        }
    }

    public void SetActive(string id)
    {
        lock (_sync)
        {
            var instance = _instances.FirstOrDefault(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase));
            if (instance == null)
                return;
            _settings.Settings.JavaActiveInstanceId = instance.Id;
            _settings.Settings.JavaActiveVersion = instance.VersionId;
            _settings.Save();
            EnsureLayout(instance);
        }
    }

    public void SetActiveVersion(string versionId)
    {
        lock (_sync)
        {
            EnsureDefaultInstance();
            var instance = _instances.FirstOrDefault(x => string.Equals(x.Id, _settings.Settings.JavaActiveInstanceId, StringComparison.OrdinalIgnoreCase)) ?? _instances.First();
            instance.VersionId = versionId;
            instance.UpdatedAt = DateTime.UtcNow.ToString("o");
            Save();
        }
    }

    public string CreateInstanceLockPath(string versionId)
    {
        var safe = string.IsNullOrWhiteSpace(versionId) ? "launch" : Slug(versionId);
        return Path.Combine(ActiveMinecraftDir, safe + ".lock");
    }

    public IReadOnlyList<JavaInstanceFile> ListFiles(string kind)
    {
        var dir = PathFor(kind);
        Directory.CreateDirectory(dir);
        var pattern = kind == "screenshots" ? "*.png" : "*";
        return Directory.EnumerateFiles(dir, pattern, SearchOption.TopDirectoryOnly)
            .Where(p => kind != "mods" || p.EndsWith(".jar", StringComparison.OrdinalIgnoreCase) || p.EndsWith(".disabled", StringComparison.OrdinalIgnoreCase))
            .Select(p => ToEntry(kind, p))
            .OrderByDescending(x => x.ModifiedAt)
            .ToList();
    }

    public void ToggleMod(string path)
    {
        if (!File.Exists(path))
            return;
        var target = path.EndsWith(".disabled", StringComparison.OrdinalIgnoreCase) ? path[..^9] : path + ".disabled";
        if (File.Exists(target))
            File.Delete(target);
        File.Move(path, target);
    }

    public void DeleteFile(string path)
    {
        if (File.Exists(path))
            File.Delete(path);
    }

    public string PathFor(string kind)
    {
        var root = ActiveMinecraftDir;
        var child = kind switch
        {
            "mods" => "mods",
            "saves" => "saves",
            "resourcepacks" => "resourcepacks",
            "shaderpacks" => "shaderpacks",
            "screenshots" => "screenshots",
            "schematics" => Path.Combine("config", "worldedit", "schematics"),
            "litematica" => "schematics",
            "backups" => "backups",
            _ => ""
        };
        return Path.Combine(root, child);
    }

    public void OpenFolder(string kind)
    {
        var dir = PathFor(kind);
        Directory.CreateDirectory(dir);
        System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo { FileName = dir, UseShellExecute = true });
    }

    public async Task<string?> BackupSavesAsync()
    {
        var saves = PathFor("saves");
        if (!Directory.Exists(saves) || !Directory.EnumerateFileSystemEntries(saves).Any())
            return null;
        var backups = PathFor("backups");
        Directory.CreateDirectory(backups);
        var zip = Path.Combine(backups, "saves-" + DateTime.Now.ToString("yyyyMMdd-HHmmss") + ".zip");
        await Task.Run(() => ZipFile.CreateFromDirectory(saves, zip, CompressionLevel.Fastest, false));
        return zip;
    }

    public async Task<string> ExportModpackAsync()
    {
        var exports = Path.Combine(RootDir, "exports");
        Directory.CreateDirectory(exports);
        var instance = ActiveInstance;
        var zip = Path.Combine(exports, instance.Id + "-" + DateTime.Now.ToString("yyyyMMdd-HHmmss") + ".zip");
        await Task.Run(() =>
        {
            using var archive = ZipFile.Open(zip, ZipArchiveMode.Create);
            AddDirectory(archive, Path.Combine(instance.Directory, "mods"), "overrides/mods");
            AddDirectory(archive, Path.Combine(instance.Directory, "config"), "overrides/config");
            AddDirectory(archive, Path.Combine(instance.Directory, "resourcepacks"), "overrides/resourcepacks");
            AddDirectory(archive, Path.Combine(instance.Directory, "shaderpacks"), "overrides/shaderpacks");
            var manifest = JsonSerializer.Serialize(new { minecraft = new { version = instance.VersionId }, manifestType = "minecraftModpack", manifestVersion = 1, name = instance.Name, version = "1.0.0", files = Array.Empty<object>(), overrides = "overrides" }, new JsonSerializerOptions { WriteIndented = true });
            var entry = archive.CreateEntry("manifest.json");
            using var writer = new StreamWriter(entry.Open());
            writer.Write(manifest);
        });
        return zip;
    }

    public async Task<JavaInstance> ImportModpackAsync(string zipPath)
    {
        if (!File.Exists(zipPath))
            throw new FileNotFoundException("Modpack file not found.", zipPath);
        var instance = Create(Path.GetFileNameWithoutExtension(zipPath));
        await Task.Run(() =>
        {
            using var archive = ZipFile.OpenRead(zipPath);
            foreach (var entry in archive.Entries)
            {
                if (string.IsNullOrEmpty(entry.Name))
                    continue;
                var path = entry.FullName.Replace('\\', '/');
                if (!path.StartsWith("overrides/", StringComparison.OrdinalIgnoreCase))
                    continue;
                var relative = path["overrides/".Length..].Replace('/', Path.DirectorySeparatorChar);
                var dest = Path.Combine(instance.Directory, relative);
                Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
                entry.ExtractToFile(dest, true);
            }
        });
        return instance;
    }

    private void Load()
    {
        try
        {
            if (!File.Exists(_indexPath))
                return;
            var json = File.ReadAllText(_indexPath);
            try
            {
                _instances = JsonSerializer.Deserialize<List<JavaInstance>>(json) ?? new();
            }
            catch (JsonException)
            {
                // Malformed index — keep a copy aside and rebuild from the default.
                JsonStore.QuarantineCorrupt(_indexPath);
                _instances = new();
            }
        }
        catch
        {
            // Transient read error: fall back to an empty list without touching disk.
            _instances = new();
        }
    }

    private void Save()
    {
        var json = JsonSerializer.Serialize(_instances, new JsonSerializerOptions { WriteIndented = true });
        JsonStore.WriteAtomic(_indexPath, json);
    }

    private void EnsureDefaultInstance()
    {
        if (_instances.Count == 0)
            Create("Default");
        if (string.IsNullOrWhiteSpace(_settings.Settings.JavaActiveInstanceId))
        {
            _settings.Settings.JavaActiveInstanceId = _instances[0].Id;
            _settings.Save();
        }
    }

    /// <summary>Sets a per-instance RAM override on the active instance (0 = use global).</summary>
    public void SetActiveRam(int minMb, int maxMb)
    {
        lock (_sync)
        {
            EnsureDefaultInstance();
            var instance = _instances.FirstOrDefault(x => string.Equals(x.Id, _settings.Settings.JavaActiveInstanceId, StringComparison.OrdinalIgnoreCase)) ?? _instances.First();
            instance.MaxRamMb = maxMb < 0 ? 0 : maxMb;
            instance.MinRamMb = minMb < 0 ? 0 : minMb;
            instance.UpdatedAt = DateTime.UtcNow.ToString("o");
            Save();
        }
    }

    private static JavaInstance Clone(JavaInstance instance) => new()
    {
        Id = instance.Id,
        Name = instance.Name,
        VersionId = instance.VersionId,
        Directory = instance.Directory,
        CreatedAt = instance.CreatedAt,
        UpdatedAt = instance.UpdatedAt,
        MaxRamMb = instance.MaxRamMb,
        MinRamMb = instance.MinRamMb
    };

    private static void EnsureLayout(JavaInstance instance)
    {
        Directory.CreateDirectory(instance.Directory);
        Directory.CreateDirectory(Path.Combine(instance.Directory, "versions"));
        Directory.CreateDirectory(Path.Combine(instance.Directory, "saves"));
        Directory.CreateDirectory(Path.Combine(instance.Directory, "mods"));
        Directory.CreateDirectory(Path.Combine(instance.Directory, "config"));
        Directory.CreateDirectory(Path.Combine(instance.Directory, "resourcepacks"));
        Directory.CreateDirectory(Path.Combine(instance.Directory, "shaderpacks"));
        Directory.CreateDirectory(Path.Combine(instance.Directory, "screenshots"));
        Directory.CreateDirectory(Path.Combine(instance.Directory, "crash-reports"));
        Directory.CreateDirectory(SharedAssetsDir);
        Directory.CreateDirectory(SharedLibrariesDir);
        EnsureLink(Path.Combine(instance.Directory, "assets"), SharedAssetsDir);
        EnsureLink(Path.Combine(instance.Directory, "libraries"), SharedLibrariesDir);
    }

    private static void EnsureLink(string linkPath, string targetPath)
    {
        if (Directory.Exists(linkPath))
            return;

        try
        {
            Directory.CreateSymbolicLink(linkPath, targetPath);
        }
        catch
        {
            // Fallback: Try creating a Junction on Windows if symbolic link fails
            try
            {
                if (System.Runtime.InteropServices.RuntimeInformation.IsOSPlatform(System.Runtime.InteropServices.OSPlatform.Windows))
                {
                    var psi = new ProcessStartInfo
                    {
                        FileName = "cmd.exe",
                        Arguments = $"/c mklink /j \"{linkPath}\" \"{targetPath}\"",
                        CreateNoWindow = true,
                        UseShellExecute = false
                    };
                    Process.Start(psi)?.WaitForExit();
                }
            }
            catch { }

            if (!Directory.Exists(linkPath))
                Directory.CreateDirectory(linkPath);
        }
    }

    private static JavaInstanceFile ToEntry(string kind, string path)
    {
        var info = new FileInfo(path);
        return new JavaInstanceFile
        {
            Name = info.Name,
            Path = info.FullName,
            Kind = kind,
            SizeBytes = info.Length,
            ModifiedAt = info.LastWriteTimeUtc.ToString("o"),
            IsDisabled = info.Name.EndsWith(".disabled", StringComparison.OrdinalIgnoreCase),
            DependencyHint = DetectDependencyHint(info.Name)
        };
    }

    private static string DetectDependencyHint(string name)
    {
        var lower = name.ToLowerInvariant();
        if (lower.Contains("sodium") || lower.Contains("iris") || lower.Contains("modmenu"))
            return "Fabric API may be required";
        if (lower.Contains("architectury"))
            return "Architectury API is required by many companion mods";
        if (lower.Contains("cloth-config"))
            return "Cloth Config is a dependency for many Fabric mods";
        if (lower.Contains("forge"))
            return "Forge profile required";
        return "";
    }

    private static void CopyDirectory(string source, string target)
    {
        Directory.CreateDirectory(target);
        foreach (var dir in Directory.EnumerateDirectories(source, "*", SearchOption.AllDirectories))
            Directory.CreateDirectory(dir.Replace(source, target));
        foreach (var file in Directory.EnumerateFiles(source, "*", SearchOption.AllDirectories))
        {
            var dest = file.Replace(source, target);
            Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
            File.Copy(file, dest, true);
        }
    }

    private static void AddDirectory(ZipArchive archive, string dir, string prefix)
    {
        if (!Directory.Exists(dir))
            return;
        foreach (var file in Directory.EnumerateFiles(dir, "*", SearchOption.AllDirectories))
        {
            var relative = Path.GetRelativePath(dir, file).Replace('\\', '/');
            archive.CreateEntryFromFile(file, prefix + "/" + relative, CompressionLevel.Fastest);
        }
    }

    private static string Slug(string value)
    {
        var raw = string.IsNullOrWhiteSpace(value) ? "default" : value.Trim().ToLowerInvariant();
        var chars = raw.Select(c => char.IsLetterOrDigit(c) ? c : '-').ToArray();
        var slug = new string(chars).Trim('-');
        while (slug.Contains("--", StringComparison.Ordinal))
            slug = slug.Replace("--", "-");
        return string.IsNullOrWhiteSpace(slug) ? "default" : slug;
    }
}
