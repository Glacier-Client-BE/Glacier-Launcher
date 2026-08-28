plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "xyz.glacierclient.launcher"
    compileSdk = 34

    defaultConfig {
        applicationId = "xyz.glacierclient.launcher"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = System.getenv("GLACIER_VERSION") ?: "0.0.0-dev"
        // The merged-in Java Edition runtime (androidx.constraintlayout,
        // viewpager2, preference, bytehook, htmlcleaner, ...) pushes total
        // method count well past the pre-multidex 64K limit. minSdk 26 has
        // native ART multidex support, so this is just the DSL flag, no
        // androidx.multidex compat library needed.
        multiDexEnabled = true

        // Mirrors GlacierLauncher.csproj's CurseForgeApiKey AssemblyMetadataAttribute:
        // baked in at build time from the same CI secret, empty for local dev builds.
        // Read from JS via AndroidBridge.curseForgeApiKey() in MainActivity.kt.
        buildConfigField(
            "String",
            "CURSEFORGE_API_KEY",
            "\"${System.getenv("CURSEFORGE_API_KEY") ?: ""}\"",
        )
    }

    // Release signing comes from CI secrets / local env vars, never committed
    // to the repo. Set ANDROID_KEYSTORE_PATH/ANDROID_KEYSTORE_PASSWORD/
    // ANDROID_KEY_ALIAS/ANDROID_KEY_PASSWORD to sign; if unset, release builds
    // stay unsigned (installable for local testing via `adb install -r`, but
    // not upgradeable in place and not suitable for distribution) rather than
    // failing the build. See android/README.md for how to generate a keystore
    // and where those secrets live in CI.
    val ksPath = System.getenv("ANDROID_KEYSTORE_PATH")
    val ksPassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
    val ksKeyAlias = System.getenv("ANDROID_KEY_ALIAS")
    val ksKeyPassword = System.getenv("ANDROID_KEY_PASSWORD")
    val hasSigningConfig = !ksPath.isNullOrBlank() && !ksPassword.isNullOrBlank() &&
        !ksKeyAlias.isNullOrBlank() && !ksKeyPassword.isNullOrBlank()

    signingConfigs {
        if (hasSigningConfig) {
            create("release") {
                storeFile = file(ksPath!!)
                storePassword = ksPassword
                keyAlias = ksKeyAlias
                keyPassword = ksKeyPassword
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            if (hasSigningConfig) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    buildFeatures {
        buildConfig = true
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    // The UI is a single WebView (assets/www/) reusing the desktop app's real
    // app.css and image assets for pixel-identical styling — see MainActivity.kt.
    // Kotlin only supplies the native bridge (root checks, launching other
    // installed apps, settings persistence), so no Compose/networking/image
    // libraries are needed here; org.json (bundled with Android) is enough
    // for the one settings-JSON read in AndroidBridge.
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-ktx:1.9.1")

    // The Java Edition runtime — the vendored PojavLauncher submodule, built
    // as a library (see settings.gradle.kts + scripts/rebrand-pojav.sh)
    // instead of a separate installable APK, so launching Java Edition is
    // an in-process Intent to net.kdt.pojavlaunch.MainActivity
    // (JavaEditionBridge.kt), not a second app the user has to install.
    implementation(project(":app_pojavlauncher"))
}
