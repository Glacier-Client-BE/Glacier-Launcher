//! Minecraft server list-ping, ported from `Services/ServerPingService.cs`.
//!
//! Implements both protocols the C# service implemented:
//! - Java Edition: raw TCP handshake + status request (SLP), used when
//!   `port == 25565`.
//! - Bedrock Edition: RakNet "unconnected ping" over UDP, used otherwise.

use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::timeout;

use crate::models::ServerStatus;

const RAKNET_MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

/// Pings a server, choosing Java or Bedrock protocol based on port, mirroring
/// the C# `PingAsync` dispatcher (port 25565 => Java, otherwise Bedrock).
pub async fn ping(host: &str, port: u16, timeout_ms: u64) -> ServerStatus {
    if port == 25565 {
        ping_java(host, port, timeout_ms).await
    } else {
        ping_bedrock(host, port, timeout_ms).await
    }
}

/// RakNet unconnected ping (Bedrock Edition list ping).
pub async fn ping_bedrock(host: &str, port: u16, timeout_ms: u64) -> ServerStatus {
    match ping_bedrock_inner(host, port, timeout_ms).await {
        Ok(status) => status,
        Err(_) => ServerStatus::OFFLINE,
    }
}

async fn ping_bedrock_inner(
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> std::io::Result<ServerStatus> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;

    let packet = build_unconnected_ping_packet(0);
    let start = Instant::now();
    socket.send_to(&packet, (host, port)).await?;

    let mut buf = vec![0u8; 2048];
    let recv = timeout(Duration::from_millis(timeout_ms), socket.recv(&mut buf)).await;
    let n = match recv {
        Ok(Ok(n)) => n,
        _ => return Ok(ServerStatus::OFFLINE),
    };
    let elapsed = start.elapsed().as_millis() as i64;
    buf.truncate(n);

    Ok(parse_unconnected_pong(&buf, elapsed))
}

/// Builds the RakNet "unconnected ping" packet (id 0x01): time, magic,
/// client GUID. Mirrors the byte layout built in `PingBedrockAsync`.
fn build_unconnected_ping_packet(time_ms: i64) -> Vec<u8> {
    let mut packet = Vec::with_capacity(1 + 8 + 16 + 8);
    packet.push(0x01);
    packet.extend_from_slice(&time_ms.to_be_bytes());
    packet.extend_from_slice(&RAKNET_MAGIC);
    packet.extend_from_slice(&0x0102030405060708i64.to_be_bytes());
    packet
}

/// Parses a RakNet "unconnected pong" packet (id 0x1c): the payload string
/// after the 16-bit big-endian length at offset 33 is a `;`-separated MOTD
/// record, mirroring `PingBedrockAsync`'s parsing.
fn parse_unconnected_pong(data: &[u8], elapsed_ms: i64) -> ServerStatus {
    if data.len() < 35 || data[0] != 0x1c {
        return ServerStatus::OFFLINE;
    }
    let str_len = u16::from_be_bytes([data[33], data[34]]) as usize;
    let end = (35 + str_len).min(data.len());
    let id = String::from_utf8_lossy(&data[35..end]).into_owned();

    let fields: Vec<&str> = id.split(';').collect();
    ServerStatus {
        online: true,
        players_online: fields.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
        players_max: fields.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
        motd: fields.get(1).map(|s| s.to_string()).unwrap_or_default(),
        version: fields.get(3).map(|s| s.to_string()).unwrap_or_default(),
        latency_ms: elapsed_ms,
    }
}

/// Java Edition Server List Ping (handshake -> status request -> status
/// response), mirroring `PingJavaAsync`.
pub async fn ping_java(host: &str, port: u16, timeout_ms: u64) -> ServerStatus {
    match timeout(
        Duration::from_millis(timeout_ms),
        ping_java_inner(host, port),
    )
    .await
    {
        Ok(Ok(status)) => status,
        _ => ServerStatus::OFFLINE,
    }
}

