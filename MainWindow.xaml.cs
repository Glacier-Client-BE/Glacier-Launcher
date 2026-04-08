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

    public MainWindow()
    {
        // In published single-file mode, extract wwwroot before anything else
        EnsureWwwroot();

        var sc = new ServiceCollection();
        sc.AddWpfBlazorWebView();
        sc.AddSingleton<SettingsService>();
        sc.AddSingleton<GameLauncher>();
        sc.AddSingleton<FlarialService>();
        sc.AddSingleton<OderSoService>();
        sc.AddSingleton<AutoUpdateService>();
        sc.AddSingleton<DiscordRpcService>();
        sc.AddSingleton<CurseForgeService>();

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

        _services.GetRequiredService<DiscordRpcService>().Start();

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
            blazorWebView.WebView.DefaultBackgroundColor = System.Drawing.Color.FromArgb(255, 35, 39, 42);
            blazorWebView.WebView.CoreWebView2.WebMessageReceived += (_, args) =>
            {
                var msg = args.TryGetWebMessageAsString();
                if (msg == null) return;
                Dispatcher.Invoke(() =>
                {
                    if (msg == "startDrag") { try { DragMove(); } catch { } }
                    else if (msg == "close") Close();
                    else if (msg == "minimize") WindowState = WindowState.Minimized;
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
        };

        Closed += (_, _) =>
        {
            var svc = _services.GetRequiredService<SettingsService>();
            if (svc.Settings.RememberWindowSize && WindowState == WindowState.Normal)
            {
                svc.Settings.WindowWidth = Width;
                svc.Settings.WindowHeight = Height;
                svc.Save();
            }
            _services.GetRequiredService<DiscordRpcService>().Stop();
        };
    }

    private void ToggleFullscreen()
    {
        if (!_isFullscreen)
        {
            _preFullscreenState = WindowState;
            _preFullscreenWidth = Width;
            _preFullscreenHeight = Height;
            _preFullscreenLeft = Left;
            _preFullscreenTop = Top;

            // Must set Normal first so MaxWidth/MaxHeight apply correctly
            WindowState = WindowState.Normal;
            ResizeMode = ResizeMode.NoResize;

            var screen = System.Windows.SystemParameters.WorkArea;
            var hwnd = new WindowInteropHelper(this).Handle;
            var monitor = MonitorFromWindow(hwnd, 2 /* MONITOR_DEFAULTTONEAREST */);
            var info = new MONITORINFO { cbSize = (uint)Marshal.SizeOf<MONITORINFO>() };
            GetMonitorInfo(monitor, ref info);

            Left   = info.rcMonitor.Left / VisualTreeHelper.GetDpi(this).PixelsPerDip;
            Top    = info.rcMonitor.Top / VisualTreeHelper.GetDpi(this).PixelsPerDip;
            Width  = (info.rcMonitor.Right - info.rcMonitor.Left) / VisualTreeHelper.GetDpi(this).PixelsPerDip;
            Height = (info.rcMonitor.Bottom - info.rcMonitor.Top) / VisualTreeHelper.GetDpi(this).PixelsPerDip;

            _isFullscreen = true;
        }
        else
        {
            ResizeMode = ResizeMode.CanResize;
            Left   = _preFullscreenLeft;
            Top    = _preFullscreenTop;
            Width  = _preFullscreenWidth;
            Height = _preFullscreenHeight;
            WindowState = _preFullscreenState;
            _isFullscreen = false;
        }
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

        // If wwwroot/index.html already exists (dev mode or already extracted), skip
        if (File.Exists(Path.Combine(wwwrootDir, "index.html")))
            return;

        // Extract embedded wwwroot.zip (contains app content + _framework files)
        // Use typeof() instead of GetExecutingAssembly() for single-file compatibility
        using var stream = typeof(MainWindow).Assembly.GetManifestResourceStream("wwwroot.zip");
        if (stream == null)
        {
            // Log to file for debugging
            File.WriteAllText(Path.Combine(AppContext.BaseDirectory, "extraction_error.txt"),
                $"wwwroot.zip not found. BaseDir={AppContext.BaseDirectory}\n" +
                $"Resources: {string.Join(", ", typeof(MainWindow).Assembly.GetManifestResourceNames())}");
            return;
        }

        Directory.CreateDirectory(wwwrootDir);
        using var archive = new ZipArchive(stream, ZipArchiveMode.Read);
        archive.ExtractToDirectory(wwwrootDir, overwriteFiles: true);
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
