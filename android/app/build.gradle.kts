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
        // Derived from the same GLACIER_VERSION the release workflow computes,
        // rather than a literal 1. Android compares versionCode (never
        // versionName) to decide whether an APK is an upgrade, so a constant 1
        // makes every release look like the same build to the package
        // installer that LauncherUpdateService.kt hands the downloaded APK to —
        // an in-app update can be declined as "already installed" by installers
        // that treat an equal versionCode as a no-op.
        //
        // major*1_000_000 + minor*1_000 + patch stays strictly increasing for
        // any version whose minor and patch are under 1000. Non-release builds
        // (GLACIER_VERSION unset, or a "pr-…"/"branch-…" value) fall back to 1,
        // which is fine for a local install and never reaches users.
        versionCode = Regex("""^(\d+)\.(\d+)\.(\d+)""")
            .find(System.getenv("GLACIER_VERSION") ?: "")
            ?.destructured
            ?.let { (major, minor, patch) ->
                major.toInt() * 1_000_000 + minor.toInt() * 1_000 + patch.toInt()
            }
            // Floored at 1: the local-dev default "0.0.0-dev" parses to 0, and
            // Android wants a positive versionCode.
            ?.coerceAtLeast(1) ?: 1
        versionName = System.getenv("GLACIER_VERSION") ?: "0.0.0-dev"

        // The vendored Java Edition runtime ships native libs (JRE, LWJGL/
        // GLFW, bytehook, ...) for all four Android ABIs, ~130MB combined,
        // in one universal APK. x86/x86_64 exist for emulators and the
        // handful of Intel-based Android tablets, not real phones — the
        // devices this launcher actually targets are all arm64-v8a (or,
        // rarely, armeabi-v7a on older hardware). Dropping the two Intel
        // ABIs from packaging cuts roughly 55-60MB with no loss of real
        // device support, without the multi-APK release/updater plumbing a
        // full ABI split would need (LauncherUpdateService.kt expects one
        // APK asset per GitHub release).
        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a")
        }

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

        // PlayLicensing.kt: the Base64 RSA public key from *this app's own*
        // Play Console listing (Play Console -> Setup -> App integrity ->
        // Play licensing -> "LICENSING & IN-APP BILLING" public key), used
        // to verify signed license-check responses from the Play Store's
        // ILicensingService. Glacier isn't published yet, so there is no
        // real key to bake in — PLAY_LICENSING_PUBLIC_KEY_PLACEHOLDER below
        // is intentionally not a fake-looking real key. Once published, set
        // the PLAY_LICENSING_PUBLIC_KEY env var (CI secret, same pattern as
        // CURSEFORGE_API_KEY) or a local.properties entry, and
        // PlayLicensing.checkLicense() will pick it up automatically.
        val playLicensingKey = System.getenv("PLAY_LICENSING_PUBLIC_KEY")
            ?: project.rootProject.file("local.properties").let { f ->
                if (f.exists()) {
                    val props = java.util.Properties().apply { load(f.inputStream()) }
                    props.getProperty("playLicensingPublicKey")
                } else null
            }
            ?: "PLAY_LICENSING_PUBLIC_KEY_PLACEHOLDER"
        buildConfigField("String", "PLAY_LICENSING_PUBLIC_KEY", "\"$playLicensingKey\"")
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
        // ILicensingService.aidl / ILicenseResultListener.aidl (PlayLicensing.kt).
        aidl = true
    }

    // app_pojavlauncher both compiles libbytehook.so from its own jni/
    // sources AND depends on a prebuilt bytehook AAR that also ships
    // lib/*/libbytehook.so — harmless duplicate (same native lib, two
    // sources), but AGP's native-lib merge step refuses to silently pick
    // one when packaging the final APK. pickFirst says either copy is fine.
    packaging {
        jniLibs {
            pickFirsts += "lib/*/libbytehook.so"
        }
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
    // DocumentFile wraps the SAF tree the user grants access to for Bedrock's
    // shared-storage world/pack/backup data (BedrockStorageService.kt) —
    // plain java.io.File can't address a content:// tree Uri.
    implementation("androidx.documentfile:documentfile:1.0.1")
    // net.kdt.pojavlaunch.MainActivity (below) extends AppCompatActivity, but
    // app_pojavlauncher only pulls androidx.appcompat in transitively via its
    // own `implementation`-scoped deps, which Gradle never re-exports to a
    // consumer — Kotlin needs AppCompatActivity directly on :app's own
    // compile classpath to resolve JavaEditionBridge.kt's reference to it.
    implementation("androidx.appcompat:appcompat:1.7.0")

    // The Java Edition runtime — the vendored PojavLauncher submodule, built
    // as a library (see settings.gradle.kts + scripts/rebrand-pojav.sh)
    // instead of a separate installable APK, so launching Java Edition is
    // an in-process Intent to net.kdt.pojavlaunch.MainActivity
    // (JavaEditionBridge.kt), not a second app the user has to install.
    implementation(project(":app_pojavlauncher"))

    // Downloader.kt (utils/) and GPlayAPI.kt (utils/): a Play Store
    // purchase-based APK fetch path alongside BedrockVersionService's
    // manual-APK one. GPlayAPI wraps Aurora Store's GPlayApi client
    // (PurchaseHelper/AuthHelper), which needs OkHttp for HTTP and
    // kotlinx-coroutines for its suspend functions.
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("com.auroraoss:gplayapi:3.4.2")

    // MicrosoftAuth.kt: Microsoft's own official MSAL Android library, the
    // alternative Xbox/Microsoft-account ownership-verification path
    // alongside GPlayAPI's Google-account one. Public artifact on
    // mavenCentral (no extra repository needed).
    implementation("com.microsoft.identity.client:msal:5.4.0")

    // ShizukuExecutor.kt: Shizuku's official, open-source client API +
    // ContentProvider that exposes privileged (adb shell / rooted-manager)
    // execution without granting this app root itself. Both artifacts are
    // published to mavenCentral.
    implementation("dev.rikka.shizuku:api:13.1.5")
    implementation("dev.rikka.shizuku:provider:13.1.5")
}
