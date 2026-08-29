package xyz.glacierclient.launcher.service

import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Minimal little-endian NBT reader for Bedrock's level.dat.
 *
 * Java Edition's NBT is big-endian and gzip-compressed; Bedrock's is
 * little-endian and raw — and level.dat specifically is prefixed with an
 * 8-byte header (4-byte storage version + 4-byte payload length, both LE)
 * that's part of the game's own save format, not the NBT spec itself.
 *
 * This only implements reading, and only the tag types level.dat actually
 * uses for the fields the world-list panels need (LevelName, LastPlayed,
 * GameType). Bedrock's actual chunk/entity data lives in db/ as a separate
 * LevelDB key-value store — a much larger format this class does not touch;
 * nothing needed for listing worlds requires reading it.
 */
object BedrockNbt {

    class Compound(private val values: MutableMap<String, Any?> = mutableMapOf()) {
        fun getString(key: String): String? = values[key] as? String
        fun getLong(key: String): Long? = when (val v = values[key]) {
            is Long -> v
            is Int -> v.toLong()
            else -> null
        }
        fun getInt(key: String): Int? = when (val v = values[key]) {
            is Int -> v
            is Long -> v.toInt()
            else -> null
        }
        internal fun put(key: String, value: Any?) { values[key] = value }
    }

    /** Parses a full level.dat file, including its 8-byte save-format header. */
    fun parseLevelDat(bytes: ByteArray): Compound {
        val buf = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
        buf.position(8)
        return parse(buf)
    }

    /** Parses a raw little-endian NBT payload starting at a root TAG_Compound. */
    fun parse(buf: ByteBuffer): Compound {
        val type = buf.get().toInt()
        require(type == 10) { "Expected root TAG_Compound, got tag id $type" }
        readString(buf) // root name, unused
        return readCompoundBody(buf)
    }

    private fun readCompoundBody(buf: ByteBuffer): Compound {
        val compound = Compound()
        while (true) {
            val type = buf.get().toInt()
            if (type == 0) break // TAG_End
            val name = readString(buf)
            compound.put(name, readPayload(buf, type))
        }
        return compound
    }

    private fun readPayload(buf: ByteBuffer, type: Int): Any? = when (type) {
        1 -> buf.get()
        2 -> buf.short
        3 -> buf.int
        4 -> buf.long
        5 -> buf.float
        6 -> buf.double
        7 -> { val len = buf.int; ByteArray(len).also { buf.get(it) } }
        8 -> readString(buf)
        9 -> { // TAG_List
            val itemType = buf.get().toInt()
            val len = buf.int
            (0 until len).map { readPayload(buf, itemType) }
        }
        10 -> readCompoundBody(buf)
        11 -> { val len = buf.int; IntArray(len) { buf.int } }
        12 -> { val len = buf.int; LongArray(len) { buf.long } }
        else -> throw IllegalArgumentException("Unknown NBT tag id $type")
    }

    private fun readString(buf: ByteBuffer): String {
        val len = buf.short.toInt() and 0xFFFF
        val bytes = ByteArray(len)
        buf.get(bytes)
        return bytes.toString(Charsets.UTF_8)
    }
}
