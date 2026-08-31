package xyz.glacierclient.launcher.service

import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import org.json.JSONObject
import java.io.File

/**
 * Moves Bedrock's worlds/packs off the SAF-granted com.mojang tree, either
 * into this app's own storage or onto a freshly-granted SAF tree.
 *
 * The concrete motivation: [BedrockStorageService.accessBlockedByPlatform]
 * means a device that upgrades past Android 13 can never grant a *new*
 * Android/data SAF tree again, but an *existing* grant made on an older
 * OS version keeps working until it's revoked. A user in that position has
 * no warning before the access disappears for good (uninstalling and
 * reinstalling Minecraft, clearing this app's data, or the OS revoking it
 * on its own), so this gives them a one-way-out: copy everything into a
 * folder this app can always reach — [GlacierStorage]'s own visible
 * folder, no SAF or special permission required to read it back — before
 * that happens. The reverse direction (app storage → a newly-picked SAF
 * tree) exists for the ordinary case: relocating onto different storage
 * the user picks by hand, same "pick a folder" flow BedrockStorageService's
 * own picker already uses.
 *
 * Same verify-then-delete-source shape as a competent migration should
 * have: every file is copied and its byte length checked against the
 * source before the source copy is removed, and nothing is deleted until
 * every file in the folder has verified clean. A failed/partial run always
 * leaves the source intact.
 */
object BedrockStorageMigrationService {

    // Same folder set BedrockBackupService already treats as "the managed
    // Bedrock data" — kept in one place would be nicer, but the two classes
    // load in different orders during tests and this list is small enough
    // that duplicating it beats introducing a shared object just for this.
    private val MANAGED_FOLDERS = listOf(
        "minecraftWorlds", "resource_packs", "behavior_packs", "skin_packs",
        "development_resource_packs", "development_behavior_packs", "development_skin_packs",
    )

    data class MigrationResult(
        val success: Boolean,
        val message: String,
        val filesCopied: Int = 0,
        val bytesCopied: Long = 0L,
    )

    private fun MigrationResult.toJson(): String = JSONObject()
        .put("success", success)
        .put("message", message)
        .put("filesCopied", filesCopied)
        .put("bytesCopied", bytesCopied)
        .toString()

    private fun migratedAppDir(context: Context): File =
        File(GlacierStorage.preferredRoot(context), "bedrock-migrated").apply { mkdirs() }

    /**
     * Copies every managed Bedrock folder from the currently-granted SAF
     * tree into this app's own storage, verifying each file, then deletes
     * the SAF-side copy once the whole run verifies clean. [onProgress]
     * receives 0..100 as files are copied; call off the main thread.
     */
    fun migrateToAppStorage(context: Context, deleteSource: Boolean, onProgress: ((Int) -> Unit)? = null): String {
        var anyFolder = false
        val copied = ArrayList<Pair<DocumentFile, File>>()
        var filesCopied = 0
        var bytesCopied = 0L

        try {
            val plan = MANAGED_FOLDERS.mapNotNull { name ->
                BedrockStorageService.findComMojangChild(context, name)?.let { name to it }
            }
            if (plan.isEmpty()) return MigrationResult(false, "Nothing to migrate — no worlds or packs found.").toJson()

            val totalFiles = plan.sumOf { (_, dir) -> countDocumentFiles(dir) }.coerceAtLeast(1)
            var processed = 0

            for ((name, sourceDir) in plan) {
                anyFolder = true
                val destDir = File(migratedAppDir(context), name).apply { mkdirs() }
                processed = copyDocumentTree(context, sourceDir, destDir, copied) { fileBytes, ok ->
                    processed++
                    if (ok) { filesCopied++; bytesCopied += fileBytes }
                    onProgress?.invoke((processed * 100 / totalFiles).coerceIn(0, 100))
                    processed
                }
            }
            if (!anyFolder) return MigrationResult(false, "Nothing to migrate — no worlds or packs found.").toJson()

            // Verify: every copied pair must exist at the destination with a
            // matching length before anything on the SAF side is touched.
            val allVerified = copied.all { (src, dest) -> dest.isFile && dest.length() == src.length() }
            if (!allVerified) {
                return MigrationResult(false, "Copy finished but some files didn't verify — nothing was deleted from the source.", filesCopied, bytesCopied).toJson()
            }

            if (deleteSource) {
                copied.forEach { (src, _) -> runCatching { src.delete() } }
            }
            onProgress?.invoke(100)
            return MigrationResult(
                true,
                if (deleteSource) "Migrated $filesCopied files to app storage and cleared the source." else "Copied $filesCopied files to app storage.",
                filesCopied,
                bytesCopied,
            ).toJson()
        } catch (e: Exception) {
            return MigrationResult(false, "Migration failed: ${e.message}", filesCopied, bytesCopied).toJson()
        }
    }

