package xyz.glacierclient.launcher.service

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.net.URL
import java.util.zip.ZipEntry
import java.util.zip.ZipFile

/**
 * Partial Android analogue of Services/ModpackInstallService.cs — Modrinth
 * .mrpack only, and deliberately does NOT install a mod loader (Fabric/
 * Quilt/Forge/NeoForge). Desktop's loader step either calls a loader API
 * (Fabric/Quilt) or downloads and runs a Java installer jar as a
 * subprocess (Forge/NeoForge) — the subprocess path in particular has no
 * Android equivalent at all (Pojav embeds its JVM in-process via JNI, there
 * is no spawnable `java` executable), and building the Fabric/Quilt-only
 * half without Forge/NeoForge would make one loader silently work and the
 * others silently not, which is worse than being upfront that loader
 * install isn't done here yet. So this only gets a pack's overrides + mod
 * files into a real new instance (JavaInstanceService) — genuinely useful
 * on its own, since Pojav's own version-install UI already handles adding
 * a Fabric/Forge profile afterward — and reports that gap back to the
 * caller (js/modpackinstall.js) rather than claiming full success.
 *
 * CurseForge modpacks aren't supported here yet either: unlike Modrinth's
 * plain per-file download URLs, CF manifests only carry project/file ID
 * pairs that need resolving through CurseForgeService's own API-keyed
 * calls (js/curseforge.js) — duplicating that natively wasn't worth it for
 * a first pass when Modrinth covers the same use case.
 */
object ModpackInstallService {

    fun installModrinthPack(context: Context, mrpackUrl: String, packName: String): String {
        val tmpFile = File.createTempFile("modpack", ".mrpack", context.cacheDir)
        return try {
            URL(mrpackUrl).openStream().use { input -> tmpFile.outputStream().use { input.copyTo(it) } }

            val index = ZipFile(tmpFile).use { zip ->
                val entry = zip.getEntry("modrinth.index.json")
                    ?: return errorResult("Not a Modrinth pack (no modrinth.index.json).")
                JSONObject(zip.getInputStream(entry).use { it.readBytes().toString(Charsets.UTF_8) })
            }

            val name = index.optString("name", packName).ifBlank { packName }
            val created = JSONObject(JavaInstanceService.create(name, ""))
            val instanceId = created.getString("id")
            val instanceDir = JavaInstanceService.directoryFor(instanceId)
                ?: return errorResult("Could not resolve the new instance's directory.")

            extractOverrides(tmpFile, instanceDir)

            var downloaded = 0
            var failed = 0
            val manualDownloads = JSONArray()
            val files = index.optJSONArray("files") ?: JSONArray()
            for (i in 0 until files.length()) {
                val f = files.getJSONObject(i)
                val path = f.optString("path")
                if (path.isBlank()) continue
                val clientEnv = f.optJSONObject("env")?.optString("client")
                if (clientEnv == "unsupported") continue

                val downloads = f.optJSONArray("downloads")
                val url = if (downloads != null && downloads.length() > 0) downloads.getString(0) else ""
                if (url.isBlank()) continue

                val dest = File(instanceDir, path.replace('/', File.separatorChar))
                runCatching {
                    dest.parentFile?.mkdirs()
                    URL(url).openStream().use { input -> dest.outputStream().use { input.copyTo(it) } }
                    downloaded++
                }.onFailure {
                    failed++
                    manualDownloads.put(path)
                }
            }

            val deps = index.optJSONObject("dependencies")
            val mcVersion = deps?.optString("minecraft").orEmpty()
            val loader = listOf("fabric-loader" to "Fabric", "quilt-loader" to "Quilt", "forge" to "Forge", "neoforge" to "NeoForge")
                .firstOrNull { (key, _) -> deps?.has(key) == true }?.second
            if (mcVersion.isNotBlank()) JavaInstanceService.setVersion(instanceId, mcVersion)

            JSONObject().apply {
                put("success", true)
                put("instanceId", instanceId)
                put("instanceName", name)
                put("minecraftVersion", mcVersion)
                put("loader", loader ?: "")
                put("downloadedFiles", downloaded)
                put("failedFiles", failed)
                put("manualDownloads", manualDownloads)
            }.toString()
        } catch (e: Exception) {
            errorResult("Modpack install failed: ${e.message}")
        } finally {
            tmpFile.delete()
        }
    }

    private fun errorResult(message: String) = JSONObject().put("success", false).put("message", message).toString()

    private fun extractOverrides(mrpackFile: File, instanceDir: File) {
        ZipFile(mrpackFile).use { zip ->
            val entries = zip.entries()
            while (entries.hasMoreElements()) {
                val entry: ZipEntry = entries.nextElement()
                val path = entry.name.replace('\\', '/')
                if (!path.startsWith("overrides/") || entry.isDirectory) continue
                val rel = path.removePrefix("overrides/")
                if (rel.isBlank()) continue
                val dest = File(instanceDir, rel.replace('/', File.separatorChar))
                dest.parentFile?.mkdirs()
                zip.getInputStream(entry).use { input -> dest.outputStream().use { input.copyTo(it) } }
            }
        }
    }
}
