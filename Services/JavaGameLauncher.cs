using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;
using Microsoft.Win32;
using System.Security.Cryptography;

namespace GlacierLauncher.Services;

public sealed class JavaGameLauncher
{
    private readonly SettingsService              _settings;
    private readonly JavaVersionService           _versions;
    private readonly GameConsoleService            _console;
    private readonly JavaRuntimeDownloadService    _javaDownload;
    private readonly JavaInstallService            _installer;
    private readonly XboxProfileService            _xbox;
    private readonly JavaInstanceService            _instances;

    public JavaGameLauncher(SettingsService settings, JavaVersionService versions, GameConsoleService console, JavaInstallService installer, XboxProfileService xbox, JavaInstanceService instances)
    {
        _settings      = settings;
        _versions      = versions;
        _console       = console;
        _javaDownload  = new JavaRuntimeDownloadService();
        _xbox          = xbox;
        _installer     = installer;
        _instances     = instances;
    }

    private static readonly List<Process> _tracked = new();
    private static readonly object        _trackedLock = new();

    public static bool IsRunning()
    {
        lock (_trackedLock)
        {
            for (int i = _tracked.Count - 1; i >= 0; i--)
            {
                try { if (_tracked[i].HasExited) _tracked.RemoveAt(i); }
                catch { _tracked.RemoveAt(i); }
            }
            return _tracked.Count > 0;
        }
    }

    private static void TrackProcess(Process p)
    {
        lock (_trackedLock) _tracked.Add(p);
    }

    public delegate void LaunchProgress(string stage, double percent);

    public async Task LaunchAsync(string versionId, LaunchProgress? onProgress = null)
    {
        if (string.IsNullOrWhiteSpace(versionId))
            throw new InvalidOperationException("No Java version selected.");

        await LaunchAsyncCore(versionId, onProgress).ConfigureAwait(false);
    }

