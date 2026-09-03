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

  WindowWidth: number
  WindowHeight: number
  RememberWindowSize: boolean
  MinimizeToTray: boolean

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

  Language: string

  LastDismissedAnnouncementId: string
}

export interface JavaInstance {
  id: string
  name: string
  version_id: string
  directory: string
  min_ram_mb: number
  max_ram_mb: number
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
}
