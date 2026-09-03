/**
 * Localization — 1:1 port of `Services/LocalizationService.cs`.
 *
 * The original resolves strings client-side from a static table keyed by
 * `Settings.Language`, so this stays in the frontend rather than becoming a
 * Tauri command: it is a pure lookup with no I/O.
 *
 * Lookup order matches the C# `T(string key)` exactly: the active language,
 * then English, then the key itself as a last resort (so a missing string is
 * visible rather than blank).
 */

export interface LanguageOption {
  code: string
  label: string
}

/** Port of `LocalizationService.Languages`. */
export const LANGUAGES: LanguageOption[] = [
  { code: 'en', label: 'English' },
  { code: 'es', label: 'Español' },
  { code: 'de', label: 'Deutsch' },
  { code: 'fr', label: 'Français' },
]

type Table = Record<string, Record<string, string>>

/** Verbatim copy of the C# `_table`. Do not "tidy" these strings. */
const TABLE: Table = {
  en: {
    'nav.launch': 'Launch',
    'nav.settings': 'Settings',
    'nav.clients': 'Clients',
    'nav.addons': 'Addons',
    'nav.servers': 'Servers',
    'nav.mcversions': 'MC Versions',
    'nav.launchers': 'Launchers',
    'nav.mods': 'Mods',
    'nav.versions': 'Versions',
    'nav.profile': 'Profile',
    'nav.photos': 'Photos',
    'nav.worlds': 'Worlds',
    'nav.packs': 'Packs',
    'nav.backups': 'Backups',
    'nav.instances': 'Instances',
    'nav.credits': 'Credits',
    'nav.downloads': 'Downloads',
    'settings.language': 'Language',
  },
  es: {
    'nav.launch': 'Iniciar',
    'nav.settings': 'Ajustes',
    'nav.clients': 'Clientes',
    'nav.addons': 'Complementos',
    'nav.servers': 'Servidores',
    'nav.mcversions': 'Versiones de MC',
    'nav.launchers': 'Launchers',
    'nav.mods': 'Mods',
    'nav.versions': 'Versiones',
    'nav.profile': 'Perfil',
    'nav.photos': 'Fotos',
    'nav.worlds': 'Mundos',
    'nav.packs': 'Paquetes',
    'nav.backups': 'Copias de seguridad',
    'nav.instances': 'Instancias',
    'nav.credits': 'Créditos',
    'nav.downloads': 'Descargas',
    'settings.language': 'Idioma',
  },
  de: {
    'nav.launch': 'Starten',
    'nav.settings': 'Einstellungen',
    'nav.clients': 'Clients',
    'nav.addons': 'Erweiterungen',
    'nav.servers': 'Server',
    'nav.mcversions': 'MC-Versionen',
    'nav.launchers': 'Launcher',
    'nav.mods': 'Mods',
    'nav.versions': 'Versionen',
    'nav.profile': 'Profil',
    'nav.photos': 'Fotos',
    'nav.worlds': 'Welten',
    'nav.packs': 'Pakete',
    'nav.backups': 'Sicherungen',
    'nav.instances': 'Instanzen',
    'nav.credits': 'Danksagungen',
    'nav.downloads': 'Downloads',
    'settings.language': 'Sprache',
  },
  fr: {
    'nav.launch': 'Lancer',
    'nav.settings': 'Paramètres',
    'nav.clients': 'Clients',
    'nav.addons': 'Extensions',
    'nav.servers': 'Serveurs',
    'nav.mcversions': 'Versions MC',
    'nav.launchers': 'Launchers',
    'nav.mods': 'Mods',
    'nav.versions': 'Versions',
    'nav.profile': 'Profil',
    'nav.photos': 'Photos',
    'nav.worlds': 'Mondes',
    'nav.packs': 'Packs',
    'nav.backups': 'Sauvegardes',
    'nav.instances': 'Instances',
    'nav.credits': 'Crédits',
    'nav.downloads': 'Téléchargements',
    'settings.language': 'Langue',
  },
}

/**
 * Port of `LocalizationService.T`. `lang` is `Settings.Language`.
 */
export function t(key: string, lang: string | undefined): string {
  const active = lang ? TABLE[lang] : undefined
  if (active && key in active) return active[key]
  const en = TABLE.en
  if (key in en) return en[key]
  return key
}
