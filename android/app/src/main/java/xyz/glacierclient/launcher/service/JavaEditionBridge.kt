package xyz.glacierclient.launcher.service

import android.app.Activity
import android.content.Context
import android.content.Intent
import net.kdt.pojavlaunch.R
import net.kdt.pojavlaunch.Tools
import net.kdt.pojavlaunch.prefs.LauncherPreferences
import net.kdt.pojavlaunch.progresskeeper.ProgressKeeper
import net.kdt.pojavlaunch.tasks.AsyncAssetManager
import net.kdt.pojavlaunch.tasks.AsyncMinecraftDownloader
import net.kdt.pojavlaunch.tasks.MinecraftDownloader
import net.kdt.pojavlaunch.value.launcherprofiles.LauncherProfiles
import net.kdt.pojavlaunch.value.launcherprofiles.MinecraftProfile
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
 * Mods/Glacier-client jars go into the game's own .minecraft/mods folder
 * under Tools.DIR_GAME_HOME, which is what Pojav's Tools.java actually
 * reads from. That root is Glacier's own games/Glacier folder now (see
 * service/GlacierStorage.kt and the redirect in scripts/rebrand-pojav.sh),
 * so it is always read from the constant rather than written out as a
 * literal path.
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
        // Without this, launching straight into the game surface is a
        // guaranteed crash on a fresh install — see ensureCurrentProfile().
        if (!ensureCurrentProfile(context, versionId)) return false

        return try {
            // TestStorageActivity is the only thing that normally runs
            // AsyncAssetManager's unpack calls (default control layout,
            // options.txt, the bundled JRE-adjacent component jars), and
            // Glacier's launch-straight-into-the-game flow never touches
            // that activity (its own launcher intent-filter was stripped —
            // see rebrand-pojav.sh — precisely so it doesn't show as a
            // second home-screen icon). Left uncalled, MainActivity's first
            // frame throws trying to read controlmap/default.json, which
            // never existed. Both calls are cheap no-ops once a version
            // file on disk already matches the bundled one, so it's safe to
            // run on every launch rather than only the first.
            AsyncAssetManager.unpackComponents(context)
            AsyncAssetManager.unpackSingleFiles(context)
            ProgressKeeper.waitUntilDone {
                if (versionId.isNullOrBlank()) {
                    startMainActivity(context, null)
                } else {
                    downloadThenLaunch(context, versionId)
                }
            }
            true
        } catch (e: Exception) {
            false
        }
    }

    private fun startMainActivity(context: Context, versionId: String?) {
        val intent = Intent(context, net.kdt.pojavlaunch.MainActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            if (!versionId.isNullOrBlank()) putExtra(net.kdt.pojavlaunch.MainActivity.INTENT_MINECRAFT_VERSION, versionId)
        }
        context.startActivity(intent)
    }

    /**
     * A version picked in our Java Versions panel is one Mojang's manifest
     * lists as installable, not one already downloaded — the panel lists
     * every released version, same as the vendored version-picker Glacier
     * doesn't use. Launching net.kdt.pojavlaunch.MainActivity straight away
     * (the old behavior here) crashed in its very first frame reading a
     * version JSON that had never been fetched. LauncherActivity's own
     * mLaunchGameListener does exactly this same
     * download-then-start-MainActivity sequence before ever touching
     * MainActivity, so this mirrors it instead of reinventing it.
     */
    private fun downloadThenLaunch(context: Context, versionId: String) {
        val normalizedVersionId = AsyncMinecraftDownloader.normalizeVersionId(versionId)
        val mcVersion = AsyncMinecraftDownloader.getListedVersion(normalizedVersionId)
        MinecraftDownloader().start(
            context as? Activity,
            mcVersion,
            normalizedVersionId,
            object : AsyncMinecraftDownloader.DoneListener {
                override fun onDownloadDone() {
                    ProgressKeeper.waitUntilDone { startMainActivity(context, normalizedVersionId) }
                }

                override fun onDownloadFailed(throwable: Throwable) {
                    Tools.showErrorRemote(context.getString(R.string.mc_download_failed), throwable)
                }
            },
        )
    }

    /**
     * Guarantees Pojav has a *selected* launcher profile before we start its
     * MainActivity, and points that profile at [versionId].
     *
     * net.kdt.pojavlaunch.MainActivity.onCreate() calls
     * LauncherProfiles.getCurrentProfile() on its very first line, which
     * resolves the "currentProfile" preference against the profile map and
     * throws `RuntimeException("The current profile stopped existing :(")`
     * when it doesn't hit. Two separate things make that throw here:
     *
     *  - LauncherProfiles.load() seeds its default profile under a *random
     *    UUID* key, but the "currentProfile" preference defaults to "". A
     *    freshly-generated profile map therefore never contains the key
     *    being looked up.
     *  - The preference is normally written by Pojav's own home screen
     *    (LauncherActivity / ProfileAdapter) when the user picks a profile,
     *    and Glacier deliberately skips that entire UI to launch the game
     *    surface directly — so on a Glacier-only install nothing ever wrote
     *    it, and every Java Edition launch crashed before rendering a frame.
     *
     * Selecting an existing profile (rather than always creating one) keeps
     * profiles made through our own Java Instances panel — which are stored
     * in this same map, see JavaInstanceService — working as the user set
     * them up.
     *
     * @return false when Pojav's storage paths aren't usable, in which case
     *         launching is aborted rather than crashed into.
     */
    // Block body rather than an expression body: Kotlin disallows `return`
    // inside an expression-bodied function, and the storage-unavailable path
    // below has to bail out early.
    private fun ensureCurrentProfile(context: Context, versionId: String?): Boolean {
        return try {
            // LauncherProfiles' own profile-file path is a `static final` read
            // from Tools.GAME_PROFILES_FILE at class-load time, so the storage
            // constants have to exist before the class is first touched or it
            // loads with a null path. PojavApplication.onCreate() normally does
            // this, but it silently skips when the storage root wasn't ready at
            // process start (permission not yet granted on first run) — in
            // which case do it now, and refuse to launch if storage still
            // isn't there.
            if (Tools.GAME_PROFILES_FILE == null) {
                if (!Tools.checkStorageRoot(context)) return false
                LauncherPreferences.loadPreferences(context)
            }

            LauncherProfiles.load()
            val profiles = LauncherProfiles.mainProfileJson.profiles

            val prefs = LauncherPreferences.DEFAULT_PREF
                ?: context.getSharedPreferences("${context.packageName}_preferences", Context.MODE_PRIVATE)

            val key: String = run {
                val selected = prefs.getString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, "")
                // isNullOrBlank() smart-casts `selected` to non-null String
                // for containsKey, whose parameter type is non-null.
                if (!selected.isNullOrBlank() && profiles.containsKey(selected)) return@run selected

                // Prefer a profile that already exists — load() has just
                // seeded a default one if the map was empty, and any instance
                // made through our own Java Instances panel lives in this same
                // map — over minting another, so repeated launches can't
                // accumulate throwaway profiles.
                val chosen = profiles.keys.firstOrNull()
                    ?: LauncherProfiles.getFreeProfileKey().also {
                        profiles[it] = MinecraftProfile.getDefaultProfile()
                    }
                prefs.edit().putString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, chosen).apply()
                chosen
            }

            // MainActivity reads the version from our Intent extra, but it
            // also reads minecraftProfile.lastVersionId directly (for the
            // window title, and as the fallback when no extra is set), and
            // persisting it keeps our Java Versions panel's choice as the
            // profile's own last-used version for the next launch.
            if (!versionId.isNullOrBlank()) {
                profiles[key]?.lastVersionId = versionId
            }
            LauncherProfiles.write()
            true
        } catch (e: Exception) {
            false
        }
    }

    // Derived from Tools.DIR_GAME_HOME rather than a hardcoded
    // "games/PojavLauncher" path: the storage root is now Glacier's own
    // folder (GlacierStorage / the redirect in rebrand-pojav.sh) and falls
    // back to the app-private directory without All Files Access, so any
    // literal path here would point at a directory the game never reads.
    private fun gameDir(): File = File(Tools.DIR_GAME_HOME ?: "", ".minecraft")

    private fun pojavModsDir(): File = File(gameDir(), "mods")

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
        val dir = File(gameDir(), "screenshots")
        return dir.listFiles { f -> f.extension.equals("png", ignoreCase = true) }
            ?.sortedByDescending { it.lastModified() }
            ?: emptyList()
    }
}
