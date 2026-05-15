using System;
using System.Collections.Specialized;
using System.IO;
using System.Threading.Tasks;
using System.Web;
using System.Windows;
using Microsoft.Web.WebView2.Core;

namespace GlacierLauncher;

public partial class LiveAuthWindow : Window
{
    public const string ClientId      = "00000000402b5328";
    public const string Scope         = "service::user.auth.xboxlive.com::MBI_SSL";
    public const string RedirectUri   = "https://login.live.com/oauth20_desktop.srf";
    public const string AuthorizeUrl  = "https://login.live.com/oauth20_authorize.srf";

    private readonly TaskCompletionSource<string?> _result = new();

    /// <summary>If true, force the account picker even if Edge has a session cookie.</summary>
    public bool PromptSelectAccount { get; init; }

    public LiveAuthWindow()
    {
        InitializeComponent();
        Loaded += async (_, _) => await InitWebViewAsync();
    }

    /// <summary>
    /// Awaits user sign-in. Returns the authorization code (response_type=code)
    /// or null if the user cancelled / closed the window.
    /// </summary>
    public Task<string?> GetAuthorizationCodeAsync() => _result.Task;

    private async Task InitWebViewAsync()
    {
        try
        {
            // Per-launcher user data folder keeps cookies isolated from the main
            // BlazorWebView. We can wipe this folder on sign-out.
            var userDataFolder = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                "Glacier Launcher", "auth-webview");
            Directory.CreateDirectory(userDataFolder);

            var env = await CoreWebView2Environment.CreateAsync(null, userDataFolder);
            await WebView.EnsureCoreWebView2Async(env);

            WebView.CoreWebView2.NavigationStarting += OnNavigationStarting;
            WebView.CoreWebView2.NavigationCompleted += (_, e) =>
            {
                StatusLabel.Text = e.IsSuccess
                    ? "Sign in to continue."
                    : $"Failed to load page ({e.WebErrorStatus}).";
            };

            // Force re-prompt by clearing the existing live.com cookies if requested.
            if (PromptSelectAccount)
            {
                try
                {
                    WebView.CoreWebView2.CookieManager.DeleteCookiesWithDomainAndPath(
                        ".live.com", "/", null);
                    WebView.CoreWebView2.CookieManager.DeleteCookiesWithDomainAndPath(
                        "login.live.com", "/", null);
                }
                catch { /* best effort */ }
            }

            WebView.CoreWebView2.Navigate(BuildAuthorizeUrl());
        }
        catch (Exception ex)
        {
            StatusLabel.Text = $"WebView failed: {ex.Message}";
        }
    }

    private static string BuildAuthorizeUrl()
    {
        var q = HttpUtility.ParseQueryString(string.Empty);
        q["client_id"]     = ClientId;
        q["response_type"] = "code";
        q["scope"]         = Scope;
        q["redirect_uri"]  = RedirectUri;
        q["display"]       = "touch";
        return $"{AuthorizeUrl}?{q}";
    }

    private void OnNavigationStarting(object? sender, CoreWebView2NavigationStartingEventArgs args)
    {
        if (!Uri.TryCreate(args.Uri, UriKind.Absolute, out var uri))
            return;

        // We only care about navigations that land on the redirect URI.
        if (!string.Equals(uri.GetLeftPart(UriPartial.Path), RedirectUri, StringComparison.OrdinalIgnoreCase))
            return;

        args.Cancel = true;

        var query = HttpUtility.ParseQueryString(uri.Query);
        var code  = query["code"];
        var error = query["error"];

        if (!string.IsNullOrEmpty(code))
        {
            StatusLabel.Text = "Authorizing…";
            _result.TrySetResult(code);
            Dispatcher.BeginInvoke((Action)Close);
        }
        else
        {
            var desc = query["error_description"];
            StatusLabel.Text = !string.IsNullOrEmpty(desc) ? desc : ($"Sign-in failed: {error}");
            _result.TrySetResult(null);
            Dispatcher.BeginInvoke((Action)Close);
        }
    }

    private void CancelButton_Click(object sender, RoutedEventArgs e)
    {
        _result.TrySetResult(null);
        Close();
    }

    protected override void OnClosed(EventArgs e)
    {
        _result.TrySetResult(null);
        try { WebView.Dispose(); } catch { }
        base.OnClosed(e);
    }
}
