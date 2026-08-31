package xyz.glacierclient.launcher.service

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.Settings
import java.io.File

/**
 * Single source of truth for where Glacier keeps everything on disk.
 *
 * Pojav's own `Tools.getPojavStorageRoot` returns `getExternalFilesDir(null)`
 * on SDK 29+ — i.e. the app's private
 * `Android/data/xyz.glacierclient.launcher/files` sandbox. That is exactly
 * the directory Android 11+ stops file managers from browsing, so every
 * Java Edition world, mod and config ended up somewhere the user could not
 * reach, under a PojavLauncher-shaped name on older devices.
 *
 * Glacier instead keeps it all in one visible, launcher-branded folder:
 *
 *     /storage/emulated/0/games/Glacier
 *       .minecraft/     worlds, mods, resourcepacks, screenshots…
 *       instances/      per-instance game directories (JavaInstanceService)
 *       exports/        exported modpacks
 *       config/         launcher settings mirror (see [configDir])
 *
 * The catch is that writing outside the sandbox needs All Files Access on
 * Android 11+, which the user grants in system settings and can decline. So
 * [preferredRoot] falls back to the old app-private directory when the
 * permission isn't held: choosing the shared folder unconditionally would
 * leave the storage root unwritable, and Pojav treats that as "no storage",
 * which breaks Java Edition outright. Better a working launcher in a hidden
 * folder than a branded folder and a broken one.
 *
 * `scripts/rebrand-pojav.sh` patches `getPojavStorageRoot` with the same
 * decision, since the vendored library cannot reference this class — it is
 * a dependency of :app, not the other way round.
 */
object GlacierStorage {

    /** Visible root, relative to primary external storage. */
    const val SHARED_RELATIVE_PATH = "games/Glacier"

    /** Where Pojav put things before the redirect, and what we migrate from. */
    private const val LEGACY_SHARED_PATH = "games/PojavLauncher"

    private const val PREFS = "glacier_settings"
    private const val KEY_MIGRATED = "glacier_storage_migrated"

    fun sharedRoot(): File = File(Environment.getExternalStorageDirectory(), SHARED_RELATIVE_PATH)

    /**
     * Whether we may write outside the app sandbox. Below API 30 the legacy
     * WRITE_EXTERNAL_STORAGE model applies and the manifest already declares
     * it; from API 30 this needs All Files Access.
     */
    fun canUseSharedStorage(): Boolean =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) Environment.isExternalStorageManager() else true

    /**
     * The storage root actually in use. Mirrors the patched
     * `Tools.getPojavStorageRoot` exactly — if these two disagree, the
     * launcher and the game runtime read different directories.
     */
    fun preferredRoot(context: Context): File {
        if (canUseSharedStorage()) {
            val shared = sharedRoot()
            if (shared.isDirectory || shared.mkdirs()) return shared
        }
        return context.getExternalFilesDir(null) ?: File(context.filesDir, "storage")
    }

    /** Launcher config, kept beside the game data rather than only in SharedPreferences. */
    fun configDir(context: Context): File = File(preferredRoot(context), "config").apply { mkdirs() }

    private fun settingsFile(context: Context) = File(configDir(context), "settings.json")

    /**
     * Mirrors the settings blob into the Glacier folder. SharedPreferences
     * stays authoritative at runtime; this copy exists so settings are
     * visible, backup-able, and — via [readMirroredSettings] — survive an
     * uninstall or a "clear data", which wipes SharedPreferences but not
     * shared storage.
     */
    fun writeMirroredSettings(context: Context, json: String) {
        runCatching { settingsFile(context).writeText(json) }
    }

    /** The mirrored settings, or null when absent/unreadable. */
    fun readMirroredSettings(context: Context): String? =
        runCatching { settingsFile(context).takeIf { it.isFile }?.readText() }
            .getOrNull()
            ?.takeIf { it.isNotBlank() }

    /**
     * Moves data from a previous location into the Glacier folder, once.
     *
     * Two sources: the app-private directory Pojav used on SDK 29+, and the
     * old `games/PojavLauncher` folder it used below that. Done with
     * [File.renameTo] rather than a recursive copy — both live on the same
     * volume as the target, so this is a cheap directory relink instead of
     * moving what can be gigabytes of worlds on the main thread. If the
     * rename fails the old data is left exactly where it was rather than
     * half-moved.
     *
     * Must run before Pojav's own storage constants are initialised, i.e.
     * before `PojavApplication.onCreate` — see GlacierApp.
     */
    fun migrateIfNeeded(context: Context) {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        if (prefs.getBoolean(KEY_MIGRATED, false)) return
        if (!canUseSharedStorage()) return // retry on a later launch once granted

        val target = sharedRoot()
        // Only migrate into an empty/absent destination: a populated Glacier
        // folder means the user is already running on it, and merging two
        // trees blindly could overwrite live saves.
        val targetEmpty = !target.exists() || (target.listFiles()?.isEmpty() ?: true)

        if (targetEmpty) {
            val candidates = listOfNotNull(
                context.getExternalFilesDir(null),
                File(Environment.getExternalStorageDirectory(), LEGACY_SHARED_PATH),
            )
            val source = candidates.firstOrNull { dir ->
                dir.isDirectory && (File(dir, ".minecraft").exists() || File(dir, "instances").exists())
            }
            if (source != null) {
                if (target.exists()) target.delete() // empty dir, so this just clears the way
                target.parentFile?.mkdirs()
                runCatching { source.renameTo(target) }
            }
        }

        prefs.edit().putBoolean(KEY_MIGRATED, true).apply()
    }

    /**
     * Opens the system All Files Access screen for this app. There is no
     * runtime-permission dialog for MANAGE_EXTERNAL_STORAGE — it is a
     * settings toggle the user has to flip themselves.
     */
    fun requestAllFilesAccess(context: Context): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return false
        return runCatching {
            context.startActivity(
                Intent(
                    Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    Uri.parse("package:${context.packageName}"),
                ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
            true
        }.getOrElse {
            // Some OEM builds don't ship the per-app screen; fall back to the
            // full list rather than doing nothing.
            runCatching {
                context.startActivity(
                    Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                )
                true
            }.getOrDefault(false)
        }
    }
}
