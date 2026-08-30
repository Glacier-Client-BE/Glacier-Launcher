package xyz.glacierclient.launcher.service

import android.content.Context
import androidx.documentfile.provider.DocumentFile
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/**
 * Android analogue of Services/BedrockBackupService.cs. Desktop writes
 * backup zips to an app-owned folder (%USERPROFILE%/Glacier Launcher/
 * bedrock-backups), not under com.mojang itself — that maps directly to
 * this app's own external-files dir, so unlike reading worlds/packs, the
 * zip *destination* needs no SAF permission at all; only the *source*
 * folders (read through BedrockStorageService's already-granted com.mojang
 * tree) do.
 */
object BedrockBackupService {

    // Same folder list as desktop's ManagedFolders, minus "minecraftpe"
    // (Minecraft's own local-settings folder) — not exposed by
    // BedrockStorageService's SAF-lookup helper here and low value compared
    // to worlds/packs for a first pass.
    private val MANAGED_FOLDERS = listOf(
        "minecraftWorlds", "resource_packs", "behavior_packs", "skin_packs",
        "development_resource_packs", "development_behavior_packs", "development_skin_packs",
    )

    private fun backupsDir(context: Context): File =
        File(context.getExternalFilesDir(null), "bedrock-backups").apply { mkdirs() }

    fun listBackups(context: Context): String {
        val dir = backupsDir(context)
        val result = JSONArray()
        dir.listFiles { f -> f.isFile && f.name.endsWith(".zip") }
            ?.sortedByDescending { it.lastModified() }
            ?.forEach { file ->
                result.put(
                    JSONObject().apply {
                        put("name", file.nameWithoutExtension)
                        put("fileName", file.name)
                        put("sizeBytes", file.length())
                        put("createdAt", file.lastModified())
                    },
                )
            }
        return result.toString()
    }

    /** Returns a JSON {"success": bool, "message": string}. */
    fun createBackup(context: Context): String {
        val timestamp = SimpleDateFormat("yyyy-MM-dd_HH-mm-ss", Locale.US).format(Date())
        val dest = File(backupsDir(context), "backup_$timestamp.zip")
        var foundAny = false

        return try {
            ZipOutputStream(dest.outputStream()).use { zos ->
                for (folder in MANAGED_FOLDERS) {
                    val dir = BedrockStorageService.findComMojangChild(context, folder) ?: continue
                    foundAny = true
                    addDocumentToZip(context, dir, folder, zos)
                }
            }
            if (!foundAny) {
                dest.delete()
                JSONObject().put("success", false).put("message", "Nothing to back up yet — no worlds or packs found.").toString()
            } else {
                JSONObject().put("success", true).put("message", "Backup saved: ${dest.name}").toString()
            }
        } catch (e: Exception) {
            dest.delete()
            JSONObject().put("success", false).put("message", "Backup failed: ${e.message}").toString()
        }
    }

    private fun addDocumentToZip(context: Context, doc: DocumentFile, entryPath: String, zos: ZipOutputStream) {
        if (doc.isDirectory) {
            for (child in doc.listFiles()) {
                val childName = child.name ?: continue
                addDocumentToZip(context, child, "$entryPath/$childName", zos)
            }
        } else if (doc.isFile) {
            runCatching {
                context.contentResolver.openInputStream(doc.uri)?.use { input ->
                    zos.putNextEntry(ZipEntry(entryPath))
                    input.copyTo(zos)
                    zos.closeEntry()
                }
            }
        }
    }

    fun deleteBackup(context: Context, fileName: String): Boolean {
        // Reject anything that isn't a plain file name (no path separators),
        // so this can't be pointed outside backupsDir.
        if (fileName.contains('/') || fileName.contains('\\')) return false
        val file = File(backupsDir(context), fileName)
        return file.exists() && file.delete()
    }

    // ── Per-world backups ───────────────────────────────────────────────
    //
    // The zip-everything path above is desktop's model (one global "Backup
    // now" covering every managed folder), which is the right default for a
    // single-machine app but wasteful here: a device with a dozen worlds
    // would re-zip all of them just to protect the one someone's about to
    // edit in the level.dat/pack tools. LeviLaunchroid's InstanceBackupManager
    // scopes a backup to one *instance* (its multi-install concept); Bedrock
    // on Android only ever has one com.mojang tree, so the equivalent unit
    // here is a single world folder, keyed the same way listWorlds/
    // LevelDatService already do — by the world's own directory name.

    private fun worldBackupsDir(context: Context, worldId: String): File =
        File(backupsDir(context), "worlds/${sanitizeWorldId(worldId)}").apply { mkdirs() }

