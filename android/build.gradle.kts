plugins {
    // 8.7.2 (not 8.6.0) to match the version app_pojavlauncher/build.gradle
    // itself pins — both :app and :app_pojavlauncher resolve com.android.*
    // through this one root pluginManagement now that the Java Edition
    // module is a subproject of this build rather than its own separate
    // Gradle root. Requires Gradle 8.9+, which gradle/wrapper already is.
    id("com.android.application") version "8.7.2" apply false
    id("com.android.library") version "8.7.2" apply false
    id("org.jetbrains.kotlin.android") version "1.9.24" apply false
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.24" apply false
}
