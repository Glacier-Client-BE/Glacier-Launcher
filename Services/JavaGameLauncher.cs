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

namespace GlacierLauncher.Services;

/// <summary>
/// Launches Java Edition Minecraft against an existing <c>%APPDATA%\.minecraft</c>
/// install. Reads the version JSON the official launcher emits and constructs the
/// command line the way the vanilla launcher would.
///
/// This is the MVP path — it does not download missing libraries / assets / a Java
/// runtime. If a version isn't already installed, <see cref="LaunchAsync"/> throws
/// with a clear message and the user is steered toward installing it with the
/// official launcher first. Full asset/library download support lands in a later
/// pass.
/// </summary>
public sealed class JavaGameLauncher
{
    private readonly SettingsService     _settings;
    private readonly JavaVersionService  _versions;
    private readonly GameConsoleService  _console;

    public JavaGameLauncher(SettingsService settings, JavaVersionService versions, GameConsoleService console)
    {
        _settings = settings;
        _versions = versions;
        _console  = console;
    }

    // Processes Glacier itself has launched. Scanning the whole system for
    // javaw.exe false-positives on anything Java-based the user is running
    // (IDEs, build tools, other launchers), so we only count our own children.
    private static readonly List<Process> _tracked = new();
    private static readonly object        _trackedLock = new();

    /// <summary>True if any Minecraft instance Glacier launched is still running.</summary>
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

    public async Task LaunchAsync(string versionId)
    {
        if (string.IsNullOrWhiteSpace(versionId))
            throw new InvalidOperationException("No Java version selected.");

        var console = _console.Open($"Minecraft Java · {versionId}");
        try
        {
            await LaunchAsyncCore(versionId, console);
        }
        catch (Exception ex)
        {
            console?.Error(ex.Message);
            console?.MarkFailed(ex.Message.Length > 60 ? ex.Message[..60] + "…" : ex.Message);
            throw;
        }
    }

    private async Task LaunchAsyncCore(string versionId, GameConsoleHandle? console)
    {
        var mcDir      = _versions.MinecraftDir;
        var versionDir = Path.Combine(mcDir, "versions", versionId);
        var jsonPath   = Path.Combine(versionDir, versionId + ".json");
        var jarPath    = Path.Combine(versionDir, versionId + ".jar");

        console?.Info($".minecraft = {mcDir}");

        if (!File.Exists(jsonPath))
        {
            throw new InvalidOperationException(
                $"Version {versionId} isn't installed in {mcDir}\\versions. " +
                "Click Install on the row in Versions to download it.");
        }
        if (!File.Exists(jarPath))
        {
            // Some versions inherit and don't have their own jar — fall through and let inheritsFrom handle it.
            // If even the parent jar is missing we'll throw when building the classpath.
            console?.Info("Primary jar missing — falling back to inheritsFrom chain.");
        }

        var profile = ReadVersionProfile(versionId, mcDir);
        console?.Info($"Main class: {profile.MainClass}");
        console?.Info($"Asset index: {profile.AssetsIndex}  ·  Java major: {(string.IsNullOrEmpty(profile.MinJavaMajor) ? "(legacy)" : profile.MinJavaMajor)}");

        var javaw = ResolveJavaRuntime(profile);
        if (string.IsNullOrEmpty(javaw))
            throw new InvalidOperationException(
                "Could not find javaw.exe. Set Settings → Java Runtime, or install Minecraft (which bundles JREs under .minecraft/runtime).");
        console?.Info($"Java runtime: {javaw}");

        var classpath = BuildClasspath(profile, mcDir);
        if (classpath.Count == 0)
            throw new InvalidOperationException(
                "Couldn't resolve any libraries on disk. Run this version once in the official launcher so libraries download, then retry.");
        console?.Info($"Classpath entries: {classpath.Count}");

        var auth = ResolveAuthValues();
        console?.Info($"Auth: {auth.Name} ({auth.UserType}) · {auth.Uuid}");

        var sb = new StringBuilder();
        AppendJvmArgs(sb, profile, mcDir, versionId);
        sb.Append(" -cp ").Append(Quote(string.Join(';', classpath)));
        sb.Append(' ').Append(profile.MainClass);
        AppendGameArgs(sb, profile, mcDir, versionId, auth);

        // The full arg vector is the single most useful thing to capture for
        // post-mortem debugging — any feature-flag / classpath bug shows up
        // here without the user having to reproduce the crash.
        console?.Info("Game args (sanitised):");
        console?.Info(SanitiseForLog(sb.ToString(), auth));

        var psi = new ProcessStartInfo
        {
            FileName        = javaw,
            Arguments       = sb.ToString(),
            WorkingDirectory = mcDir,
            UseShellExecute        = false,
            CreateNoWindow         = true,
            // Capture stdout/stderr only when we have a console window to write
            // them to — otherwise the streams fill up the kernel pipe and the
            // game stalls waiting for someone to read.
            RedirectStandardOutput = console != null,
            RedirectStandardError  = console != null,
        };

        Process? proc = null;
        await Task.Run(() => proc = Process.Start(psi));

        if (proc != null)
        {
            TrackProcess(proc);
            console?.Info($"Started PID {proc.Id}");
            console?.SetPid(proc.Id);
            console?.MarkRunning();
            console?.Attach(proc);
        }

        _settings.Settings.JavaLastUsedVersion = versionId;
        _settings.Save();
    }

