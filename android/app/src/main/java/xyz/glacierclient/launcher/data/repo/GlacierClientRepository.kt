package xyz.glacierclient.launcher.data.repo

import android.content.Context
import io.ktor.client.call.body
import io.ktor.client.request.get
import io.ktor.client.request.header
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.io.File
import java.security.MessageDigest
import xyz.glacierclient.launcher.data.model.GlacierManifest
import xyz.glacierclient.launcher.data.remote.HttpClientFactory

/**
 * Mirrors GlacierLauncher.Services.GlacierClientService: fetches the
 * versions.json manifest from the same CDN and installs client jars into the
 * app's private storage (the Android sandbox equivalent of ~/.glacier).
 */
class GlacierClientRepository(context: Context) {
    private val manifestUrl = "https://cdn.glacierclient.xyz/versions.json"
    private val client = HttpClientFactory.shared
    private val glacierDir = File(context.filesDir, ".glacier/versions").apply { mkdirs() }

    private val _manifest = MutableStateFlow<GlacierManifest?>(null)
    val manifest = _manifest.asStateFlow()

    var lastError: String? = null
        private set

    suspend fun refreshManifest(): GlacierManifest? {
        lastError = null
        return try {
            val result: GlacierManifest = client.get(manifestUrl).body()
            _manifest.value = result
            result
        } catch (e: Exception) {
            lastError = "Failed to fetch manifest: ${e.message}"
            null
        }
    }

    fun jarFile(versionId: String) = File(File(glacierDir, versionId), "Glacier-$versionId.jar")

    fun isInstalled(versionId: String) = jarFile(versionId).exists()

    suspend fun install(
        versionId: String,
        url: String,
        expectedSha256: String,
        onProgress: (Double) -> Unit = {},
    ) {
        val dest = jarFile(versionId)
        dest.parentFile?.mkdirs()
        val tmp = File(dest.parentFile, "${dest.name}.part")

        val response = client.get(url) { header("Accept", "application/octet-stream") }
        val bytes = response.body<ByteArray>()
        tmp.writeBytes(bytes)
        onProgress(1.0)

        if (expectedSha256.isNotBlank()) {
            val digest = MessageDigest.getInstance("SHA-256").digest(tmp.readBytes())
            val hex = digest.joinToString("") { "%02x".format(it) }
            if (!hex.equals(expectedSha256, ignoreCase = true)) {
                tmp.delete()
                throw IllegalStateException("SHA-256 mismatch for $versionId")
            }
        }
        tmp.copyTo(dest, overwrite = true)
        tmp.delete()

        // Also stage into the Java Edition companion app's shared mods folder so it
        // loads on the next Pojav launch — see JavaEditionBridge for why that's a
        // shared-storage copy rather than a cross-app IPC call.
        xyz.glacierclient.launcher.service.JavaEditionBridge.installModJar(dest)
    }

    fun uninstall(versionId: String) {
        jarFile(versionId).delete()
    }

    fun installedSizeBytes(): Long =
        glacierDir.walkTopDown().filter { it.isFile }.sumOf { it.length() }
}
