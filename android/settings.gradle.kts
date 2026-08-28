pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
        // Pulls a handful of the Java Edition module's own dependencies
        // (PojavLauncherTeam/Mathias-Boulay GitHub-hosted libraries) — see
        // ":app_pojavlauncher" below.
        maven { url = uri("https://jitpack.io") }
    }
}

rootProject.name = "GlacierLauncher"
include(":app")

// The vendored PojavLauncher submodule (android/pojavlauncher), built
// directly into this app instead of as a separate installable APK — see
// scripts/rebrand-pojav.sh, which converts app_pojavlauncher from
// com.android.application to com.android.library at build time (never
// committed into the submodule itself), and android/README.md's
// "Single-APK distribution" section for the full rationale. The three
// plain `java`/`java-library` support modules build small jars that
// app_pojavlauncher's own build copies straight into its assets/ folder.
// Kept as PojavLauncher's own original project names (rather than renamed/
// namespaced) because app_pojavlauncher/build.gradle's own afterEvaluate
// block references them by these exact paths
// (tasks.mergeDebugAssets.dependsOn(":forge_installer:jar", ...)) —
// renaming them would mean patching that reference too, for no benefit.
include(":app_pojavlauncher", ":jre_lwjgl3glfw", ":arc_dns_injector", ":forge_installer")
project(":app_pojavlauncher").projectDir = file("pojavlauncher/app_pojavlauncher")
project(":jre_lwjgl3glfw").projectDir = file("pojavlauncher/jre_lwjgl3glfw")
project(":arc_dns_injector").projectDir = file("pojavlauncher/arc_dns_injector")
project(":forge_installer").projectDir = file("pojavlauncher/forge_installer")
