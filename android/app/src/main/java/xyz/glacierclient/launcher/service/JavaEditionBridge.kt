package xyz.glacierclient.launcher.service

import android.content.Context
import android.content.Intent
import android.os.Environment
import java.io.File

/**
 * Bridges our shell app to the vendored PojavLauncher submodule
 * (android/pojavlauncher), built directly into this app as a library module
 * (see settings.gradle.kts and scripts/rebrand-pojav.sh) rather than a
 * separate installable APK — one process, one app, no "install the Java
 * Edition companion" step. net.kdt.pojavlaunch.MainActivity is now just
 * another Activity in this same package, launched by explicit class
 * reference like any other in-process Activity.
 *
 * Mods/Glacier-client jars are shared through Pojav's own external storage
 * layout (getExternalStorageDirectory()/games/PojavLauncher/.minecraft/mods)
 * rather than app-private storage, since that directory is what Pojav's
 * Tools.java actually reads from — see android/pojavlauncher/.../Tools.java
 * DIR_GAME_HOME.
 */
object JavaEditionBridge {

    // Pojav's MainActivity IS the real JVM/GLFW game-render surface (its
    // manifest LAUNCHER activity, TestStorageActivity, had its own
    // intent-filter stripped by rebrand-pojav.sh so it doesn't show as a
    // second home-screen icon; MainActivity is what actually runs
    // runCraft() and boots straight into gameplay once its render surface is
    // ready). It reads net.kdt.pojavlaunch.MainActivity.INTENT_MINECRAFT_VERSION
    // ("intent_version") to pick which installed version to launch, falling
    // back to Pojav's own last-used profile when omitted — so passing the
    // version the user picked in *our* Java Versions panel makes launching
    // from our own UI a real, direct, native launch instead of just
    // reopening Pojav's own home screen. This only works once Pojav's own
    // one-time setup (JRE download, a saved launcher profile) has happened
    // at least once — a completely fresh install may still land in Pojav's
    // own setup screens first, same as any first run of PojavLauncher itself.
    fun launch(context: Context, versionId: String? = null): Boolean {
        val intent = Intent(context, net.kdt.pojavlaunch.MainActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            if (!versionId.isNullOrBlank()) putExtra(net.kdt.pojavlaunch.MainActivity.INTENT_MINECRAFT_VERSION, versionId)
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
