using System;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using Microsoft.Extensions.DependencyInjection;
using MinecraftLauncher.Services;

namespace MinecraftLauncher;

public partial class MainWindow : Window
{
    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, uint attr, ref int attrValue, int attrSize);

    private const uint DWMWA_WINDOW_CORNER_PREFERENCE = 33;
    private const uint DWMWA_USE_IMMERSIVE_DARK_MODE  = 20;
    private const int  DWMWCP_ROUND = 2;

    public MainWindow()
    {
        var serviceCollection = new ServiceCollection();
        serviceCollection.AddWpfBlazorWebView();
        serviceCollection.AddSingleton<SettingsService>();
        serviceCollection.AddSingleton<GameLauncher>();
        serviceCollection.AddSingleton<DiscordRpcService>();
        serviceCollection.AddBlazorWebViewDeveloperTools();

        var services = serviceCollection.BuildServiceProvider();
        Resources.Add("services", services);

        InitializeComponent();

        services.GetRequiredService<DiscordRpcService>().Start();

        SourceInitialized += (_, _) =>
        {
            var hwnd = new WindowInteropHelper(this).Handle;
            int round = DWMWCP_ROUND;
            DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, ref round, sizeof(int));
            int dark = 1;
            DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, ref dark, sizeof(int));
        };

        Loaded += async (_, _) =>
        {
            await blazorWebView.WebView.EnsureCoreWebView2Async();
            blazorWebView.WebView.DefaultBackgroundColor = System.Drawing.Color.FromArgb(255, 26, 26, 32);
            blazorWebView.WebView.CoreWebView2.WebMessageReceived += (_, args) =>
            {
                var msg = args.TryGetWebMessageAsString();
                Dispatcher.Invoke(() =>
                {
                    switch (msg)
                    {
                        case "startDrag":
                            try { DragMove(); } catch { }
                            break;
                        case "close":
                            Close();
                            break;
                        case "minimize":
                            WindowState = WindowState.Minimized;
                            break;
                    }
                });
            };
        };

        Closed += (_, _) =>
        {
            services.GetRequiredService<DiscordRpcService>().Stop();
        };
    }
}
