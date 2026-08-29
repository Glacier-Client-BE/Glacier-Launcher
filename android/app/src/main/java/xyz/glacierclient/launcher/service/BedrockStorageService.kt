package xyz.glacierclient.launcher.service

import android.content.Context
import android.net.Uri
import androidx.core.content.edit
import androidx.documentfile.provider.DocumentFile
import org.json.JSONArray
import org.json.JSONObject

/**
 * Android analogue of Pages/Home.Worlds.cs's world-listing half (not the
 * launch/export/delete half — those need write access and are a follow-up,
 * see android/README.md). Scoped storage means there's no java.io.File path
 * to Bedrock's shared data the way desktop just walks a folder — the user
 * grants a Storage Access Framework tree once (MainActivity's
 * requestBedrockStorageAccess()), and everything here reads through that
 * DocumentFile tree instead.
 *
 * Only level.dat (via BedrockNbt) and levelname.txt are read — Bedrock's
 * actual per-world chunk data in db/ is a LevelDB store this intentionally
 * does not parse; nothing about listing worlds needs it.
 */
object BedrockStorageService {
    private const val PREFS = "glacier_settings"
    private const val KEY_TREE_URI = "bedrock_storage_uri"

    fun onAccessGranted(context: Context, treeUri: Uri) {
        context.getSharedPreferences(PREFS, 0).edit { putString(KEY_TREE_URI, treeUri.toString()) }
    }

    fun hasAccess(context: Context): Boolean {
        val uri = savedTreeUri(context) ?: return false
        return context.contentResolver.persistedUriPermissions.any { it.uri == uri && it.isReadPermission }
    }

    private fun savedTreeUri(context: Context): Uri? =
        context.getSharedPreferences(PREFS, 0).getString(KEY_TREE_URI, null)?.let { Uri.parse(it) }

    private fun rootDocument(context: Context): DocumentFile? {
        val uri = savedTreeUri(context) ?: return null
        return DocumentFile.fromTreeUri(context, uri)
    }

    // The user might pick games/com.mojang itself, one of its child folders
    // directly, or a parent folder above games/ — walk down to find a named
    // child instead of requiring one exact folder, since SAF's tree picker
    // doesn't let this app suggest a starting path on most devices/OEM file
    // pickers. Shared by worlds, packs, and BedrockBackupService since
    // they're all read through the same com.mojang root.
    fun findComMojangChild(context: Context, name: String): DocumentFile? {
        val root = rootDocument(context) ?: return null
        return findChildDir(root, name)
    }

    private fun findChildDir(root: DocumentFile, name: String): DocumentFile? {
        if (root.name == name) return root
        root.findFile(name)?.let { return it }
        root.findFile("games")?.findFile("com.mojang")?.findFile(name)?.let { return it }
        root.findFile("com.mojang")?.findFile(name)?.let { return it }
        return null
    }

    fun listWorlds(context: Context): String {
        val root = rootDocument(context) ?: return "[]"
        val worldsDir = findChildDir(root, "minecraftWorlds") ?: return "[]"
        val result = JSONArray()

        for (dir in worldsDir.listFiles()) {
            if (!dir.isDirectory) continue
            var name = dir.findFile("levelname.txt")
                ?.let { readText(context, it.uri) }
                ?.trim()
                .orEmpty()
            var lastPlayed = 0L
            var gameType = -1

            dir.findFile("level.dat")?.let { levelDat ->
                runCatching {
                    val bytes = context.contentResolver.openInputStream(levelDat.uri)?.use { it.readBytes() }
                    if (bytes != null && bytes.size > 8) {
                        val nbt = BedrockNbt.parseLevelDat(bytes)
                        if (name.isBlank()) name = nbt.getString("LevelName").orEmpty()
                        lastPlayed = nbt.getLong("LastPlayed") ?: 0L
                        gameType = nbt.getInt("GameType") ?: -1
                    }
                }
            }
            if (name.isBlank()) name = dir.name.orEmpty()

            val sizeBytes = dir.listFiles().sumOf { if (it.isFile) it.length() else 0L }
            val icon = dir.findFile("world_icon.jpeg")

            result.put(
                JSONObject().apply {
                    put("id", dir.name)
                    put("name", name)
                    put("lastPlayed", lastPlayed)
                    put("gameType", gameType)
                    put("sizeBytes", sizeBytes)
                    put("iconUri", icon?.uri?.toString() ?: "")
                    put("folderUri", dir.uri.toString())
                },
            )
        }
        return result.toString()
    }

