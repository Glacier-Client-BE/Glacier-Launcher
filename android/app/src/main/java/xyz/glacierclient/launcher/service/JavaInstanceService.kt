package xyz.glacierclient.launcher.service

import net.kdt.pojavlaunch.Tools
import net.kdt.pojavlaunch.prefs.LauncherPreferences
import net.kdt.pojavlaunch.value.launcherprofiles.LauncherProfiles
import net.kdt.pojavlaunch.value.launcherprofiles.MinecraftProfile
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/**
 * Android analogue of Services/JavaInstanceService.cs — built directly on
 * top of the vendored Pojav library's OWN multi-profile system
 * (LauncherProfiles/MinecraftProfile, backed by a real launcher_profiles.json
 * in the same format the vanilla launcher uses) instead of inventing a
 * parallel one. Pojav's own MainActivity already reads
 * LauncherProfiles.getCurrentProfile().gameDir at launch
 * (MainActivity.java's onCreate -> Tools.getGameDirPath), so creating a
 * profile whose gameDir points at a distinct folder and switching
 * LauncherPreferences.PREF_KEY_CURRENT_PROFILE before launching a version
 * IS a real, working instance switch — no changes to Pojav's own launch
 * code needed, and no risk of diverging from how it already resolves
 * mods/saves/config directories.
 *
 * Pojav's LauncherPreferences.loadPreferences()/Tools.initEarlyConstants()
 * run in PojavApplication.onCreate() (GlacierApp extends it and calls
 * super.onCreate()), so DEFAULT_PREF/DIR_GAME_HOME are already initialized
 * by the time any of this runs from our own UI.
 */
object JavaInstanceService {

    private fun instanceDirRelative(slug: String) = "instances/$slug"

