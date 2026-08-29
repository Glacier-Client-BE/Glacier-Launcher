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
 * Reads and writes every NBT tag type, because the level.dat editor has to
 * round-trip the whole file: it changes a handful of fields and must put
 * everything else back byte-for-byte, so a tag it cannot represent would be
 * silently dropped and corrupt the world. Two details make that round-trip
 * exact — TAG_List's element type is kept (an empty list would otherwise
 * lose it) and the root compound's name is preserved, and the backing map is
 * insertion-ordered so tags are rewritten in their original order.
 *
 * Bedrock's actual chunk/entity data lives in db/ as a separate LevelDB
 * key-value store — a much larger format this class does not touch; neither
 * listing worlds nor editing level.dat requires reading it.
 */
object BedrockNbt {

    /** A TAG_List, carrying the element type so an empty list still round-trips. */
    class NbtList(val elementType: Int, val items: List<Any?>)

    // mutableMapOf() is a LinkedHashMap, so iteration order is insertion
    // order — which for a parsed file is the order the tags appeared in.
    class Compound(private val values: MutableMap<String, Any?> = mutableMapOf()) {
        /**
         * The root compound's own name, preserved so a rewritten file matches
         * the original. Held per-Compound rather than on the object: parsing
         * is otherwise stateless, and a shared field would be clobbered by a
         * concurrent or subsequent parse.
         */
        var name: String = ""
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
        fun getByte(key: String): Int? = when (val v = values[key]) {
            is Byte -> v.toInt()
            is Int -> v
            else -> null
        }
        fun has(key: String): Boolean = values.containsKey(key)
        fun getCompound(key: String): Compound? = values[key] as? Compound
        internal fun entries(): Map<String, Any?> = values
        internal fun put(key: String, value: Any?) { values[key] = value }

        /** Replaces [key] keeping its position, or appends it if new. */
        fun set(key: String, value: Any?) { values[key] = value }
    }

    /** The 4-byte storage version from a level.dat header, preserved on write. */
    fun levelDatVersion(bytes: ByteArray): Int =
        ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN).int

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
        val name = readString(buf)
        return readCompoundBody(buf).also { it.name = name }
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
        9 -> { // TAG_List — element type kept so an empty list still round-trips
            val itemType = buf.get().toInt()
            val len = buf.int
            NbtList(itemType, (0 until len).map { readPayload(buf, itemType) })
        }
        10 -> readCompoundBody(buf)
        11 -> { val len = buf.int; IntArray(len) { buf.int } }
        12 -> { val len = buf.int; LongArray(len) { buf.long } }
        else -> throw IllegalArgumentException("Unknown NBT tag id $type")
    }

    // ── Writing ──────────────────────────────────────────────────────────

    /**
     * Serialises [root] back into a complete level.dat, header included.
     * The payload is built first so its real length can go into the header,
     * and [version] is the storage version read off the original file.
     */
    fun writeLevelDat(root: Compound, version: Int): ByteArray {
        val payload = writePayloadBytes(root)
        val out = ByteBuffer.allocate(8 + payload.size).order(ByteOrder.LITTLE_ENDIAN)
        out.putInt(version)
        out.putInt(payload.size)
        out.put(payload)
        return out.array()
    }

    private fun writePayloadBytes(root: Compound): ByteArray {
        val sink = java.io.ByteArrayOutputStream()
        writeByte(sink, 10)
        writeString(sink, root.name)
        writeCompoundBody(sink, root)
        return sink.toByteArray()
    }

    private fun writeCompoundBody(sink: java.io.ByteArrayOutputStream, compound: Compound) {
        for ((key, value) in compound.entries()) {
            val type = tagIdOf(value)
            writeByte(sink, type)
            writeString(sink, key)
            writePayload(sink, type, value)
        }
        writeByte(sink, 0) // TAG_End
    }

    /**
     * Maps a decoded value back to its NBT tag id. Every type [readPayload]
     * can produce is handled — an unmapped one would mean silently dropping a
     * tag from a file we are about to overwrite, so it throws instead.
     */
    private fun tagIdOf(value: Any?): Int = when (value) {
        is Byte -> 1
        is Short -> 2
        is Int -> 3
        is Long -> 4
        is Float -> 5
        is Double -> 6
        is ByteArray -> 7
        is String -> 8
        is NbtList -> 9
        is Compound -> 10
        is IntArray -> 11
        is LongArray -> 12
        else -> throw IllegalArgumentException("Cannot write NBT value of type ${value?.javaClass}")
    }

    private fun writePayload(sink: java.io.ByteArrayOutputStream, type: Int, value: Any?) {
        when (type) {
            1 -> writeByte(sink, (value as Byte).toInt())
            2 -> writeRaw(sink, 2) { it.putShort(value as Short) }
            3 -> writeRaw(sink, 4) { it.putInt(value as Int) }
            4 -> writeRaw(sink, 8) { it.putLong(value as Long) }
            5 -> writeRaw(sink, 4) { it.putFloat(value as Float) }
            6 -> writeRaw(sink, 8) { it.putDouble(value as Double) }
            7 -> {
                val bytes = value as ByteArray
                writeRaw(sink, 4) { it.putInt(bytes.size) }
                sink.write(bytes)
            }
            8 -> writeString(sink, value as String)
            9 -> {
                val list = value as NbtList
                writeByte(sink, list.elementType)
                writeRaw(sink, 4) { it.putInt(list.items.size) }
                list.items.forEach { writePayload(sink, list.elementType, it) }
            }
            10 -> writeCompoundBody(sink, value as Compound)
            11 -> {
                val ints = value as IntArray
                writeRaw(sink, 4) { it.putInt(ints.size) }
                ints.forEach { v -> writeRaw(sink, 4) { it.putInt(v) } }
            }
            12 -> {
                val longs = value as LongArray
                writeRaw(sink, 4) { it.putInt(longs.size) }
                longs.forEach { v -> writeRaw(sink, 8) { it.putLong(v) } }
            }
            else -> throw IllegalArgumentException("Unknown NBT tag id $type")
        }
    }

    private inline fun writeRaw(sink: java.io.ByteArrayOutputStream, size: Int, fill: (ByteBuffer) -> Unit) {
        val buf = ByteBuffer.allocate(size).order(ByteOrder.LITTLE_ENDIAN)
        fill(buf)
        sink.write(buf.array())
    }

    private fun writeByte(sink: java.io.ByteArrayOutputStream, value: Int) = sink.write(value and 0xFF)

    private fun writeString(sink: java.io.ByteArrayOutputStream, value: String) {
        val bytes = value.toByteArray(Charsets.UTF_8)
        writeRaw(sink, 2) { it.putShort(bytes.size.toShort()) }
        sink.write(bytes)
    }

    private fun readString(buf: ByteBuffer): String {
        val len = buf.short.toInt() and 0xFFFF
        val bytes = ByteArray(len)
        buf.get(bytes)
        return bytes.toString(Charsets.UTF_8)
    }
}
