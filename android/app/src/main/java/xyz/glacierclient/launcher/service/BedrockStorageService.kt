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

    // The user might pick games/com.mojang itself, its minecraftWorlds child
    // directly, or a parent folder above games/ — walk down to find it
    // instead of requiring one exact folder, since SAF's tree picker doesn't
    // let this app suggest a starting path on most devices/OEM file pickers.
    private fun findWorldsDir(root: DocumentFile): DocumentFile? {
        if (root.name == "minecraftWorlds") return root
        root.findFile("minecraftWorlds")?.let { return it }
        root.findFile("games")?.findFile("com.mojang")?.findFile("minecraftWorlds")?.let { return it }
        root.findFile("com.mojang")?.findFile("minecraftWorlds")?.let { return it }
        return null
    }

    fun listWorlds(context: Context): String {
        val root = rootDocument(context) ?: return "[]"
        val worldsDir = findWorldsDir(root) ?: return "[]"
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
}
