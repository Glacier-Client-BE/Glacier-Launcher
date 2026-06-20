using System.Buffers.Binary;
using System.Diagnostics;
using System.IO;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;

namespace GlacierLauncher.Services;

/// <summary>Result of a server status query. <see cref="Online"/> is false on any failure/timeout.</summary>
public sealed record ServerStatus(
    bool Online, int PlayersOnline, int PlayersMax, string Motd, string Version, long LatencyMs)
{
    public static readonly ServerStatus Offline = new(false, 0, 0, "", "", 0);
}

/// <summary>
/// Pings Minecraft servers for live status (player counts, MOTD, latency) without
/// any third-party API: Bedrock via the RakNet unconnected-ping (UDP), Java via the
/// Server List Ping handshake (TCP). Every failure path resolves to <see
/// cref="ServerStatus.Offline"/> so callers never have to catch.
/// </summary>
public static class ServerPingService
{
    private static readonly byte[] RakNetMagic =
    {
        0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe,
        0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78
    };

    /// <summary>Picks Java SLP for port 25565, otherwise Bedrock RakNet.</summary>
    public static Task<ServerStatus> PingAsync(string host, int port, int timeoutMs = 2500) =>
        port == 25565 ? PingJavaAsync(host, port, timeoutMs) : PingBedrockAsync(host, port, timeoutMs);

