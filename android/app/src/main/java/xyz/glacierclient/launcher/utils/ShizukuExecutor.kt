@file:Suppress("unused")
package xyz.glacierclient.launcher.utils

import android.content.ComponentName
import android.content.pm.PackageManager
import android.os.IBinder
import kotlinx.coroutines.suspendCancellableCoroutine
import rikka.shizuku.Shizuku
import xyz.glacierclient.launcher.BuildConfig
import xyz.glacierclient.launcher.shizuku.IShizukuUserService
import xyz.glacierclient.launcher.shizuku.ShizukuUserService
import kotlin.coroutines.resume

/**
 * Alternative privileged-execution path for ClientInjectionService.kt,
 * usable when the device isn't rooted (ClientInjectionService.isRootAvailable()
 * == false) but the user is running Shizuku (dev.rikka.shizuku — official,
 * open-source, on mavenCentral) — either the standalone Shizuku Manager app
 * started over `adb shell` (no root needed at all) or Shizuku running via
 * root. This binds Shizuku's own documented UserService mechanism
 * (Shizuku.bindUserService -> ShizukuUserService.kt running with Shizuku's
 * elevated UID) rather than reflecting into any Shizuku-internal API.
 *
 * Root stays the PRIMARY path everywhere it's already wired (see
 * ClientInjectionService.attemptInject/isRootAvailable) — this is
 * deliberately a fallback, tried only after a root attempt has already
 * failed or been confirmed unavailable.
 */
object ShizukuExecutor {

    data class ExecResult(val exitCode: Int, val stdout: String, val stderr: String)

    private val userServiceArgs = Shizuku.UserServiceArgs(
        ComponentName(BuildConfig.APPLICATION_ID, ShizukuUserService::class.java.name)
    )
        .daemon(false)
        .processNameSuffix("shizuku")
        .debuggable(BuildConfig.DEBUG)
        .version(1)

    private var boundService: IShizukuUserService? = null

    /** True once Shizuku (manager app or the `adb shell app_process` service) is running and reachable. */
    fun isShizukuAvailable(): Boolean = try {
        Shizuku.pingBinder()
    } catch (e: Throwable) {
        // Shizuku.pingBinder() throws (not just returns false) when the
        // Shizuku process/manager isn't installed at all — that's a normal,
        // expected "not available" outcome here, not an error to surface.
        false
    }

    /** Whether this app currently holds Shizuku's shell-execution permission. */
    fun hasPermission(): Boolean {
        if (!isShizukuAvailable()) return false
        return try {
            Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED
        } catch (e: Throwable) {
            false
        }
    }

    /**
     * Requests Shizuku's runtime permission. Shizuku surfaces its own
     * system-style permission dialog — the result arrives via
     * Shizuku.addRequestPermissionResultListener, which the caller (e.g.
     * MainActivity's AndroidBridge) should register once at startup and
     * forward to JS the same way other sign-in flows report back
     * asynchronously.
     */
    fun requestPermission(requestCode: Int) {
        if (isShizukuAvailable() && !hasPermission()) {
            Shizuku.requestPermission(requestCode)
        }
    }

    private suspend fun bind(): IShizukuUserService = suspendCancellableCoroutine { cont ->
        boundService?.let { cont.resume(it); return@suspendCancellableCoroutine }
        Shizuku.bindUserService(userServiceArgs, object : android.content.ServiceConnection {
            override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                val service = IShizukuUserService.Stub.asInterface(binder)
                boundService = service
                if (cont.isActive) cont.resume(service)
            }
            override fun onServiceDisconnected(name: ComponentName) {
                boundService = null
            }
        })
    }

    /**
     * Runs [command] inside Shizuku's elevated UserService process instead
     * of `su`. Requires [hasPermission] to already be true — callers should
     * gate on that (and on [isShizukuAvailable]) the same way
     * ClientInjectionService gates its root path on isRootAvailable().
     */
    suspend fun exec(command: Array<String>): ExecResult {
        check(hasPermission()) { "Shizuku permission not granted." }
        val service = bind()
        val raw = service.exec(command)
        val parts = raw.split("\n---STDERR---\n", limit = 2)
        val head = parts[0]
        val stderr = parts.getOrElse(1) { "" }
        val exitCode = head.substringBefore('\n').toIntOrNull() ?: -1
        val stdout = head.substringAfter('\n', "")
        return ExecResult(exitCode, stdout, stderr)
    }

    /**
     * Convenience wrapper mirroring ClientInjectionService.attemptInject's
     * shape, so the service can try root first and fall back to this with
     * near-identical call sites.
     */
    suspend fun attemptInjectViaShizuku(nativeLibPath: String, targetPackage: String): Pair<Boolean, String> {
        if (!isShizukuAvailable()) {
            return false to "Shizuku is not running. Install Shizuku and start it (via its Manager app over ADB, or with root) to use this fallback."
        }
        if (!hasPermission()) {
            return false to "Shizuku permission not granted yet — call requestPermission() first."
        }
        return try {
            val mkdir = exec(arrayOf("run-as", targetPackage, "mkdir", "-p", "files/glacier_mods"))
            if (mkdir.exitCode != 0) {
                return false to "Shizuku mkdir failed: ${mkdir.stderr.ifBlank { mkdir.stdout }}"
            }
            val copy = exec(arrayOf("cp", nativeLibPath, "/data/data/$targetPackage/files/glacier_mods/"))
            if (copy.exitCode == 0) {
                true to "Staged client library for $targetPackage via Shizuku. Restart Minecraft to apply."
            } else {
                false to "Shizuku copy failed: ${copy.stderr.ifBlank { copy.stdout }}"
            }
        } catch (e: Exception) {
            false to "Shizuku execution failed: ${e.message}"
        }
    }
}
