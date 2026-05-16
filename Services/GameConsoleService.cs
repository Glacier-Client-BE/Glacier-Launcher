using System;
using System.Diagnostics;
using System.Threading.Tasks;
using System.Windows;
using static GlacierLauncher.GameConsoleWindow;

namespace GlacierLauncher.Services;

/// <summary>
/// Opens and feeds <see cref="GameConsoleWindow"/> instances. The launcher
/// services (Java + Bedrock) push log lines through this so the Blazor UI
/// doesn't need to know about WPF windows directly.
///
/// One window per launch attempt — closing the window detaches but doesn't
/// kill the game. Multiple concurrent launches each get their own window.
/// </summary>
public sealed class GameConsoleService
{
    private readonly SettingsService _settings;

    public GameConsoleService(SettingsService settings)
    {
        _settings = settings;
    }

    /// <summary>True only when the user has the toggle enabled in Settings.</summary>
    public bool Enabled => _settings.Settings.ShowLaunchConsole;

    /// <summary>
    /// Open a console window on the WPF dispatcher and return a handle the
    /// caller can push lines through. Returns null if the toggle is off or
    /// the WPF app isn't initialised yet (e.g. unit tests).
    /// </summary>
    public GameConsoleHandle? Open(string title)
    {
        if (!Enabled) return null;

        var app = Application.Current;
        if (app == null) return null;

        GameConsoleWindow? win = null;
        app.Dispatcher.Invoke(() =>
        {
            win = new GameConsoleWindow
            {
                Owner = app.MainWindow,
            };
            // Adopt the launcher's accent so the auto-scroll active state and
            // the status dot's "Running" colour match the rest of the UI.
            win.SetAccent(_settings.Settings.AccentColor);
            win.SetTitle(title);
            win.SetStatus("Starting…");
            win.Show();
        });

        return win != null ? new GameConsoleHandle(win, app.Dispatcher) : null;
    }
}

/// <summary>
/// Thin facade used by launchers to push state into a console window. Safe
/// to call from any thread — internally marshals to the UI dispatcher.
/// </summary>
public sealed class GameConsoleHandle : IDisposable
{
    private readonly GameConsoleWindow _window;
    private readonly System.Windows.Threading.Dispatcher _dispatcher;
    private Process? _attached;

    internal GameConsoleHandle(GameConsoleWindow window, System.Windows.Threading.Dispatcher dispatcher)
    {
        _window     = window;
        _dispatcher = dispatcher;
    }

    public void Info(string line)   => _window.AppendLine(line, LineKind.Info);
    public void Stdout(string line) => _window.AppendLine(line, LineKind.Stdout);
    public void Stderr(string line) => _window.AppendLine(line, LineKind.Stderr);
    public void Error(string line)  => _window.AppendLine(line, LineKind.Error);

    public void MarkRunning() => _window.MarkRunning();
    public void MarkExited(int code) => _window.MarkExited(code);
    public void MarkFailed(string why) => _window.MarkFailed(why);
    public void SetPid(int? pid) => _window.SetPid(pid);

    /// <summary>
    /// Attaches a Process so its stdout/stderr flow into the console and the
    /// Stop button can kill it. The console keeps reading until the process
    /// exits, even if the user closes the window.
    /// </summary>
    public void Attach(Process process)
    {
        _attached = process;
        _dispatcher.Invoke(() => _window.AttachedProcess = process);

        if (process.StartInfo.RedirectStandardOutput)
        {
            process.OutputDataReceived += (_, e) =>
            {
                if (e.Data != null) Stdout(e.Data);
            };
            process.BeginOutputReadLine();
        }
        if (process.StartInfo.RedirectStandardError)
        {
            process.ErrorDataReceived += (_, e) =>
            {
                if (e.Data != null) Stderr(e.Data);
            };
            process.BeginErrorReadLine();
        }

        // Background-await the exit so we update status without blocking the caller.
        _ = Task.Run(async () =>
        {
            try
            {
                await process.WaitForExitAsync();
                int code = 0;
                try { code = process.ExitCode; } catch { }
                MarkExited(code);
                Info($"Process exited with code {code}.");
            }
            catch (Exception ex)
            {
                Error("Wait failed: " + ex.Message);
            }
        });
    }

    public void Dispose()
    {
        // Don't auto-close the window — users may want to read the log after
        // the game crashes. The window has its own close button.
    }
}
