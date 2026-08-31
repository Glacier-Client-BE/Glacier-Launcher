package xyz.glacierclient.launcher

import net.kdt.pojavlaunch.PojavApplication
import xyz.glacierclient.launcher.service.GlacierStorage

// Extends Pojav's own Application class (now a library module built
// directly into this app — see settings.gradle.kts and
// android/README.md's "Single-APK distribution" section) instead of plain
// android.app.Application, so its one-time setup (crash handler,
// LauncherPreferences, device-architecture detection, and unpacking the
// bundled JRE/LWJGL runtime out of assets — see PojavApplication.onCreate())
// still runs. Without this, net.kdt.pojavlaunch.MainActivity would crash
// immediately on first touch, since that setup is what makes its static
// Tools/ContextExecutor state usable at all.
class GlacierApp : PojavApplication() {

    override fun onCreate() {
        // Before super.onCreate(): that is where PojavApplication calls
        // LauncherPreferences.loadPreferences -> Tools.initStorageConstants,
        // which resolves the storage root and caches it in static fields for
        // the rest of the process. Moving data afterwards would leave every
        // one of those constants pointing at the directory we just emptied.
        GlacierStorage.migrateIfNeeded(this)
        super.onCreate()
    }
}
