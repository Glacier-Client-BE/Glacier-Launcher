using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text.Json;
using Microsoft.Win32;

namespace GlacierLauncher.Services;

/// <summary>
/// Detects and launches Lunar Client (which now also covers Badlion since the
/// Lunar Network / Badlion merger). We prefer a direct game launch through
/// Lunar's bundled JRE when its offline-multiver tree is present — that's the
/// folder Lunar writes after it's downloaded a version at least once. If we
/// can't locate that tree we fall back to launching the Electron app, which
/// always works but shows the splash + update step.
/// </summary>
public sealed class LunarBadlionService
{
    public string? LunarExePath   { get; private set; }
    public string? BadlionExePath { get; private set; }
    public string? LunarOfflineRoot { get; private set; }
    public string? LunarJrePath  { get; private set; }
    public string[] LunarInstalledVersions { get; private set; } = Array.Empty<string>();

    public bool LunarInstalled    => !string.IsNullOrEmpty(LunarExePath);
    public bool BadlionInstalled  => !string.IsNullOrEmpty(BadlionExePath);

    /// <summary>True when we have everything needed to skip the launcher splash.</summary>
    public bool CanDirectLaunchLunar =>
        !string.IsNullOrEmpty(LunarJrePath)
        && LunarInstalledVersions.Length > 0;

    public LunarBadlionService()
    {
        Detect();
    }

    /// <summary>Re-runs detection. Cheap; safe to call from UI refresh paths.</summary>
    public void Detect()
    {
        LunarExePath   = FindLunar();
        BadlionExePath = FindBadlion();

        var userProfile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var offlineRoot = Path.Combine(userProfile, ".lunarclient", "offline", "multiver");
        if (Directory.Exists(offlineRoot))
        {
            LunarOfflineRoot = offlineRoot;
            try
            {
                LunarInstalledVersions = Directory
                    .EnumerateDirectories(offlineRoot)
                    .Select(Path.GetFileName)
                    .Where(n => !string.IsNullOrEmpty(n))
                    .Select(n => n!)
                    .OrderByDescending(n => n) // newest minor version first
                    .ToArray();
            }
            catch { LunarInstalledVersions = Array.Empty<string>(); }
        }
        else
        {
            LunarOfflineRoot = null;
            LunarInstalledVersions = Array.Empty<string>();
        }

        LunarJrePath = FindLunarJre();
    }

    /// <summary>
    /// Launches Lunar. If <paramref name="versionId"/> is non-empty we try to
    /// skip the Electron launcher and start javaw against Lunar's bundled
    /// game JAR; otherwise we open the Electron app.
    /// </summary>
    public void LaunchLunar(string? versionId = null)
    {
        if (!LunarInstalled && !CanDirectLaunchLunar)
            throw new InvalidOperationException("Lunar Client isn't installed. Get it from lunarclient.com.");

        if (!string.IsNullOrEmpty(versionId)
            && CanDirectLaunchLunar
            && TryDirectLaunch(versionId))
        {
            return;
        }

        if (!string.IsNullOrEmpty(LunarExePath))
        {
            Start(LunarExePath);
            return;
        }

        throw new InvalidOperationException(
            "Lunar's installed game files are present but the launcher app is missing. " +
            "Reinstall Lunar so the Electron app is on disk too.");
    }

    public void LaunchBadlion()
    {
        // The Lunar Network bought Badlion and many users now launch Badlion-
        // equivalent content from Lunar. If a dedicated Badlion install is on
        // disk we still prefer it; otherwise we just open Lunar.
        if (!string.IsNullOrEmpty(BadlionExePath))
        {
            Start(BadlionExePath);
            return;
        }
        if (!string.IsNullOrEmpty(LunarExePath))
        {
            Start(LunarExePath);
            return;
        }
        throw new InvalidOperationException(
            "Badlion Client isn't installed. Lunar (which acquired Badlion) is also not detected — get one from lunarclient.com or badlion.net.");
    }

