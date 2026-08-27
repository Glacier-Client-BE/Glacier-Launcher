package xyz.glacierclient.launcher.data.model

import kotlinx.serialization.Serializable

/**
 * Mirrors GlacierLauncher.Models.LauncherSettings, trimmed to the fields that
 * are meaningful on Android — Java-Edition-desktop-path settings and Windows-only
 * knobs (window size, tray, custom DLL path) are dropped rather than faked.
 */
@Serializable
data class LauncherSettings(
    val selectedClient: String = "Latite Client",
    val discordRichPresence: Boolean = true,
    val lastUsedVersion: String = "",
    val username: String = "",
    val autoInject: Boolean = false,
    val injectionDelayMs: Int = 2000,

    val accentColor: String = "#7289da",
    val themePreset: String = "dark",
    val backgroundOpacity: Double = 0.80,
    val compactMode: Boolean = false,
    val animationsEnabled: Boolean = true,

    val showRecentlyLaunched: Boolean = true,
    val recentlyLaunched: List<String> = emptyList(),

    val totalPlaytimeSeconds: Long = 0,
    val lastPlayed: String = "",

    val pinnedVersions: List<String> = emptyList(),
    val versionSortMode: String = "newest",
    val showOnlyDownloaded: Boolean = false,

    val checkUpdatesOnStartup: Boolean = true,
    val skippedLauncherVersion: String = "",

    val xboxGamertag: String = "",
    val xboxXuid: String = "",
    val xboxGamerPictureUrl: String = "",

    val savedServers: List<SavedServer> = emptyList(),
    val onboardingCompleted: Boolean = false,
    val language: String = "en",
    val lastDismissedAnnouncementId: String = "",
)