    // ── Bedrock (RakNet unconnected ping over UDP) ───────────────────────────
    public static async Task<ServerStatus> PingBedrockAsync(string host, int port, int timeoutMs = 2500)
    {
        try
        {
            using var udp = new UdpClient();
            var sw = Stopwatch.StartNew();

            var packet = new byte[1 + 8 + 16 + 8];
            packet[0] = 0x01;                                                   // ID_UNCONNECTED_PING
            BinaryPrimitives.WriteInt64BigEndian(packet.AsSpan(1, 8), sw.ElapsedMilliseconds);
            RakNetMagic.CopyTo(packet.AsSpan(9, 16));
            BinaryPrimitives.WriteInt64BigEndian(packet.AsSpan(25, 8), 0x0102030405060708);

            await udp.SendAsync(packet, packet.Length, host, port).ConfigureAwait(false);

            var recv = udp.ReceiveAsync();
            if (await Task.WhenAny(recv, Task.Delay(timeoutMs)).ConfigureAwait(false) != recv)
                return ServerStatus.Offline;
            sw.Stop();

            var data = recv.Result.Buffer;                                      // 0x1c | time(8) | guid(8) | magic(16) | len(2) | id-string
            if (data.Length < 35 || data[0] != 0x1c) return ServerStatus.Offline;

            int strLen = BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(33, 2));
            var id = Encoding.UTF8.GetString(data, 35, Math.Min(strLen, data.Length - 35));

            // edition;motd;protocol;version;online;max;guid;motd2;gamemode;...
            var f = id.Split(';');
            return new ServerStatus(
                Online:        true,
                PlayersOnline: f.Length > 4 && int.TryParse(f[4], out var o) ? o : 0,
                PlayersMax:    f.Length > 5 && int.TryParse(f[5], out var m) ? m : 0,
                Motd:          f.Length > 1 ? f[1] : "",
                Version:       f.Length > 3 ? f[3] : "",
                LatencyMs:     sw.ElapsedMilliseconds);
        }
        catch { return ServerStatus.Offline; }
    }

    // ── Java (Server List Ping over TCP) ─────────────────────────────────────
    public static async Task<ServerStatus> PingJavaAsync(string host, int port, int timeoutMs = 2500)
    {
        using var cts = new CancellationTokenSource(timeoutMs);
        var ct = cts.Token;
        try
        {
            using var tcp = new TcpClient();
            var sw = Stopwatch.StartNew();
            await tcp.ConnectAsync(host, port, ct).ConfigureAwait(false);
            await using var stream = tcp.GetStream();

            using var hs = new MemoryStream();
            WriteVarInt(hs, 0x00);                       // handshake packet id
            WriteVarInt(hs, -1);                         // protocol version (-1 = "any")
            WriteString(hs, host);
            hs.WriteByte((byte)(port >> 8));
            hs.WriteByte((byte)(port & 0xFF));           // port as unsigned short, big-endian
            WriteVarInt(hs, 1);                          // next state = status
            await WritePacketAsync(stream, hs.ToArray(), ct).ConfigureAwait(false);

            await WritePacketAsync(stream, new byte[] { 0x00 }, ct).ConfigureAwait(false); // status request

            _ = await ReadVarIntAsync(stream, ct).ConfigureAwait(false);        // total length
            _ = await ReadVarIntAsync(stream, ct).ConfigureAwait(false);        // packet id (0x00)
            int jsonLen = await ReadVarIntAsync(stream, ct).ConfigureAwait(false);
            var json = await ReadExactAsync(stream, jsonLen, ct).ConfigureAwait(false);
            sw.Stop();

            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;
            int online = 0, max = 0; string ver = "", motd = "";
            if (root.TryGetProperty("players", out var pl))
            {
                if (pl.TryGetProperty("online", out var o)) online = o.GetInt32();
                if (pl.TryGetProperty("max", out var m))    max    = m.GetInt32();
            }
            if (root.TryGetProperty("version", out var v) && v.TryGetProperty("name", out var vn))
                ver = vn.GetString() ?? "";
            if (root.TryGetProperty("description", out var d))
                motd = ExtractMotd(d);

            return new ServerStatus(true, online, max, motd, ver, sw.ElapsedMilliseconds);
        }
        catch { return ServerStatus.Offline; }
    }

    // ── VarInt / packet helpers ──────────────────────────────────────────────
    private static void WriteVarInt(Stream s, int value)
    {
        uint v = (uint)value;
        do
        {
            byte b = (byte)(v & 0x7F);
            v >>= 7;
            if (v != 0) b |= 0x80;
            s.WriteByte(b);
        } while (v != 0);
    }

    private static void WriteString(Stream s, string str)
    {
        var bytes = Encoding.UTF8.GetBytes(str);
        WriteVarInt(s, bytes.Length);
        s.Write(bytes, 0, bytes.Length);
    }

    private static async Task WritePacketAsync(Stream s, byte[] data, CancellationToken ct)
    {
        using var ms = new MemoryStream();
        WriteVarInt(ms, data.Length);
        ms.Write(data, 0, data.Length);
        var arr = ms.ToArray();
        await s.WriteAsync(arr, ct).ConfigureAwait(false);
    }

    private static async Task<int> ReadVarIntAsync(Stream s, CancellationToken ct)
    {
        int numRead = 0, result = 0; byte read;
        do
        {
            var one = new byte[1];
            if (await s.ReadAsync(one.AsMemory(0, 1), ct).ConfigureAwait(false) == 0)
                throw new EndOfStreamException();
            read = one[0];
            result |= (read & 0x7F) << (7 * numRead);
            if (++numRead > 5) throw new InvalidDataException("VarInt too big");
        } while ((read & 0x80) != 0);
        return result;
    }

    private static async Task<byte[]> ReadExactAsync(Stream s, int len, CancellationToken ct)
    {
        if (len < 0 || len > 1 << 20) throw new InvalidDataException("Bad length");
        var buf = new byte[len];
        int off = 0;
        while (off < len)
        {
            int n = await s.ReadAsync(buf.AsMemory(off, len - off), ct).ConfigureAwait(false);
            if (n == 0) throw new EndOfStreamException();
            off += n;
        }
        return buf;
    }

    private static string ExtractMotd(JsonElement d)
    {
        if (d.ValueKind == JsonValueKind.String) return d.GetString() ?? "";
        var sb = new StringBuilder();
        Walk(d, sb);
        return sb.ToString();

        static void Walk(JsonElement e, StringBuilder sb)
        {
            if (e.ValueKind == JsonValueKind.String) { sb.Append(e.GetString()); return; }
            if (e.ValueKind != JsonValueKind.Object) return;
            if (e.TryGetProperty("text", out var t)) sb.Append(t.GetString());
            if (e.TryGetProperty("extra", out var ex) && ex.ValueKind == JsonValueKind.Array)
                foreach (var c in ex.EnumerateArray()) Walk(c, sb);
        }
    }
}
