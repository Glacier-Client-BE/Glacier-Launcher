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

