package xyz.glacierclient.launcher.service

import java.io.DataOutputStream

/**
 * Android analogue of GlacierLauncher.Services.InjectionService.
 *
 * IMPORTANT — this is NOT a 1:1 port of Windows DLL injection, because the
 * mechanism it relies on doesn't exist on Android:
 *   - Windows: CreateRemoteThread + LoadLibrary into minecraft.exe's address
 *     space, giving Latite/Flarial/OderSo full in-process hooking.
 *   - Android: apps are sandboxed per-UID; there is no supported (or even
 *     unsupported-but-reliable) API to load a foreign .so into another app's
 *     process without root, and Minecraft Bedrock for Android does not expose
 *     a mod-loader hook the way the Windows DLL clients target.
 *
 * What this service actually does, best-effort, only on rooted devices:
 * pushes a native library into Minecraft's app-specific storage and (on
 * devices where `su` is available) attempts a `run-as`/root restart of the
 * target so a wrapped native loader picks it up next launch. This mirrors
 * how community Bedrock-Android mod loaders (Bedrock Sandbox Mod-Loader
 * style tools) work today, and is inherently fragile across Minecraft
 * versions/devices. Non-rooted devices get a clear "not supported" result.
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
                message = "Client injection requires root on Android — Windows-style DLL " +
                    "injection has no non-root equivalent here. Install Magisk/root and retry.",
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