    private async Task LaunchAsyncCore(string versionId, LaunchProgress? onProgress)
    {
        var mcDir      = _versions.MinecraftDir;
        var versionDir = Path.Combine(mcDir, "versions", versionId);
        var jsonPath   = Path.Combine(versionDir, versionId + ".json");
        var jarPath    = Path.Combine(versionDir, versionId + ".jar");

        if (!File.Exists(jsonPath))
        {
            throw new InvalidOperationException(
                $"Version {versionId} isn't installed in {mcDir}\\versions. " +
                "Click Install on the row in Versions to download it.");
        }

        var instanceLock = AcquireInstanceLock(versionId);
        var profile = ReadVersionProfile(versionId, mcDir);

        if (_settings.Settings.JavaBackupSavesBeforeLaunch)
            await _instances.BackupSavesAsync().ConfigureAwait(false);

        var javaw = ResolveJavaRuntime(profile);
        int requiredMajor = int.TryParse(profile.MinJavaMajor, out var rm) ? rm : 8;

        int actualJavaMajor = !string.IsNullOrEmpty(javaw) ? DetectJavaMajor(javaw) : 0;
        bool needsJavaDownload = string.IsNullOrEmpty(javaw)
                           || (actualJavaMajor > 0 && actualJavaMajor < requiredMajor);

        if (needsJavaDownload && requiredMajor >= 8)
        {
            onProgress?.Invoke($"Downloading Java {requiredMajor}…", 0);
            try
            {
                javaw = await _javaDownload.DownloadAsync(
                    requiredMajor,
                    onProgress: (stage, pct) => onProgress?.Invoke(stage, pct)).ConfigureAwait(false);
                actualJavaMajor = DetectJavaMajor(javaw);
            }
            catch (Exception)
            {
                if (string.IsNullOrEmpty(javaw))
                    throw new InvalidOperationException(
                        $"Could not find or download Java {requiredMajor}. " +
                        "Install it manually and set Settings → Java Runtime, or check your internet connection.");
            }
        }

        if (string.IsNullOrEmpty(javaw))
            throw new InvalidOperationException(
                "Could not find javaw.exe. Set Settings → Java Runtime, or install Minecraft (which bundles JREs under .minecraft/runtime).");

        var classpath = BuildClasspath(profile, mcDir);
        if (classpath.Count == 0)
            throw new InvalidOperationException(
                "Couldn't resolve any libraries on disk. Run this version once in the official launcher so libraries download, then retry.");

        var missing = classpath.Where(p => !File.Exists(p)).ToList();
        if (missing.Count > 0)
        {
            onProgress?.Invoke($"Downloading {missing.Count} missing libraries…", 0);
            try
            {
                var allVersions = await _versions.GetVersionsAsync().ConfigureAwait(false);
                var jv = allVersions.FirstOrDefault(v => v.Id == versionId);
                if (jv != null && !string.IsNullOrEmpty(jv.Url))
                {
                    await _installer.InstallAsync(jv,
                        report: p => onProgress?.Invoke(p.Stage, p.Percent)).ConfigureAwait(false);
                }
                else
                {
                    throw new InvalidOperationException(
                        $"Version {versionId} not found in manifest — can't auto-download missing libraries. " +
                        "Try installing this version from the Versions tab first.");
                }
            }
            catch (InvalidOperationException) { throw; }
            catch (Exception ex)
            {
                throw new InvalidOperationException(
                    $"Auto-install failed: {ex.Message}. Try installing this version from the Versions tab first.");
            }
        }

        onProgress?.Invoke("Launching…", 100);

        var console = _console.Open($"Minecraft Java · {versionId}");
        console?.Info($".minecraft = {mcDir}");
        console?.Info($"Main class: {profile.MainClass}");
        console?.Info($"Asset index: {profile.AssetsIndex}  ·  Java major: {(string.IsNullOrEmpty(profile.MinJavaMajor) ? "(legacy)" : profile.MinJavaMajor)}");
        console?.Info($"Java runtime: {javaw}  (detected major: {actualJavaMajor})");
        console?.Info($"Classpath entries: {classpath.Count}");

        var auth = await ResolveAuthValuesAsync().ConfigureAwait(false);
        console?.Info($"Auth: {auth.Name} ({auth.UserType}) · {auth.Uuid}");

        var sb = new StringBuilder();
        AppendJvmArgs(sb, profile, mcDir, versionId, actualJavaMajor);
        sb.Append(" -cp ").Append(Quote(string.Join(';', classpath)));
        sb.Append(' ').Append(profile.MainClass);
        AppendGameArgs(sb, profile, mcDir, versionId, auth);

        console?.Info("Game args (sanitised):");
        console?.Info(SanitiseForLog(sb.ToString(), auth));

        var psi = new ProcessStartInfo
        {
            FileName        = javaw,
            Arguments       = sb.ToString(),
            WorkingDirectory = mcDir,
            UseShellExecute        = false,
            CreateNoWindow         = true,
            RedirectStandardOutput = console != null,
            RedirectStandardError  = console != null,
        };

        Process? proc = null;
        await Task.Run(() => proc = Process.Start(psi)).ConfigureAwait(false);

        if (proc != null)
        {
            TrackProcess(proc);
            proc.EnableRaisingEvents = true;
            proc.Exited += (_, _) =>
            {
                try { instanceLock.Dispose(); } catch { }
            };
            console?.Info($"Started PID {proc.Id}");
            console?.SetPid(proc.Id);
            console?.MarkRunning();
            console?.Attach(proc);
        }
        else
        {
            instanceLock.Dispose();
        }

        _settings.Settings.JavaLastUsedVersion = versionId;
        _instances.SetActiveVersion(versionId);
        _settings.Save();
    }

    private FileStream AcquireInstanceLock(string versionId)
    {
        var lockPath = _instances.CreateInstanceLockPath(versionId);
        Directory.CreateDirectory(Path.GetDirectoryName(lockPath)!);
        try
        {
            return new FileStream(lockPath, FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None);
        }
        catch (IOException)
        {
            throw new InvalidOperationException("This Java instance is already launching or running.");
        }
    }

    private sealed record VersionProfile(
        string Id,
        string MainClass,
        string AssetsIndex,
        string AssetsDir,
        List<LibraryEntry> Libraries,
        List<string> JvmArgs,
        List<string> GameArgs,
        string MinJavaMajor,
        string JarVersionId);

