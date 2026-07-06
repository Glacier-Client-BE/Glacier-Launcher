using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Installs modpacks into fresh Java instances. Composes the existing content
/// services: CurseForge/Modrinth for the manifest + file URLs, DownloadService
/// for verified downloads, JavaInstanceService for the target instance and
/// override extraction, JavaModLoaderService for the loader, and
/// JavaRuntimeDownloadService as a fallback JRE for the Forge installer.
/// </summary>
public sealed class ModpackInstallService
{
    private readonly CurseForgeService        _cf;
    private readonly ModrinthService          _mr;
    private readonly JavaInstanceService      _instances;
    private readonly JavaModLoaderService     _loaders;
    private readonly JavaRuntimeDownloadService _runtime;
    private readonly DownloadService          _download = new();

    public ModpackInstallService(
        CurseForgeService cf, ModrinthService mr, JavaInstanceService instances,
        JavaModLoaderService loaders, JavaRuntimeDownloadService runtime)
    {
        _cf = cf; _mr = mr; _instances = instances; _loaders = loaders; _runtime = runtime;
    }

    public sealed class Progress
    {
        public string Stage   { get; set; } = "";
        public double Percent { get; set; }   // 0-100
        public List<string> ManualDownloads { get; } = new();
    }

    /// <summary>Installs a CurseForge modpack (classId 4471) by its addon id.</summary>
    public async Task<JavaInstance> InstallCurseForgeAsync(int modId, string packName, Action<Progress>? report, CancellationToken cancel)
    {
        var p = new Progress();
        void Tick(string s, double pct) { p.Stage = s; p.Percent = pct; report?.Invoke(p); }

        var files = await _cf.GetFilesAsync(modId);
        var packFile = files.FirstOrDefault()
            ?? throw new InvalidOperationException("This modpack has no downloadable files.");

        var work = NewWorkDir();
        var instance = _instances.Create(packName);
        try
        {
            Tick("Downloading pack…", 3);
            var zipPath = Path.Combine(work, "pack.zip");
            await _cf.DownloadFileToAsync(packFile, zipPath, new SimpleProgress(f => Tick("Downloading pack…", 3 + f * 12)), cancel);

            Tick("Reading manifest…", 16);
            var plan = ParseCurseForgeManifest(zipPath, packName);

            Tick("Applying overrides…", 20);
            await _instances.ExtractOverridesAsync(zipPath, instance.Id);

            await ResolveAndDownloadCurseForgeMods(plan, instance.Id, p, report, cancel);

            await InstallLoaderAsync(plan, instance, p, report, cancel);

            _instances.SetInstanceVersion(instance.Id, LoaderProfileId(plan));
            Tick("Done", 100);
            return instance;
        }
        catch
        {
            SafeDelete(instance.Id);
            throw;
        }
        finally { TryDeleteDir(work); }
    }

    /// <summary>Installs a Modrinth modpack from a .mrpack (downloaded by projectId or a local file).</summary>
    public async Task<JavaInstance> InstallModrinthAsync(string projectId, string packName, Action<Progress>? report, CancellationToken cancel)
    {
        var p = new Progress();
        void Tick(string s, double pct) { p.Stage = s; p.Percent = pct; report?.Invoke(p); }

        var version = await _mr.GetModpackFileAsync(projectId)
            ?? throw new InvalidOperationException("No downloadable .mrpack version found.");

        var work = NewWorkDir();
        try
        {
            Tick("Downloading pack…", 3);
            var mrpack = Path.Combine(work, "pack.mrpack");
            await _mr.DownloadToAsync(version, mrpack, new SimpleProgress(f => Tick("Downloading pack…", 3 + f * 12)), cancel);
            return await InstallMrpackFileAsync(mrpack, packName, report, cancel);
        }
        finally { TryDeleteDir(work); }
    }

