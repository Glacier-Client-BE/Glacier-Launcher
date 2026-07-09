using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Gives Bedrock the same "isolated instance" idea Java already has, without
/// the risk of redirecting the UWP package's LocalState folder (that needs
/// elevated ACL work and can leave Minecraft unable to find its own data if
/// anything goes wrong). Instead, each instance is a stored copy of the
/// managed com.mojang folders (worlds/packs/settings); switching instances
/// copy-syncs the previously active instance's data OUT of the live
/// com.mojang folder and the target instance's data IN. Existing services
/// (BedrockWorldService, BedrockPackService, BedrockBackupService) already
/// operate on the live com.mojang folder, so they automatically become
/// per-instance — no changes needed there.
/// </summary>
public sealed class BedrockInstanceService
{
    private readonly SettingsService _settings;
    private readonly BedrockBackupService _backup;
    private readonly string _indexPath;
    private readonly object _sync = new();
    private List<BedrockInstance> _instances = new();

    public BedrockInstanceService(SettingsService settings, BedrockBackupService backup)
    {
        _settings = settings;
        _backup = backup;
        _indexPath = Path.Combine(RootDir, "instances.json");
        Directory.CreateDirectory(RootDir);
        Directory.CreateDirectory(InstancesDir);
        Load();
        EnsureDefaultInstance();
    }