    private sealed record LibraryEntry(string Path, bool IsNative);

    private VersionProfile ReadVersionProfile(string id, string mcDir)
    {
        var libraries = new List<LibraryEntry>();
        var jvmArgs   = new List<string>();
        var gameArgs  = new List<string>();

        string?  mainClass  = null;
        string?  assetsIdx  = null;
        string?  jarVerId   = null;
        string   minJava    = "";

        var visited = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var stack   = new Stack<string>();
        stack.Push(id);

        while (stack.Count > 0)
        {
            var curId = stack.Pop();
            if (!visited.Add(curId)) continue;

            var jsonPath = Path.Combine(mcDir, "versions", curId, curId + ".json");
            if (!File.Exists(jsonPath))
                throw new InvalidOperationException($"Missing version JSON for {curId}.");

            using var doc = JsonDocument.Parse(File.ReadAllText(jsonPath));
            var root = doc.RootElement;

            mainClass ??= root.TryGetProperty("mainClass", out var mc) ? mc.GetString() : null;
            assetsIdx ??= root.TryGetProperty("assets",    out var a)  ? a.GetString()  : null;
            jarVerId  ??= root.TryGetProperty("jar",       out var jv) ? jv.GetString() : curId;

            if (root.TryGetProperty("javaVersion", out var jav)
                && jav.TryGetProperty("majorVersion", out var mj)
                && string.IsNullOrEmpty(minJava))
            {
                minJava = mj.ToString();
            }

            if (root.TryGetProperty("libraries", out var libs) && libs.ValueKind == JsonValueKind.Array)
            {
                foreach (var lib in libs.EnumerateArray())
                {
                    if (!RulesAllow(lib)) continue;

                    if (lib.TryGetProperty("downloads", out var dl))
                    {
                        if (dl.TryGetProperty("artifact", out var art)
                            && art.TryGetProperty("path", out var path))
                        {
                            var p = Path.Combine(mcDir, "libraries", path.GetString()!.Replace('/', Path.DirectorySeparatorChar));
                            libraries.Add(new LibraryEntry(p, false));
                        }

                        if (dl.TryGetProperty("classifiers", out var classifiers)
                            && classifiers.ValueKind == JsonValueKind.Object
                            && lib.TryGetProperty("natives", out var natives)
                            && natives.TryGetProperty("windows", out var winKey))
                        {
                            var key = winKey.GetString() ?? "";
                            key = key.Replace("${arch}", "64");
                            if (classifiers.TryGetProperty(key, out var native)
                                && native.TryGetProperty("path", out var npath))
                            {
                                var p = Path.Combine(mcDir, "libraries", npath.GetString()!.Replace('/', Path.DirectorySeparatorChar));
                                libraries.Add(new LibraryEntry(p, true));
                            }
                        }
                    }
                    else if (lib.TryGetProperty("name", out var name))
                    {
                        var coord = name.GetString() ?? "";
                        var p = MavenCoordToPath(mcDir, coord);
                        if (p != null) libraries.Add(new LibraryEntry(p, false));
                    }
                }
            }

            if (root.TryGetProperty("arguments", out var args))
            {
                if (args.TryGetProperty("jvm",  out var j)) CollectArgs(j, jvmArgs);
                if (args.TryGetProperty("game", out var g)) CollectArgs(g, gameArgs);
            }
            else if (root.TryGetProperty("minecraftArguments", out var legacy))
            {
                gameArgs.AddRange((legacy.GetString() ?? "").Split(' ', StringSplitOptions.RemoveEmptyEntries));
            }

            if (root.TryGetProperty("inheritsFrom", out var parent))
            {
                var p = parent.GetString();
                if (!string.IsNullOrEmpty(p)) stack.Push(p);
            }
        }

        var jarId = jarVerId ?? id;
        var primary = Path.Combine(mcDir, "versions", jarId, jarId + ".jar");
        if (File.Exists(primary)) libraries.Add(new LibraryEntry(primary, false));

        if (string.IsNullOrEmpty(mainClass))
            throw new InvalidOperationException($"Version JSON for {id} is missing mainClass.");

        return new VersionProfile(
            id, mainClass!, assetsIdx ?? "legacy",
            Path.Combine(mcDir, "assets"),
            libraries, jvmArgs, gameArgs, minJava, jarId);
    }