    fun list(): String {
        LauncherProfiles.load()
        val currentKey = LauncherPreferences.DEFAULT_PREF.getString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, "")
        val out = JSONArray()
        for ((key, profile) in LauncherProfiles.mainProfileJson.profiles) {
            out.put(profileToJson(key, profile, key == currentKey))
        }
        return out.toString()
    }

    fun create(name: String, versionId: String?): String {
        LauncherProfiles.load()
        val slug = uniqueSlug(slug(name))
        val profile = MinecraftProfile().apply {
            this.name = name.ifBlank { "Instance" }
            this.lastVersionId = versionId?.takeIf { it.isNotBlank() } ?: MinecraftProfile.LATEST_RELEASE
            this.gameDir = Tools.LAUNCHERPROFILES_RTPREFIX + instanceDirRelative(slug)
        }
        LauncherProfiles.insertMinecraftProfile(profile)
        LauncherProfiles.write()
        File(Tools.DIR_GAME_HOME, instanceDirRelative(slug)).mkdirs()

        val key = LauncherProfiles.mainProfileJson.profiles.entries.first { it.value === profile }.key
        return profileToJson(key, profile, false).toString()
    }

    fun rename(id: String, newName: String): Boolean {
        if (newName.isBlank()) return false
        LauncherProfiles.load()
        val profile = LauncherProfiles.mainProfileJson.profiles[id] ?: return false
        profile.name = newName.trim()
        LauncherProfiles.write()
        return true
    }

    /** Never removes the last remaining instance, mirroring desktop's JavaInstanceService.Delete. */
    fun delete(id: String): Boolean {
        LauncherProfiles.load()
        val profiles = LauncherProfiles.mainProfileJson.profiles
        if (profiles.size <= 1) return false
        val removed = profiles.remove(id) ?: return false
        LauncherProfiles.write()

        val currentKey = LauncherPreferences.DEFAULT_PREF.getString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, "")
        if (currentKey == id) {
            val fallbackKey = profiles.keys.first()
            LauncherPreferences.DEFAULT_PREF.edit().putString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, fallbackKey).apply()
        }

        removed.gameDir?.let { gameDir ->
            if (gameDir.startsWith(Tools.LAUNCHERPROFILES_RTPREFIX)) {
                runCatching {
                    File(Tools.DIR_GAME_HOME, gameDir.removePrefix(Tools.LAUNCHERPROFILES_RTPREFIX)).deleteRecursively()
                }
            }
        }
        return true
    }

    fun setActive(id: String): Boolean {
        LauncherProfiles.load()
        if (!LauncherProfiles.mainProfileJson.profiles.containsKey(id)) return false
        LauncherPreferences.DEFAULT_PREF.edit().putString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, id).apply()
        return true
    }

    fun setVersion(id: String, versionId: String): Boolean {
        if (versionId.isBlank()) return false
        LauncherProfiles.load()
        val profile = LauncherProfiles.mainProfileJson.profiles[id] ?: return false
        profile.lastVersionId = versionId
        LauncherProfiles.write()
        return true
    }

    // ── Java Tools (desktop's JavaInstanceService.cs) ────────────────────
    //
    // These three were rendered as permanently disabled cards in the Java
    // Tools panel. They are plain file I/O over the instance directory that
    // directoryFor() already resolves, so nothing blocked them beyond not
    // being written yet.

    private fun timestamp(): String =
        SimpleDateFormat("yyyyMMdd-HHmmss", Locale.US).format(Date())

    private fun activeProfileId(): String? {
        LauncherProfiles.load()
        val current = LauncherPreferences.DEFAULT_PREF
            ?.getString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, "")
        if (!current.isNullOrBlank() && LauncherProfiles.mainProfileJson.profiles.containsKey(current)) return current
        return LauncherProfiles.mainProfileJson.profiles.keys.firstOrNull()
    }

    /**
     * Desktop's BackupSavesAsync(): zips the active instance's saves folder
     * into backups/saves-<timestamp>.zip. Returns null when there is nothing
     * to back up, exactly as desktop does for an empty/missing saves dir,
     * so the caller can say so rather than producing an empty archive.
     */
    fun backupSaves(): String? {
        val dir = activeProfileId()?.let { directoryFor(it) } ?: return null
        val saves = File(dir, "saves")
        if (!saves.isDirectory || saves.listFiles().isNullOrEmpty()) return null

        val backups = File(dir, "backups").apply { mkdirs() }
        val zip = File(backups, "saves-${timestamp()}.zip")
        return try {
            ZipOutputStream(zip.outputStream().buffered()).use { out -> zipTree(out, saves, "") }
            zip.absolutePath
        } catch (e: Exception) {
            zip.delete()
            null
        }
    }

    /**
     * Desktop's ExportModpackAsync(): a CurseForge-shaped modpack zip —
     * mods/config/resourcepacks/shaderpacks under overrides/, plus the same
     * manifest.json. Written to exports/<id>-<timestamp>.zip.
     */
    fun exportModpack(): String? {
        val id = activeProfileId() ?: return null
        val dir = directoryFor(id) ?: return null
        val profile = LauncherProfiles.mainProfileJson.profiles[id] ?: return null

        val exports = File(Tools.DIR_GAME_HOME, "exports").apply { mkdirs() }
        val zip = File(exports, "$id-${timestamp()}.zip")
        return try {
            ZipOutputStream(zip.outputStream().buffered()).use { out ->
                for (name in listOf("mods", "config", "resourcepacks", "shaderpacks")) {
                    val source = File(dir, name)
                    if (source.isDirectory) zipTree(out, source, "overrides/$name/")
                }
                val manifest = JSONObject()
                    .put("minecraft", JSONObject().put("version", profile.lastVersionId ?: ""))
                    .put("manifestType", "minecraftModpack")
                    .put("manifestVersion", 1)
                    .put("name", profile.name ?: "Instance")
                    .put("version", "1.0.0")
                    .put("files", JSONArray())
                    .put("overrides", "overrides")
                out.putNextEntry(ZipEntry("manifest.json"))
                out.write(manifest.toString(2).toByteArray())
                out.closeEntry()
            }
            zip.absolutePath
        } catch (e: Exception) {
            zip.delete()
            null
        }
    }

    /** Desktop's Duplicate(id): a new profile named "<name> Copy" with the whole directory tree copied. */
    fun duplicate(id: String): String? {
        LauncherProfiles.load()
        val source = LauncherProfiles.mainProfileJson.profiles[id] ?: return null
        val sourceDir = directoryFor(id)

        val createdJson = create("${source.name ?: "Instance"} Copy", source.lastVersionId)
        val copyId = JSONObject(createdJson).optString("id")
        if (copyId.isBlank()) return null

        val destDir = directoryFor(copyId)
        if (sourceDir != null && sourceDir.isDirectory && destDir != null) {
            runCatching { sourceDir.copyRecursively(destDir, overwrite = true) }
        }
        return createdJson
    }

    /**
     * Writes [dir]'s tree into [out] under [prefix]. Directories are not
     * given their own entries — a zip is defined by its file paths, and
     * every consumer recreates the intermediate folders.
     */
    private fun zipTree(out: ZipOutputStream, dir: File, prefix: String) {
        dir.walkTopDown().filter { it.isFile }.forEach { file ->
            val relative = file.relativeTo(dir).path.replace(File.separatorChar, '/')
            out.putNextEntry(ZipEntry(prefix + relative))
            file.inputStream().use { it.copyTo(out) }
            out.closeEntry()
        }
    }

    /** Resolves an instance's real game directory the same way Pojav's own MainActivity does (Tools.getGameDirPath). */
    fun directoryFor(id: String): File? {
        LauncherProfiles.load()
        val profile = LauncherProfiles.mainProfileJson.profiles[id] ?: return null
        return Tools.getGameDirPath(profile)
    }

    private fun uniqueSlug(base: String): String {
        var candidate = base
        var n = 2
        while (File(Tools.DIR_GAME_HOME, instanceDirRelative(candidate)).exists()) {
            candidate = "$base-${n++}"
        }
        return candidate
    }

    private fun slug(value: String): String {
        val raw = value.ifBlank { "instance" }.trim().lowercase()
        val cleaned = raw.map { if (it.isLetterOrDigit()) it else '-' }.joinToString("").trim('-')
        return cleaned.ifBlank { "instance" }
    }

    private fun profileToJson(key: String, profile: MinecraftProfile, isActive: Boolean) = JSONObject().apply {
        put("id", key)
        put("name", profile.name ?: "")
        put("versionId", profile.lastVersionId ?: "")
        put("isActive", isActive)
    }
}