    /// <summary>Installs a modpack from a local .mrpack file (drag-and-drop / file picker).</summary>
    public async Task<JavaInstance> InstallMrpackFileAsync(string mrpackPath, string packName, Action<Progress>? report, CancellationToken cancel)
    {
        var p = new Progress();
        void Tick(string s, double pct) { p.Stage = s; p.Percent = pct; report?.Invoke(p); }

        Tick("Reading index…", 16);
        var plan = ParseModrinthIndex(mrpackPath, packName);
        var instance = _instances.Create(string.IsNullOrWhiteSpace(plan.Name) ? packName : plan.Name);
        try
        {
            Tick("Applying overrides…", 20);
            await _instances.ExtractOverridesAsync(mrpackPath, instance.Id);

            await DownloadPlanFiles(plan, instance.Id, p, report, cancel);

            await InstallLoaderAsync(plan, instance, p, report, cancel);

            _instances.SetInstanceVersion(instance.Id, LoaderProfileId(plan));
            Tick("Done", 100);
            return instance;
        }
        catch
        {
            SafeDelete(instance.Id);
            throw;
        }
    }

    // ── Manifest parsing ───────────────────────────────────────

    private static ModpackPlan ParseCurseForgeManifest(string zipPath, string fallbackName)
    {
        using var archive = ZipFile.OpenRead(zipPath);
        var entry = archive.GetEntry("manifest.json")
            ?? throw new InvalidOperationException("Not a CurseForge modpack (no manifest.json).");
        using var reader = new StreamReader(entry.Open());
        using var doc = JsonDocument.Parse(reader.ReadToEnd());
        var root = doc.RootElement;

        var plan = new ModpackPlan { Name = fallbackName };
        if (root.TryGetProperty("name", out var n)) plan.Name = n.GetString() ?? fallbackName;

        if (root.TryGetProperty("minecraft", out var mc))
        {
            if (mc.TryGetProperty("version", out var mv)) plan.MinecraftVersion = mv.GetString() ?? "";
            if (mc.TryGetProperty("modLoaders", out var loaders) && loaders.GetArrayLength() > 0)
            {
                var id = loaders[0].TryGetProperty("id", out var lid) ? lid.GetString() ?? "" : "";
                (plan.Loader, plan.LoaderVersion) = ParseLoaderId(id);
            }
        }

        // CF manifests carry projectID/fileID pairs; URLs are resolved later.
        if (root.TryGetProperty("files", out var files))
        {
            foreach (var f in files.EnumerateArray())
            {
                var pid = f.TryGetProperty("projectID", out var pp) ? pp.GetInt32() : 0;
                var fid = f.TryGetProperty("fileID", out var ff) ? ff.GetInt32() : 0;
                if (pid > 0 && fid > 0)
                    plan.Files.Add(new ModpackFile { RelativePath = $"cf:{pid}:{fid}" });
            }
        }
        return plan;
    }

    private static ModpackPlan ParseModrinthIndex(string mrpackPath, string fallbackName)
    {
        using var archive = ZipFile.OpenRead(mrpackPath);
        var entry = archive.GetEntry("modrinth.index.json")
            ?? throw new InvalidOperationException("Not a Modrinth pack (no modrinth.index.json).");
        using var reader = new StreamReader(entry.Open());
        using var doc = JsonDocument.Parse(reader.ReadToEnd());
        var root = doc.RootElement;

        var plan = new ModpackPlan { Name = fallbackName };
        if (root.TryGetProperty("name", out var n)) plan.Name = n.GetString() ?? fallbackName;

        if (root.TryGetProperty("dependencies", out var deps))
        {
            if (deps.TryGetProperty("minecraft", out var mc)) plan.MinecraftVersion = mc.GetString() ?? "";
            foreach (var (key, loader) in new[] { ("fabric-loader", "fabric"), ("quilt-loader", "quilt"), ("forge", "forge"), ("neoforge", "neoforge") })
            {
                if (deps.TryGetProperty(key, out var lv))
                {
                    plan.Loader = loader;
                    plan.LoaderVersion = lv.GetString() ?? "";
                    break;
                }
            }
        }

        if (root.TryGetProperty("files", out var files))
        {
            foreach (var f in files.EnumerateArray())
            {
                var path = f.TryGetProperty("path", out var pp) ? pp.GetString() ?? "" : "";
                if (string.IsNullOrEmpty(path)) continue;

                // Skip files flagged for the server environment / unsupported on client.
                if (f.TryGetProperty("env", out var env) && env.TryGetProperty("client", out var client)
                    && string.Equals(client.GetString(), "unsupported", StringComparison.OrdinalIgnoreCase))
                    continue;

                var url = "";
                if (f.TryGetProperty("downloads", out var dls) && dls.GetArrayLength() > 0)
                    url = dls[0].GetString() ?? "";

                var sha512 = "";
                if (f.TryGetProperty("hashes", out var hashes) && hashes.TryGetProperty("sha512", out var s5))
                    sha512 = s5.GetString() ?? "";

                long size = f.TryGetProperty("fileSize", out var fs) ? fs.GetInt64() : -1;

                if (!string.IsNullOrEmpty(url))
                    plan.Files.Add(new ModpackFile { Url = url, RelativePath = path, Sha512 = sha512, Size = size });
            }
        }
        return plan;
    }