    // worldId comes from a folder name BedrockStorageService already read
    // off disk, so it's trustworthy, but this still becomes a path segment
    // below — reject traversal rather than trust it twice.
    private fun sanitizeWorldId(worldId: String): String {
        require(worldId.isNotBlank() && !worldId.contains('/') && !worldId.contains('\\') && worldId != "..") {
            "Invalid world id"
        }
        return worldId
    }

    fun listWorldBackups(context: Context, worldId: String): String {
        val dir = try { worldBackupsDir(context, worldId) } catch (e: Exception) { return "[]" }
        val result = JSONArray()
        dir.listFiles { f -> f.isFile && f.name.endsWith(".zip") }
            ?.sortedByDescending { it.lastModified() }
            ?.forEach { file ->
                result.put(
                    JSONObject().apply {
                        put("name", file.nameWithoutExtension)
                        put("fileName", file.name)
                        put("sizeBytes", file.length())
                        put("createdAt", file.lastModified())
                    },
                )
            }
        return result.toString()
    }

    /** Zips just [worldId]'s own folder under minecraftWorlds/. Returns {"success","message"}. */
    fun createWorldBackup(context: Context, worldId: String): String {
        val worldDir = BedrockStorageService.worldDir(context, worldId)
            ?: return JSONObject().put("success", false).put("message", "World not found.").toString()

        val dest = try {
            val timestamp = SimpleDateFormat("yyyy-MM-dd_HH-mm-ss", Locale.US).format(Date())
            File(worldBackupsDir(context, worldId), "backup_$timestamp.zip")
        } catch (e: Exception) {
            return JSONObject().put("success", false).put("message", e.message ?: "Invalid world id.").toString()
        }

        return try {
            ZipOutputStream(dest.outputStream()).use { zos ->
                addDocumentToZip(context, worldDir, worldDir.name ?: worldId, zos)
            }
            JSONObject().put("success", true).put("message", "Backup saved: ${dest.name}").toString()
        } catch (e: Exception) {
            dest.delete()
            JSONObject().put("success", false).put("message", "Backup failed: ${e.message}").toString()
        }
    }

    fun deleteWorldBackup(context: Context, worldId: String, fileName: String): Boolean {
        if (fileName.contains('/') || fileName.contains('\\')) return false
        val dir = try { worldBackupsDir(context, worldId) } catch (e: Exception) { return false }
        val file = File(dir, fileName)
        return file.exists() && file.delete()
    }

    /**
     * Restores a per-world backup zip over the live world folder. The zip's
     * single top-level entry is the world's own folder name (see
     * createWorldBackup) — existing files are overwritten in place and
     * anything the zip doesn't mention is left untouched, same
     * read-modify-rewrite caution LevelDatService takes rather than
     * deleting the folder first and risking a half-restored world if
     * extraction fails partway through.
     */
    fun restoreWorldBackup(context: Context, worldId: String, fileName: String): String {
        if (fileName.contains('/') || fileName.contains('\\')) {
            return JSONObject().put("success", false).put("message", "Invalid backup file name.").toString()
        }
        val dir = try { worldBackupsDir(context, worldId) } catch (e: Exception) {
            return JSONObject().put("success", false).put("message", e.message ?: "Invalid world id.").toString()
        }
        val zipFile = File(dir, fileName).takeIf { it.isFile }
            ?: return JSONObject().put("success", false).put("message", "Backup not found.").toString()
        val worldDir = BedrockStorageService.worldDir(context, worldId)
            ?: return JSONObject().put("success", false).put("message", "World not found.").toString()

        return try {
            java.util.zip.ZipFile(zipFile).use { zip ->
                val entries = zip.entries()
                while (entries.hasMoreElements()) {
                    val entry = entries.nextElement()
                    if (entry.isDirectory) continue
                    // Drop the leading "<worldFolderName>/" segment the zip
                    // was written with, so entries land relative to worldDir.
                    val relative = entry.name.substringAfter('/', missingDelimiterValue = "")
                    if (relative.isBlank() || relative.contains("..")) continue
                    writeIntoDocumentTree(context, worldDir, relative, zip.getInputStream(entry))
                }
            }
            JSONObject().put("success", true).put("message", "World restored from ${zipFile.name}").toString()
        } catch (e: Exception) {
            JSONObject().put("success", false).put("message", "Restore failed: ${e.message}").toString()
        }
    }

    private fun writeIntoDocumentTree(context: Context, root: DocumentFile, relativePath: String, input: java.io.InputStream) {
        val segments = relativePath.split('/')
        var dir = root
        for (i in 0 until segments.size - 1) {
            val name = segments[i]
            dir = dir.findFile(name)?.takeIf { it.isDirectory } ?: dir.createDirectory(name) ?: return
        }
        val fileName = segments.last()
        val target = dir.findFile(fileName) ?: dir.createFile("application/octet-stream", fileName) ?: return
        context.contentResolver.openOutputStream(target.uri, "wt")?.use { output -> input.copyTo(output) }
    }
}
