using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using GlacierLauncher.Models;
using Microsoft.JSInterop;

namespace GlacierLauncher.Services;

/// <summary>
/// Persistence and live application of custom themes (Theme Studio).
/// Themes live in ~/Glacier Launcher/themes.json (atomic writes, corrupt files
/// quarantined — same guarantees as settings). Exported themes are standalone
/// .glaciertheme.json files that can be shared and re-imported.
/// </summary>
public class ThemeService
{
    private static readonly JsonSerializerOptions SerializerOptions = new() { WriteIndented = true };
    private static readonly JsonSerializerOptions ImportOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        AllowTrailingCommas = true,
        ReadCommentHandling = JsonCommentHandling.Skip,
    };

    private readonly string _themesPath;
    private readonly object _sync = new();

    public List<ThemeDefinition> Themes { get; private set; } = new();

    public string ExportsDir => Path.Combine(LauncherUtilityService.LauncherRoot, "themes", "exports");

    public ThemeService()
    {
        _themesPath = Path.Combine(LauncherUtilityService.LauncherRoot, "themes.json");
        Load();
    }

    // ── Preset seeds — mirror the [data-theme] blocks in app.css so the editor
    //    can start from any built-in look. ──
    public static readonly (string Preset, string Label, string Bg, string BgPanel, string Text, string TextDim)[] PresetSeeds =
    {
        ("dark",     "Dark",     "#23272a", "#2c2f33", "#ffffff", "#99aab5"),
        ("darker",   "Darker",   "#1e2124", "#26292c", "#ffffff", "#99aab5"),
        ("midnight", "Midnight", "#141520", "#1c1e30", "#ffffff", "#99aab5"),
        ("slate",    "Slate",    "#2a2d35", "#33373f", "#ffffff", "#99aab5"),
        ("ocean",    "Ocean",    "#0e1f2a", "#13293a", "#ffffff", "#99aab5"),
        ("forest",   "Forest",   "#16241a", "#1d2f22", "#ffffff", "#99aab5"),
        ("sunset",   "Sunset",   "#2a1a26", "#33212f", "#ffffff", "#99aab5"),
        ("light",    "Light",    "#f5f6f8", "#ffffff", "#0e1116", "#5b6470"),
    };

    public static ThemeDefinition SeedFrom(string preset, string accent)
    {
        var seed = PresetSeeds.FirstOrDefault(p => p.Preset == preset);
        if (seed.Preset == null) seed = PresetSeeds[0];
        return new ThemeDefinition
        {
            Name       = seed.Label + " Custom",
            BasePreset = seed.Preset == "light" ? "light" : seed.Preset,
            Accent     = string.IsNullOrEmpty(accent) ? "#7289da" : accent,
            Bg         = seed.Bg,
            BgPanel    = seed.BgPanel,
            Text       = seed.Text,
            TextDim    = seed.TextDim,
        };
    }

    public ThemeDefinition? Get(string id) =>
        string.IsNullOrEmpty(id) ? null : Themes.FirstOrDefault(t => t.Id == id);

    public void Load()
    {
        try
        {
            if (!File.Exists(_themesPath)) return;
            var json = File.ReadAllText(_themesPath);
            try
            {
                Themes = JsonSerializer.Deserialize<List<ThemeDefinition>>(json, ImportOptions) ?? new();
            }
            catch (JsonException)
            {
                JsonStore.QuarantineCorrupt(_themesPath);
                Themes = new();
            }
        }
        catch { Themes = new(); }
    }

    public void Save()
    {
        try
        {
            lock (_sync)
            {
                JsonStore.WriteAtomic(_themesPath, JsonSerializer.Serialize(Themes, SerializerOptions));
            }
        }
        catch { }
    }

    public ThemeDefinition Add(ThemeDefinition theme)
    {
        Themes.Add(theme);
        Save();
        return theme;
    }

    public ThemeDefinition? Duplicate(string id)
    {
        var src = Get(id);
        if (src == null) return null;
        var copy = src.Clone();
        copy.Name = src.Name + " Copy";
        return Add(copy);
    }

    public void Delete(string id)
    {
        Themes.RemoveAll(t => t.Id == id);
        Save();
    }

    /// <summary>Writes the theme as a shareable file and returns its path.</summary>
    public string Export(ThemeDefinition theme)
    {
        Directory.CreateDirectory(ExportsDir);
        var slug = new string(theme.Name.ToLowerInvariant()
            .Select(c => char.IsLetterOrDigit(c) ? c : '-').ToArray()).Trim('-');
        if (string.IsNullOrEmpty(slug)) slug = "theme";
        var path = Path.Combine(ExportsDir, slug + ".glaciertheme.json");
        JsonStore.WriteAtomic(path, JsonSerializer.Serialize(theme, SerializerOptions));
        return path;
    }

    /// <summary>Imports a theme file; returns null when the file isn't a valid theme.</summary>
    public ThemeDefinition? Import(string filePath)
    {
        try
        {
            var json = File.ReadAllText(filePath);
            var theme = JsonSerializer.Deserialize<ThemeDefinition>(json, ImportOptions);
            if (theme == null || string.IsNullOrWhiteSpace(theme.Name)) return null;

            // Future schema versions may add fields we don't know; unknown JSON
            // properties are already ignored by System.Text.Json. Sanitize the
            // parts that flow into CSS/JS.
            theme.Id = Guid.NewGuid().ToString("N");
            if (!PresetSeeds.Any(p => p.Preset == theme.BasePreset)) theme.BasePreset = "dark";
            theme.Blur           = Math.Clamp(theme.Blur, 0, 30);
            theme.RadiusSm       = Math.Clamp(theme.RadiusSm, 0, 24);
            theme.RadiusMd       = Math.Clamp(theme.RadiusMd, 0, 32);
            theme.AnimationSpeed = Math.Clamp(theme.AnimationSpeed, 0.25, 2.0);
            theme.BackgroundImage = ""; // background images don't travel with the file

            return Add(theme);
        }
        catch { return null; }
    }

    /// <summary>
    /// Pushes a theme into the live page: base preset attribute, every CSS
    /// variable, font, blur, animation speed, custom CSS and background.
    /// </summary>
    public async Task ApplyAsync(IJSRuntime js, ThemeDefinition t)
    {
        try
        {
            await js.InvokeVoidAsync("setTheme", t.BasePreset);
            await js.InvokeVoidAsync("setThemeVars", t.BuildCssVars());
            await js.InvokeVoidAsync("setFont", t.FontFamily);
            await js.InvokeVoidAsync("setBlurIntensity", t.Blur);
            await js.InvokeVoidAsync("setAnimationSpeed", t.AnimationSpeed);
            await js.InvokeVoidAsync("setCustomCss", t.CustomCss);
            await js.InvokeVoidAsync("setBackgroundFit", t.BackgroundFit, t.BackgroundOpacity);
            if (!string.IsNullOrEmpty(t.BackgroundImage))
                await js.InvokeVoidAsync("setCustomBackground", t.BackgroundImage);
        }
        catch { }
    }

    /// <summary>Removes every theme override, returning the page to the plain preset.</summary>
    public async Task ClearAsync(IJSRuntime js)
    {
        try
        {
            await js.InvokeVoidAsync("clearThemeVars");
            await js.InvokeVoidAsync("setFont", "");
            await js.InvokeVoidAsync("setCustomCss", "");
            await js.InvokeVoidAsync("setBackgroundFit", "cover", 1.0);
        }
        catch { }
    }

    /// <summary>
    /// Copies a picked wallpaper into the launcher folder under a theme-unique
    /// name so multiple themes can each keep their own background.
    /// </summary>
    public async Task<string?> StashBackgroundAsync(string sourcePath, string themeId)
    {
        try
        {
            var fi = new FileInfo(sourcePath);
            if (!fi.Exists || fi.Length > 20 * 1024 * 1024) return null;
            var dest = Path.Combine(LauncherUtilityService.LauncherRoot,
                $"theme-bg-{themeId}{fi.Extension.ToLowerInvariant()}");
            await Task.Run(() => File.Copy(sourcePath, dest, true));
            return dest;
        }
        catch { return null; }
    }
}