    // ── Version JSON parsing (handles inheritsFrom) ─────────────────────

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
                            if (File.Exists(p)) libraries.Add(new LibraryEntry(p, false));
                        }

                        // Native classifiers (lwjgl-*-natives-windows etc.). We don't
                        // extract them into a temp folder here — modern (1.19+) lwjgl
                        // packages natives inline, and older versions can be launched
                        // through the official launcher once which will leave the
                        // natives directory populated.
                        if (dl.TryGetProperty("classifiers", out var classifiers)
                            && classifiers.ValueKind == JsonValueKind.Object
                            && lib.TryGetProperty("natives", out var natives)
                            && natives.TryGetProperty("windows", out var winKey))
                        {
                            var key = winKey.GetString() ?? "";
                            // The "${arch}" placeholder is a 32/64 marker — we only target x64.
                            key = key.Replace("${arch}", "64");
                            if (classifiers.TryGetProperty(key, out var native)
                                && native.TryGetProperty("path", out var npath))
                            {
                                var p = Path.Combine(mcDir, "libraries", npath.GetString()!.Replace('/', Path.DirectorySeparatorChar));
                                if (File.Exists(p)) libraries.Add(new LibraryEntry(p, true));
                            }
                        }
                    }
                    else if (lib.TryGetProperty("name", out var name))
                    {
                        // Forge/Fabric libraries without an explicit `downloads` block.
                        // Maven coordinate `group:artifact:version` → group/artifact/version/artifact-version.jar
                        var coord = name.GetString() ?? "";
                        var p = MavenCoordToPath(mcDir, coord);
                        if (p != null && File.Exists(p)) libraries.Add(new LibraryEntry(p, false));
                    }
                }
            }

            // Modern arguments block { game: [...], jvm: [...] }
            if (root.TryGetProperty("arguments", out var args))
            {
                if (args.TryGetProperty("jvm",  out var j)) CollectArgs(j, jvmArgs);
                if (args.TryGetProperty("game", out var g)) CollectArgs(g, gameArgs);
            }
            // Legacy minecraftArguments (pre-1.13 string form)
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

        // Always include the version's primary jar at the END of the classpath. The
        // game expects the launcher-chosen jar (or its inherited parent) last.
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

        // Default action when no rule matches is "disallow" per Mojang's spec.
        bool allowed = false;
        foreach (var rule in rules.EnumerateArray())
        {
            var action = rule.TryGetProperty("action", out var a) ? a.GetString() : "allow";
            // A rule only matches when ALL of its filters match. We support
            // the `os` filter; any `features` filter (quick-play, demo mode,
            // custom resolution) is treated as not-matching because we don't
            // enable any features. Without this check Minecraft 1.20+ crashes
            // at startup with "Only one quick play option can be specified"
            // because all four quick-play arg blocks fall through.
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
        // We could check `arch` and `version` regex too — for x64 Windows the
        // common cases (osx/linux excludes) are covered above.
        return true;
    }

    private static bool FeaturesMatch(JsonElement rule)
    {
        // No `features` block → no feature requirement → trivially matches.
        if (!rule.TryGetProperty("features", out var features)) return true;
        if (features.ValueKind != JsonValueKind.Object) return true;

        // Glacier doesn't expose demo mode, quick-play, or custom-resolution
        // launches, so any rule requiring one of those features cannot match.
        // (If we later add quick-play we'd populate a whitelist of enabled
        // feature names and check against it here.)
        foreach (var _ in features.EnumerateObject())
            return false;
        return true;
    }

    private static string? MavenCoordToPath(string mcDir, string coord)
    {
        // group:artifact:version[:classifier][@ext]
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
        // Deduplicate while preserving order (the version's own jar is intentionally last).
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var cp   = new List<string>(profile.Libraries.Count);
        foreach (var lib in profile.Libraries)
        {
            if (lib.IsNative) continue; // natives go on java.library.path, not classpath
            if (seen.Add(lib.Path)) cp.Add(lib.Path);
        }
        return cp;
    }

    // ── Args ─────────────────────────────────────────────────────────

    private void AppendJvmArgs(StringBuilder sb, VersionProfile profile, string mcDir, string versionId)
    {
        // Build the substitution table used by both jvm and game arg templates.
        var nativesDir = Path.Combine(mcDir, "versions", versionId, "natives");
        Directory.CreateDirectory(nativesDir); // make sure the path exists even if empty

        var subs = new Dictionary<string, string>
        {
            ["natives_directory"] = nativesDir,
            ["launcher_name"]     = "glacier",
            ["launcher_version"]  = "1.0",
            ["classpath"]         = "<set-by-cp-flag>",
        };

        // Strip JVM args that the chosen Java is too old to understand. Mojang ships
        // some of these unconditionally in arguments.jvm even when javaVersion still
        // declares an older major (e.g. 1.21.6+ includes --sun-misc-unsafe-memory-access=allow
        // alongside javaVersion.majorVersion=21, but the flag was only added in Java 23
        // so a Java 21 runtime hard-crashes with "Unrecognized option").
        int javaMajor = int.TryParse(profile.MinJavaMajor, out var m) ? m : 8;

        if (profile.JvmArgs.Count > 0)
        {
            foreach (var raw in profile.JvmArgs)
            {
                var v = Substitute(raw, subs);
                // We supply -cp ourselves outside this list, so skip Mojang's classpath placeholder.
                if (v.Contains("${classpath}", StringComparison.Ordinal)) continue;
                if (!IsJvmArgCompatible(v, javaMajor)) continue;
                sb.Append(' ').Append(Quote(v));
            }
        }
        else
        {
            // Legacy versions don't ship JVM args — supply the bare minimum.
            sb.Append(" -Djava.library.path=").Append(Quote(nativesDir));
        }

        // Heap size: respect the user's setting but cap at 16 GB to avoid typos.
        var ram = Math.Clamp(_settings.Settings.JavaMaxRamMb, 512, 16384);
        sb.Append(" -Xmx").Append(ram).Append('M');
        sb.Append(" -Xms").Append(Math.Min(512, ram)).Append('M');

        // User-supplied JVM args last so they can override our defaults.
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
            ["user_properties"]     = "{}", // legacy field; modern versions ignore it
            ["auth_session"]        = "token:" + auth.AccessToken + ":" + auth.Uuid, // legacy
            ["game_assets"]         = profile.AssetsDir,                              // legacy
        };

        foreach (var raw in profile.GameArgs)
        {
            var v = Substitute(raw, subs);
            // Skip args that resolved to empty — a feature-gated arg that slipped
            // past RulesAllow would otherwise become `""` on the command line,
            // which some Minecraft versions choke on.
            if (string.IsNullOrEmpty(v)) continue;
            // Drop unsubstituted placeholders like ${quickPlayPath} — these mean
            // the version JSON wanted a value Glacier doesn't supply.
            if (v.StartsWith("${", StringComparison.Ordinal) && v.EndsWith("}", StringComparison.Ordinal))
                continue;
            sb.Append(' ').Append(Quote(v));
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
        // --sun-misc-unsafe-memory-access was introduced in JEP 498 (Java 23).
        // Older JVMs reject it as "Unrecognized option" and refuse to start.
        if (javaMajor < 23 && arg.StartsWith("--sun-misc-unsafe-memory-access", StringComparison.Ordinal))
            return false;
        return true;
    }

    /// <summary>
    /// Returns the command line with the auth access token masked, so users
    /// can paste console output into bug reports without leaking their MSA
    /// session.
    /// </summary>
    private static string SanitiseForLog(string args, AuthValues auth)
    {
        if (string.IsNullOrEmpty(auth.AccessToken) || auth.AccessToken == "0")
            return args;
        return args.Replace(auth.AccessToken, "<access-token>");
    }

    // ── Auth ─────────────────────────────────────────────────────────

    private sealed record AuthValues(string Name, string Uuid, string AccessToken, string UserType);

    private AuthValues ResolveAuthValues()
    {
        var s = _settings.Settings;

        // Prefer the Minecraft Services profile we obtained ourselves.
        if (!string.IsNullOrEmpty(s.JavaAccessToken)
            && !string.IsNullOrEmpty(s.JavaUuid)
            && !string.IsNullOrEmpty(s.JavaUsername))
        {
            return new AuthValues(s.JavaUsername, s.JavaUuid, s.JavaAccessToken, "msa");
        }

        // Offline mode — use the launcher's username if set, else a sensible default.
        // We deliberately DO NOT read the official launcher's launcher_accounts.json
        // here: reaching into another launcher's credential store is a malware tell
        // and gets Glacier flagged by AV. Users without a Glacier MSA sign-in get
        // offline-mode worlds tied to their chosen name.
        var name = !string.IsNullOrWhiteSpace(s.Username) ? s.Username : "Player";
        var uuid = OfflineUuid(name);
        return new AuthValues(name, uuid, "0", "legacy");
    }

    private static string OfflineUuid(string name)
    {
        // Mojang's "OfflinePlayer:<name>" UUIDv3 (md5) — matches what vanilla
        // assigns offline accounts, so worlds/inventories stay associated.
        using var md5 = System.Security.Cryptography.MD5.Create();
        var hash = md5.ComputeHash(Encoding.UTF8.GetBytes("OfflinePlayer:" + name));
        // Set version (3) and variant bits to make it a valid UUIDv3.
        hash[6] = (byte)((hash[6] & 0x0f) | 0x30);
        hash[8] = (byte)((hash[8] & 0x3f) | 0x80);
        return new Guid(hash).ToString("N");
    }

    // ── Java runtime resolution ──────────────────────────────────────

    private string? ResolveJavaRuntime(VersionProfile profile)
    {
        // 1. Explicit override.
        var explicitPath = _settings.Settings.JavaRuntimePath;
        if (!string.IsNullOrEmpty(explicitPath) && File.Exists(explicitPath))
            return explicitPath;

        // 2. Minecraft's bundled JREs under .minecraft\runtime (the official launcher
        //    installs these per major version; reuse them so we match the version's
        //    declared javaVersion.majorVersion).
        var runtimeRoot = Path.Combine(_versions.MinecraftDir, "runtime");
        if (Directory.Exists(runtimeRoot))
        {
            foreach (var component in Directory.EnumerateDirectories(runtimeRoot))
            {
                // The official layout is runtime\<component>\<os>\<arch>\<...>\bin\javaw.exe
                if (!string.IsNullOrEmpty(profile.MinJavaMajor)
                    && !ComponentMatchesMajor(component, profile.MinJavaMajor))
                    continue;
                foreach (var javaw in EnumerateJavawUnder(component))
                    return javaw;
            }
            // No major-version match — return whatever we found first so the user
            // at least gets a launch attempt (and a clear error in the console)
            // rather than a silent "Java not found".
            foreach (var component in Directory.EnumerateDirectories(runtimeRoot))
                foreach (var javaw in EnumerateJavawUnder(component))
                    return javaw;
        }

        // 3. JAVA_HOME / PATH.
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

        // 4. Registry — Adoptium/Temurin/Microsoft Build of OpenJDK keys.
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
        // Official launcher names them with Greek codenames, not version numbers:
        //   jre-legacy → Java 8
        //   java-runtime-alpha → Java 16
        //   java-runtime-beta / java-runtime-gamma → Java 17
        //   java-runtime-delta → Java 21
        //   java-runtime-epsilon → Java 25 (anticipated, not yet shipped)
        // Matching by raw major-number substring was wrong — "21" doesn't appear
        // anywhere in delta's path, and "16" would false-match "java-runtime-…".
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
