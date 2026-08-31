package xyz.glacierclient.launcher.shizuku

/**
 * Runs inside the elevated process Shizuku spawns for this app
 * (Shizuku.bindUserService) — NOT inside Glacier's normal app process. This
 * is Shizuku's official, documented mechanism for running app-supplied code
 * with its elevated UID, as opposed to reflecting into Shizuku's internal
 * process-launch APIs. See ShizukuExecutor.kt for the client side that
 * binds this and android:process / <service> registration notes in
 * AndroidManifest.xml.
 */
class ShizukuUserService : IShizukuUserService.Stub() {
    override fun exec(command: Array<out String>): String {
        return try {
            val process = ProcessBuilder(*command).redirectErrorStream(false).start()
            val stdout = process.inputStream.bufferedReader().readText()
            val stderr = process.errorStream.bufferedReader().readText()
            val exitCode = process.waitFor()
            "$exitCode\n$stdout\n---STDERR---\n$stderr"
        } catch (e: Exception) {
            "-1\n\n---STDERR---\n${e.message}"
        }
    }

    override fun destroy() {
        // Shizuku calls this (reserved transaction code) when unbinding —
        // nothing to clean up, this service holds no state between calls.
    }
}
