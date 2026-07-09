using System;
using System.IO;
using System.Linq;
using Windows.Management.Deployment;

namespace GlacierLauncher.Services;

/// <summary>
/// Pre-flight checks that turn "Minecraft failed to launch" into a specific,
/// actionable reason before we even try — missing Gaming Services, a
/// sideloaded (non-Store) Minecraft install, or a custom DLL that's the
/// wrong architecture/not a real PE. Also classifies exceptions thrown by
/// <see cref="InjectionService"/>/<see cref="GameLauncher"/> into the same
/// typed reasons so the UI can show one consistent message either way.
/// </summary>
public static class LaunchDiagnosticsService
{
    public enum Reason
    {
        Ok,
        GamingServicesMissing,
        MinecraftNotInstalled,
        MinecraftSideloaded,
        DllNotFound,
        DllInvalidPe,
        DllWrongArchitecture,
        InjectionFailed,
        LaunchTimedOut,
        Unknown
    }

    public sealed record Diagnosis(Reason Reason, string Title, string Message, bool CanOfferRepair = false);

    private static readonly PackageManager _packages = new();

    /// <summary>
    /// Checks Gaming Services + Minecraft package presence. Cheap (local package
    /// enumeration, no network) — safe to call right before every launch.
    /// </summary>
    public static Diagnosis CheckPrerequisites()
    {
        try
        {
            var gamingServices = _packages.FindPackagesForUser(string.Empty, "Microsoft.GamingServices_8wekyb3d8bbwe").Any();
            if (!gamingServices)
            {
                return new Diagnosis(Reason.GamingServicesMissing,
                    "Gaming Services missing",
                    "Minecraft needs Microsoft Gaming Services to launch, and it isn't installed. " +
                    "Install it from the Microsoft Store, then try again.",
                    CanOfferRepair: true);
            }

            var release = _packages.FindPackagesForUser(string.Empty, "Microsoft.MinecraftUWP_8wekyb3d8bbwe").Any();
            var preview = _packages.FindPackagesForUser(string.Empty, "Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe").Any();
            if (!release && !preview)
            {
                // A sideloaded install registers under a different package family (or with a
                // signature that FindPackagesForUser under the interactive user won't surface
                // the same way as a Store install). Fall back to checking for the running exe
                // path via WMI-free heuristics: if AppxManifest.xml exists in our own sideload
                // versions folder for the active version, treat it as sideloaded rather than
                // "not installed" so the message tells the truth.
                var activeVersionDir = VanillaVersionServiceActiveDir();
                if (activeVersionDir != null && File.Exists(Path.Combine(activeVersionDir, "AppxManifest.xml")))
                {
                    return new Diagnosis(Reason.MinecraftSideloaded,
                        "Sideloaded Minecraft detected",
                        "Minecraft was sideloaded rather than installed from the Store. Launch and injection " +
                        "still work, but if Minecraft won't start, re-install the version from the Versions tab.",
                        CanOfferRepair: false);
                }

                return new Diagnosis(Reason.MinecraftNotInstalled,
                    "Minecraft not installed",
                    "Minecraft Bedrock isn't installed. Install a version from the Versions tab first.",
                    CanOfferRepair: true);
            }
        }
        catch
        {
            // Package enumeration itself failing isn't fatal — just skip the pre-check
            // rather than blocking launch on a diagnostic that couldn't run.
        }

        return new Diagnosis(Reason.Ok, "", "");
    }

