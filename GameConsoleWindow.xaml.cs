using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Interop;
using System.Windows.Media;
using Microsoft.Win32;

namespace GlacierLauncher;

public partial class GameConsoleWindow : Window
{
    public enum LineKind { Stdout, Stderr, Info, Error }

    // Default palette — picked to read well on the launcher's dark background.
    // Stderr/error use the launcher's red, info uses its green, stdout sits at
    // the launcher's --text-dim so the timestamp doesn't outshout the line.
    private static readonly SolidColorBrush BrushStdout = new(Color.FromRgb(0xcb, 0xd5, 0xe1));
    private static readonly SolidColorBrush BrushStderr = new(Color.FromRgb(0xfb, 0xa1, 0xa1));
    private static readonly SolidColorBrush BrushInfo   = new(Color.FromRgb(0x86, 0xef, 0xac));
    private static readonly SolidColorBrush BrushError  = new(Color.FromRgb(0xff, 0x6b, 0x6b));
    private static readonly SolidColorBrush BrushDimTs  = new(Color.FromRgb(0x6b, 0x72, 0x80));

    private readonly FlowDocument _doc;
    private readonly Paragraph    _paragraph;

    private bool _autoScroll = true;
    private bool _wrap       = false;
    private bool _autoClose  = true;
    private int  _lineCount;

    private System.Windows.Threading.DispatcherTimer? _autoCloseTimer;
    private int _autoCloseSecondsLeft;

    /// <summary>Set by the host so Stop can ask the process to exit.</summary>
    public Process? AttachedProcess { get; set; }