    private static void CollectArgs(JsonElement arr, List<string> sink)
    {
        if (arr.ValueKind != JsonValueKind.Array) return;
        foreach (var el in arr.EnumerateArray())
        {
            if (el.ValueKind == JsonValueKind.String)
            {
                sink.Add(el.GetString() ?? "");
            }
            else if (el.ValueKind == JsonValueKind.Object && RulesAllow(el))
            {
                if (el.TryGetProperty("value", out var v))
                {
                    if (v.ValueKind == JsonValueKind.String)
                        sink.Add(v.GetString() ?? "");
                    else if (v.ValueKind == JsonValueKind.Array)
                        foreach (var s in v.EnumerateArray())
                            sink.Add(s.GetString() ?? "");
                }
            }
        }
    }

    private static bool RulesAllow(JsonElement el)
    {
        if (!el.TryGetProperty("rules", out var rules) || rules.ValueKind != JsonValueKind.Array)
            return true;

        bool allowed = false;
        foreach (var rule in rules.EnumerateArray())
        {
            var action = rule.TryGetProperty("action", out var a) ? a.GetString() : "allow";
            if (!OsMatches(rule)) continue;
            if (!FeaturesMatch(rule)) continue;
            allowed = action == "allow";
        }
        return allowed;
    }

    private static bool OsMatches(JsonElement rule)
    {
        if (!rule.TryGetProperty("os", out var os)) return true;
        if (os.TryGetProperty("name", out var n))
        {
            var name = n.GetString();
            if (!string.Equals(name, "windows", StringComparison.OrdinalIgnoreCase))
                return false;
        }
        return true;
    }

    private static bool FeaturesMatch(JsonElement rule)
    {
        if (!rule.TryGetProperty("features", out var features)) return true;
        if (features.ValueKind != JsonValueKind.Object) return true;

        foreach (var _ in features.EnumerateObject())
            return false;
        return true;
    }

    private static string? MavenCoordToPath(string mcDir, string coord)
    {
        var ext   = "jar";
        var atIdx = coord.IndexOf('@');
        if (atIdx >= 0) { ext = coord[(atIdx + 1)..]; coord = coord[..atIdx]; }

        var parts = coord.Split(':');
        if (parts.Length < 3) return null;

        var group     = parts[0].Replace('.', '/');
        var artifact  = parts[1];
        var version   = parts[2];
        var classifier= parts.Length > 3 ? "-" + parts[3] : "";

        return Path.Combine(mcDir, "libraries",
            group.Replace('/', Path.DirectorySeparatorChar),
            artifact, version,
            $"{artifact}-{version}{classifier}.{ext}");
    }

