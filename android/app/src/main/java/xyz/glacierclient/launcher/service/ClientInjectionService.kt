package xyz.glacierclient.launcher.service

import java.io.DataOutputStream

/**
 * Android analogue of GlacierLauncher.Services.InjectionService.
 *
 * IMPORTANT — this is NOT a 1:1 port of Windows DLL injection:
 *   - Windows: CreateRemoteThread + LoadLibrary into minecraft.exe's address
 *     space, giving Latite/Flarial/OderSo full in-process hooking.
 *   - Android: apps are sandboxed per-UID, so there's no cross-process
 *     equivalent of CreateRemoteThread. But this does NOT mean injection
 *     needs root — real, published, non-root Android Bedrock launchers
 *     (Kitsuri-Studios/Minimal-Launcher, LiteLDev/LeviLaunchroid, LeviLamina's
 *     own official Android launcher) all use the same non-root technique:
 *       1. Get a Context for the already-installed, licensed
 *          com.mojang.minecraftpe package via
 *          createPackageContext(pkg, CONTEXT_IGNORE_SECURITY or
 *          CONTEXT_INCLUDE_CODE) — this hands you that package's own
 *          ClassLoader/AssetManager/native lib dir from inside YOUR process.
 *       2. dlopen() libminecraftpe.so (+ its dependency libs) from a small
 *          JNI shim and forward ANativeActivity_onCreate/android_main to the
 *          real symbols — your launcher's activity becomes Minecraft's own
 *          compiled native code, instead of reaching into another process.
 *       3. A launcher-owned native module loads alongside and hooks the real
 *          library the same way a Windows DLL hooks Minecraft.Windows.exe —
 *          that hook code (the actual "client") is inherently
 *          version-specific reverse-engineering work no generic launcher can
 *          produce generically.
 *     Building and on-device-verifying that dlopen/JNI shim is real native
 *     engineering work outside what this pass could safely ship untested
 *     (see android/README.md's "Deep gap analysis" section) — this service
 *     currently still only offers the root-based file-staging fallback
 *     below, which is real but far more limited than the technique above.
 */
object ClientInjectionService {

    data class InjectionResult(val success: Boolean, val message: String)

    fun isRootAvailable(): Boolean = try {
        val process = Runtime.getRuntime().exec(arrayOf("su", "-c", "id"))
        process.waitFor() == 0
    } catch (e: Exception) {
        false
    }

    /**
     * Attempts to stage [nativeLibPath] for the Minecraft Bedrock package.
     * Returns immediately with a descriptive failure on non-rooted devices
     * instead of pretending to succeed.
     */
    fun attemptInject(nativeLibPath: String, targetPackage: String = "com.mojang.minecraftpe"): InjectionResult {
        if (!isRootAvailable()) {
            return InjectionResult(
                success = false,
                message = "This build only supports the root-based staging fallback. The real " +
                    "non-root technique (package-context + dlopen, same as other Android Bedrock " +
                    "launchers) isn't implemented yet — see android/README.md. Install Magisk/root " +
                    "to use this fallback instead.",
            )
        }
        return try {
            val process = Runtime.getRuntime().exec("su")
            DataOutputStream(process.outputStream).use { out ->
                out.writeBytes("run-as $targetPackage mkdir -p files/glacier_mods\n")
                out.writeBytes("cp \"$nativeLibPath\" /data/data/$targetPackage/files/glacier_mods/\n")
                out.writeBytes("exit\n")
                out.flush()
            }
            val code = process.waitFor()
            if (code == 0) {
                InjectionResult(true, "Staged client library for $targetPackage. Restart Minecraft to apply.")
            } else {
                InjectionResult(false, "Root command exited with code $code.")
            }
        } catch (e: Exception) {
            InjectionResult(false, "Injection failed: ${e.message}")
        }
    }
}
