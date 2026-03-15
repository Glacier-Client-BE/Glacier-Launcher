using System.Windows;
using Microsoft.Extensions.DependencyInjection;
using MinecraftLauncher.Services;

namespace MinecraftLauncher;

public partial class MainWindow : Window
{
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

        // Auto-start Discord RPC if it was enabled on last run
        var rpc = services.GetRequiredService<DiscordRpcService>();
        rpc.Start();

        // Wire up window-level messages from the Blazor UI via WebView2 postMessage
        Loaded += async (_, _) =>
        {
            await blazorWebView.WebView.EnsureCoreWebView2Async();
            blazorWebView.WebView.DefaultBackgroundColor = System.Drawing.Color.FromArgb(0, 22, 22, 30);
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

        // Clean up Discord RPC when the window is closed
        Closed += (_, _) =>
        {
            services.GetRequiredService<DiscordRpcService>().Stop();
        };
    }
}