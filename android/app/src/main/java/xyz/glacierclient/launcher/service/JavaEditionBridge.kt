package xyz.glacierclient.launcher.service

import android.content.Context
import android.content.Intent
import android.os.Environment
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

    fun isInstalled(context: Context): Boolean = try {
        context.packageManager.getPackageInfo(JAVA_EDITION_PACKAGE, 0)
        true
    } catch (e: Exception) {
        false
    }

    fun launch(context: Context): Boolean {
        val intent = Intent().apply {
            setClassName(JAVA_EDITION_PACKAGE, LAUNCH_ACTIVITY)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
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
