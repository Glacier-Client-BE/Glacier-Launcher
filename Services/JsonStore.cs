using System.IO;

namespace GlacierLauncher.Services;

/// <summary>
/// Helpers for crash-safe JSON persistence. A plain <see cref="File.WriteAllText(string,string)"/>
/// leaves a truncated file if the process dies mid-write — and this launcher
/// deliberately exits during self-update and kills child processes — which would
/// wipe the user's settings or instance list on the next load. Writing to a temp
/// sibling and atomically moving it into place means a reader only ever sees the
/// complete old file or the complete new one, never a half-written one.
/// </summary>
internal static class JsonStore
{
    /// <summary>Atomically replaces <paramref name="path"/> with <paramref name="contents"/>.</summary>
    public static void WriteAtomic(string path, string contents)
    {
        var dir = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(dir)) Directory.CreateDirectory(dir);

        var tmp = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
        try
        {
            File.WriteAllText(tmp, contents);
            File.Move(tmp, path, overwrite: true);
        }
        finally
        {
            try { if (File.Exists(tmp)) File.Delete(tmp); } catch { /* best effort */ }
        }
    }

    /// <summary>
    /// Moves a file that failed to parse aside to <c>*.corrupt</c>, so the next load
    /// starts from a clean default instead of choking on the same bytes forever, and
    /// the user keeps a copy they could recover by hand. Only call this for genuine
    /// parse failures — never for transient read errors, or you'd discard good data.
    /// </summary>
    public static void QuarantineCorrupt(string path)
    {
        try
        {
            if (File.Exists(path)) File.Move(path, path + ".corrupt", overwrite: true);
        }
        catch { /* best effort */ }
    }
}
