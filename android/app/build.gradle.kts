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

        // Mirrors GlacierLauncher.csproj's CurseForgeApiKey AssemblyMetadataAttribute:
        // baked in at build time from the same CI secret, empty for local dev builds.
        // Read from JS via AndroidBridge.curseForgeApiKey() in MainActivity.kt.
        buildConfigField(
            "String",
            "CURSEFORGE_API_KEY",
            "\"${System.getenv("CURSEFORGE_API_KEY") ?: ""}\"",
        )
    }

    buildTypes {
        release {
            isMinifyEnabled = false
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
}
