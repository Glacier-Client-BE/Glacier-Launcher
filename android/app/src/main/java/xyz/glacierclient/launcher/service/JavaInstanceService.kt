package xyz.glacierclient.launcher.service

import net.kdt.pojavlaunch.Tools
import net.kdt.pojavlaunch.prefs.LauncherPreferences
import net.kdt.pojavlaunch.value.launcherprofiles.LauncherProfiles
import net.kdt.pojavlaunch.value.launcherprofiles.MinecraftProfile
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

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
