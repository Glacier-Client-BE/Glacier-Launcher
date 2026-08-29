package xyz.glacierclient.launcher.service

import android.content.Context
import androidx.documentfile.provider.DocumentFile
import org.json.JSONObject

/**
 * Android port of Services/LevelDatEditorService.cs — reads and edits a
 * Bedrock world's level.dat.
 *
 * Same field set as desktop (GameType, Difficulty, commandsEnabled,
 * Generator, the seed read-only, and the experiments toggles), and the same
 * read-modify-rewrite approach: the whole tag tree is parsed, a handful of
 * values are replaced, and everything else is written back untouched. See
 * BedrockNbt for why that round-trip is byte-exact.
 *
 * The one real divergence from desktop is how the file is replaced. Desktop
 * writes a .tmp beside it and does an atomic File.Move, which a half-written
 * level.dat can never survive. Storage Access Framework has no atomic
 * rename — a document's contents are replaced in place — so instead a
 * level.dat.glacierbak copy is written first and only then is the real file
 * overwritten. That is strictly weaker than an atomic swap, but it means an
 * interrupted write leaves a recoverable copy next to the world rather than
 * just a corrupt one.
 */
object LevelDatService {

    private const val BACKUP_NAME = "level.dat.glacierbak"

    private fun worldDir(context: Context, worldId: String): DocumentFile? =
        BedrockStorageService.worldDir(context, worldId)

    private fun levelDat(context: Context, worldId: String): DocumentFile? =
        worldDir(context, worldId)?.findFile("level.dat")?.takeIf { it.isFile }

    /** Reads the editable fields. Mirrors desktop's Load(). */
    fun summary(context: Context, worldId: String): String {
        val file = levelDat(context, worldId)
            ?: return error("This world has no level.dat.")

        return try {
            val bytes = context.contentResolver.openInputStream(file.uri)?.use { it.readBytes() }
                ?: return error("Couldn't read level.dat.")
            val root = BedrockNbt.parseLevelDat(bytes)

            val experiments = JSONObject()
            root.getCompound("experiments")?.let { compound ->
                for ((name, value) in compound.entries()) {
                    // Only the byte-valued toggles are surfaced; the compound
                    // also carries bookkeeping fields that aren't switches.
                    if (value is Byte) experiments.put(name, value.toInt() != 0)
                }
            }

            JSONObject()
                .put("ok", true)
                .put("worldName", root.getString("LevelName") ?: "")
                .put("gameType", root.getInt("GameType") ?: 0)
                .put("difficulty", root.getInt("Difficulty") ?: 2)
                .put("cheats", (root.getByte("commandsEnabled") ?: 0) != 0)
                .put("generator", root.getInt("Generator") ?: 1)
                .put("hasSeed", root.has("RandomSeed"))
                .put("seed", root.getLong("RandomSeed") ?: 0L)
                .put("experiments", experiments)
                .toString()
        } catch (e: Exception) {
            error("Couldn't parse level.dat: ${e.message}")
        }
    }

    /**
     * Applies the edited fields. Mirrors desktop's Save() — only the keys
     * present in [patchJson] are changed, so a caller can toggle cheats
     * without having to resend the whole summary.
     */
    fun save(context: Context, worldId: String, patchJson: String): String {
        val dir = worldDir(context, worldId) ?: return error("Couldn't open the world folder.")
        val file = dir.findFile("level.dat")?.takeIf { it.isFile }
            ?: return error("This world has no level.dat.")

        return try {
            val original = context.contentResolver.openInputStream(file.uri)?.use { it.readBytes() }
                ?: return error("Couldn't read level.dat.")
            val version = BedrockNbt.levelDatVersion(original)
            val root = BedrockNbt.parseLevelDat(original)
            val patch = JSONObject(patchJson)

            // Types must match what Bedrock expects for each key, not what
            // JSON happens to give us: GameType/Difficulty/Generator are
            // TAG_Int and commandsEnabled is TAG_Byte. Writing the wrong
            // width here would produce a file the game rejects.
            if (patch.has("gameType")) root.set("GameType", patch.getInt("gameType"))
            if (patch.has("difficulty")) root.set("Difficulty", patch.getInt("difficulty"))
            if (patch.has("generator")) root.set("Generator", patch.getInt("generator"))
            if (patch.has("cheats")) {
                root.set("commandsEnabled", (if (patch.getBoolean("cheats")) 1 else 0).toByte())
            }
            if (patch.has("experiments")) {
                val requested = patch.getJSONObject("experiments")
                val compound = root.getCompound("experiments")
                if (compound != null) {
                    for (name in requested.keys()) {
                        // Only toggle switches that already exist. Inventing
                        // an experiment key the world never had is how you
                        // get a world the game refuses to load.
                        if (compound.has(name)) {
                            compound.set(name, (if (requested.getBoolean(name)) 1 else 0).toByte())
                        }
                    }
                }
            }

            val encoded = BedrockNbt.writeLevelDat(root, version)

            // Backup before overwriting — see the class comment on why SAF
            // can't do desktop's atomic tmp-then-move.
            val backup = dir.findFile(BACKUP_NAME) ?: dir.createFile("application/octet-stream", BACKUP_NAME)
            if (backup != null) {
                context.contentResolver.openOutputStream(backup.uri, "wt")?.use { it.write(original) }
            }

            context.contentResolver.openOutputStream(file.uri, "wt")?.use { it.write(encoded) }
                ?: return error("Couldn't open level.dat for writing.")

            JSONObject().put("ok", true).toString()
        } catch (e: Exception) {
            error("Couldn't save level.dat: ${e.message}")
        }
    }

    private fun error(message: String): String =
        JSONObject().put("ok", false).put("error", message).toString()
}