    private fun readText(context: Context, uri: Uri): String? =
        runCatching { context.contentResolver.openInputStream(uri)?.use { it.readBytes().toString(Charsets.UTF_8) } }
            .getOrNull()

    // Android analogue of Services/BedrockPackService.cs — same folder
    // names, same manifest.json "header.name" read (falling back to the
    // folder name for a "pack.xyz" language-key reference, which needs the
    // pack's own texts/en_US.lang to resolve and isn't worth it here either).
    private fun packDirName(kind: String) = when (kind) {
        "behavior" -> "behavior_packs"
        "skin" -> "skin_packs"
        "resource-dev" -> "development_resource_packs"
        "behavior-dev" -> "development_behavior_packs"
        "skin-dev" -> "development_skin_packs"
        else -> "resource_packs"
    }

    private val packIconNames = listOf("pack_icon.png", "pack_icon.jpg", "pack_icon.jpeg")

    fun listPacks(context: Context, kind: String): String {
        val root = rootDocument(context) ?: return "[]"
        val packsDir = findChildDir(root, packDirName(kind)) ?: return "[]"
        val result = JSONArray()

        for (dir in packsDir.listFiles()) {
            if (!dir.isDirectory) continue
            val name = readPackName(context, dir) ?: dir.name.orEmpty()
            val sizeBytes = dir.listFiles().sumOf { if (it.isFile) it.length() else 0L }
            val icon = packIconNames.firstNotNullOfOrNull { dir.findFile(it) }

            result.put(
                JSONObject().apply {
                    put("id", dir.name)
                    put("name", name)
                    put("kind", kind)
                    put("sizeBytes", sizeBytes)
                    put("iconUri", icon?.uri?.toString() ?: "")
                },
            )
        }
        return result.toString()
    }

    // Android equivalent of desktop's OpenXxxFolder shortcuts
    // (Process.Start on the real path) — SAF has no "reveal in file
    // manager" primitive, so this hands the folder's own content:// Uri to
    // whatever app on the device declares it can view a document tree
    // (most file managers do); best-effort, same as desktop's own shortcut
    // failing quietly if no Explorer-equivalent is registered.
    fun folderUri(context: Context, name: String): String? =
        findComMojangChild(context, name)?.uri?.toString()

    private fun readPackName(context: Context, packDir: DocumentFile): String? {
        val manifest = packDir.findFile("manifest.json") ?: return null
        val text = readText(context, manifest.uri) ?: return null
        return runCatching {
            val name = JSONObject(text).optJSONObject("header")?.optString("name")
            if (name.isNullOrBlank() || name.startsWith("pack.", ignoreCase = true)) null else name
        }.getOrNull()
    }

    // Android analogue of Services/BedrockScreenshotService.cs's real half —
    // in-game captures under com.mojang/Screenshots/<xbox-user-id>/*.jpeg.
    // Xbox Game Bar's Captures folder (Win+Alt+PrtScn) is Windows-only, so
    // that merge source is skipped entirely rather than faked.
    fun listScreenshots(context: Context): String {
        val root = rootDocument(context) ?: return "[]"
        val screenshotsDir = findChildDir(root, "Screenshots") ?: return "[]"
        val result = JSONArray()
        collectScreenshots(screenshotsDir, result)
        return result.toString()
    }

    private fun collectScreenshots(dir: DocumentFile, out: JSONArray) {
        for (child in dir.listFiles()) {
            if (child.isDirectory) {
                collectScreenshots(child, out)
            } else if (child.isFile && (child.name?.endsWith(".jpeg", ignoreCase = true) == true)) {
                out.put(
                    JSONObject().apply {
                        put("name", child.name)
                        put("uri", child.uri.toString())
                        put("sizeBytes", child.length())
                        put("modifiedAt", child.lastModified())
                    },
                )
            }
        }
    }
}