    private List<string> BuildClasspath(VersionProfile profile, string mcDir)
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var cp   = new List<string>(profile.Libraries.Count);
        foreach (var lib in profile.Libraries)
        {
            if (lib.IsNative) continue;
            if (seen.Add(lib.Path)) cp.Add(lib.Path);
        }
        return cp;
    }

    private void AppendJvmArgs(StringBuilder sb, VersionProfile profile, string mcDir, string versionId, int actualJavaMajor)
    {
        var nativesDir = Path.Combine(mcDir, "versions", versionId, "natives");
        Directory.CreateDirectory(nativesDir);

        var subs = new Dictionary<string, string>
        {
            ["natives_directory"] = nativesDir,
            ["launcher_name"]     = "glacier",
            ["launcher_version"]  = "1.0",
            ["classpath"]         = "<set-by-cp-flag>",
        };

        if (profile.JvmArgs.Count > 0)
        {
            foreach (var raw in profile.JvmArgs)
            {
                var v = Substitute(raw, subs);
                if (v.Contains("${classpath}", StringComparison.Ordinal)) continue;
                if (!IsJvmArgCompatible(v, actualJavaMajor)) continue;
                sb.Append(' ').Append(Quote(v));
            }
        }
        else
        {
            sb.Append(" -Djava.library.path=").Append(Quote(nativesDir));
        }

        var ram = Math.Clamp(_settings.Settings.JavaMaxRamMb, 512, 16384);
        var minRam = Math.Clamp(_settings.Settings.JavaMinRamMb, 256, ram);
        sb.Append(" -Xmx").Append(ram).Append('M');
        sb.Append(" -Xms").Append(minRam).Append('M');

        var custom = _settings.Settings.JavaCustomJvmArgs;
        if (!string.IsNullOrWhiteSpace(custom))
            sb.Append(' ').Append(custom);
    }

    private void AppendGameArgs(
        StringBuilder sb, VersionProfile profile, string mcDir, string versionId, AuthValues auth)
    {
        var subs = new Dictionary<string, string>
        {
            ["auth_player_name"]    = auth.Name,
            ["version_name"]        = versionId,
            ["game_directory"]      = mcDir,
            ["assets_root"]         = profile.AssetsDir,
            ["assets_index_name"]   = profile.AssetsIndex,
            ["auth_uuid"]           = auth.Uuid,
            ["auth_access_token"]   = auth.AccessToken,
            ["clientid"]            = "0",
            ["auth_xuid"]           = _settings.Settings.XboxXuid,
            ["user_type"]           = auth.UserType,
            ["version_type"]        = profile.JarVersionId == versionId ? "release" : "release",
            ["user_properties"]     = "{}",
            ["auth_session"]        = "token:" + auth.AccessToken + ":" + auth.Uuid,
            ["game_assets"]         = profile.AssetsDir,
        };

        foreach (var raw in profile.GameArgs)
        {
            var v = Substitute(raw, subs);
            if (string.IsNullOrEmpty(v)) continue;
            if (v.StartsWith("${", StringComparison.Ordinal) && v.EndsWith("}", StringComparison.Ordinal))
                continue;
            sb.Append(' ').Append(Quote(v));
        }

        if (_settings.Settings.JavaFullscreen)
        {
            sb.Append(" --fullscreen");
        }
        else if (_settings.Settings.JavaUseCustomResolution)
        {
            sb.Append(" --width ").Append(Math.Clamp(_settings.Settings.JavaWindowWidth, 320, 7680));
            sb.Append(" --height ").Append(Math.Clamp(_settings.Settings.JavaWindowHeight, 240, 4320));
        }

        if (!string.IsNullOrWhiteSpace(_settings.Settings.JavaServerAddress))
        {
            sb.Append(" --server ").Append(Quote(_settings.Settings.JavaServerAddress.Trim()));
            sb.Append(" --port ").Append(Math.Clamp(_settings.Settings.JavaServerPort, 1, 65535));
        }
    }

    private static string Substitute(string template, Dictionary<string, string> subs)
    {
        if (string.IsNullOrEmpty(template) || template.IndexOf("${", StringComparison.Ordinal) < 0)
            return template;

        var sb = new StringBuilder(template.Length);
        int i = 0;
        while (i < template.Length)
        {
            if (template[i] == '$' && i + 1 < template.Length && template[i + 1] == '{')
            {
                var end = template.IndexOf('}', i + 2);
                if (end > 0)
                {
                    var key = template.Substring(i + 2, end - i - 2);
                    sb.Append(subs.TryGetValue(key, out var val) ? val : "");
                    i = end + 1;
                    continue;
                }
            }
            sb.Append(template[i++]);
        }
        return sb.ToString();
    }

    private static string Quote(string v) =>
        v.Length == 0 || v.IndexOfAny(new[] { ' ', '\t', '"' }) >= 0
            ? "\"" + v.Replace("\"", "\\\"") + "\""
            : v;

    private static bool IsJvmArgCompatible(string arg, int javaMajor)
    {
        if (javaMajor <= 0) return true;

        if (javaMajor < 23 && arg.StartsWith("--sun-misc-unsafe-memory-access", StringComparison.Ordinal))
            return false;
        return true;
    }

    private static int DetectJavaMajor(string javawPath)
    {
        try
        {
            var dir = Path.GetDirectoryName(javawPath);
            for (int depth = 0; depth < 6 && dir != null; depth++)
            {
                dir = Path.GetDirectoryName(dir);
                if (dir == null) break;
                var release = Path.Combine(dir, "release");
                if (File.Exists(release))
                {
                    foreach (var line in File.ReadLines(release))
                    {
                        if (!line.StartsWith("JAVA_VERSION", StringComparison.OrdinalIgnoreCase))
                            continue;
                        var eq = line.IndexOf('=');
                        if (eq < 0) continue;
                        var ver = line[(eq + 1)..].Trim().Trim('"');
                        if (ver.StartsWith("1.", StringComparison.Ordinal) && ver.Length >= 3)
                        {
                            if (int.TryParse(ver.Split('.')[1], out var legacy)) return legacy;
                        }
                        else
                        {
                            if (int.TryParse(ver.Split('.')[0], out var modern)) return modern;
                        }
                    }
                }
            }
        }
        catch { }

        var m2 = System.Text.RegularExpressions.Regex.Match(
            javawPath,
            @"(?:jdk|jre|java|openjdk)-?(\d+)",
            System.Text.RegularExpressions.RegexOptions.IgnoreCase);
        if (m2.Success && int.TryParse(m2.Groups[1].Value, out var heuristic))
        {
            if (heuristic == 1)
            {
                var m3 = System.Text.RegularExpressions.Regex.Match(
                    javawPath, @"(?:jdk|jre|java)-?1\.(\d+)",
                    System.Text.RegularExpressions.RegexOptions.IgnoreCase);
                if (m3.Success && int.TryParse(m3.Groups[1].Value, out var legacy))
                    return legacy;
            }
            return heuristic;
        }

        return 0;
    }

    private static string SanitiseForLog(string args, AuthValues auth)
    {
        if (string.IsNullOrEmpty(auth.AccessToken) || auth.AccessToken == "0")
            return args;
        return args.Replace(auth.AccessToken, "<access-token>");
    }

    private sealed record AuthValues(string Name, string Uuid, string AccessToken, string UserType);

    private async Task<AuthValues> ResolveAuthValuesAsync()
    {
        var s = _settings.Settings;

        if (!string.IsNullOrEmpty(s.JavaAccessToken)
            && !string.IsNullOrEmpty(s.JavaUuid)
            && !string.IsNullOrEmpty(s.JavaUsername))
        {
            bool expired = false;
            if (DateTime.TryParse(s.JavaAccessTokenExpiry, null,
                    System.Globalization.DateTimeStyles.RoundtripKind, out var expiry))
            {
                expired = DateTime.UtcNow >= expiry;
            }

            if (!expired)
                return new AuthValues(s.JavaUsername, s.JavaUuid, s.JavaAccessToken, "msa");

            try
            {
                await _xbox.RefreshProfileAsync().ConfigureAwait(false);
                if (!string.IsNullOrEmpty(s.JavaAccessToken))
                    return new AuthValues(s.JavaUsername, s.JavaUuid, s.JavaAccessToken, "msa");
            }
            catch { }
        }

        if (!string.IsNullOrEmpty(s.XboxLiveRefreshToken) && _xbox.IsSignedIn)
        {
            try
            {
                await _xbox.RefreshProfileAsync().ConfigureAwait(false);
                if (!string.IsNullOrEmpty(s.JavaAccessToken)
                    && !string.IsNullOrEmpty(s.JavaUuid)
                    && !string.IsNullOrEmpty(s.JavaUsername))
                {
                    return new AuthValues(s.JavaUsername, s.JavaUuid, s.JavaAccessToken, "msa");
                }
            }
            catch { }
        }

        var name = !string.IsNullOrWhiteSpace(s.Username) ? s.Username : "Player";
        var uuid = OfflineUuid(name);
        return new AuthValues(name, uuid, "0", "legacy");
    }

    private static string OfflineUuid(string name)
    {
        int nameByteCount = Encoding.UTF8.GetByteCount(name);
        int totalByteCount = 14 + nameByteCount;
        Span<byte> utf8Bytes = totalByteCount <= 512 ? stackalloc byte[totalByteCount] : new byte[totalByteCount];
        
        "OfflinePlayer:"u8.CopyTo(utf8Bytes);
        
        Encoding.UTF8.GetBytes(name, utf8Bytes[14..]);

        Span<byte> hash = stackalloc byte[16];
        MD5.HashData(utf8Bytes, hash);

        hash[6] = (byte)((hash[6] & 0x0f) | 0x30);
        hash[8] = (byte)((hash[8] & 0x3f) | 0x80);
        return new Guid(hash).ToString("N");
    }

    private string? ResolveJavaRuntime(VersionProfile profile)
    {
        var explicitPath = _settings.Settings.JavaRuntimePath;
        if (!string.IsNullOrEmpty(explicitPath) && File.Exists(explicitPath))
            return explicitPath;

        if (int.TryParse(profile.MinJavaMajor, out var reqMajor) && reqMajor > 0)
        {
            var glacierJavaw = JavaRuntimeDownloadService.GetCachedJavaw(reqMajor);
            if (glacierJavaw != null) return glacierJavaw;
        }

        var runtimeRoot = Path.Combine(_versions.MinecraftDir, "runtime");
        if (Directory.Exists(runtimeRoot))
        {
            foreach (var component in Directory.EnumerateDirectories(runtimeRoot))
            {
                if (!string.IsNullOrEmpty(profile.MinJavaMajor)
                    && !ComponentMatchesMajor(component, profile.MinJavaMajor))
                    continue;
                foreach (var javaw in EnumerateJavawUnder(component))
                    return javaw;
            }
            foreach (var component in Directory.EnumerateDirectories(runtimeRoot))
                foreach (var javaw in EnumerateJavawUnder(component))
                    return javaw;
        }

        var javaHome = Environment.GetEnvironmentVariable("JAVA_HOME");
        if (!string.IsNullOrEmpty(javaHome))
        {
            var p = Path.Combine(javaHome, "bin", "javaw.exe");
            if (File.Exists(p)) return p;
        }
        var pathEnv = Environment.GetEnvironmentVariable("PATH") ?? "";
        foreach (var seg in pathEnv.Split(';', StringSplitOptions.RemoveEmptyEntries))
        {
            try
            {
                var p = Path.Combine(seg.Trim(), "javaw.exe");
                if (File.Exists(p)) return p;
            }
            catch { }
        }

        try
        {
            foreach (var rootKey in new[] { Registry.LocalMachine, Registry.CurrentUser })
            {
                foreach (var sub in new[]
                {
                    @"SOFTWARE\Eclipse Adoptium\JDK",
                    @"SOFTWARE\Eclipse Adoptium\JRE",
                    @"SOFTWARE\Microsoft\JDK",
                    @"SOFTWARE\JavaSoft\Java Runtime Environment",
                    @"SOFTWARE\JavaSoft\JRE",
                })
                {
                    using var k = rootKey.OpenSubKey(sub);
                    if (k == null) continue;
                    foreach (var ver in k.GetSubKeyNames())
                    {
                        using var v = k.OpenSubKey(ver);
                        var jh = v?.GetValue("JavaHome") as string;
                        if (string.IsNullOrEmpty(jh)) continue;
                        var p = Path.Combine(jh, "bin", "javaw.exe");
                        if (File.Exists(p)) return p;
                    }
                }
            }
        }
        catch { }

        return null;
    }

    private static IEnumerable<string> EnumerateJavawUnder(string root)
    {
        if (!Directory.Exists(root)) yield break;
        IEnumerable<string> hits;
        try
        {
            hits = Directory.EnumerateFiles(root, "javaw.exe", SearchOption.AllDirectories);
        }
        catch { yield break; }
        foreach (var p in hits) yield return p;
    }

    private static bool ComponentMatchesMajor(string componentDir, string major)
    {
        var name = Path.GetFileName(componentDir);
        return major switch
        {
            "8"  => name.Contains("legacy"),
            "16" => name.Contains("alpha"),
            "17" => name.Contains("beta") || name.Contains("gamma"),
            "21" => name.Contains("delta"),
            "25" => name.Contains("epsilon"),
            _    => false
        };
    }
}
