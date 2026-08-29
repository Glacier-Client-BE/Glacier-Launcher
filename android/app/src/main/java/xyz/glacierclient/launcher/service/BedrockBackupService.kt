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
}
