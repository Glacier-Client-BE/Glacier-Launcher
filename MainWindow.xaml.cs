using System;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using Microsoft.Extensions.DependencyInjection;
using GlacierLauncher.Services;
using System.IO;

namespace GlacierLauncher;

public partial class MainWindow : Window
{
    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, uint attr, ref int v, int size);

    private const uint DWMWA_WINDOW_CORNER_PREFERENCE = 33;
    private const uint DWMWA_USE_IMMERSIVE_DARK_MODE  = 20;
    private const int  DWMWCP_ROUND = 2;

    private const int WM_NCHITTEST       = 0x0084;
    private const int WM_NCLBUTTONDBLCLK = 0x00A3;
    private const int HTLEFT        = 10;
    private const int HTRIGHT       = 11;
    private const int HTTOP         = 12;
    private const int HTTOPLEFT     = 13;
    private const int HTTOPRIGHT    = 14;
    private const int HTBOTTOM      = 15;
    private const int HTBOTTOMLEFT  = 16;
    private const int HTBOTTOMRIGHT = 17;
    private const int ResizeBorder  = 6;

    private ServiceProvider? _services;

    public MainWindow()
    {
        // 1. Setup Dependency Injection
        var sc = new ServiceCollection();
        sc.AddWpfBlazorWebView();
        sc.AddSingleton<SettingsService>();
        sc.AddSingleton<GameLauncher>();
        sc.AddSingleton<FlarialService>();
        sc.AddSingleton<OderSoService>();
        sc.AddSingleton<AutoUpdateService>();
        sc.AddSingleton<DiscordRpcService>();

        // Only add dev tools if specifically needed, usually wrapped in #if DEBUG
#if DEBUG
        sc.AddBlazorWebViewDeveloperTools();
#endif

        _services = sc.BuildServiceProvider();
        Resources.Add("services", _services);

        // 2. Initialize UI
        InitializeComponent();

        // FIX FOR SINGLE-FILE: Explicitly set the HostPage to ensure the WebView 
        // finds index.html in the extracted wwwroot folder.
        blazorWebView.HostPage = "wwwroot/index.html";

        // 3. Load Settings
        var settings = _services.GetRequiredService<SettingsService>().Settings;
        if (settings.RememberWindowSize && settings.WindowWidth >= 500)
        {
            Width  = settings.WindowWidth;
            Height = settings.WindowHeight;
        }

        // 4. Start Services
        _services.GetRequiredService<DiscordRpcService>().Start();

        // 5. Native Window Customization (Dark Mode & Rounded Corners)
        SourceInitialized += (_, _) =>
        {
            var hwnd = new WindowInteropHelper(this).Handle;
            int round = DWMWCP_ROUND;
            DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, ref round, sizeof(int));
            int dark = 1;
            DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, ref dark, sizeof(int));
            HwndSource.FromHwnd(hwnd)!.AddHook(WndProc);
        };

        // 6. WebView Configuration & JS Interop
        Loaded += async (_, _) =>
        {
            await blazorWebView.WebView.EnsureCoreWebView2Async();
            
            // Set background to match your launcher's theme to prevent white flashes
            blazorWebView.WebView.DefaultBackgroundColor = System.Drawing.Color.FromArgb(255, 35, 39, 42);
            
            blazorWebView.WebView.CoreWebView2.WebMessageReceived += (_, args) =>
            {
                var msg = args.TryGetWebMessageAsString();
                if (msg == null) return;
                
                Dispatcher.Invoke(() =>
                {
                    if      (msg == "startDrag")     { try { DragMove(); } catch { } }
                    else if (msg == "close")         Close();
                    else if (msg == "minimize")      WindowState = WindowState.Minimized;
                    else if (msg == "minimizeToTray") WindowState = WindowState.Minimized;
                    else if (msg == "maximize")
                    {
                        WindowState = WindowState == WindowState.Maximized
                            ? WindowState.Normal
                            : WindowState.Maximized;
                    }
                    else if (msg.StartsWith("openUrl:"))
                    {
                        var url = msg.Substring("openUrl:".Length);
                        try 
                        { 
                            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(url) 
                            { 
                                UseShellExecute = true 
                            }); 
                        }
                        catch { }
                    }
                });
            };
        };

        // 7. Cleanup & Save on Close
        Closed += (_, _) =>
        {
            var svc = _services.GetRequiredService<SettingsService>();
            if (svc.Settings.RememberWindowSize && WindowState == WindowState.Normal)
            {
                svc.Settings.WindowWidth  = Width;
                svc.Settings.WindowHeight = Height;
                svc.Save();
            }
            _services.GetRequiredService<DiscordRpcService>().Stop();
        };
    }

    // Native Window Management for Custom Chrome (Resize Borders)
    private IntPtr WndProc(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (msg == WM_NCLBUTTONDBLCLK) { handled = true; return IntPtr.Zero; }
        if (msg != WM_NCHITTEST) return IntPtr.Zero;

        int x  = (short)(lParam.ToInt32() & 0xFFFF);
        int y  = (short)((lParam.ToInt32() >> 16) & 0xFFFF);
        
        try 
        {
            var pt = PointFromScreen(new Point(x, y));
            double w = ActualWidth, h = ActualHeight;
            int    b = ResizeBorder;

            bool left = pt.X <= b, right = pt.X >= w - b, top = pt.Y <= b, bottom = pt.Y >= h - b;

            if (top    && left)  { handled = true; return (IntPtr)HTTOPLEFT;     }
            if (top    && right) { handled = true; return (IntPtr)HTTOPRIGHT;    }
            if (bottom && left)  { handled = true; return (IntPtr)HTBOTTOMLEFT;  }
            if (bottom && right) { handled = true; return (IntPtr)HTBOTTOMRIGHT; }
            if (left)            { handled = true; return (IntPtr)HTLEFT;        }
            if (right)           { handled = true; return (IntPtr)HTRIGHT;       }
            if (top)             { handled = true; return (IntPtr)HTTOP;         }
            if (bottom)          { handled = true; return (IntPtr)HTBOTTOM;      }
        }
        catch { }

        return IntPtr.Zero;
    }
}