    public GameConsoleWindow()
    {
        InitializeComponent();

        _paragraph = new Paragraph { Margin = new Thickness(0) };
        _doc = new FlowDocument(_paragraph)
        {
            PageWidth = _wrap ? double.NaN : 4000,
            FontFamily = LogBox.FontFamily,
            FontSize   = LogBox.FontSize,
            Background = Brushes.Transparent,
        };
        LogBox.Document = _doc;

        UpdateAutoScrollButton();
        UpdateWrapButton();
        UpdateAutoCloseButton();

        // Mirror the launcher's window chrome: rounded corners + dark title bar.
        SourceInitialized += (_, _) =>
        {
            try
            {
                var hwnd = new WindowInteropHelper(this).Handle;
                int round = DWMWCP_ROUND;
                DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, ref round, sizeof(int));
                int dark = 1;
                DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, ref dark, sizeof(int));
            }
            catch { /* old Windows builds don't expose these attributes */ }
        };

        Closed += (_, _) =>
        {
            // Closing the console must NOT kill the game — detach quietly so
            // the still-running process keeps owning its own lifecycle.
            AttachedProcess = null;
        };
    }

    // ── DWM ────────────────────────────────────────────────────

    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, uint attr, ref int v, int size);
    private const uint DWMWA_WINDOW_CORNER_PREFERENCE = 33;
    private const uint DWMWA_USE_IMMERSIVE_DARK_MODE  = 20;
    private const int  DWMWCP_ROUND = 2;

    // ── Theme accent (driven by Glacier's accent setting) ──────

    /// <summary>
    /// Re-points the "Accent" / "AccentSoft" resources to the user's launcher
    /// accent so the auto-scroll active state and the running-status dot use
    /// the same colour as the rest of the launcher.
    /// </summary>
    public void SetAccent(string hex)
    {
        try
        {
            var c = (Color)ColorConverter.ConvertFromString(hex);
            Resources["Accent"]     = new SolidColorBrush(c);
            // Same colour at 10% — matches --accent-bg in app.css.
            Resources["AccentSoft"] = new SolidColorBrush(Color.FromArgb(0x1a, c.R, c.G, c.B));
            // The status dot starts on "Starting…" → leave it orange. The
            // running state uses the launcher's green to match success states
            // in the rest of the UI.
            UpdateAutoScrollButton();
        }
        catch { /* invalid hex — keep the default */ }
    }

    public void SetTitle(string title) => TitleLabel.Text = title;

    public void SetStatus(string status, Brush? dotColor = null)
    {
        Dispatcher.Invoke(() =>
        {
            StatusLabel.Text = status;
            if (dotColor != null) StatusDot.Fill = dotColor;
        });
    }

    public void SetPid(int? pid) =>
        Dispatcher.Invoke(() => PidLabel.Text = pid.HasValue ? $"PID {pid.Value}" : "");

    public void MarkRunning()
    {
        SetStatus("Running", (SolidColorBrush)Resources["Green"]);
        // The window is mid-life — any earlier auto-close timer (e.g. from a
        // previous crash + relaunch in the same window) shouldn't fire now.
        CancelAutoClose();
    }

    public void MarkExited(int code)
    {
        var dot = (SolidColorBrush)Resources["TextDimmer"];
        SetStatus($"Exited · code {code}", dot);
        // Clean exit → close quickly so the launcher window comes back into
        // focus. Non-zero → assume a crash, leave more time for the user to
        // skim the stack trace before we tear the window down.
        if (_autoClose)
            StartAutoClose(code == 0 ? 3 : 8);
    }

    public void MarkFailed(string why)
    {
        SetStatus("Failed: " + why, (SolidColorBrush)Resources["Red"]);
        if (_autoClose)
            StartAutoClose(8);
    }

    // ── Auto-close timer ────────────────────────────────────────

    private void StartAutoClose(int seconds)
    {
        Dispatcher.Invoke(() =>
        {
            CancelAutoClose();
            _autoCloseSecondsLeft = seconds;
            UpdateFooterCountdown();
            _autoCloseTimer = new System.Windows.Threading.DispatcherTimer
            {
                Interval = TimeSpan.FromSeconds(1),
            };
            _autoCloseTimer.Tick += AutoCloseTick;
            _autoCloseTimer.Start();
        });
    }

    private void AutoCloseTick(object? sender, EventArgs e)
    {
        _autoCloseSecondsLeft--;
        if (_autoCloseSecondsLeft <= 0)
        {
            CancelAutoClose();
            try { Close(); } catch { }
            return;
        }
        UpdateFooterCountdown();
    }

    private void UpdateFooterCountdown()
    {
        FooterLabel.Text = $"Closing in {_autoCloseSecondsLeft}s · click anywhere to keep open";
    }

    private void CancelAutoClose()
    {
        if (_autoCloseTimer != null)
        {
            _autoCloseTimer.Stop();
            _autoCloseTimer.Tick -= AutoCloseTick;
            _autoCloseTimer = null;
        }
    }

    /// <summary>Hooked from the window's PreviewMouseDown so a click anywhere abandons the countdown.</summary>
    protected override void OnPreviewMouseDown(System.Windows.Input.MouseButtonEventArgs e)
    {
        if (_autoCloseTimer != null)
        {
            CancelAutoClose();
            FooterLabel.Text = _lineCount == 1 ? "1 line" : $"{_lineCount} lines · auto-close cancelled";
        }
        base.OnPreviewMouseDown(e);
    }

    public void AppendLine(string text, LineKind kind = LineKind.Stdout)
    {
        if (text == null) return;
        if (!Dispatcher.CheckAccess())
        {
            Dispatcher.BeginInvoke(new Action(() => AppendLine(text, kind)));
            return;
        }

        var ts = DateTime.Now.ToString("HH:mm:ss");

        var tsRun = new Run("[" + ts + "] ") { Foreground = BrushDimTs };
        var body  = new Run(text) { Foreground = ColorFor(kind) };

        _paragraph.Inlines.Add(tsRun);
        _paragraph.Inlines.Add(body);
        _paragraph.Inlines.Add(new LineBreak());

        _lineCount++;
        FooterLabel.Text = _lineCount == 1 ? "1 line" : $"{_lineCount} lines";

        // Keep memory bounded for long-running games.
        if (_lineCount > 5000)
        {
            const int dropInlines = 1000 * 3; // 1 line = ts + body + linebreak
            for (int i = 0; i < dropInlines && _paragraph.Inlines.Count > 0; i++)
                _paragraph.Inlines.Remove(_paragraph.Inlines.FirstInline);
            _lineCount -= 1000;
        }

        if (_autoScroll) LogScroll.ScrollToEnd();
    }

    private static Brush ColorFor(LineKind kind) => kind switch
    {
        LineKind.Stderr => BrushStderr,
        LineKind.Info   => BrushInfo,
        LineKind.Error  => BrushError,
        _               => BrushStdout,
    };

    // ── Buttons ─────────────────────────────────────────────────

    private void AutoScrollBtn_Click(object sender, RoutedEventArgs e)
    {
        _autoScroll = !_autoScroll;
        UpdateAutoScrollButton();
        if (_autoScroll) LogScroll.ScrollToEnd();
    }

    private void UpdateAutoScrollButton()
    {
        var styleKey = _autoScroll ? "PillButtonActive" : "PillButton";
        AutoScrollBtn.Style = (Style)FindResource(styleKey);
    }

    private void WrapBtn_Click(object sender, RoutedEventArgs e)
    {
        _wrap = !_wrap;
        _doc.PageWidth = _wrap ? double.NaN : 4000;
        UpdateWrapButton();
    }

    private void UpdateWrapButton()
    {
        var styleKey = _wrap ? "PillButtonActive" : "PillButton";
        WrapBtn.Style = (Style)FindResource(styleKey);
    }

    private void AutoCloseBtn_Click(object sender, RoutedEventArgs e)
    {
        _autoClose = !_autoClose;
        UpdateAutoCloseButton();
        if (!_autoClose) CancelAutoClose();
    }

    private void UpdateAutoCloseButton()
    {
        var styleKey = _autoClose ? "PillButtonActive" : "PillButton";
        AutoCloseBtn.Style = (Style)FindResource(styleKey);
    }

    private void ClearBtn_Click(object sender, RoutedEventArgs e)
    {
        _paragraph.Inlines.Clear();
        _lineCount = 0;
        FooterLabel.Text = "0 lines";
    }

    private void CopyBtn_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            var range = new TextRange(_doc.ContentStart, _doc.ContentEnd);
            Clipboard.SetText(range.Text);
            FooterLabel.Text = $"Copied {_lineCount} line(s)";
        }
        catch { /* clipboard can transiently fail */ }
    }

    private void SaveBtn_Click(object sender, RoutedEventArgs e)
    {
        var dlg = new SaveFileDialog
        {
            Filter   = "Log files (*.log;*.txt)|*.log;*.txt|All files (*.*)|*.*",
            FileName = "minecraft-" + DateTime.Now.ToString("yyyyMMdd-HHmmss") + ".log",
        };
        if (dlg.ShowDialog() != true) return;
        try
        {
            var range = new TextRange(_doc.ContentStart, _doc.ContentEnd);
            File.WriteAllText(dlg.FileName, range.Text, Encoding.UTF8);
            FooterLabel.Text = "Saved " + Path.GetFileName(dlg.FileName);
        }
        catch (Exception ex)
        {
            AppendLine("Failed to save log: " + ex.Message, LineKind.Error);
        }
    }

    private async void ShareBtn_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            ShareBtn.IsEnabled = false;
            ShareBtn.Content   = "Uploading…";

            var range = new TextRange(_doc.ContentStart, _doc.ContentEnd);
            var text  = range.Text;
            if (string.IsNullOrWhiteSpace(text))
            {
                FooterLabel.Text = "Nothing to upload — log is empty.";
                return;
            }

            using var http    = new System.Net.Http.HttpClient();
            http.Timeout      = TimeSpan.FromSeconds(15);
            var content       = new System.Net.Http.FormUrlEncodedContent(new[]
            {
                new System.Collections.Generic.KeyValuePair<string, string>("content", text),
            });

            var resp = await http.PostAsync("https://api.mclo.gs/1/log", content);
            var body = await resp.Content.ReadAsStringAsync();

            if (resp.IsSuccessStatusCode)
            {
                // Response: {"success":true,"id":"abc123","url":"https://mclo.gs/abc123", ...}
                var doc  = System.Text.Json.JsonDocument.Parse(body);
                var url  = doc.RootElement.GetProperty("url").GetString() ?? "";
                Clipboard.SetText(url);
                AppendLine($"Log uploaded → {url}  (copied to clipboard)", LineKind.Info);
                FooterLabel.Text = "mclo.gs link copied!";
            }
            else
            {
                AppendLine($"mclo.gs upload failed (HTTP {(int)resp.StatusCode}): {body}", LineKind.Error);
            }
        }
        catch (Exception ex)
        {
            AppendLine("mclo.gs upload failed: " + ex.Message, LineKind.Error);
        }
        finally
        {
            ShareBtn.IsEnabled = true;
            ShareBtn.Content   = "mclo.gs";
        }
    }

    private void StopBtn_Click(object sender, RoutedEventArgs e)
    {
        var p = AttachedProcess;
        if (p == null)
        {
            AppendLine("No attached process to stop.", LineKind.Info);
            return;
        }
        try
        {
            if (!p.HasExited)
            {
                AppendLine("Stop requested — killing process tree…", LineKind.Info);
                p.Kill(entireProcessTree: true);
            }
        }
        catch (Exception ex)
        {
            AppendLine("Stop failed: " + ex.Message, LineKind.Error);
        }
    }
}