    private static string? VanillaVersionServiceActiveDir()
    {
        try
        {
            var settingsPath = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                "Glacier Launcher", "glacier-settings.json");
            if (!File.Exists(settingsPath)) return null;
            var json = File.ReadAllText(settingsPath);
            using var doc = System.Text.Json.JsonDocument.Parse(json);
            if (!doc.RootElement.TryGetProperty("ActiveVanillaVersion", out var v)) return null;
            var ver = v.GetString();
            if (string.IsNullOrEmpty(ver)) return null;
            return Path.Combine(VanillaVersionService.VersionsDirectory, ver);
        }
        catch { return null; }
    }

    /// <summary>
    /// Validates a user-supplied DLL before we attempt injection: file exists,
    /// looks like a real PE image, and is x64 (Minecraft.Windows is always x64,
    /// so an x86/ARM64 DLL would just fail LoadLibraryW inside the target).
    /// </summary>
    public static Diagnosis ValidateDll(string dllPath)
    {
        if (string.IsNullOrWhiteSpace(dllPath) || !File.Exists(dllPath))
        {
            return new Diagnosis(Reason.DllNotFound,
                "DLL not found",
                $"Couldn't find the file: {dllPath}");
        }

        try
        {
            using var fs = new FileStream(dllPath, FileMode.Open, FileAccess.Read, FileShare.Read);
            using var br = new BinaryReader(fs);

            if (fs.Length < 0x40)
                return InvalidPe(dllPath);

            if (br.ReadUInt16() != 0x5A4D) // "MZ"
                return InvalidPe(dllPath);

            fs.Seek(0x3C, SeekOrigin.Begin);
            var peHeaderOffset = br.ReadInt32();
            if (peHeaderOffset <= 0 || peHeaderOffset + 6 > fs.Length)
                return InvalidPe(dllPath);

            fs.Seek(peHeaderOffset, SeekOrigin.Begin);
            if (br.ReadUInt32() != 0x00004550) // "PE\0\0"
                return InvalidPe(dllPath);

            var machine = br.ReadUInt16();
            const ushort IMAGE_FILE_MACHINE_AMD64 = 0x8664;
            if (machine != IMAGE_FILE_MACHINE_AMD64)
            {
                var arch = machine switch
                {
                    0x014c => "x86 (32-bit)",
                    0xAA64 => "ARM64",
                    _      => $"unknown (0x{machine:X4})"
                };
                return new Diagnosis(Reason.DllWrongArchitecture,
                    "Wrong architecture",
                    $"This DLL is {arch}, but Minecraft.Windows is x64. Use an x64-built DLL instead.");
            }
        }
        catch (IOException)
        {
            // File is locked/in-use — not a validity problem, let injection itself
            // surface the real error rather than guessing here.
            return new Diagnosis(Reason.Ok, "", "");
        }
        catch
        {
            return InvalidPe(dllPath);
        }

        return new Diagnosis(Reason.Ok, "", "");
    }

    private static Diagnosis InvalidPe(string dllPath) => new(Reason.DllInvalidPe,
        "Invalid DLL",
        $"'{Path.GetFileName(dllPath)}' doesn't look like a valid Windows DLL (bad PE header).");

    /// <summary>
    /// Maps an exception from the launch/inject pipeline to a typed reason with
    /// a friendlier message, so different failure causes don't all read the same.
    /// </summary>
    public static Diagnosis Classify(Exception ex)
    {
        var msg = ex.Message ?? "";

        if (ex is TimeoutException)
        {
            return new Diagnosis(Reason.LaunchTimedOut,
                "Minecraft didn't start",
                msg.Length > 0 ? msg : "Minecraft did not start in time. Try launching it manually once, then relaunch here.");
        }

        if (msg.Contains("LoadLibraryW returned NULL", StringComparison.OrdinalIgnoreCase) ||
            msg.Contains("CreateRemoteThread failed", StringComparison.OrdinalIgnoreCase) ||
            msg.Contains("VirtualAllocEx failed", StringComparison.OrdinalIgnoreCase) ||
            msg.Contains("WriteProcessMemory failed", StringComparison.OrdinalIgnoreCase))
        {
            return new Diagnosis(Reason.InjectionFailed, "Injection failed", msg);
        }

        if (msg.Contains("Could not open Minecraft process", StringComparison.OrdinalIgnoreCase))
        {
            return new Diagnosis(Reason.InjectionFailed, "Injection failed",
                msg + " If this keeps happening, try running Glacier Launcher as administrator.");
        }

        return new Diagnosis(Reason.Unknown, "Launch failed", msg.Length > 0 ? msg : "An unknown error occurred.");
    }
}
