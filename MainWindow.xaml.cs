using System;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using Microsoft.Extensions.DependencyInjection;
using GlacierLauncher.Services;

namespace GlacierLauncher;

public partial class MainWindow : Window
{
    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, uint attr, ref int v, int size);

    private const uint DWMWA_WINDOW_CORNER_PREFERENCE = 33;
    private const uint DWMWA_USE_IMMERSIVE_DARK_MODE = 20;
    private const int DWMWCP_ROUND = 2;

    private const int WM_NCHITTEST = 0x0084;
    private const int WM_NCLBUTTONDBLCLK = 0x00A3;
    private const int HTLEFT = 10;
    private const int HTRIGHT = 11;
    private const int HTTOP = 12;
    private const int HTTOPLEFT = 13;
    private const int HTTOPRIGHT = 14;
    private const int HTBOTTOM = 15;
    private const int HTBOTTOMLEFT = 16;
    private const int HTBOTTOMRIGHT = 17;
    private const int ResizeBorder = 6;

    private ServiceProvider? _services;
    private bool _isFullscreen;
    private WindowState _preFullscreenState;
    private double _preFullscreenWidth, _preFullscreenHeight;
    private double _preFullscreenLeft, _preFullscreenTop;
    private TrayIcon? _tray;

    public MainWindow()
    {
        // In published single-file mode, extract wwwroot before anything else
        EnsureWwwroot();

        var sc = new ServiceCollection();
        sc.AddWpfBlazorWebView();
        sc.AddSingleton<SettingsService>();
        sc.AddSingleton<DownloadService>();
        sc.AddSingleton<GameConsoleService>();
        sc.AddSingleton<GameLauncher>();
        sc.AddSingleton<FlarialService>();
        sc.AddSingleton<OderSoService>();
        sc.AddSingleton<AutoUpdateService>();
        sc.AddSingleton<DiscordRpcService>();
        sc.AddSingleton<CurseForgeService>();
        sc.AddSingleton<VanillaVersionService>();
        sc.AddSingleton<StoreInstallService>();
        sc.AddSingleton<LiveAuthService>();
        sc.AddSingleton<XboxProfileService>();
        sc.AddSingleton<JavaInstanceService>();
        sc.AddSingleton<JavaVersionService>();
        sc.AddSingleton<JavaGameLauncher>();
        sc.AddSingleton<JavaInstallService>();
        sc.AddSingleton<JavaModLoaderService>();
        sc.AddSingleton<LunarBadlionService>();
        sc.AddSingleton<ModrinthService>();
        sc.AddSingleton<GlacierClientService>();
        sc.AddSingleton<ThemeService>();
        sc.AddSingleton<JavaRuntimeDownloadService>();
        sc.AddSingleton<ModpackInstallService>();
        sc.AddSingleton<StatsService>();
        sc.AddSingleton<LogService>();
        sc.AddSingleton<SkinLibraryService>();

#if DEBUG
        sc.AddBlazorWebViewDeveloperTools();
#endif

        _services = sc.BuildServiceProvider();
        Resources.Add("services", _services);

        InitializeComponent();

        var settings = _services.GetRequiredService<SettingsService>().Settings;
        if (settings.RememberWindowSize && settings.WindowWidth >= 500)
        {
            Width = settings.WindowWidth;
            Height = settings.WindowHeight;
        }

        // Defer Discord IPC init off the ctor path — its named-pipe handshake can
        // add a noticeable hitch to cold start before the window paints.
        Dispatcher.BeginInvoke(System.Windows.Threading.DispatcherPriority.ApplicationIdle,
            new Action(() => _services!.GetRequiredService<DiscordRpcService>().Start()));

        SourceInitialized += (_, _) =>
        {
            var hwnd = new WindowInteropHelper(this).Handle;
            int round = DWMWCP_ROUND;
            DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, ref round, sizeof(int));
            int dark = 1;
            DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, ref dark, sizeof(int));
            HwndSource.FromHwnd(hwnd)!.AddHook(WndProc);
        };

        blazorWebView.UrlLoading += (_, e) =>
        {
            if ((e.Url.Scheme == "https" || e.Url.Scheme == "http") && e.Url.Host != "0.0.0.1")
            {
                e.UrlLoadingStrategy = Microsoft.AspNetCore.Components.WebView.UrlLoadingStrategy.OpenExternally;
            }
        };

        Loaded += async (_, _) =>
        {
            await blazorWebView.WebView.EnsureCoreWebView2Async();

            // The await above yields the UI thread; if the window was closed (and the
            // WebView2 control disposed) while we were suspended, touching any
            // CoreWebView2 member now throws InvalidOperationException/COMException on a
            // dead COM object. Bail out cleanly instead of crashing the process.
            if (!IsLoaded || blazorWebView.WebView.CoreWebView2 == null) return;

            try
            {
                blazorWebView.WebView.DefaultBackgroundColor = System.Drawing.Color.FromArgb(255, 35, 39, 42);

            // Map the user's Glacier Launcher folder to a virtual host so the WebView can load
            // local files (custom wallpaper, etc.) without falling foul of WebView2's same-origin
            // policy on file:// URLs. The page can request https://glacier-files.local/<file>.
            try
            {
                var glacierFolder = Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                    "Glacier Launcher");
                Directory.CreateDirectory(glacierFolder);
                blazorWebView.WebView.CoreWebView2.SetVirtualHostNameToFolderMapping(
                    "glacier-files.local",
                    glacierFolder,
                    Microsoft.Web.WebView2.Core.CoreWebView2HostResourceAccessKind.Allow);
            }
            catch { /* best effort — feature is available since WebView2 1.0.864.35 */ }

            blazorWebView.WebView.CoreWebView2.WebMessageReceived += (_, args) =>
            {
                var msg = args.TryGetWebMessageAsString();
                if (msg == null) return;
                Dispatcher.Invoke(() =>
                {
                    if (msg == "startDrag") { try { DragMove(); } catch { } }
                    else if (msg == "close") Close();
                    else if (msg == "minimize") WindowState = WindowState.Minimized;
                    else if (msg == "minimizeToTray") MinimizeToTray();
                    else if (msg == "maximize")
                    {
                        if (_isFullscreen) ToggleFullscreen(); // exit fullscreen first
                        else WindowState = WindowState == WindowState.Maximized ? WindowState.Normal : WindowState.Maximized;
                    }
                    else if (msg == "fullscreen") ToggleFullscreen();
                    else if (msg.StartsWith("openUrl:"))
                    {
                        var url = msg.Substring("openUrl:".Length);
                        try { System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(url) { UseShellExecute = true }); } catch { }
                    }
                });
                };
            }
            catch (Exception ex) when (ex is InvalidOperationException || ex is System.Runtime.InteropServices.COMException)
            {
                // WebView disposed mid-initialization (window closed during startup) — ignore.
            }
        };

        Closed += (_, _) =>
        {
            var svc = _services.GetRequiredService<SettingsService>();
            if (svc.Settings.RememberWindowSize && WindowState == WindowState.Normal)
            {
                svc.Settings.WindowWidth = Width;
                svc.Settings.WindowHeight = Height;
            }
            // Also lands any pending debounced save from slider/filter writes.
            svc.Flush();
            _services.GetRequiredService<DiscordRpcService>().Stop();
            _tray?.Dispose();
        };
    }

    // Hides the window to a tray icon, creating the icon on first use.
    private void MinimizeToTray()
    {
        try
        {
            if (_tray == null)
            {
                var iconPath = Path.Combine(AppContext.BaseDirectory, "icon.ico");
                _tray = new TrayIcon("Glacier Launcher", iconPath);
                _tray.OnOpen += () => Dispatcher.Invoke(RestoreFromTray);
                _tray.OnExit += () => Dispatcher.Invoke(Close);
            }
            Hide();
        }
        catch
        {
            // If the tray icon can't be created, fall back to a normal minimize
            // so the button never appears to do nothing.
            WindowState = WindowState.Minimized;
        }
    }

    private void RestoreFromTray()
    {
        Show();
        WindowState = WindowState.Normal;
        Activate();
        Topmost = true;
        Topmost = false;
    }

    private System.Windows.Shell.WindowChrome? _origChrome;

    private void ToggleFullscreen()
    {
        if (!_isFullscreen)
        {
            // Save state so we can restore it later
            _preFullscreenState = WindowState;
            _preFullscreenWidth = Width;
            _preFullscreenHeight = Height;
            _preFullscreenLeft = Left;
            _preFullscreenTop = Top;
            _origChrome = System.Windows.Shell.WindowChrome.GetWindowChrome(this);

            // Drop the resize border so the window is truly edge-to-edge
            System.Windows.Shell.WindowChrome.SetWindowChrome(this, null);

            // Must set Normal first so manual sizing applies
            WindowState = WindowState.Normal;
            ResizeMode = ResizeMode.NoResize;

            var hwnd = new WindowInteropHelper(this).Handle;
            var monitor = MonitorFromWindow(hwnd, 2 /* MONITOR_DEFAULTTONEAREST */);
            var info = new MONITORINFO { cbSize = (uint)Marshal.SizeOf<MONITORINFO>() };
            GetMonitorInfo(monitor, ref info);

            // info.rcMonitor covers the FULL monitor including the taskbar (vs. rcWork
            // which excludes it). With WindowStyle=None and no chrome, the window now
            // sits over the taskbar like a real fullscreen app.
            var dpi = VisualTreeHelper.GetDpi(this).PixelsPerDip;
            Left   = info.rcMonitor.Left / dpi;
            Top    = info.rcMonitor.Top / dpi;
            Width  = (info.rcMonitor.Right - info.rcMonitor.Left) / dpi;
            Height = (info.rcMonitor.Bottom - info.rcMonitor.Top) / dpi;

            _isFullscreen = true;
        }
        else
        {
            // Restore chrome FIRST so the resize border is back when we re-apply size
            if (_origChrome != null)
                System.Windows.Shell.WindowChrome.SetWindowChrome(this, _origChrome);

            ResizeMode = ResizeMode.CanResize;
            Left   = _preFullscreenLeft;
            Top    = _preFullscreenTop;
            Width  = _preFullscreenWidth;
            Height = _preFullscreenHeight;
            WindowState = _preFullscreenState;
            _isFullscreen = false;
        }

        NotifyFullscreenState();
    }

    private void NotifyFullscreenState()
    {
        try
        {
            blazorWebView.WebView?.CoreWebView2?.PostWebMessageAsString(
                _isFullscreen ? "fullscreen:on" : "fullscreen:off");
        }
        catch { /* webview may not be initialised yet */ }
    }

    [DllImport("user32.dll")]
    private static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint dwFlags);

    [DllImport("user32.dll")]
    private static extern bool GetMonitorInfo(IntPtr hMonitor, ref MONITORINFO lpmi);

    [StructLayout(LayoutKind.Sequential)]
    private struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    private struct MONITORINFO
    {
        public uint cbSize;
        public RECT rcMonitor;
        public RECT rcWork;
        public uint dwFlags;
    }

    private static void EnsureWwwroot()
    {
        var wwwrootDir = Path.Combine(AppContext.BaseDirectory, "wwwroot");
        var stampFile  = Path.Combine(AppContext.BaseDirectory, "wwwroot.stamp");

        using var stream = typeof(MainWindow).Assembly.GetManifestResourceStream("wwwroot.zip");
        if (stream == null)
            return;

        // Build a cheap stamp (length + first 256 bytes' CRC-ish). The single-file
        // host extracts the embedded zip into a new temp folder on every build, so
        // a length-only check is enough to detect a rebuild without paying for the
        // ~1.2 MB ExtractToDirectory call on every cold start.
        long zipLen = stream.Length;
        var stamp = "len=" + zipLen.ToString();

        try
        {
            if (Directory.Exists(wwwrootDir) && File.Exists(stampFile)
                && File.ReadAllText(stampFile) == stamp)
            {
                return; // already extracted for this build
            }
        }
        catch { /* fall through and re-extract */ }

        try
        {
            Directory.CreateDirectory(wwwrootDir);
            using var archive = new ZipArchive(stream, ZipArchiveMode.Read);
            archive.ExtractToDirectory(wwwrootDir, overwriteFiles: true);
            try { File.WriteAllText(stampFile, stamp); } catch { }
        }
        catch (IOException)
        {
            // If a file is locked (e.g. by another running instance), we ignore it.
            // The existing files will just be used.
        }
        catch (UnauthorizedAccessException)
        {
            // Ignore access errors
        }
    }

    private IntPtr WndProc(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (msg == WM_NCLBUTTONDBLCLK) { handled = true; return IntPtr.Zero; }
        if (msg != WM_NCHITTEST) return IntPtr.Zero;
        try
        {
            int x = (short)(lParam.ToInt32() & 0xFFFF);
            int y = (short)((lParam.ToInt32() >> 16) & 0xFFFF);
            var pt = PointFromScreen(new Point(x, y));
            double w = ActualWidth, h = ActualHeight;
            int b = ResizeBorder;

            bool left = pt.X <= b, right = pt.X >= w - b, top = pt.Y <= b, bottom = pt.Y >= h - b;

            if (top && left) { handled = true; return (IntPtr)HTTOPLEFT; }
            if (top && right) { handled = true; return (IntPtr)HTTOPRIGHT; }
            if (bottom && left) { handled = true; return (IntPtr)HTBOTTOMLEFT; }
            if (bottom && right) { handled = true; return (IntPtr)HTBOTTOMRIGHT; }
            if (left) { handled = true; return (IntPtr)HTLEFT; }
            if (right) { handled = true; return (IntPtr)HTRIGHT; }
            if (top) { handled = true; return (IntPtr)HTTOP; }
            if (bottom) { handled = true; return (IntPtr)HTBOTTOM; }
        }
        catch { }
        return IntPtr.Zero;
    }
}
