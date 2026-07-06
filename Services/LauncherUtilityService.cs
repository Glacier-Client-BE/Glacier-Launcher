using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace GlacierLauncher.Services;

/// <summary>
/// Small, side-effect-light helpers powering the launcher's maintenance and
/// diagnostics quick-actions: on-disk footprint, cache clearing and a copy-paste
/// system report. Pure file/system queries — no UI, no async state.
/// </summary>
public static class LauncherUtilityService
{
    public static string LauncherRoot =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "Glacier Launcher");

    public static string CacheDir => Path.Combine(LauncherRoot, "cache");

    /// <summary>
    /// Shows a native Win32 open-file dialog on a dedicated STA thread and returns the
    /// chosen path (null if cancelled). WebView2's &lt;input type=file&gt; does NOT expose
    /// the real filesystem path (file.path is Electron-only), so any picker that needs a
    /// usable path must go native — otherwise File.Exists fails with "file not found".
    /// </summary>
    public static Task<string?> PickFileAsync(string title, string filter)
    {
        return Task.Run(() =>
        {
            string? result = null;
            var thread = new System.Threading.Thread(() =>
            {
                try
                {
                    var dlg = new Microsoft.Win32.OpenFileDialog { Title = title, Filter = filter };
                    if (dlg.ShowDialog() == true) result = dlg.FileName;
                }
                catch { result = null; }
            });
            thread.SetApartmentState(System.Threading.ApartmentState.STA);
            thread.Start();
            thread.Join();
            return result;
        });
    }

    /// <summary>Total size of every file under <paramref name="path"/>, best-effort.</summary>
    public static long DirectorySize(string path)
    {
        if (!Directory.Exists(path)) return 0;
        long total = 0;
        try
        {
            foreach (var f in Directory.EnumerateFiles(path, "*", SearchOption.AllDirectories))
            {
                try { total += new FileInfo(f).Length; } catch { /* skip locked/vanished files */ }
            }
        }
        catch { /* enumeration raced a delete — return what we counted */ }
        return total;
    }

    /// <summary>
    /// Deletes the cached GitHub/version data. Returns the number of bytes freed.
    /// The caches rebuild themselves on next use, so this is always safe.
    /// </summary>
    public static long ClearCache()
    {
        var freed = DirectorySize(CacheDir);
        try { if (Directory.Exists(CacheDir)) Directory.Delete(CacheDir, recursive: true); }
        catch { freed = 0; }
        return freed;
    }

    /// <summary>A copy-pasteable system report for bug threads / support.</summary>
    public static string Diagnostics(string launcherVersion, string edition, string client)
    {
        var sb = new StringBuilder();
        sb.AppendLine($"Glacier Launcher v{launcherVersion}");
        sb.AppendLine($"Edition: {edition}");
        sb.AppendLine($"Client:  {client}");
        sb.AppendLine($"OS:      {RuntimeInformation.OSDescription} ({RuntimeInformation.OSArchitecture})");
        sb.AppendLine($".NET:    {RuntimeInformation.FrameworkDescription}");
        sb.AppendLine($"CPU:     {Environment.ProcessorCount} logical cores");
        sb.AppendLine($"Memory:  {FormatBytes(Environment.WorkingSet)} in use by launcher");
        sb.Append    ($"Data:    {FormatBytes(DirectorySize(LauncherRoot))} on disk");
        return sb.ToString();
    }

    public static string FormatBytes(long bytes)
    {
        if (bytes <= 0) return "0 B";
        string[] u = { "B", "KB", "MB", "GB", "TB" };
        double v = bytes; int i = 0;
        while (v >= 1024 && i < u.Length - 1) { v /= 1024; i++; }
        return $"{v:0.##} {u[i]}";
    }
}