    // ── Download orchestration ─────────────────────────────────

    private async Task ResolveAndDownloadCurseForgeMods(ModpackPlan plan, string instanceId, Progress p, Action<Progress>? report, CancellationToken cancel)
    {
        var total = plan.Files.Count;
        if (total == 0) return;
        int done = 0;
        var modsDir = _instances.PathForInstance(instanceId, "mods");
        Directory.CreateDirectory(modsDir);

        foreach (var f in plan.Files)
        {
            cancel.ThrowIfCancellationRequested();
            var parts = f.RelativePath.Split(':'); // cf:projectId:fileId
            if (parts.Length == 3 && int.TryParse(parts[1], out var pid) && int.TryParse(parts[2], out var fid))
            {
                try
                {
                    var (file, projectName) = await _cf.ResolveManifestFileAsync(pid, fid);
                    if (file == null)
                    {
                        p.ManualDownloads.Add($"{projectName} (project {pid}, file {fid})");
                    }
                    else
                    {
                        var dest = Path.Combine(modsDir, SanitizeName(file.FileName));
                        await _cf.DownloadFileToAsync(file, dest, cancel: cancel);
                    }
                }
                catch (OperationCanceledException) { throw; }
                catch (Exception ex) { p.ManualDownloads.Add($"project {pid}, file {fid} — {ex.Message}"); }
            }
            done++;
            p.Stage = $"Downloading mods ({done}/{total})…";
            p.Percent = 22 + (double)done / total * 55;
            report?.Invoke(p);
        }
    }

    private async Task DownloadPlanFiles(ModpackPlan plan, string instanceId, Progress p, Action<Progress>? report, CancellationToken cancel)
    {
        var total = plan.Files.Count;
        if (total == 0) return;
        int done = 0;
        var instanceRoot = _instances.PathForInstance(instanceId, "");

        foreach (var f in plan.Files)
        {
            cancel.ThrowIfCancellationRequested();
            try
            {
                var dest = Path.Combine(instanceRoot, f.RelativePath.Replace('/', Path.DirectorySeparatorChar));
                Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
                await _download.DownloadAsync(f.Url, dest,
                    knownTotalBytes: f.Size,
                    configureRequest: req => req.Headers.TryAddWithoutValidation("User-Agent", "GlacierLauncher/1.0 (glacier-launcher)"),
                    cancel: cancel);
            }
            catch (OperationCanceledException) { throw; }
            catch (Exception ex) { p.ManualDownloads.Add($"{f.RelativePath} — {ex.Message}"); }

            done++;
            p.Stage = $"Downloading files ({done}/{total})…";
            p.Percent = 22 + (double)done / total * 55;
            report?.Invoke(p);
        }
    }

    // ── Loader install ─────────────────────────────────────────

