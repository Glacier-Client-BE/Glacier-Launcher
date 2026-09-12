//! LAN world discovery. Minecraft Java Edition broadcasts an open-to-LAN
//! world via a plain-text UDP multicast packet on `224.0.2.60:4445`:
//! `[MOTD]<motd>[/MOTD][AD]<port>[/AD]`, repeated roughly every 1.5s while
//! the world is open. There is no equivalent C# service to port from (this
//! launcher never had a LAN browser); this is a new feature.
//!
//! We join the multicast group for a short scan window, collect any
//! packets received, and return the de-duplicated results. Any failure to
//! bind/join the multicast group (e.g. no multicast-capable interface) is
//! reported as an empty result with an error string rather than panicking
//! or blocking indefinitely.

use std::net::Ipv4Addr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::time::timeout;

const LAN_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 2, 60);
const LAN_MULTICAST_PORT: u16 = 4445;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanGame {
    pub motd: String,
    pub port: u16,
    /// The IP address the broadcast packet arrived from (the host's LAN IP).
    pub host: String,
}

/// Scans for LAN worlds for `scan_ms` milliseconds. Never panics or blocks
/// past the scan window: any bind/join/recv error simply yields no results.
pub async fn scan_lan_games(scan_ms: u64) -> Result<Vec<LanGame>, String> {
    let socket = match UdpSocket::bind((Ipv4Addr::UNSPECIFIED, LAN_MULTICAST_PORT)).await {
        Ok(s) => s,
        Err(e) => return Err(format!("could not bind multicast port: {e}")),
    };

    if let Err(e) = socket.join_multicast_v4(LAN_MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED) {
        return Err(format!(
            "could not join LAN multicast group (interface may not support multicast): {e}"
        ));
    }

    let mut games: Vec<LanGame> = Vec::new();
    let mut buf = vec![0u8; 2048];
    let deadline = tokio::time::Instant::now() + Duration::from_millis(scan_ms);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, addr))) => {
                if let Some(game) = parse_lan_packet(&buf[..n], addr.ip().to_string()) {
                    if !games
                        .iter()
                        .any(|g| g.host == game.host && g.port == game.port)
                    {
                        games.push(game);
                    }
                }
            }
            _ => break, // timeout elapsed or socket error — stop scanning
        }
    }

    let _ = socket.leave_multicast_v4(LAN_MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED);
    Ok(games)
}

/// Parses a `[MOTD]<motd>[/MOTD][AD]<port>[/AD]` packet, mirroring vanilla
/// Java's `LanServerPingThread`/`LanServerDetector` wire format.
fn parse_lan_packet(data: &[u8], host: String) -> Option<LanGame> {
    let text = String::from_utf8_lossy(data);
    let motd = extract_between(&text, "[MOTD]", "[/MOTD]")?;
    let port_str = extract_between(&text, "[AD]", "[/AD]")?;
    let port: u16 = port_str.trim().parse().ok()?;
    Some(LanGame { motd, port, host })
}

fn extract_between(text: &str, start: &str, end: &str) -> Option<String> {
    let start_idx = text.find(start)? + start.len();
    let end_idx = text[start_idx..].find(end)? + start_idx;
    Some(text[start_idx..end_idx].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_lan_packet() {
        let packet = b"[MOTD]My World[/MOTD][AD]12345[/AD]";
        let game = parse_lan_packet(packet, "192.168.1.5".to_string()).unwrap();
        assert_eq!(game.motd, "My World");
        assert_eq!(game.port, 12345);
        assert_eq!(game.host, "192.168.1.5");
    }

    #[test]
    fn rejects_malformed_packets() {
        assert!(parse_lan_packet(b"garbage", "1.2.3.4".to_string()).is_none());
        assert!(parse_lan_packet(b"[MOTD]No AD tag[/MOTD]", "1.2.3.4".to_string()).is_none());
        assert!(parse_lan_packet(b"[AD]1234[/AD]", "1.2.3.4".to_string()).is_none());
    }

    #[test]
    fn rejects_non_numeric_port() {
        assert!(
            parse_lan_packet(b"[MOTD]x[/MOTD][AD]notaport[/AD]", "1.2.3.4".to_string()).is_none()
        );
    }
}
