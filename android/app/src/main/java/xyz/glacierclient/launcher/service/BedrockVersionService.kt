package xyz.glacierclient.launcher.service

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import androidx.core.content.FileProvider
import org.json.JSONArray
import org.json.JSONObject
import java.io.DataOutputStream
import java.io.File

/**
 * Android-only alternative to desktop's Windows-Store-based Bedrock version
 * switching (VanillaVersionService.cs's AppX download/register/rollback) —
 * there is no Android equivalent of that API, and no legitimate one exists
 * (an unofficial Google Play client library was considered and rejected:
 * it needs a Google account master/AAS token, a far bigger blast radius
 * than what it buys — a single side-loaded APK's install still goes through
 * Android's own PackageInstaller, same as any other app).
 *
 * Instead: the user supplies their own Bedrock APKs (already downloaded
 * from wherever they'd otherwise get one), this keeps them in one place,
 * and installing one is a real android.content.pm.PackageInstaller flow —
 * exactly the same mechanism LauncherUpdateService.kt already uses for
 * self-updating this app. Two real Android platform constraints apply and
 * are surfaced honestly rather than worked around:
 *
 *  - A normal install Intent can only replace an installed app with an
 *    equal-or-higher versionCode; PackageInstaller itself rejects a lower
 *    one with INSTALL_FAILED_VERSION_DOWNGRADE. That flag can only be
 *    bypassed by whoever holds INSTALL_ALLOW_DOWNGRADE, which is a
 *    privileged/shell-only permission — this app doesn't hold it, so a real
 *    downgrade only works with root (adb/shell effectively runs as a
 *    privileged installer), which is why the root path below passes `-d`
 *    and the non-root path is left to Android's own installer UI to accept
 *    or reject.
 *  - A package name that doesn't match the currently-installed Bedrock
 *    (com.mojang.minecraftpe) installs as a SEPARATE app rather than
 *    replacing it — surfaced in the listing so the user isn't surprised.
 */
object BedrockVersionService {

    private const val BEDROCK_PACKAGE = "com.mojang.minecraftpe"

    private fun buildsDir(context: Context): File =
        File(GlacierStorage.preferredRoot(context), "bedrock_builds").apply { mkdirs() }

    /** Copies a user-picked APK into the builds folder and returns its parsed metadata, or null if unreadable/not an APK. */
    fun importApk(context: Context, source: File, displayName: String): String? {
        val info = context.packageManager.getPackageArchiveInfo(source.absolutePath, 0) ?: return null
        val safeName = displayName.takeIf { it.endsWith(".apk", ignoreCase = true) } ?: "$displayName.apk"
        val dest = File(buildsDir(context), safeName)
        return try {
            source.copyTo(dest, overwrite = true)
            buildToJson(context, dest, info).toString()
        } catch (e: Exception) {
            dest.delete()
            null
        }
    }

    fun listBuilds(context: Context): String {
        val out = JSONArray()
        for (f in buildsDir(context).listFiles { it -> it.extension.equals("apk", ignoreCase = true) }.orEmpty()) {
            val info = context.packageManager.getPackageArchiveInfo(f.absolutePath, 0) ?: continue
            out.put(buildToJson(context, f, info))
        }
        return out.toString()
    }

    private fun buildToJson(context: Context, file: File, info: android.content.pm.PackageInfo) = JSONObject().apply {
        put("fileName", file.name)
        put("packageName", info.packageName)
        put("versionName", info.versionName ?: "unknown")
        @Suppress("DEPRECATION")
        put("versionCode", if (android.os.Build.VERSION.SDK_INT >= 28) info.longVersionCode else info.versionCode.toLong())
        put("matchesInstalledPackage", info.packageName == BEDROCK_PACKAGE)
        put("size", file.length())
    }

    fun deleteBuild(context: Context, fileName: String): Boolean =
        File(buildsDir(context), fileName).let { it.isFile && it.delete() }

    /**
     * Installs [fileName] via the real PackageInstaller. Root, when
     * available, additionally passes `-d` so a genuine downgrade can
     * succeed (see class doc) — without root this simply opens Android's
     * own install confirmation, which is honest about rejecting a
     * downgrade rather than pretending to force one.
     */
    fun install(context: Context, fileName: String): InstallResult {
        val apk = File(buildsDir(context), fileName)
        if (!apk.isFile) return InstallResult(false, "That build is no longer on disk.")

        if (ClientInjectionService.isRootAvailable()) {
            return try {
                val process = Runtime.getRuntime().exec("su")
                DataOutputStream(process.outputStream).use { out ->
                    out.writeBytes("pm install -r -d \"${apk.absolutePath}\"\n")
                    out.writeBytes("exit\n")
                    out.flush()
                }
                if (process.waitFor() == 0) {
                    InstallResult(true, "Installed via root (downgrade allowed).")
                } else {
                    InstallResult(false, "Root install command failed — see logcat for pm's own output.")
                }
            } catch (e: Exception) {
                InstallResult(false, "Root install failed: ${e.message}")
            }
        }

        return try {
            val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", apk)
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            context.startActivity(intent)
            InstallResult(true, "Opened Android's installer — a downgrade will be rejected there without root.")
        } catch (e: Exception) {
            InstallResult(false, "Couldn't open the installer: ${e.message}")
        }
    }

    data class InstallResult(val success: Boolean, val message: String)
}