    public static string RootDir => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "Glacier Launcher", "bedrock-instances");
    public static string InstancesDir => Path.Combine(RootDir, "instances");

    public IReadOnlyList<BedrockInstance> Instances
    {
        get { lock (_sync) return _instances.Select(Clone).ToList(); }
    }

    public BedrockInstance ActiveInstance
    {
        get
        {
            lock (_sync)
            {
                EnsureDefaultInstance();
                var id = _settings.Settings.BedrockActiveInstanceId;
                var instance = _instances.FirstOrDefault(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase))
                               ?? _instances.First();
                return Clone(instance);
            }
        }
    }

    public BedrockInstance Create(string name, string versionTag = "")
    {
        lock (_sync)
        {
            var now = DateTime.UtcNow.ToString("o");
            var id = Slug(name);
            var baseId = id;
            var n = 2;
            while (_instances.Any(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase)))
                id = baseId + "-" + n++;

            var instance = new BedrockInstance
            {
                Id = id,
                Name = string.IsNullOrWhiteSpace(name) ? "New Instance" : name.Trim(),
                VersionTag = versionTag,
                Directory = Path.Combine(InstancesDir, id),
                CreatedAt = now,
                UpdatedAt = now
            };
            Directory.CreateDirectory(instance.Directory);
            _instances.Add(instance);
            Save();
            return Clone(instance);
        }
    }

    public void Rename(string id, string newName)
    {
        if (string.IsNullOrWhiteSpace(newName)) return;
        lock (_sync)
        {
            var instance = _instances.FirstOrDefault(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase));
            if (instance == null) return;
            instance.Name = newName.Trim();
            instance.UpdatedAt = DateTime.UtcNow.ToString("o");
            Save();
        }
    }

    /// <summary>Deletes a stored (non-active) instance and its saved data. Never removes the last one or the active one.</summary>
    public bool Delete(string id)
    {
        lock (_sync)
        {
            if (_instances.Count <= 1) return false;
            if (string.Equals(_settings.Settings.BedrockActiveInstanceId, id, StringComparison.OrdinalIgnoreCase))
                return false; // must switch away first — deleting the active one would strand live data

            var instance = _instances.FirstOrDefault(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase));
            if (instance == null) return false;

            try { if (Directory.Exists(instance.Directory)) Directory.Delete(instance.Directory, true); }
            catch { /* files may be locked; drop the index entry regardless */ }

            _instances.Remove(instance);
            Save();
            return true;
        }
    }

    /// <summary>
    /// Swaps the live com.mojang folder to another instance's data: saves the
    /// currently active instance's live state into its stored folder, then
    /// copies the target instance's stored folder into the live folder. A
    /// brand-new (never-activated) target instance simply starts the live
    /// folder empty, matching the "new isolated profile" expectation.
    /// </summary>
    public async Task SwitchToAsync(string id)
    {
        BedrockInstance? target;
        BedrockInstance? current;
        lock (_sync)
        {
            EnsureDefaultInstance();
            target = _instances.FirstOrDefault(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase));
            if (target == null) throw new InvalidOperationException("Instance not found.");
            if (string.Equals(_settings.Settings.BedrockActiveInstanceId, id, StringComparison.OrdinalIgnoreCase))
                return; // already active
            current = _instances.FirstOrDefault(x => string.Equals(x.Id, _settings.Settings.BedrockActiveInstanceId, StringComparison.OrdinalIgnoreCase));
        }

        var root = BedrockBackupService.ComMojangRoot;

        if (current != null)
            await Task.Run(() => SyncFolderSet(root, current.Directory));

        await Task.Run(() => SyncFolderSet(target.Directory, root));

        lock (_sync)
        {
            _settings.Settings.BedrockActiveInstanceId = target.Id;
            _settings.Save();
            var stored = _instances.First(x => x.Id == target.Id);
            stored.UpdatedAt = DateTime.UtcNow.ToString("o");
            Save();
        }
    }

    /// <summary>Saves the live com.mojang state into the active instance's stored folder without switching — call before anything risky.</summary>
    public async Task SaveActiveAsync()
    {
        var active = ActiveInstance;
        await Task.Run(() => SyncFolderSet(BedrockBackupService.ComMojangRoot, active.Directory));
    }

    public Task<string> BackupInstanceAsync(BedrockInstance instance) =>
        _backup.CreateBackupFromFolderAsync(instance.Directory, instance.Name);

    public Task RestoreIntoInstanceAsync(string backupPath, BedrockInstance instance) =>
        _backup.RestoreBackupIntoFolderAsync(backupPath, instance.Directory);

    private static void SyncFolderSet(string sourceRoot, string destRoot)
    {
        Directory.CreateDirectory(destRoot);
        foreach (var folder in BedrockBackupService.ManagedFolders)
        {
            var src = Path.Combine(sourceRoot, folder);
            var dst = Path.Combine(destRoot, folder);

            if (Directory.Exists(dst))
            {
                try { Directory.Delete(dst, recursive: true); } catch { /* best effort */ }
            }

            if (!Directory.Exists(src)) continue;
            CopyDirectory(src, dst);
        }
    }

    private static void CopyDirectory(string source, string target)
    {
        Directory.CreateDirectory(target);
        foreach (var dir in Directory.EnumerateDirectories(source, "*", SearchOption.AllDirectories))
            Directory.CreateDirectory(dir.Replace(source, target));
        foreach (var file in Directory.EnumerateFiles(source, "*", SearchOption.AllDirectories))
        {
            var dest = file.Replace(source, target);
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
                File.Copy(file, dest, true);
            }
            catch { /* skip locked/unreadable file, keep syncing the rest */ }
        }
    }

    private void Load()
    {
        try
        {
            if (!File.Exists(_indexPath)) return;
            var json = File.ReadAllText(_indexPath);
            try { _instances = JsonSerializer.Deserialize<List<BedrockInstance>>(json) ?? new(); }
            catch (JsonException)
            {
                JsonStore.QuarantineCorrupt(_indexPath);
                _instances = new();
            }
        }
        catch { _instances = new(); }
    }

    private void Save()
    {
        var json = JsonSerializer.Serialize(_instances, new JsonSerializerOptions { WriteIndented = true });
        JsonStore.WriteAtomic(_indexPath, json);
    }

    // First-ever run of this feature: seed a "Default" instance from whatever is
    // currently live in com.mojang, so switching TO it later restores exactly
    // what was there before instances existed — nothing the user already had
    // gets lost or silently wiped by enabling this feature.
    private void EnsureDefaultInstance()
    {
        if (_instances.Count == 0)
        {
            var instance = Create("Default");
            var root = BedrockBackupService.ComMojangRoot;
            try { SyncFolderSet(root, instance.Directory); } catch { /* best effort seed */ }
            _settings.Settings.BedrockActiveInstanceId = instance.Id;
            _settings.Save();
        }
        else if (string.IsNullOrWhiteSpace(_settings.Settings.BedrockActiveInstanceId))
        {
            _settings.Settings.BedrockActiveInstanceId = _instances[0].Id;
            _settings.Save();
        }
    }

    private static BedrockInstance Clone(BedrockInstance instance) => new()
    {
        Id = instance.Id,
        Name = instance.Name,
        VersionTag = instance.VersionTag,
        Directory = instance.Directory,
        CreatedAt = instance.CreatedAt,
        UpdatedAt = instance.UpdatedAt
    };

    private static string Slug(string value)
    {
        var raw = string.IsNullOrWhiteSpace(value) ? "instance" : value.Trim().ToLowerInvariant();
        var chars = raw.Select(c => char.IsLetterOrDigit(c) ? c : '-').ToArray();
        var slug = new string(chars).Trim('-');
        while (slug.Contains("--", StringComparison.Ordinal))
            slug = slug.Replace("--", "-");
        return string.IsNullOrWhiteSpace(slug) ? "instance" : slug;
    }
}
