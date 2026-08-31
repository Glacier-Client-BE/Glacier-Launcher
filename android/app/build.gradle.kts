import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import java.util.Base64
import java.util.Properties
import org.gradle.api.GradleException

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// Release signing comes from CI secrets / local env vars, never committed
// to the repo. Set ANDROID_KEYSTORE_PATH/ANDROID_KEYSTORE_PASSWORD/
// ANDROID_KEY_ALIAS/ANDROID_KEY_PASSWORD to sign; if unset, release builds
// stay unsigned (installable for local testing via `adb install -r`, but
// not upgradeable in place and not suitable for distribution) rather than
// failing the build. See android/README.md for how to generate a keystore
// and where those secrets live in CI.
//
// Read up here, before the `android {}` block, rather than down by
// signingConfigs where they used to live: defaultConfig's manifestPlaceholders
// (below) needs hasSigningConfig/computeMsalSignatureHash() too, and a
// Kotlin script can't reference a val that's declared later in the file.
val ksPath = System.getenv("ANDROID_KEYSTORE_PATH")
val ksPassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
val ksKeyAlias = System.getenv("ANDROID_KEY_ALIAS")
val ksKeyPassword = System.getenv("ANDROID_KEY_PASSWORD")
val hasSigningConfig = !ksPath.isNullOrBlank() && !ksPassword.isNullOrBlank() &&
    !ksKeyAlias.isNullOrBlank() && !ksKeyPassword.isNullOrBlank()

// MSAL's Android redirect_uri is msauth://<package>/<base64 SHA-1 of the
// signing cert>, per Microsoft's own documented public MSAL Android
// redirect URI format (https://learn.microsoft.com/en-us/entra/identity-platform/msal-android-single-sign-on-across-devices).
// Both res/raw/msal_config.json (generateMsalConfig below) and
// AndroidManifest.xml's BrowserTabActivity intent-filter need this exact
// same hash — it used to be a literal "YOUR_SIGNATURE_HASH_HERE" hardcoded
// into the manifest, out of sync with whatever generateMsalConfig computed
// for the JSON side, so a signed build's redirect URI never actually
// matched what the manifest declared it could handle. One function, used
// by both.
val placeholderSignatureHash = "YOUR_SIGNATURE_HASH_HERE"

fun computeMsalSignatureHash(): String {
    if (!hasSigningConfig) return placeholderSignatureHash
    // File(...), not Gradle's file(...) helper: this is a plain top-level
    // function, not a script-block lambda, so it has no implicit Project
    // receiver for file(...) to resolve against — ksPath is already an
    // absolute path (from an env var), so a plain java.io.File needs no
    // project-relative resolution anyway.
    //
    // These are imported at the top of the file rather than referenced by
    // qualified name (java.security.KeyStore etc.) inline: AGP registers a
    // Project extension named "java" (JavaPluginExtension), and a bare
    // `java` identifier inside this script resolves to that extension
    // rather than the java.* root package — so `java.security.KeyStore`
    // doesn't compile here even though `import java.security.KeyStore` +
    // plain `KeyStore` does. Same reason this file's other `java.util.*`
    // references (below, in defaultConfig) needed the same treatment.
    //
    // Modern `keytool -genkeypair` defaults to PKCS12; older keystores may
    // still be JKS. Try both rather than guessing.
    val keyStore = try {
        KeyStore.getInstance("PKCS12").apply {
            File(ksPath!!).inputStream().use { load(it, ksPassword!!.toCharArray()) }
        }
    } catch (e: Exception) {
        KeyStore.getInstance("JKS").apply {
            File(ksPath!!).inputStream().use { load(it, ksPassword!!.toCharArray()) }
        }
    }
    val cert = keyStore.getCertificate(ksKeyAlias)
        ?: throw GradleException(
            "computeMsalSignatureHash: alias '$ksKeyAlias' not found in keystore $ksPath"
        )
    val sha1 = MessageDigest.getInstance("SHA-1").digest(cert.encoded)
    return Base64.getEncoder().encodeToString(sha1)
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
                    val props = Properties().apply { load(f.inputStream()) }
                    props.getProperty("playLicensingPublicKey")
                } else null
            }
            ?: "PLAY_LICENSING_PUBLIC_KEY_PLACEHOLDER"
        buildConfigField("String", "PLAY_LICENSING_PUBLIC_KEY", "\"$playLicensingKey\"")

        // AndroidManifest.xml's BrowserTabActivity intent-filter reads this
        // as ${msalSignatureHash} — see computeMsalSignatureHash() above for
        // why it has to be computed once and shared with generateMsalConfig's
        // JSON output rather than hardcoded separately in the manifest.
        manifestPlaceholders["msalSignatureHash"] = computeMsalSignatureHash()
    }

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

