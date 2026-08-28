package xyz.glacierclient.launcher

import net.kdt.pojavlaunch.PojavApplication

// Extends Pojav's own Application class (now a library module built
// directly into this app — see settings.gradle.kts and
// android/README.md's "Single-APK distribution" section) instead of plain
// android.app.Application, so its one-time setup (crash handler,
// LauncherPreferences, device-architecture detection, and unpacking the
// bundled JRE/LWJGL runtime out of assets — see PojavApplication.onCreate())
// still runs. Without this, net.kdt.pojavlaunch.MainActivity would crash
// immediately on first touch, since that setup is what makes its static
// Tools/ContextExecutor state usable at all.
class GlacierApp : PojavApplication()