    /**
     * Copies previously-migrated app storage onto a freshly-picked SAF
     * tree (from [newTreeUri], already granted by the caller's own picker
     * flow — see BedrockStorageService.bedrockPickerInitialUri/onAccessGranted
     * for how that grant is obtained). Source verified the same way as the
     * other direction before any deletion.
     */
    fun migrateAppStorageToSaf(context: Context, newTreeUri: Uri, deleteSource: Boolean, onProgress: ((Int) -> Unit)? = null): String {
        val root = migratedAppDir(context)
        if (!root.isDirectory || root.listFiles().isNullOrEmpty()) {
            return MigrationResult(false, "No migrated data in app storage to move.").toJson()
        }
        val destRoot = runCatching { DocumentFile.fromTreeUri(context, newTreeUri) }.getOrNull()
            ?: return MigrationResult(false, "Couldn't open the destination folder.").toJson()

        var filesCopied = 0
        var bytesCopied = 0L
        val copiedFiles = ArrayList<Pair<File, DocumentFile>>()

        try {
            val totalFiles = countFiles(root).coerceAtLeast(1)
            var processed = 0

            for (folder in root.listFiles() ?: emptyArray()) {
                if (!folder.isDirectory) continue
                val destDir = destRoot.findFile(folder.name)?.takeIf { it.isDirectory }
                    ?: destRoot.createDirectory(folder.name) ?: continue
                processed = copyFileTreeToDocument(context, folder, destDir, copiedFiles) { fileBytes, ok ->
                    processed++
                    if (ok) { filesCopied++; bytesCopied += fileBytes }
                    onProgress?.invoke((processed * 100 / totalFiles).coerceIn(0, 100))
                    processed
                }
            }

            val allVerified = copiedFiles.all { (src, dest) -> dest.isFile && dest.length() == src.length() }
            if (!allVerified) {
                return MigrationResult(false, "Copy finished but some files didn't verify — the app-storage copy was kept.", filesCopied, bytesCopied).toJson()
            }
            if (deleteSource) root.deleteRecursively()
            onProgress?.invoke(100)
            return MigrationResult(
                true,
                if (deleteSource) "Migrated $filesCopied files onto the new folder and cleared app storage." else "Copied $filesCopied files onto the new folder.",
                filesCopied,
                bytesCopied,
            ).toJson()
        } catch (e: Exception) {
            return MigrationResult(false, "Migration failed: ${e.message}", filesCopied, bytesCopied).toJson()
        }
    }

    /** Whether a previous app-storage migration left data behind (used to gate the "restore to SAF" UI). */
    fun hasMigratedAppData(context: Context): Boolean {
        val dir = migratedAppDir(context)
        return dir.isDirectory && !dir.listFiles().isNullOrEmpty()
    }

    private fun countDocumentFiles(doc: DocumentFile): Int =
        if (doc.isFile) 1 else doc.listFiles().sumOf { countDocumentFiles(it) }

    private fun countFiles(file: File): Int =
        if (file.isFile) 1 else (file.listFiles()?.sumOf { countFiles(it) } ?: 0)

    private fun copyDocumentTree(
        context: Context,
        source: DocumentFile,
        destDir: File,
        record: ArrayList<Pair<DocumentFile, File>>,
        onFile: (Long, Boolean) -> Int,
    ): Int {
        var processed = 0
        for (child in source.listFiles()) {
            val name = child.name ?: continue
            if (child.isDirectory) {
                processed = copyDocumentTree(context, child, File(destDir, name).apply { mkdirs() }, record, onFile)
            } else if (child.isFile) {
                val dest = File(destDir, name)
                val ok = runCatching {
                    context.contentResolver.openInputStream(child.uri)?.use { input ->
                        dest.outputStream().use { output -> input.copyTo(output) }
                    } ?: throw java.io.IOException("Couldn't open ${child.uri}")
                }.isSuccess
                if (ok) record.add(child to dest)
                processed = onFile(dest.length(), ok)
            }
        }
        return processed
    }

    private fun copyFileTreeToDocument(
        context: Context,
        source: File,
        destDir: DocumentFile,
        record: ArrayList<Pair<File, DocumentFile>>,
        onFile: (Long, Boolean) -> Int,
    ): Int {
        var processed = 0
        for (child in source.listFiles().orEmpty()) {
            if (child.isDirectory) {
                val childDest = destDir.findFile(child.name)?.takeIf { it.isDirectory }
                    ?: destDir.createDirectory(child.name) ?: continue
                processed = copyFileTreeToDocument(context, child, childDest, record, onFile)
            } else {
                val dest = destDir.findFile(child.name) ?: destDir.createFile("application/octet-stream", child.name)
                val ok = if (dest != null) {
                    runCatching {
                        context.contentResolver.openOutputStream(dest.uri, "wt")?.use { output ->
                            child.inputStream().use { input -> input.copyTo(output) }
                        } ?: throw java.io.IOException("Couldn't open ${dest.uri}")
                    }.isSuccess
                } else false
                if (ok && dest != null) record.add(child to dest)
                processed = onFile(child.length(), ok)
            }
        }
        return processed
    }
}