// Generates res/raw/msal_config.json (MicrosoftAuth.kt) from CI secrets /
// local signing env vars, overwriting the committed placeholder at build
// time — same idea as CURSEFORGE_API_KEY's buildConfigField default, just
// targeting a resource file instead of a BuildConfig constant, since MSAL
// reads its client config from a raw JSON resource, not code.
//
// Shares computeMsalSignatureHash() (top of file) with defaultConfig's
// msalSignatureHash manifestPlaceholders entry, so the JSON's redirect_uri
// and the manifest's BrowserTabActivity intent-filter path can never drift
// out of sync with each other again.
//
// A build only works against the real Azure app registration if BOTH the
// client_id and the redirect signature hash match what's registered there:
//  - if a release keystore is configured (hasSigningConfig above), compute
//    the real hash from it and bake in the real AZURE_CLIENT_ID.
//  - otherwise (local/dev/PR/debug-signed builds) the redirect URI can
//    never match Azure's registered one anyway, so client_id is left as
//    the placeholder too — never let a real client ID leak into a config
//    that can't complete sign-in, and never mislead a debug build into
//    thinking it's fully configured.
val generateMsalConfig = tasks.register("generateMsalConfig") {
    val outputFile = layout.projectDirectory.file("src/main/res/raw/msal_config.json").asFile
    val placeholderClientId = "00000000-0000-0000-0000-000000000000"

    inputs.property("hasSigningConfig", hasSigningConfig)
    inputs.property("azureClientIdSet", !System.getenv("AZURE_CLIENT_ID").isNullOrBlank())
    outputs.file(outputFile)

    doLast {
        val signatureHash = computeMsalSignatureHash()
        val clientId = if (hasSigningConfig) {
            System.getenv("AZURE_CLIENT_ID")?.takeIf { it.isNotBlank() } ?: placeholderClientId
        } else {
            placeholderClientId
        }

        val json = """
            {
              "client_id": "$clientId",
              "authorization_user_agent": "DEFAULT",
              "redirect_uri": "msauth://xyz.glacierclient.launcher/$signatureHash",
              "account_mode": "SINGLE",
              "broker_redirect_uri_registered": false,
              "authorities": [
                {
                  "type": "AAD",
                  "audience": {
                    "type": "AzureADandPersonalMicrosoftAccount",
                    "tenant_id": "consumers"
                  }
                }
              ]
            }
        """.trimIndent()
        outputFile.writeText(json + "\n")
    }
}

tasks.named("preBuild") {
    dependsOn(generateMsalConfig)
}

// msal's transitive com.microsoft.identity:common brings in an older gson
// (2.8.6) than whatever else in the dependency graph already pulls 2.8.9,
// and Gradle's default "highest version wins" resolution doesn't apply
// across two different *jars* packaging overlapping classes the way it
// does across two versions of the *same* artifact coordinate — both
// gson-2.8.6.jar and gson-2.8.9.jar (com.google.code.gson:gson:2.8.9) were
// landing on the classpath together, which AGP's checkDebugDuplicateClasses
// task correctly refuses to package (two copies of com.google.gson.Gson
// with no defined precedence). Forcing a single resolved version collapses
// them back into the normal single-artifact case.
configurations.all {
    resolutionStrategy.force("com.google.code.gson:gson:2.8.9")
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

    // MicrosoftAuth.kt: Microsoft's own official MSAL Android library and
    // OkHttp/kotlinx-coroutines for its XSTS token exchange and suspend
    // functions — a real, scoped OAuth login used to verify Bedrock
    // ownership through the account's own Xbox/Minecraft entitlement.
    // Public artifacts, no extra repository needed.
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    // msal's transitive com.microsoft.identity:common pulls in
    // com.microsoft.device.display:display-mask (Surface Duo dual-screen/
    // fold-aware layout support) from Microsoft's own Maven feed, which
    // isn't in any repository this project's settings.gradle.kts declares
    // (google/mavenCentral/jitpack) — leaving the dependency graph
    // unresolvable. This app has no foldable-specific UI, so the dep is
    // dropped rather than adding a fourth, Microsoft-specific repository
    // just to resolve a feature this app never uses.
    implementation("com.microsoft.identity.client:msal:5.4.0") {
        exclude(group = "com.microsoft.device.display", module = "display-mask")
    }

    // ShizukuExecutor.kt: Shizuku's official, open-source client API +
    // ContentProvider that exposes privileged (adb shell / rooted-manager)
    // execution without granting this app root itself. Both artifacts are
    // published to mavenCentral.
    implementation("dev.rikka.shizuku:api:13.1.5")
    implementation("dev.rikka.shizuku:provider:13.1.5")
}
