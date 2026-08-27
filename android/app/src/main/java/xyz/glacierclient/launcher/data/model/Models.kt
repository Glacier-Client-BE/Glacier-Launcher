package xyz.glacierclient.launcher.data.model

import kotlinx.serialization.Serializable

/** Mirrors GlacierLauncher.Models.ClientType / ClientInfo (Windows DLL clients). */
enum class ClientType { Latite, Flarial, OderSo }

data class ClientInfo(
    val type: ClientType,
    val name: String,
    val version: String? = null,
    val isDownloaded: Boolean = false,
    val isDownloading: Boolean = false,
    val isUpToDate: Boolean = false,
    val errorMessage: String? = null,
    val progress: Double = 0.0,
)

/** Mirrors GlacierLauncher.Models.GlacierClientVersion / GlacierManifest. */
@Serializable
data class GlacierClientVersion(
    val id: String = "",
    val name: String = "",
    val releaseDate: String = "",
    val tag: String = "",
    val url: String = "",
    val sha256: String = "",
    val fabric: Boolean = false,
    val forge: Boolean = false,
    val javaVersion: Int = 0,
    val changelog: String = "",
) {
    val loaderLabel: String get() = if (fabric) "Fabric" else if (forge) "Forge" else "Unknown"
    val localFileName: String get() = "Glacier-$id.jar"
}

@Serializable
data class GlacierLauncherMeta(
    val version: String = "",
    val url: String = "",
    val sha256: String = "",
)

@Serializable
data class GlacierManifest(
    val schemaVersion: Int = 0,
    val latestRelease: String = "",
    val releaseDate: String = "",
    val versions: List<GlacierClientVersion> = emptyList(),
    val launcher: GlacierLauncherMeta? = null,
)

/** Mirrors GlacierLauncher.Models.AppNotification. */
data class AppNotification(
    val id: String,
    val title: String,
    val message: String,
    val level: String = "info", // info | success | warning | error
    val timestamp: Long = System.currentTimeMillis(),
)

/** Mirrors GlacierLauncher.Models.BedrockWorld (subset relevant to Android). */
data class BedrockWorld(
    val name: String,
    val folderName: String,
    val sizeBytes: Long,
    val lastPlayedEpochMs: Long,
)

/** Mirrors GlacierLauncher.Models.BedrockBackup. */
data class BedrockBackup(
    val worldName: String,
    val fileName: String,
    val sizeBytes: Long,
    val createdEpochMs: Long,
)

/** Mirrors GlacierLauncher.Models.BedrockPack (behavior/resource packs). */
data class BedrockPack(
    val name: String,
    val description: String,
    val isBehaviorPack: Boolean,
    val version: String = "",
)

/** Mirrors GlacierLauncher.Models.SavedServer. */
data class SavedServer(
    val name: String,
    val address: String,
    val port: Int = 19132,
)
