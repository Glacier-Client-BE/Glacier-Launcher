package xyz.glacierclient.launcher.service

import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.zip.GZIPInputStream

/**
 * Android analogue of Services/LogService.cs. Redaction and the mclo.gs
 * upload itself happen in JS (js/logs.js), same fetch()-based pattern as
 * every other read-only API integration in this app (CurseForge, Modrinth,
 * news) — this side only needs real file access, which JS can't do.
 */
object LogService {

    private fun logsDir(): File? = JavaInstanceService.activeGameDir()?.let { File(it, "logs") }
    private fun crashReportsDir(): File? = JavaInstanceService.activeGameDir()?.let { File(it, "crash-reports") }

    /** Mirrors LogService.cs's ListLogs(): logs + crash reports for the active instance, newest first. */
    fun listLogs(): String {
        val out = JSONArray()
        fun scan(dir: File?, crash: Boolean) {
            if (dir == null || !dir.isDirectory) return
            for (f in dir.listFiles().orEmpty()) {
                val ext = f.extension.lowercase()
                if (ext != "log" && ext != "txt" && ext != "gz") continue
                out.put(
                    JSONObject().apply {
                        put("name", f.name)
                        put("path", f.absolutePath)
                        put("size", f.length())
                        put("modifiedAt", f.lastModified())
                        put("isCrash", crash)
                    },
                )
            }
        }
        scan(logsDir(), crash = false)
        scan(crashReportsDir(), crash = true)

        val list = (0 until out.length()).map { out.getJSONObject(it) }
        val sorted = list.sortedByDescending { it.getLong("modifiedAt") }
        val result = JSONArray()
        sorted.forEach { result.put(it) }
        return result.toString()
    }

    /** Mirrors ReadLogAsync(): reads a log file, decompressing .gz, returning the last [maxLines] lines. */
    fun readLog(path: String, maxLines: Int = 2000): String {
        val file = File(path)
        return try {
            val text = if (file.extension.equals("gz", ignoreCase = true)) {
                GZIPInputStream(file.inputStream()).bufferedReader().use { it.readText() }
            } else {
                file.readText()
            }
            val lines = text.split("\n")
            if (lines.size > maxLines) lines.takeLast(maxLines).joinToString("\n") else text
        } catch (e: Exception) {
            "Could not read log: ${e.message}"
        }
    }
}
