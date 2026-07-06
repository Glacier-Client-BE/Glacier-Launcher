using System;
using System.Collections.Generic;
using System.Globalization;

namespace GlacierLauncher.Models;

/// <summary>
/// A fully custom launcher theme. Every CSS variable the stylesheet exposes is
/// representable here; derived values (accent glow, hover tint, background
/// overlays) are computed in <see cref="BuildCssVars"/> so exported themes stay
/// small and humans only edit meaningful knobs.
/// </summary>
public class ThemeDefinition
{
    // Bump when the shape changes. Import tolerates older versions and unknown
    // fields so shared .glaciertheme.json files keep working across releases.
    public int    SchemaVersion { get; set; } = 1;

    public string Id         { get; set; } = Guid.NewGuid().ToString("N");
    public string Name       { get; set; } = "Custom Theme";

    // The built-in preset whose component-level overrides we inherit — matters
    // mostly for "light", which restyles inputs/panels beyond CSS variables.
    public string BasePreset { get; set; } = "dark";

    // ── Colors ──
    public string Accent  { get; set; } = "#7289da";
    public string Bg      { get; set; } = "#23272a";
    public string BgPanel { get; set; } = "#2c2f33";
    public string Text    { get; set; } = "#ffffff";
    public string TextDim { get; set; } = "#99aab5";
    public string Red     { get; set; } = "#ff5c5c";
    public string Green   { get; set; } = "#43b581";
    public string Orange  { get; set; } = "#faa61a";

    // ── Shape & effects ──
    public int    RadiusSm        { get; set; } = 8;
    public int    RadiusMd        { get; set; } = 12;
    public int    Blur            { get; set; } = 14;
    /// <summary>0.0–1.0 — how strongly the background image is dimmed toward Bg.</summary>
    public double OverlayStrength { get; set; } = 1.0;

    // ── Typography & motion ──
    public string FontFamily     { get; set; } = "";   // empty = Segoe UI stack
    public double AnimationSpeed { get; set; } = 1.0;

    // ── Background image ──
    /// <summary>Absolute path of the wallpaper copied under ~/Glacier Launcher (empty = default bg).</summary>
    public string BackgroundImage   { get; set; } = "";
    public string BackgroundFit     { get; set; } = "cover"; // cover | contain | tile | center
    public double BackgroundOpacity { get; set; } = 1.0;     // opacity of the image layer itself

    // ── Power users ──
    public string CustomCss { get; set; } = "";

    /// <summary>
    /// Flattens the theme into the CSS custom properties app.css consumes,
    /// including all derived values.
    /// </summary>
    public Dictionary<string, string> BuildCssVars()
    {
        var (ar, ag, ab) = ParseHex(Accent);
        var (br, bg2, bb) = ParseHex(Bg);
        var (tr, tg, tb) = ParseHex(Text);
        var s = Math.Clamp(OverlayStrength, 0.0, 1.0);
        bool lightText = (tr + tg + tb) / 3.0 > 128;

        string F(double v) => v.ToString("0.###", CultureInfo.InvariantCulture);

        return new Dictionary<string, string>
        {
            ["--accent"]        = Accent,
            ["--accent-hover"]  = Lighten(Accent, 0.12),
            ["--accent-glow"]   = $"rgba({ar},{ag},{ab},0.42)",
            ["--accent-bg"]     = $"rgba({ar},{ag},{ab},0.10)",
            ["--bg"]            = Bg,
            ["--bg-panel"]      = BgPanel,
            // Item tiles ride on the text color so they read on any background.
            ["--bg-item"]       = $"rgba({tr},{tg},{tb},{(lightText ? "0.04" : "0.05")})",
            ["--bg-item-hover"] = $"rgba({tr},{tg},{tb},{(lightText ? "0.075" : "0.09")})",
            ["--text"]          = Text,
            ["--text-dim"]      = TextDim,
            ["--red"]           = Red,
            ["--green"]         = Green,
            ["--orange"]        = Orange,
            ["--r-sm"]          = $"{RadiusSm}px",
            ["--r-md"]          = $"{RadiusMd}px",
            ["--overlay-top"]   = $"rgba({br},{bg2},{bb},{F(0.55 * s)})",
            ["--overlay-mid"]   = $"rgba({br},{bg2},{bb},{F(0.20 * s)})",
            ["--overlay-bot"]   = $"rgba({br},{bg2},{bb},{F(0.85 * s)})",
        };
    }

    public ThemeDefinition Clone()
    {
        var copy = (ThemeDefinition)MemberwiseClone();
        copy.Id = Guid.NewGuid().ToString("N");
        return copy;
    }

    /// <summary>
    /// Parses "#RRGGBB", "#RGB", "#RRGGBBAA" or "rgba(r,g,b,a)"/"rgb(r,g,b)" — the
    /// custom colour picker can emit any of these once alpha is involved.
    /// </summary>
    internal static (int r, int g, int b) ParseHex(string color)
    {
        var (r, g, b, _) = ParseColor(color);
        return (r, g, b);
    }

    internal static (int r, int g, int b, double a) ParseColor(string color)
    {
        try
        {
            var s = (color ?? "").Trim();
            if (s.StartsWith("rgb", StringComparison.OrdinalIgnoreCase))
            {
                var inner = s[(s.IndexOf('(') + 1)..s.IndexOf(')')];
                var parts = inner.Split(',', StringSplitOptions.TrimEntries);
                int r = int.Parse(parts[0]), g = int.Parse(parts[1]), b = int.Parse(parts[2]);
                double a = parts.Length > 3 ? double.Parse(parts[3], CultureInfo.InvariantCulture) : 1.0;
                return (r, g, b, a);
            }
            var h = s.TrimStart('#');
            if (h.Length == 3) h = $"{h[0]}{h[0]}{h[1]}{h[1]}{h[2]}{h[2]}";
            int rr = Convert.ToInt32(h[..2], 16), gg = Convert.ToInt32(h[2..4], 16), bb = Convert.ToInt32(h[4..6], 16);
            double aa = h.Length >= 8 ? Convert.ToInt32(h[6..8], 16) / 255.0 : 1.0;
            return (rr, gg, bb, aa);
        }
        catch { return (114, 137, 218, 1.0); }
    }

    internal static string Lighten(string color, double amount)
    {
        var (r, g, b, a) = ParseColor(color);
        int L(int c) => Math.Clamp((int)(c + (255 - c) * amount), 0, 255);
        int lr = L(r), lg = L(g), lb = L(b);
        return a >= 0.999
            ? $"#{lr:x2}{lg:x2}{lb:x2}"
            : $"rgba({lr},{lg},{lb},{a.ToString("0.###", CultureInfo.InvariantCulture)})";
    }
}
