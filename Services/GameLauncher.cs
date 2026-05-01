using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public class GameLauncher
{
    // LatiteClient/Latite is the active source. The old Imrglop/Latite-Releases is
    // archived and no longer receives builds, so we fetch via the JSON API and use
    // each asset's browser_download_url (asset names vary: LatiteNightly.dll,
    // LatiteDebug.dll, etc., and tags like nightly-YYYYMMDD-HHMMSS aren't constructable).
    private const string ApiUrl        = "https://api.github.com/repos/LatiteClient/Latite/releases";
    private const string UwpAppModelId = "Microsoft.MinecraftUWP_8wekyb3d8bbwe!App";
    private const string ProcessName   = "Minecraft.Windows";

    // ── Kernel32 / Shell32 / Advapi32 ────────────────────────────

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr VirtualAllocEx(IntPtr proc, IntPtr addr, uint size, uint type, uint protect);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool WriteProcessMemory(IntPtr proc, IntPtr addr, byte[] buf, uint size, out IntPtr written);

    [DllImport("kernel32.dll")]
    private static extern IntPtr GetModuleHandle(string mod);

    [DllImport("kernel32.dll")]
    private static extern IntPtr GetProcAddress(IntPtr mod, string proc);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr CreateRemoteThread(IntPtr proc, IntPtr attrs, uint stack,
        IntPtr start, IntPtr param, uint flags, out IntPtr tid);

    [DllImport("kernel32.dll")]
    private static extern uint WaitForSingleObject(IntPtr handle, uint ms);

    [DllImport("kernel32.dll")]
    private static extern bool CloseHandle(IntPtr handle);

    [DllImport("kernel32.dll")]
    private static extern bool VirtualFreeEx(IntPtr proc, IntPtr addr, uint size, uint type);

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
    private static extern uint SetEntriesInAclW(uint count,
        [MarshalAs(UnmanagedType.LPArray)] ExplicitAccess[] entries,
        IntPtr oldAcl, out IntPtr newAcl);

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
    private static extern uint SetNamedSecurityInfoW(string name, uint objType, uint secInfo,
        IntPtr owner, IntPtr group, IntPtr dacl, IntPtr sacl);

    [DllImport("kernel32.dll")]
    private static extern IntPtr LocalFree(IntPtr p);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct Trustee
    {
        public IntPtr pMultipleTrustee;
        public int    MultipleTrusteeOperation;
        public int    TrusteeForm;
        public int    TrusteeType;
        public IntPtr ptstrName;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ExplicitAccess
    {
        public uint    grfAccessPermissions;
        public int     grfAccessMode;
        public uint    grfInheritance;
        public Trustee Trustee;
    }

    private const uint PROCESS_ALL_ACCESS        = 0x1F0FFF;
    private const uint MEM_COMMIT                = 0x1000;
    private const uint MEM_RESERVE               = 0x2000;
    private const uint MEM_RELEASE               = 0x8000;
    private const uint PAGE_READWRITE            = 0x04;
    private const uint INFINITE                  = 0xFFFFFFFF;
    private const uint SE_FILE_OBJECT            = 1;
    private const uint DACL_SECURITY_INFORMATION = 0x4;
    private const uint GENERIC_ALL               = 0x10000000;
    private const int  SET_ACCESS                = 2;
    private const uint SUB_CONTAINERS_AND_OBJECTS = 3;
    private const int  TRUSTEE_IS_NAME           = 1;
    private const int  TRUSTEE_IS_WELL_KNOWN_GROUP = 5;

    // ────────────────────────────────────────────────────────────

    private readonly SettingsService _settingsService;
    private readonly HttpClient      _httpClient;

    public static string DownloadsDirectory =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "downloads");

    public static string LatiteDirectory =>
        Path.Combine(DownloadsDirectory, "Latite");

    public GameLauncher(SettingsService settingsService)
    {
        _settingsService = settingsService;
        _httpClient      = HttpFactory.Shared;
        Directory.CreateDirectory(LatiteDirectory);
    }

    public async Task<List<MinecraftVersion>> GetVersionsAsync()
    {
        var versions = new List<MinecraftVersion>();
        try
        {
            using var req = new HttpRequestMessage(HttpMethod.Get, ApiUrl + "?per_page=60");
            req.Headers.Accept.ParseAdd("application/vnd.github+json");
            req.Headers.TryAddWithoutValidation("X-GitHub-Api-Version", "2022-11-28");

            using var resp = await _httpClient.SendAsync(req);
            resp.EnsureSuccessStatusCode();
            var json = await resp.Content.ReadAsStringAsync();

            using var doc = JsonDocument.Parse(json);
            foreach (var release in doc.RootElement.EnumerateArray())
            {
                var tag = release.TryGetProperty("tag_name", out var t) ? t.GetString() ?? "" : "";
                if (string.IsNullOrEmpty(tag)) continue;

                // Skip floating tags — they duplicate the most recent timestamped build.
                if (tag is "nightly" or "debug" or "latest") continue;
                if (release.TryGetProperty("draft", out var d) && d.GetBoolean()) continue;

                var name        = release.TryGetProperty("name", out var n) ? n.GetString() ?? tag : tag;
                var publishedAt = release.TryGetProperty("published_at", out var pa) ? pa.GetString() : null;

                // Pick the runnable .dll. Prefer Release > Nightly > anything; never debug
                // (giant .pdb-paired builds aren't useful for end users).
                string? bestUrl = null; long bestSize = 0; string? bestAssetName = null; int bestRank = int.MaxValue;
                if (release.TryGetProperty("assets", out var assets) && assets.ValueKind == JsonValueKind.Array)
                {
                    foreach (var asset in assets.EnumerateArray())
                    {
                        var aName = asset.TryGetProperty("name", out var an) ? an.GetString() ?? "" : "";
                        if (!aName.EndsWith(".dll", StringComparison.OrdinalIgnoreCase)) continue;
                        if (aName.Contains("Debug", StringComparison.OrdinalIgnoreCase)) continue;

                        int rank = aName.Equals("Latite.dll", StringComparison.OrdinalIgnoreCase)        ? 0
                                 : aName.Contains("Nightly",  StringComparison.OrdinalIgnoreCase)        ? 1
                                                                                                          : 2;
                        if (rank < bestRank)
                        {
                            bestRank      = rank;
                            bestAssetName = aName;
                            bestUrl       = asset.TryGetProperty("browser_download_url", out var u) ? u.GetString() : null;
                            bestSize      = asset.TryGetProperty("size",                  out var s) ? s.GetInt64() : 0;
                        }
                    }
                }

                if (string.IsNullOrEmpty(bestUrl)) continue; // no usable .dll → skip release

                var variant = bestAssetName!.Contains("Nightly", StringComparison.OrdinalIgnoreCase) ? "Nightly"
                            : bestAssetName.Contains("Debug",   StringComparison.OrdinalIgnoreCase) ? "Debug"
                                                                                                    : "Release";

                versions.Add(new MinecraftVersion
                {
                    Tag          = tag,
                    DisplayName  = FormatLatiteDisplayName(name, tag, publishedAt),
                    IsDownloaded = File.Exists(GetDllPath(tag)),
                    DownloadUrl  = bestUrl,
                    AssetSize    = bestSize,
                    PublishedAt  = publishedAt,
                    Variant      = variant
                });
            }
        }
        catch (Exception ex)
        {
            versions.Add(new MinecraftVersion
            { Tag = "error", DisplayName = $"Failed to fetch: {ex.Message}", ErrorMessage = ex.Message });
        }
        return versions;
    }

    private static readonly Regex _latiteTagRegex =
        new(@"^(nightly|debug|release)-(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})$",
            RegexOptions.IgnoreCase | RegexOptions.Compiled);

    private static string FormatLatiteDisplayName(string apiName, string tag, string? publishedAt)
    {
        var m = _latiteTagRegex.Match(tag);
        if (m.Success)
        {
            var variant = char.ToUpperInvariant(m.Groups[1].Value[0]) + m.Groups[1].Value[1..].ToLowerInvariant();
            var date = $"{m.Groups[2].Value}-{m.Groups[3].Value}-{m.Groups[4].Value} {m.Groups[5].Value}:{m.Groups[6].Value}";
            return $"Latite {variant} · {date}";
        }
        if (!string.IsNullOrEmpty(publishedAt) &&
            DateTime.TryParse(publishedAt, CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal, out var dt))
        {
            return $"{apiName} · {dt:yyyy-MM-dd HH:mm}";
        }
        return apiName;
    }

    public async Task DownloadVersionAsync(MinecraftVersion version, IProgress<double>? progress = null)
    {
        if (string.IsNullOrEmpty(version.DownloadUrl))
            throw new InvalidOperationException("This version has no download URL. Refresh the versions list and try again.");

        var dllPath = GetDllPath(version.Tag);

        using var response = await _httpClient.GetAsync(version.DownloadUrl, HttpCompletionOption.ResponseHeadersRead);
        response.EnsureSuccessStatusCode();

        var total = response.Content.Headers.ContentLength ?? -1;
        var buf   = new byte[8192];
        long dl   = 0;

        await using var stream = await response.Content.ReadAsStreamAsync();
        await using var fs     = File.Create(dllPath);

        int read;
        while ((read = await stream.ReadAsync(buf)) > 0)
        {
            await fs.WriteAsync(buf.AsMemory(0, read));
            dl += read;
            if (total > 0) progress?.Report((double)dl / total * 100);
        }

        version.IsDownloaded = true;
        _settingsService.Settings.LastUsedVersion = version.Tag;
        _settingsService.Save();
    }

    public void DeleteVersion(MinecraftVersion version)
    {
        var path = GetDllPath(version.Tag);
        if (File.Exists(path)) File.Delete(path);
        version.IsDownloaded = false;
        if (_settingsService.Settings.LastUsedVersion == version.Tag)
        { _settingsService.Settings.LastUsedVersion = ""; _settingsService.Save(); }
    }

    public Task LaunchWithDllAsync(string dllPath)
    {
        if (!File.Exists(dllPath)) throw new FileNotFoundException($"DLL not found: {dllPath}");
        return LaunchWithPathAsync(dllPath);
    }

    /// <summary>
    /// Launches Minecraft with no DLL injection at all (vanilla / un-modified).
    /// </summary>
    public Task LaunchVanillaAsync() => LaunchWithPathAsync(null);

    public async Task LaunchAsync(string? versionTag = null, bool useFlarial = false)
    {
        // Determine DLL path — null means launch MC only, no injection
        string? dllPath = null;

        if (useFlarial)
        {
            dllPath = FlarialService.FilePath;
            if (!File.Exists(dllPath))
                throw new FileNotFoundException("Flarial DLL not found. Download it from the Clients tab first.");
        }
        else
        {
            var tag = versionTag ?? _settingsService.Settings.LastUsedVersion;
            if (!string.IsNullOrEmpty(tag))
            {
                var path = GetDllPath(tag);
                // If a version is selected but not downloaded yet, still launch MC — just skip injection
                dllPath = File.Exists(path) ? path : null;
            }
            // If no version is selected at all, dllPath stays null — launch MC without injection
        }

        await LaunchWithPathAsync(dllPath);

        if (!useFlarial && !string.IsNullOrEmpty(versionTag ?? _settingsService.Settings.LastUsedVersion))
        {
            _settingsService.Settings.LastUsedVersion = versionTag ?? _settingsService.Settings.LastUsedVersion;
            _settingsService.Save();
        }
    }

    public static string GetDllPath(string tag) =>
        Path.Combine(LatiteDirectory, $"Latite_{tag}.dll");

    /// <summary>True if a Minecraft.Windows process is currently running.</summary>
    public static bool IsMinecraftRunning() => GetMinecraftProcess() != null;

    // ── Core launch + inject ─────────────────────────────────────

    private async Task LaunchWithPathAsync(string? dllPath)
    {
        // Check if Minecraft is already running before launching
        var alreadyRunning = GetMinecraftProcess();

        if (alreadyRunning == null)
        {
            await TryLaunchMinecraftAsync();

            // Wait up to 60 seconds — GDK can be very slow on first launch
            for (int i = 0; i < 120; i++)
            {
                await Task.Delay(500);
                if (GetMinecraftProcess() != null) break;
            }
        }

        var mcProcess = GetMinecraftProcess();
        if (mcProcess == null)
            throw new TimeoutException("Minecraft did not start. Please launch Minecraft manually and then click Launch again.");

        // If no DLL was provided, just launch MC without injecting
        if (string.IsNullOrEmpty(dllPath)) return;

        // Wait for the game engine to initialise before injecting
        await Task.Delay(_settingsService.Settings.InjectionDelayMs);

        // Re-acquire — GDK sometimes restarts the process during init
        mcProcess = GetMinecraftProcess()
            ?? throw new TimeoutException("Minecraft exited before injection could complete.");

        GrantUwpAccess(dllPath);
        Inject((uint)mcProcess.Id, dllPath);
    }

    // Checks all known Minecraft process names
    private static Process? GetMinecraftProcess()
    {
        foreach (var name in new[] { "Minecraft.Windows", "Minecraft", "MinecraftUWP" })
        {
            var procs = Process.GetProcessesByName(name);
            if (procs.Length > 0) return procs[0];
        }
        return null;
    }

    // Tries each launch method in order, silently moving to the next on failure
    private static async Task TryLaunchMinecraftAsync()
    {
        // Method 1: minecraft:// URI — standard protocol, works on UWP and GDK
        if (await TryLaunchAsync(() =>
            Process.Start(new ProcessStartInfo("minecraft:")
            {
                UseShellExecute = true
            }))) return;

        // Method 2: start command via cmd — fallback if URI handler isn't registered
        await TryLaunchAsync(() =>
            Process.Start(new ProcessStartInfo("cmd.exe")
            {
                Arguments       = "/c start minecraft:",
                UseShellExecute = false,
                CreateNoWindow  = true,
                WindowStyle     = ProcessWindowStyle.Hidden
            }));
    }

    private static async Task<bool> TryLaunchAsync(Func<Process?> launcher)
    {
        try
        {
            launcher();
            // Give it 3 seconds to see if the process appears before trying the next method
            for (int i = 0; i < 6; i++)
            {
                await Task.Delay(500);
                if (GetMinecraftProcess() != null) return true;
            }
            return false;
        }
        catch { return false; }
    }

    private static void GrantUwpAccess(string dllPath)
    {
        IntPtr namePtr = Marshal.StringToHGlobalUni("ALL APPLICATION PACKAGES");
        try
        {
            var trustee = new Trustee
            {
                pMultipleTrustee         = IntPtr.Zero,
                MultipleTrusteeOperation = 0,
                TrusteeForm              = TRUSTEE_IS_NAME,
                TrusteeType              = TRUSTEE_IS_WELL_KNOWN_GROUP,
                ptstrName                = namePtr
            };
            var entry = new ExplicitAccess
            {
                grfAccessPermissions = GENERIC_ALL,
                grfAccessMode        = SET_ACCESS,
                grfInheritance       = SUB_CONTAINERS_AND_OBJECTS,
                Trustee              = trustee
            };

            uint err = SetEntriesInAclW(1, new[] { entry }, IntPtr.Zero, out IntPtr newAcl);
            if (err != 0) return;
            try
            {
                SetNamedSecurityInfoW(dllPath, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION,
                    IntPtr.Zero, IntPtr.Zero, newAcl, IntPtr.Zero);
            }
            finally { LocalFree(newAcl); }
        }
        finally { Marshal.FreeHGlobal(namePtr); }
    }

    private static void Inject(uint pid, string dllPath)
    {
        IntPtr hProc = OpenProcess(PROCESS_ALL_ACCESS, false, pid);
        if (hProc == IntPtr.Zero)
            throw new Exception($"OpenProcess failed (error {Marshal.GetLastWin32Error()}). Try running as Administrator.");

        IntPtr allocAddr = IntPtr.Zero;
        IntPtr hThread   = IntPtr.Zero;

        try
        {
            byte[] pathBytes = Encoding.Unicode.GetBytes(dllPath + "\0");
            uint   size      = (uint)pathBytes.Length;

            allocAddr = VirtualAllocEx(hProc, IntPtr.Zero, size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
            if (allocAddr == IntPtr.Zero)
                throw new Exception($"VirtualAllocEx failed (error {Marshal.GetLastWin32Error()}).");

            if (!WriteProcessMemory(hProc, allocAddr, pathBytes, size, out _))
                throw new Exception($"WriteProcessMemory failed (error {Marshal.GetLastWin32Error()}).");

            IntPtr k32      = GetModuleHandle("kernel32.dll");
            IntPtr loadLibW = GetProcAddress(k32, "LoadLibraryW");
            if (loadLibW == IntPtr.Zero) throw new Exception("Could not locate LoadLibraryW.");

            hThread = CreateRemoteThread(hProc, IntPtr.Zero, 0, loadLibW, allocAddr, 0, out _);
            if (hThread == IntPtr.Zero)
                throw new Exception($"CreateRemoteThread failed (error {Marshal.GetLastWin32Error()}).");

            WaitForSingleObject(hThread, INFINITE);
        }
        finally
        {
            if (allocAddr != IntPtr.Zero) VirtualFreeEx(hProc, allocAddr, 0, MEM_RELEASE);
            if (hThread   != IntPtr.Zero) CloseHandle(hThread);
            CloseHandle(hProc);
        }
    }
}