    private bool TryDirectLaunch(string versionId)
    {
        // We don't claim to know every Lunar release's launch line — version
        // dirs vary in classpath layout between Lunar builds. This is the
        // best-effort form: throw all jars in the version dir onto -cp and
        // try the Genesis main class. If it doesn't start the user falls
        // back to the Electron app on next click.
        if (string.IsNullOrEmpty(LunarOfflineRoot) || string.IsNullOrEmpty(LunarJrePath))
            return false;

        var versionDir = Path.Combine(LunarOfflineRoot, versionId);
        if (!Directory.Exists(versionDir)) return false;

        try
        {
            var jars = Directory.EnumerateFiles(versionDir, "*.jar", SearchOption.TopDirectoryOnly).ToArray();
            if (jars.Length == 0) return false;

            var nativesDir = Path.Combine(versionDir, "natives");
            var classpath  = string.Join(';', jars);

            // Lunar's Genesis entrypoint takes its own switches via a textproto
            // shipped under the version dir; we forward the bare minimum and
            // rely on Lunar's defaults for everything else.
            var args =
                $"-Xms512m -Xmx2048m " +
                $"-Djava.library.path=\"{nativesDir}\" " +
                $"-cp \"{classpath}\" " +
                $"com.moonsworth.lunar.genesis.Genesis " +
                $"--version {versionId} --launcherVersion glacier";

            Process.Start(new ProcessStartInfo
            {
                FileName         = LunarJrePath,
                Arguments        = args,
                WorkingDirectory = versionDir,
                UseShellExecute  = false,
                CreateNoWindow   = true,
            });
            return true;
        }
        catch
        {
            return false;
        }
    }

    private static void Start(string exe)
    {
        Process.Start(new ProcessStartInfo
        {
            FileName         = exe,
            WorkingDirectory = Path.GetDirectoryName(exe) ?? "",
            UseShellExecute  = true,
        });
    }

    // ── Detection ───────────────────────────────────────────────

    private static string? FindLunar()
    {
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var candidates = new[]
        {
            Path.Combine(local, "Programs", "lunarclient", "Lunar Client.exe"),
            Path.Combine(local, "Programs", "Lunar Client", "Lunar Client.exe"),
            Path.Combine(local, "lunarclient", "Lunar Client.exe"),
        };
        foreach (var p in candidates)
            if (File.Exists(p)) return p;

        return ReadUninstallExe("Lunar Client");
    }

    private static string? FindBadlion()
    {
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var candidates = new[]
        {
            Path.Combine(local, "Programs", "Badlion Client", "Badlion Client.exe"),
            Path.Combine(local, "Badlion Client", "Badlion Client.exe"),
        };
        foreach (var p in candidates)
            if (File.Exists(p)) return p;

        return ReadUninstallExe("Badlion Client");
    }

    private static string? FindLunarJre()
    {
        var userProfile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var jreRoot = Path.Combine(userProfile, ".lunarclient", "jre");
        if (!Directory.Exists(jreRoot)) return null;

        try
        {
            // Lunar typically ships multiple JREs (e.g. one per game-version).
            // We just need any javaw.exe to launch — pick the most recently-
            // modified one so newer Java wins.
            return Directory
                .EnumerateFiles(jreRoot, "javaw.exe", SearchOption.AllDirectories)
                .OrderByDescending(File.GetLastWriteTimeUtc)
                .FirstOrDefault();
        }
        catch { return null; }
    }

    /// <summary>Walks HKCU + HKLM Uninstall keys for a DisplayName match.</summary>
    private static string? ReadUninstallExe(string displayNameContains)
    {
        try
        {
            foreach (var rootKey in new[] { Registry.CurrentUser, Registry.LocalMachine })
            {
                foreach (var sub in new[]
                {
                    @"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                    @"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
                })
                {
                    using var k = rootKey.OpenSubKey(sub);
                    if (k == null) continue;
                    foreach (var name in k.GetSubKeyNames())
                    {
                        using var entry = k.OpenSubKey(name);
                        if (entry == null) continue;
                        var dn   = entry.GetValue("DisplayName") as string;
                        if (string.IsNullOrEmpty(dn) ||
                            !dn.Contains(displayNameContains, StringComparison.OrdinalIgnoreCase)) continue;
                        var icon = entry.GetValue("DisplayIcon") as string;
                        if (string.IsNullOrEmpty(icon)) continue;
                        var comma = icon.IndexOf(',');
                        if (comma >= 0) icon = icon[..comma];
                        icon = icon.Trim('"');
                        if (File.Exists(icon)) return icon;
                    }
                }
            }
        }
        catch { }
        return null;
    }
}