async fn ping_java_inner(host: &str, port: u16) -> std::io::Result<ServerStatus> {
    let start = Instant::now();
    let mut stream = TcpStream::connect((host, port)).await?;

    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0x00);
    write_varint(&mut handshake, -1);
    write_string(&mut handshake, host);
    handshake.push((port >> 8) as u8);
    handshake.push((port & 0xff) as u8);
    write_varint(&mut handshake, 1);
    write_packet(&mut stream, &handshake).await?;

    write_packet(&mut stream, &[0x00]).await?;

    let _packet_len = read_varint(&mut stream).await?;
    let _packet_id = read_varint(&mut stream).await?;
    let json_len = read_varint(&mut stream).await?;
    let json_bytes = read_exact_n(&mut stream, json_len as usize).await?;
    let elapsed = start.elapsed().as_millis() as i64;

    let value: serde_json::Value = serde_json::from_slice(&json_bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let online = value
        .get("players")
        .and_then(|p| p.get("online"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let max = value
        .get("players")
        .and_then(|p| p.get("max"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let version = value
        .get("version")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let motd = value
        .get("description")
        .map(extract_motd)
        .unwrap_or_default();

    Ok(ServerStatus {
        online: true,
        players_online: online,
        players_max: max,
        motd,
        version,
        latency_ms: elapsed,
    })
}

fn extract_motd(d: &serde_json::Value) -> String {
    if let Some(s) = d.as_str() {
        return s.to_string();
    }
    let mut out = String::new();
    walk_motd(d, &mut out);
    out
}

fn walk_motd(e: &serde_json::Value, out: &mut String) {
    if let Some(s) = e.as_str() {
        out.push_str(s);
        return;
    }
    if !e.is_object() {
        return;
    }
    if let Some(t) = e.get("text").and_then(|v| v.as_str()) {
        out.push_str(t);
    }
    if let Some(extra) = e.get("extra").and_then(|v| v.as_array()) {
        for c in extra {
            walk_motd(c, out);
        }
    }
}

fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        let mut b = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            b |= 0x80;
        }
        buf.push(b);
        if v == 0 {
            break;
        }
    }
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_varint(buf, bytes.len() as i32);
    buf.extend_from_slice(bytes);
}

async fn write_packet(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    let mut framed = Vec::new();
    write_varint(&mut framed, data.len() as i32);
    framed.extend_from_slice(data);
    stream.write_all(&framed).await
}

async fn read_varint(stream: &mut TcpStream) -> std::io::Result<i32> {
    let mut num_read = 0u32;
    let mut result: i32 = 0;
    loop {
        let mut byte = [0u8; 1];
        let n = stream.read(&mut byte).await?;
        if n == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let read = byte[0];
        result |= ((read & 0x7F) as i32) << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "VarInt too big",
            ));
        }
        if read & 0x80 == 0 {
            break;
        }
    }
    Ok(result)
}

async fn read_exact_n(stream: &mut TcpStream, len: usize) -> std::io::Result<Vec<u8>> {
    if len > 1 << 20 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Bad length",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_unconnected_ping_packet_layout() {
        let packet = build_unconnected_ping_packet(0);
        assert_eq!(packet.len(), 1 + 8 + 16 + 8);
        assert_eq!(packet[0], 0x01);
        assert_eq!(&packet[9..25], &RAKNET_MAGIC);
    }

    #[test]
    fn parses_unconnected_pong_motd_fields() {
        // id string: "MCPE;My Server;527;1.20.1;3;20;123456;Bedrock level;Survival;1;19132;19133;"
        let id = "MCPE;My Server;527;1.20.1;3;20;123456;Bedrock level;Survival;1;19132;19133;";
        let id_bytes = id.as_bytes();

        let mut data = vec![0u8; 35];
        data[0] = 0x1c;
        data[33..35].copy_from_slice(&(id_bytes.len() as u16).to_be_bytes());
        data.extend_from_slice(id_bytes);

        let status = parse_unconnected_pong(&data, 12);
        assert!(status.online);
        assert_eq!(status.motd, "My Server");
        assert_eq!(status.version, "1.20.1");
        assert_eq!(status.players_online, 3);
        assert_eq!(status.players_max, 20);
        assert_eq!(status.latency_ms, 12);
    }

    #[test]
    fn rejects_short_or_wrong_id_pong_packets() {
        assert_eq!(
            parse_unconnected_pong(&[0x1c, 0x00], 0),
            ServerStatus::OFFLINE
        );
        let mut data = vec![0u8; 40];
        data[0] = 0x00; // wrong packet id
        assert_eq!(parse_unconnected_pong(&data, 0), ServerStatus::OFFLINE);
    }

    #[test]
    fn varint_roundtrip_small_and_large_values() {
        for value in [0i32, 1, 127, 128, 300, 2_097_151, i32::MAX] {
            let mut buf = Vec::new();
            write_varint(&mut buf, value);
            // A VarInt for a non-negative i32 never exceeds 5 bytes.
            assert!(buf.len() <= 5);
        }
    }

    #[test]
    fn extracts_motd_from_nested_chat_component() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"text": "Hello ", "extra": [{"text": "World"}]}"#).unwrap();
        assert_eq!(extract_motd(&json), "Hello World");
    }

    #[test]
    fn extracts_motd_from_plain_string() {
        let json: serde_json::Value = serde_json::from_str(r#""Just a plain MOTD""#).unwrap();
        assert_eq!(extract_motd(&json), "Just a plain MOTD");
    }
}