    private async Task InstallLoaderAsync(ModpackPlan plan, JavaInstance instance, Progress p, Action<Progress>? report, CancellationToken cancel)
    {
        if (string.IsNullOrEmpty(plan.MinecraftVersion) || string.IsNullOrEmpty(plan.Loader))
            return;

        p.Stage = $"Installing {plan.Loader}…"; p.Percent = 82; report?.Invoke(p);
        switch (plan.Loader)
        {
            case "fabric":
                await _loaders.InstallFabricAsync(plan.MinecraftVersion, plan.LoaderVersion);
                break;
            case "quilt":
                await _loaders.InstallQuiltAsync(plan.MinecraftVersion, plan.LoaderVersion);
                break;
            case "forge":
                await RunLoaderInstallerAsync(await _loaders.DownloadForgeInstallerAsync(plan.MinecraftVersion, plan.LoaderVersion), instance, p, report, cancel);
                break;
            case "neoforge":
                await RunLoaderInstallerAsync(await _loaders.DownloadNeoForgeInstallerAsync(plan.MinecraftVersion), instance, p, report, cancel);
                break;
        }
    }

    // Forge/NeoForge ship a jar installer; run it headless against the instance's
    // .minecraft. Needs a JRE — use a bundled one, falling back to a download.
    private async Task RunLoaderInstallerAsync(string installerJar, JavaInstance instance, Progress p, Action<Progress>? report, CancellationToken cancel)
    {
        p.Stage = "Running loader installer…"; p.Percent = 90; report?.Invoke(p);
        var javaw = JavaRuntimeDownloadService.GetCachedJavaw(17)
                    ?? await _runtime.DownloadAsync(17, null, cancel);
        var java = javaw.Replace("javaw.exe", "java.exe", StringComparison.OrdinalIgnoreCase);
        if (!File.Exists(java)) java = javaw;

        var psi = new ProcessStartInfo
        {
            FileName = java,
            CreateNoWindow = true,
            UseShellExecute = false,
            WorkingDirectory = instance.Directory,
        };
        psi.ArgumentList.Add("-jar");
        psi.ArgumentList.Add(installerJar);
        psi.ArgumentList.Add("--installClient");
        psi.ArgumentList.Add(instance.Directory);

        using var proc = Process.Start(psi) ?? throw new InvalidOperationException("Could not start the loader installer.");
        await proc.WaitForExitAsync(cancel);
    }

    // ── Helpers ────────────────────────────────────────────────

    private static (string loader, string version) ParseLoaderId(string id)
    {
        // e.g. "forge-43.2.0", "fabric-0.15.0", "neoforge-21.1.5", "quilt-0.20.0"
        if (string.IsNullOrEmpty(id)) return ("", "");
        var idx = id.IndexOf('-');
        if (idx < 0) return (id.ToLowerInvariant(), "");
        var loader = id[..idx].ToLowerInvariant();
        var version = id[(idx + 1)..];
        return (loader, version);
    }

    private static string LoaderProfileId(ModpackPlan plan) => plan.Loader switch
    {
        "fabric"   => $"fabric-loader-{plan.LoaderVersion}-{plan.MinecraftVersion}",
        "quilt"    => $"quilt-loader-{plan.LoaderVersion}-{plan.MinecraftVersion}",
        // Forge/NeoForge profile ids are written by their installers; the version
        // panel re-scans installed profiles, so fall back to the MC version here.
        _          => plan.MinecraftVersion,
    };

    private void SafeDelete(string instanceId)
    {
        try { _instances.Delete(instanceId); } catch { }
    }

    private static string NewWorkDir()
    {
        var dir = Path.Combine(Path.GetTempPath(), "glacier_modpack_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        return dir;
    }

    private static void TryDeleteDir(string dir)
    {
        try { if (Directory.Exists(dir)) Directory.Delete(dir, true); } catch { }
    }

    private static string SanitizeName(string name)
    {
        foreach (var c in Path.GetInvalidFileNameChars()) name = name.Replace(c, '_');
        return name;
    }

    private sealed class SimpleProgress : IProgress<double>
    {
        private readonly Action<double> _cb;
        public SimpleProgress(Action<double> cb) => _cb = cb;
        public void Report(double value) => _cb(value);
    }
}
