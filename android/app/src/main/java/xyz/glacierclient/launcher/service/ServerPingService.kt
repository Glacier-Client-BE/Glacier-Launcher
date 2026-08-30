package xyz.glacierclient.launcher.service

import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import java.io.InputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetSocketAddress
import java.net.Socket
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.charset.StandardCharsets

/**
 * Android analogue of Services/ServerPingService.cs. Same two wire protocols,
 * same "every failure resolves to offline" contract — this is called from a
 * JS `setInterval`-driven refresh over N saved/suggested servers, so a single
 * bad host must never throw and abort the batch, just report offline.
 *
 * Runs on whatever thread the @JavascriptInterface call lands on (WebView's
 * background JS-bridge thread, not the UI thread — same as every other
 * blocking-network bridge method in this app, e.g. BedrockVersionService's
 * downloadApk), so a plain blocking socket with a short timeout is fine; no
 * coroutine/executor plumbing needed.
 */
object ServerPingService {

    private val RAKNET_MAGIC = byteArrayOf(
        0x00, -0x01, -0x01, 0x00, -0x02, -0x02, -0x02, -0x02,
        -0x03, -0x03, -0x03, -0x03, 0x12, 0x34, 0x56, 0x78
    )

    private fun offline(): JSONObject = JSONObject()
        .put("online", false).put("playersOnline", 0).put("playersMax", 0)
        .put("motd", "").put("version", "").put("latencyMs", 0)

    /** Picks Java SLP for port 25565, otherwise Bedrock RakNet — same rule as the Windows service. */
    fun ping(host: String, port: Int, timeoutMs: Int = 2500): String =
        (if (port == 25565) pingJava(host, port, timeoutMs) else pingBedrock(host, port, timeoutMs)).toString()

    // ── Bedrock (RakNet unconnected ping over UDP) ────────────────────────
    private fun pingBedrock(host: String, port: Int, timeoutMs: Int): JSONObject {
        return try {
            DatagramSocket().use { socket ->
                socket.soTimeout = timeoutMs
                val start = System.nanoTime()

                val packet = ByteBuffer.allocate(1 + 8 + 16 + 8).order(ByteOrder.BIG_ENDIAN)
                packet.put(0x01)                          // ID_UNCONNECTED_PING
                packet.putLong(0L)                         // client send time (unused by us)
                packet.put(RAKNET_MAGIC)
                packet.putLong(0x0102030405060708L)        // client GUID
                val out = DatagramPacket(packet.array(), packet.array().size, InetSocketAddress(host, port))
                socket.send(out)

                val buf = ByteArray(1500)
                val inPkt = DatagramPacket(buf, buf.size)
                socket.receive(inPkt)                       // throws SocketTimeoutException on timeout
                val elapsedMs = (System.nanoTime() - start) / 1_000_000

                val data = inPkt.data
                // 0x1c | time(8) | guid(8) | magic(16) | idLen(2) | idString
                if (inPkt.length < 35 || data[0] != 0x1c.toByte()) return offline()
                val idLen = ((data[33].toInt() and 0xFF) shl 8) or (data[34].toInt() and 0xFF)
                val avail = (inPkt.length - 35).coerceAtLeast(0)
                val id = String(data, 35, minOf(idLen, avail), StandardCharsets.UTF_8)

                // edition;motd;protocol;version;online;max;guid;motd2;gamemode;...
                val f = id.split(";")
                JSONObject()
                    .put("online", true)
                    .put("playersOnline", f.getOrNull(4)?.toIntOrNull() ?: 0)
                    .put("playersMax", f.getOrNull(5)?.toIntOrNull() ?: 0)
                    .put("motd", f.getOrNull(1) ?: "")
                    .put("version", f.getOrNull(3) ?: "")
                    .put("latencyMs", elapsedMs)
            }
        } catch (_: Exception) {
            offline()
        }
    }

    // ── Java (Server List Ping over TCP) ──────────────────────────────────
    private fun pingJava(host: String, port: Int, timeoutMs: Int): JSONObject {
        return try {
            Socket().use { socket ->
                val start = System.nanoTime()
                socket.connect(InetSocketAddress(host, port), timeoutMs)
                socket.soTimeout = timeoutMs
                val out = DataOutputStream(socket.getOutputStream())
                val input = socket.getInputStream()

                val handshake = ByteArrayOutputStream().also { hs ->
                    writeVarInt(hs, 0x00)                  // handshake packet id
                    writeVarInt(hs, -1)                     // protocol version = "any"
                    writeString(hs, host)
                    hs.write((port shr 8) and 0xFF)
                    hs.write(port and 0xFF)                 // port, unsigned short big-endian
                    writeVarInt(hs, 1)                       // next state = status
                }
                writePacket(out, handshake.toByteArray())
                writePacket(out, byteArrayOf(0x00))          // status request

                readVarInt(input)                            // total packet length
                readVarInt(input)                            // packet id (0x00)
                val jsonLen = readVarInt(input)
                val jsonBytes = readExact(input, jsonLen)
                val elapsedMs = (System.nanoTime() - start) / 1_000_000

                val root = JSONObject(String(jsonBytes, StandardCharsets.UTF_8))
                val players = root.optJSONObject("players")
                val version = root.optJSONObject("version")
                val description = root.opt("description")
                val motd = when (description) {
                    is JSONObject -> description.optString("text", "")
                    is String -> description
                    else -> ""
                }

                JSONObject()
                    .put("online", true)
                    .put("playersOnline", players?.optInt("online", 0) ?: 0)
                    .put("playersMax", players?.optInt("max", 0) ?: 0)
                    .put("motd", motd)
                    .put("version", version?.optString("name", "") ?: "")
                    .put("latencyMs", elapsedMs)
            }
        } catch (_: Exception) {
            offline()
        }
    }

    // ── VarInt / packet helpers (mirrors the Windows service's own) ───────
    private fun writeVarInt(out: ByteArrayOutputStream, value: Int) {
        var v = value
        do {
            var b = v and 0x7F
            v = v ushr 7
            if (v != 0) b = b or 0x80
            out.write(b)
        } while (v != 0)
    }

    private fun writeString(out: ByteArrayOutputStream, s: String) {
        val bytes = s.toByteArray(StandardCharsets.UTF_8)
        writeVarInt(out, bytes.size)
        out.write(bytes)
    }

    private fun writePacket(out: DataOutputStream, payload: ByteArray) {
        val lenBuf = ByteArrayOutputStream()
        writeVarInt(lenBuf, payload.size)
        out.write(lenBuf.toByteArray())
        out.write(payload)
        out.flush()
    }

    private fun readVarInt(input: InputStream): Int {
        var result = 0
        var shift = 0
        while (true) {
            val b = input.read()
            if (b == -1) throw java.io.EOFException("stream closed mid-varint")
            result = result or ((b and 0x7F) shl shift)
            if (b and 0x80 == 0) break
            shift += 7
            if (shift >= 35) throw IllegalStateException("VarInt too long")
        }
        return result
    }

    private fun readExact(input: InputStream, length: Int): ByteArray {
        val buf = ByteArray(length)
        var off = 0
        while (off < length) {
            val n = input.read(buf, off, length - off)
            if (n == -1) throw java.io.EOFException("stream closed before $length bytes read")
            off += n
        }
        return buf
    }
}
