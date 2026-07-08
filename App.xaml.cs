using System.Configuration;
using System.Data;
using System.Windows;
using GlacierLauncher.Services;

namespace GlacierLauncher;

/// <summary>
/// Interaction logic for App.xaml
/// </summary>
public partial class App : Application
{
    public App()
    {
        // If a previous update's file-copy retry loop ran out while our own exe handle
        // was still held, retry the swap now — before showing any UI — instead of
        // silently staying on the old version forever.
        if (AutoUpdateService.TryResumePendingUpdate())
        {
            Environment.Exit(0);
        }

        this.DispatcherUnhandledException += (s, e) =>
        {
            System.IO.File.WriteAllText("crash.txt", e.Exception.ToString());
        };

        AppDomain.CurrentDomain.UnhandledException += (s, e) =>
        {
            System.IO.File.WriteAllText("crash_domain.txt", e.ExceptionObject.ToString());
        };

        // Best-effort: enable SeDebugPrivilege so DLL injection into UWP Minecraft works
        // when the launcher runs as admin. Silently no-ops at standard user level.
        InjectionService.EnableDebugPrivilege();
    }
}

