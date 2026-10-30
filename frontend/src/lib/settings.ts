// Hand-written mirror of the Rust `gl_settings::LauncherSettings` struct.
// TODO(Phase 0 follow-up): generate this via tauri-specta instead of
// hand-maintaining it once the command surface grows past a handful of
// commands.
export interface SavedServer {
  Name: string
  Address: string
  Port: number
  Icon: string
  IconColor: string
}

export interface JavaAccount {
  Id: string
  Username: string
  Uuid: string
  Gamertag: string
  Xuid: string
  GamerPictureUrl: string
  MinecraftAccessToken: string
  MinecraftAccessTokenExpiry: string
  MinecraftSkinUrl: string
  RefreshToken: string
  LastUsedAt: string
}

export interface LauncherSettings {
  Edition: string
  SelectedClient: string
  DiscordRichPresence: boolean
  LastUsedVersion: string
  Username: string
  UserHandle: string
  CloseAfterLaunch: boolean
  AutoInject: boolean
  InjectionDelayMs: number

  AccentColor: string
  ThemePreset: string
  BlurIntensity: number
  BackgroundOpacity: number
  CustomBackgroundPath: string
  CompactMode: boolean
  AnimationsEnabled: boolean
  AnimationSpeed: number
  ActiveThemeId: string
  FontFamily: string
  UiScalePct: number
  BackgroundFit: string

  PanelSide: 'left' | 'right'
  PanelWidthPx: number

  WindowWidth: number
  WindowHeight: number
  RememberWindowSize: boolean
  MinimizeToTray: boolean
  AutoClearDownloadsAfterDays: number
  TooltipDelayMs: number
  MaxConcurrentDownloads: number
  MutedNotificationCategories: string[]
  DesktopNotificationsEnabled: boolean
  PinnedPanels: string[]
  QuickLaunchHotkey: string
  LastSeenChangelogVersion: string

  ShowRecentlyLaunched: boolean
  RecentlyLaunched: string[]

  TotalPlaytimeSeconds: number
  LastPlayed: string

  PinnedVersions: string[]

  VersionSortMode: string
  ShowOnlyDownloaded: boolean

  ActiveVanillaVersion: string

  JavaActiveVersion: string
  JavaLastUsedVersion: string
  JavaActiveInstanceId: string

  JavaUseDotMinecraft: boolean
  JavaMinecraftDir: string

  JavaRuntimePath: string
  JavaMinRamMb: number
  JavaMaxRamMb: number

  JavaCustomJvmArgs: string
  JavaBackupSavesBeforeLaunch: boolean
  JavaFullscreen: boolean
  JavaWindowWidth: number
  JavaWindowHeight: number
  JavaUseCustomResolution: boolean
  JavaServerAddress: string
  JavaServerPort: number

  ShowLaunchConsole: boolean

  CollapseInstanceGroups: boolean

  JavaShowSnapshots: boolean
  JavaShowHistorical: boolean

  JavaUsername: string
  JavaUuid: string
  JavaAccessToken: string

  JavaOfflineMode: boolean
  JavaOfflineUsername: string
  JavaAccessTokenExpiry: string
  JavaSkinUrl: string

  JavaSkinSlimModel: boolean

  SkinViewerMode: string
  SkinViewerModel: string
  SkinViewerCapeMode: string

  SkinChangedTicks: number

  LastAppliedSkinPath: string
  ActiveJavaAccountId: string
  JavaAccounts: JavaAccount[]

  CustomDllPath: string

  DefaultCustomClientId: string

  AccelerateLoadingMs: number

  PreventGameSuspend: boolean

  VerifyInjection: boolean

  CurseForgeApiKey: string
  GeminiApiKey: string

  CheckUpdatesOnStartup: boolean
  SkippedLauncherVersion: string
  LastUpdateCheck: string

  XboxGamertag: string
  XboxXuid: string
  XboxGamerPictureUrl: string
  XboxGamerscore: string
  XboxAccountTier: string
  XboxBio: string

  XboxLiveRefreshToken: string

  DiscordLoggedIn: boolean
  DiscordUserId: string
  DiscordUsername: string
  DiscordAvatar: string
  DiscordToken: string

  ProfileDisplayMode: string

  SavedServers: SavedServer[]

  BedrockActiveInstanceId: string

  OnboardingCompleted: boolean

  AccessibilityMode: boolean

  Language: string

  LastDismissedAnnouncementId: string

  KeyboardShortcuts: string
  OnboardingChecklistDismissed: boolean

  StarredModIds: string[]
}

export type ModLoaderKind = 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge'

export interface JavaInstance {
  id: string
  name: string
  version_id: string
  directory: string
  min_ram_mb: number
  max_ram_mb: number
  accent_color?: string | null
  extra_jvm_args?: string[]
  jvm_args_preset?: string | null
  group?: string | null
  loader?: ModLoaderKind | null
  loader_version?: string | null
  loader_profile_id?: string | null
  playtime_seconds?: number
  sync_pack_ids?: string[]
  pinned?: boolean
}

/** Mirrors `gl_java::InstanceSettings` (Rust). Every field is an override —
 * `undefined`/`null` means "inherit from the global default". */
export interface InstanceSettings {
  java_path?: string | null
  memory?: [number, number] | null
  resolution?: [number, number] | null
  use_custom_resolution?: boolean | null
  fullscreen?: boolean | null
  extra_game_args?: string[]
  custom_jvm_args?: string | null
  quick_play?: string | null
}

/** Mirrors `gl_java::EffectiveLaunchConfig` (Rust) — the resolved
 * global-defaults-layered-with-overrides view used by settings UI. */
export interface EffectiveLaunchConfig {
  max_ram_mb: number
  min_ram_mb: number
  custom_jvm_args: string
  extra_game_args: string[]
  fullscreen: boolean
  use_custom_resolution: boolean
  window_width: number
  window_height: number
  server_address: string
  server_port: number
  java_path?: string | null
  quick_play?: string | null
  /** Field names resolved from per-instance overrides rather than global defaults. */
  overridden: string[]
}

export interface JvmPreset {
  id: string
  label: string
  description: string
  args: string[]
}

export interface PrelaunchWarning {
  level: string
  message: string
}

export interface JavaVersion {
  id: string
  type: string
  url: string
  sha1: string
  release_time: string
  time: string
  compliance_level: number
  is_installed: boolean
  has_jar: boolean
  is_active: boolean
}

export interface BedrockInstance {
  id: string
  name: string
  version_tag: string
  directory: string
  accent_color?: string | null
}
