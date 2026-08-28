package xyz.glacierclient.launcher.service

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Environment
import androidx.core.content.FileProvider
import java.io.File

/**
 * Bridges our shell app to the rebranded PojavLauncher build (android/pojavlauncher,
 * applicationId xyz.glacierclient.launcher.java) — our "own version of Pojav" for
 * Java Edition, built as a companion APK rather than merged line-for-line into this
 * module (a true single-APK merge of two independent Android apps, one of them
 * PojavLauncher's own native-JVM/JNI runtime, is a much larger follow-on effort).
 *
 * Mods/Glacier-client jars are shared through Pojav's own external storage layout
 * (getExternalStorageDirectory()/games/PojavLauncher/.minecraft/mods) rather than
 * app-private storage, since that directory is what Pojav's Tools.java actually
 * reads from — see android/pojavlauncher/.../Tools.java DIR_GAME_HOME.
 */
object JavaEditionBridge {
    const val JAVA_EDITION_PACKAGE = "xyz.glacierclient.launcher.java"
    private const val LAUNCH_ACTIVITY = "net.kdt.pojavlaunch.MainActivity"

    private const val BUNDLED_ASSET_PATH = "companion/java-edition.apk"

    /** True when this build actually bundled the companion APK as an asset. */
    fun hasBundledInstaller(context: Context): Boolean = try {
        context.assets.open(BUNDLED_ASSET_PATH).use { }
        true
    } catch (e: Exception) {
        false
    }

    /**
     * Extracts the bundled companion APK (assets/companion/java-edition.apk,
     * populated at CI build time from the rebranded PojavLauncher submodule's
     * own release APK — see android-release.yml) into the app's cache dir and
     * hands it to the system package installer via a FileProvider URI. This
     * is a single-APK *distribution*, not a process merge: Pojav still runs
     * as its own installed app/process (it owns a full native-JVM/JNI Minecraft
     * runtime), but the user only ever downloads one APK from us. Android's
     * own "install unknown apps" confirmation still appears — that's a
     * system security control this app has no way to bypass without root.
     */
    fun installBundled(context: Context): Boolean {
        if (!hasBundledInstaller(context)) return false
        return try {
            val cacheDir = File(context.cacheDir, "companion").apply { mkdirs() }
            val dest = File(cacheDir, "java-edition.apk")
            context.assets.open(BUNDLED_ASSET_PATH).use { input ->
                dest.outputStream().use { output -> input.copyTo(output) }
            }
            val uri: Uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", dest)
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            context.startActivity(intent)
            true
        } catch (e: Exception) {
            false
        }
    }

    fun isInstalled(context: Context): Boolean = try {
        context.packageManager.getPackageInfo(JAVA_EDITION_PACKAGE, 0)
        true
    } catch (e: Exception) {
        false
    }

    // Pojav's MainActivity IS the real JVM/GLFW game-render surface (its
    // manifest LAUNCHER activity is TestStorageActivity -> LauncherActivity,
    // a setup/version-picker flow; MainActivity is what actually runs
    // runCraft() and boots straight into gameplay once its render surface is
    // ready). It reads net.kdt.pojavlaunch.MainActivity.INTENT_MINECRAFT_VERSION
    // ("intent_version") to pick which installed version to launch, falling
    // back to Pojav's own last-used profile when omitted — so passing the
    // version the user picked in *our* Java Versions panel makes launching
    // from our own UI behave like a real, direct, native launch instead of
    // just reopening Pojav's separate home screen. This only works once
    // Pojav's own one-time setup (JRE download, a saved launcher profile)
    // has happened at least once — a completely fresh install may still
    // land in Pojav's own setup screens first, same as any first run of
    // PojavLauncher itself.
    fun launch(context: Context, versionId: String? = null): Boolean {
        val intent = Intent().apply {
            setClassName(JAVA_EDITION_PACKAGE, LAUNCH_ACTIVITY)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            if (!versionId.isNullOrBlank()) putExtra("intent_version", versionId)
        }
        return try {
            context.startActivity(intent)
            true
        } catch (e: Exception) {
            false
        }
    }

    private fun pojavModsDir(): File =
        File(Environment.getExternalStorageDirectory(), "games/PojavLauncher/.minecraft/mods")

    /** Copies an installed Glacier client / mod jar into Pojav's shared mods folder. */
    fun installModJar(sourceJar: File): Boolean = try {
        val dest = pojavModsDir().apply { mkdirs() }
        sourceJar.copyTo(File(dest, sourceJar.name), overwrite = true)
        true
    } catch (e: Exception) {
        false
    }

    fun listScreenshots(): List<File> {
        // Standard Java Edition path: Minecraft itself (not Pojav) writes here.
        val dir = File(Environment.getExternalStorageDirectory(), "games/PojavLauncher/.minecraft/screenshots")
        return dir.listFiles { f -> f.extension.equals("png", ignoreCase = true) }
            ?.sortedByDescending { it.lastModified() }
            ?: emptyList()
    }
}
