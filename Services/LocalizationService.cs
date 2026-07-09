using System.Collections.Generic;

namespace GlacierLauncher.Services;

/// <summary>
/// Small string-table localization: a scoped foundation covering the home
/// screen's main action dock and the language picker itself, not an
/// exhaustive translation of every string in a 4000+ line UI. Extending
/// coverage means adding more T("...") calls at call sites and filling in
/// the matching key in every language's dictionary below.
/// </summary>
public class LocalizationService
{
    private readonly SettingsService _settings;

    public LocalizationService(SettingsService settings) => _settings = settings;

    public sealed record LanguageOption(string Code, string Label);

    public static readonly LanguageOption[] Languages =
    {
        new("en", "English"),
        new("es", "Español"),
        new("de", "Deutsch"),
        new("fr", "Français"),
    };

    /// <summary>Translates a key for the active language, falling back to English then the key itself.</summary>
    public string T(string key)
    {
        var lang = _settings.Settings.Language;
        if (_table.TryGetValue(lang, out var dict) && dict.TryGetValue(key, out var v)) return v;
        if (_table.TryGetValue("en", out var en) && en.TryGetValue(key, out var ev)) return ev;
        return key;
    }

    private static readonly Dictionary<string, Dictionary<string, string>> _table = new()
    {
        ["en"] = new()
        {
            ["nav.launch"]     = "Launch",
            ["nav.settings"]   = "Settings",
            ["nav.clients"]    = "Clients",
            ["nav.addons"]     = "Addons",
            ["nav.servers"]    = "Servers",
            ["nav.mcversions"] = "MC Versions",
            ["nav.launchers"]  = "Launchers",
            ["nav.mods"]       = "Mods",
            ["nav.versions"]   = "Versions",
            ["nav.profile"]    = "Profile",
            ["nav.photos"]     = "Photos",
            ["nav.worlds"]     = "Worlds",
            ["nav.packs"]      = "Packs",
            ["nav.backups"]    = "Backups",
            ["nav.instances"]  = "Instances",
            ["nav.credits"]    = "Credits",
            ["nav.downloads"]  = "Downloads",
            ["settings.language"] = "Language",
        },
        ["es"] = new()
        {
            ["nav.launch"]     = "Iniciar",
            ["nav.settings"]   = "Ajustes",
            ["nav.clients"]    = "Clientes",
            ["nav.addons"]     = "Complementos",
            ["nav.servers"]    = "Servidores",
            ["nav.mcversions"] = "Versiones de MC",
            ["nav.launchers"]  = "Launchers",
            ["nav.mods"]       = "Mods",
            ["nav.versions"]   = "Versiones",
            ["nav.profile"]    = "Perfil",
            ["nav.photos"]     = "Fotos",
            ["nav.worlds"]     = "Mundos",
            ["nav.packs"]      = "Paquetes",
            ["nav.backups"]    = "Copias de seguridad",
            ["nav.instances"]  = "Instancias",
            ["nav.credits"]    = "Créditos",
            ["nav.downloads"]  = "Descargas",
            ["settings.language"] = "Idioma",
        },
        ["de"] = new()
        {
            ["nav.launch"]     = "Starten",
            ["nav.settings"]   = "Einstellungen",
            ["nav.clients"]    = "Clients",
            ["nav.addons"]     = "Erweiterungen",
            ["nav.servers"]    = "Server",
            ["nav.mcversions"] = "MC-Versionen",
            ["nav.launchers"]  = "Launcher",
            ["nav.mods"]       = "Mods",
            ["nav.versions"]   = "Versionen",
            ["nav.profile"]    = "Profil",
            ["nav.photos"]     = "Fotos",
            ["nav.worlds"]     = "Welten",
            ["nav.packs"]      = "Pakete",
            ["nav.backups"]    = "Sicherungen",
            ["nav.instances"]  = "Instanzen",
            ["nav.credits"]    = "Danksagungen",
            ["nav.downloads"]  = "Downloads",
            ["settings.language"] = "Sprache",
        },
        ["fr"] = new()
        {
            ["nav.launch"]     = "Lancer",
            ["nav.settings"]   = "Paramètres",
            ["nav.clients"]    = "Clients",
            ["nav.addons"]     = "Extensions",
            ["nav.servers"]    = "Serveurs",
            ["nav.mcversions"] = "Versions MC",
            ["nav.launchers"]  = "Launchers",
            ["nav.mods"]       = "Mods",
            ["nav.versions"]   = "Versions",
            ["nav.profile"]    = "Profil",
            ["nav.photos"]     = "Photos",
            ["nav.worlds"]     = "Mondes",
            ["nav.packs"]      = "Packs",
            ["nav.backups"]    = "Sauvegardes",
            ["nav.instances"]  = "Instances",
            ["nav.credits"]    = "Crédits",
            ["nav.downloads"]  = "Téléchargements",
            ["settings.language"] = "Langue",
        },
    };
}
