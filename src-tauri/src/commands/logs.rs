//! Log browsing/sharing (`Services/LogService.cs`) and play-time stats
//! (`Services/StatsService.cs`, `Models/PlaySession.cs`).
//!
//! Neither had an existing Rust port anywhere in `crates/`, so both are
//! implemented here directly rather than wrapping a lower-level crate — they
//! are thin enough (file scan + regex redaction; a small JSON-persisted
//! list) that a dedicated crate would be overkill, matching how this file is
//! scoped as "lowest risk, finish it" in the task brief.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LogFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: String,
    pub is_crash: bool,
}

/// Lists `.log`/`.txt`/`.gz` files under `<instance_dir>/logs` and
/// `<instance_dir>/crash-reports`, newest first — mirrors `LogService.ListLogs`.
#[tauri::command(async)]
pub fn list_logs(instance_dir: String) -> Result<Vec<LogFile>, String> {
    let mut result = Vec::new();
    scan_log_dir(&Path::new(&instance_dir).join("logs"), false, &mut result);
    scan_log_dir(
        &Path::new(&instance_dir).join("crash-reports"),
        true,
        &mut result,
    );
    result.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(result)
}

fn scan_log_dir(dir: &Path, is_crash: bool, out: &mut Vec<LogFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();
        if !matches!(ext.as_str(), "log" | "txt" | "gz") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        out.push(LogFile {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_string_lossy().into_owned(),
            size: meta.len(),
            modified_at,
            is_crash,
        });
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CrashReportInfo {
    pub name: String,
    pub path: String,
    /// Unix seconds, matching `LogFile::modified_at`'s convention.
    pub modified_at: String,
}

/// Finds the newest `crash-*.txt` under `<minecraft_dir>/crash-reports` for
/// the Settings panel's "Latest crash report" row. Mirrors
/// `Home.Panels.cs::LatestJavaCrashFile` — this is the flat, version-scoped
/// `.minecraft` layout (`JavaVersions.MinecraftDir`), distinct from
/// `list_logs`'s per-instance `crash-reports` folder.
#[tauri::command(async)]
pub fn latest_java_crash_report(minecraft_dir: String) -> Option<CrashReportInfo> {
    let dir = Path::new(&minecraft_dir).join("crash-reports");
    let entries = std::fs::read_dir(&dir).ok()?;

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy().into_owned();
            if !name.starts_with("crash-") || !name.to_ascii_lowercase().ends_with(".txt") {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((path, name, modified))
        })
        .max_by_key(|(_, _, modified)| *modified)
        .map(|(path, name, modified)| CrashReportInfo {
            name,
            modified_at: modified
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default(),
            path: path.to_string_lossy().into_owned(),
        })
}

/// Reads a log file (transparently gunzipping `.gz`), truncated to the last
/// `max_lines` lines — mirrors `LogService.ReadLogAsync`.
#[tauri::command(async)]
pub fn read_log(path: String, max_lines: Option<usize>) -> Result<String, String> {
    let p = Path::new(&path);
    let max_lines = max_lines.unwrap_or(2000);

    let text = if p
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .as_deref()
        == Some("gz")
    {
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        let decompressed = gl_nbt::gzip_decompress(&bytes).map_err(|e| e.to_string())?;
        String::from_utf8_lossy(&decompressed).into_owned()
    } else {
        read_plain_tail(p, max_lines)?
    };

    let lines: Vec<&str> = text.split('\n').collect();
    let text = if lines.len() > max_lines {
        lines[lines.len() - max_lines..].join("\n")
    } else {
        text
    };
    Ok(text)
}

/// Reads only the bytes that hold the last `max_lines` lines of a plain file,
/// walking backwards in chunks. Large `latest.log` files are not loaded whole.
/// A file that fits entirely is read strictly as UTF-8, like `read_to_string`.
fn read_plain_tail(p: &Path, max_lines: usize) -> Result<String, String> {
    use std::io::{Read, Seek, SeekFrom};

    const CHUNK: u64 = 64 * 1024;
    let mut file = std::fs::File::open(p).map_err(|e| e.to_string())?;
    let mut pos = file.metadata().map_err(|e| e.to_string())?.len();
    let mut buf: Vec<u8> = Vec::new();
    let mut newlines = 0usize;
    while pos > 0 && newlines <= max_lines {
        let step = CHUNK.min(pos);
        pos -= step;
        file.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string())?;
        let mut chunk = vec![0u8; step as usize];
        file.read_exact(&mut chunk).map_err(|e| e.to_string())?;
        newlines += chunk.iter().filter(|&&b| b == b'\n').count();
        chunk.extend_from_slice(&buf);
        buf = chunk;
    }
    if pos == 0 {
        String::from_utf8(buf).map_err(|e| e.to_string())
    } else {
        // The partial first segment is dropped by the caller's line slicing
        // because more than `max_lines` newlines were read.
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
}

/// Heuristic crash-report detection, mirrors `LogService.LooksLikeCrash`.
#[tauri::command]
pub fn log_looks_like_crash(text: String) -> bool {
    text.contains("---- Minecraft Crash Report ----")
        || text.contains("Exception in thread \"main\"")
        || text
            .to_lowercase()
            .contains("a fatal error has been detected")
}

/// Redacts access tokens / session args / JWT-shaped strings / the local
/// Windows username (from `Users\<name>\` paths) / IPv4 addresses (server
/// connection lines) before sharing, mirrors `LogService.Redact`.
fn redact(text: &str) -> String {
    let text = redact_user_paths(text);
    let text = redact_ipv4(&text);
    redact_tokens(&text)
}

/// Masks the username segment of `...\Users\<name>\...` / `.../Users/<name>/...`
/// paths (Windows profile dirs leak the OS account name, which is frequently
/// the player's real name) while leaving the rest of the path intact.
fn redact_user_paths(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let has_users_word = i + 5 <= bytes.len() && text[i..i + 5].eq_ignore_ascii_case("users");
        if has_users_word {
            // Confirm whole-word "users" (not e.g. "byusers") and followed by
            // a path separator, then a username segment, then a separator.
            let word_start_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let after = i + 5;
            if word_start_ok
                && after < bytes.len()
                && matches!(bytes[after], b'\\' | b'/')
            {
                let sep = bytes[after];
                let name_start = after + 1;
                let name_end = text[name_start..]
                    .find(|c: char| c == '\\' || c == '/')
                    .map(|off| name_start + off);
                if let Some(name_end) = name_end {
                    if name_end > name_start {
                        out.push_str(&text[i..=after]);
                        out.push_str("<redacted-user>");
                        out.push(sep as char);
                        i = name_end + 1;
                        continue;
                    }
                }
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Masks IPv4 addresses (e.g. `Connecting to <ip>:<port>` lines) so a shared
/// log doesn't leak the player's or a server's real IP.
fn redact_ipv4(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let rest = &text[i..];
            let mut end = 0;
            let mut octets = 0;
            let mut ok = true;
            let mut chars = rest.char_indices().peekable();
            loop {
                let seg_start = end;
                let mut seg_len = 0;
                while let Some(&(pos, c)) = chars.peek() {
                    if c.is_ascii_digit() {
                        seg_len += 1;
                        end = pos + c.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                if seg_len == 0 || seg_len > 3 || rest[seg_start..end].parse::<u16>().unwrap_or(999) > 255 {
                    ok = false;
                    break;
                }
                octets += 1;
                if octets == 4 {
                    break;
                }
                if let Some(&(pos, '.')) = chars.peek() {
                    end = pos + 1;
                    chars.next();
                } else {
                    ok = false;
                    break;
                }
            }
            // Must not be immediately followed by another digit/dot (avoid
            // chopping a longer numeric/version string) and must have hit 4 octets.
            let boundary_ok = end >= rest.len()
                || !matches!(rest.as_bytes()[end], b'.' | b'0'..=b'9');
            if ok && octets == 4 && boundary_ok {
                out.push_str("<redacted-ip>");
                i += end;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn redact_tokens(text: &str) -> String {
    // Small hand-rolled scanner rather than pulling in `regex` for four
    // patterns: walks token-by-token looking for the flag/key names and
    // masks the value that follows, plus any JWT-shaped substring anywhere.
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let lower = rest.to_lowercase();
        let hit = ["--accesstoken ", "--session ", "accesstoken"]
            .iter()
            .filter_map(|needle| lower.find(needle).map(|i| (i, *needle)))
            .min_by_key(|(i, _)| *i);

        let Some((idx, needle)) = hit else {
            out.push_str(&redact_jwts(rest));
            break;
        };

        out.push_str(&redact_jwts(&rest[..idx]));
        let after_needle = idx + needle.len();
        // Skip past an optional `: ` / `= ` / `"` separator for the
        // `accessToken` (no leading `--`) form.
        let mut val_start = after_needle;
        let bytes = rest.as_bytes();
        while val_start < rest.len()
            && matches!(bytes[val_start], b':' | b'=' | b'"' | b'\'' | b' ')
        {
            val_start += 1;
        }
        let val_end = rest[val_start..]
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            .map(|off| val_start + off)
            .unwrap_or(rest.len());

        out.push_str(&rest[idx..after_needle.min(rest.len())]);
        out.push_str(" <redacted> ");
        rest = &rest[val_end..];
    }
    out
}

fn redact_jwts(text: &str) -> String {
    // Matches the shape `eyJ...`.`...`.`...` (three base64url segments,
    // first starting with the standard JWT header prefix).
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if text[i..].starts_with("eyJ") {
            let mut j = i;
            let is_b64url = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-';
            let mut segments = 0;
            while j < bytes.len() {
                let start = j;
                while j < bytes.len() && is_b64url(bytes[j]) {
                    j += 1;
                }
                if j == start {
                    break;
                }
                segments += 1;
                if j < bytes.len() && bytes[j] == b'.' && segments < 3 {
                    j += 1;
                } else {
                    break;
                }
            }
            if segments == 3 {
                out.push_str("<redacted-token>");
                i = j;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod redact_tests {
    use super::redact;

    #[test]
    fn masks_windows_username_in_path() {
        let text = r"Loading mods from C:\Users\JohnDoe\AppData\Roaming\.glacier\mods";
        let out = redact(text);
        assert!(!out.contains("JohnDoe"), "username leaked: {out}");
        assert!(out.contains(r"C:\Users\<redacted-user>\AppData"));
    }

    #[test]
    fn masks_unix_style_users_path() {
        let text = "path=/Users/jane.doe/Library/glacier";
        let out = redact(text);
        assert!(!out.contains("jane.doe"), "username leaked: {out}");
        assert!(out.contains("/Users/<redacted-user>/Library"));
    }

    #[test]
    fn masks_ipv4_addresses() {
        let text = "Connecting to server 203.0.113.42:25565 ...";
        let out = redact(text);
        assert!(!out.contains("203.0.113.42"), "ip leaked: {out}");
        assert!(out.contains("<redacted-ip>:25565"));
    }

    #[test]
    fn leaves_version_like_numbers_alone() {
        // A 3-segment version string must never be treated as an IPv4 address.
        let text = "Minecraft version 1.21.4 loaded";
        let out = redact(text);
        assert_eq!(out, text);
    }

    #[test]
    fn still_masks_tokens_and_jwts() {
        let text = "--accessToken abc123 done eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let out = redact(text);
        assert!(!out.contains("abc123"));
        assert!(out.contains("<redacted-token>"));
    }
}

/// Redacts and uploads log text to mclo.gs, returning the share URL —
/// mirrors `LogService.ShareAsync`.
#[tauri::command]
pub async fn share_log(state: State<'_, AppState>, content: String) -> Result<String, String> {
    let redacted = redact(&content);
    let resp = state
        .http
        .post("https://api.mclo.gs/1/log")
        .form(&[("content", redacted.as_str())])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    json.get("url")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "mclo.gs response had no url field".to_string())
}

/// QOL: "Export logs for support" - a single zip with the launcher version
/// + OS info, redacted (never-truncated) settings JSON, and the newest
/// `max_logs` redacted log files from `instance_dir`, so a user reporting a
/// problem can attach one file instead of hunting for the settings export
/// button, screenshotting version info, and finding the right log
/// separately. Returns the zip's path.
#[tauri::command(async)]
pub fn export_support_bundle(instance_dir: String, max_logs: Option<usize>) -> Result<String, String> {
    use std::io::Write;

    let dir = gl_settings::launcher_root().join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let zip_path = dir.join(format!("glacier-support-{stamp}.zip"));

    let file = std::fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // launcher_info.txt - version + platform, the two things any support
    // request needs first and that a user might otherwise misreport.
    let info = format!(
        "Glacier Launcher {}\nOS: {} ({})\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    zip.start_file("launcher_info.txt", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(info.as_bytes()).map_err(|e| e.to_string())?;

    // settings.json - same as the manual export, but with the two actual
    // secrets (Discord/Xbox refresh tokens) stripped. Everything else in
    // settings is preferences/paths, useful for reproducing a bug and not
    // sensitive the way a live auth token is.
    if let Ok(mut settings) = gl_settings::load() {
        settings.discord_token = String::new();
        settings.xbox_live_refresh_token = String::new();
        if let Ok(json) = serde_json::to_string_pretty(&settings) {
            zip.start_file("settings.json", options)
                .map_err(|e| e.to_string())?;
            zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    // The newest `max_logs` log/crash files, redacted the same way
    // `share_log` redacts before uploading anywhere.
    let mut logs = Vec::new();
    scan_log_dir(&Path::new(&instance_dir).join("logs"), false, &mut logs);
    scan_log_dir(&Path::new(&instance_dir).join("crash-reports"), true, &mut logs);
    logs.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    for log in logs.into_iter().take(max_logs.unwrap_or(5)) {
        let Ok(content) = std::fs::read_to_string(&log.path) else {
            continue;
        };
        let redacted = redact(&content);
        if zip.start_file(format!("logs/{}", log.name), options).is_ok() {
            let _ = zip.write_all(redacted.as_bytes());
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(zip_path.display().to_string())
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

const STATS_SCHEMA_VERSION: u32 = 1;
const MAX_SESSIONS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaySession {
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub seconds: i64,
    #[serde(default)]
    pub edition: String,
    #[serde(default)]
    pub label: String,
}

fn stats_path() -> PathBuf {
    gl_settings::launcher_root().join("stats.json")
}

fn load_sessions() -> Vec<PlaySession> {
    match gl_persistence::load::<Vec<PlaySession>>(&stats_path()) {
        Ok(Some(envelope)) => envelope.data,
        Ok(None) | Err(_) => Vec::new(),
    }
}

fn save_sessions(sessions: &[PlaySession]) -> Result<(), String> {
    gl_persistence::save(&stats_path(), STATS_SCHEMA_VERSION, &sessions.to_vec())
        .map_err(|e| e.to_string())
}

/// Appends a completed play session, trimming to the most recent
/// `MAX_SESSIONS` — mirrors `StatsService.RecordSession`.
#[tauri::command(async)]
pub fn record_play_session(
    started_at: String,
    seconds: i64,
    edition: String,
    label: String,
) -> Result<(), String> {
    if seconds <= 0 {
        return Ok(());
    }
    let mut sessions = load_sessions();
    sessions.push(PlaySession {
        started_at,
        seconds,
        edition,
        label,
    });
    if sessions.len() > MAX_SESSIONS {
        let excess = sessions.len() - MAX_SESSIONS;
        sessions.drain(0..excess);
    }
    save_sessions(&sessions)
}

#[derive(Debug, Serialize)]
pub struct StatsSummary {
    pub total_seconds: i64,
    pub session_count: usize,
    pub longest_session_seconds: i64,
    /// `(day, seconds)` pairs for the last `days` days, oldest first.
    pub daily_totals: Vec<(String, i64)>,
    pub recent_sessions: Vec<PlaySession>,
    /// `(label, count)` pairs, most-played first.
    pub top_labels: Vec<(String, usize)>,
}

/// Computes the full stats summary the frontend needs in one call, mirroring
/// `StatsService`'s `TotalSeconds`/`SessionCount`/`LongestSessionSeconds`/
/// `DailyTotals`/`RecentSessions`/`TopLabels`.
#[tauri::command(async)]
pub fn get_stats_summary(days: Option<u32>, recent_count: Option<usize>) -> StatsSummary {
    let sessions = load_sessions();
    let days = days.unwrap_or(14) as i64;
    let recent_count = recent_count.unwrap_or(20);

    let total_seconds: i64 = sessions.iter().map(|s| s.seconds).sum();
    let longest_session_seconds = sessions.iter().map(|s| s.seconds).max().unwrap_or(0);

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let today = now_secs / 86400;

    let mut buckets: Vec<(i64, i64)> = (0..days).map(|i| (today - (days - 1 - i), 0)).collect();
    for s in &sessions {
        if let Ok(dt) = chrono_like_day(&s.started_at) {
            if let Some(b) = buckets.iter_mut().find(|(day, _)| *day == dt) {
                b.1 += s.seconds;
            }
        }
    }
    let daily_totals = buckets
        .into_iter()
        .map(|(day, secs)| (day_to_iso(day), secs))
        .collect();

    let recent_sessions: Vec<PlaySession> =
        sessions.iter().rev().take(recent_count).cloned().collect();

    let mut label_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for s in &sessions {
        if !s.label.is_empty() {
            *label_counts.entry(s.label.clone()).or_insert(0) += 1;
        }
    }
    let mut top_labels: Vec<(String, usize)> = label_counts.into_iter().collect();
    top_labels.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    top_labels.truncate(5);

    StatsSummary {
        total_seconds,
        session_count: sessions.len(),
        longest_session_seconds,
        daily_totals,
        recent_sessions,
        top_labels,
    }
}

/// Parses an ISO-8601 `started_at` timestamp down to a day-number-since-epoch,
/// without pulling in a datetime crate — only the leading `YYYY-MM-DD` matters.
fn chrono_like_day(started_at: &str) -> Result<i64, ()> {
    let date_part = started_at.get(0..10).ok_or(())?;
    let mut parts = date_part.split('-');
    let y: i64 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
    let m: i64 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
    let d: i64 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
    Ok(days_from_civil(y, m, d))
}

fn day_to_iso(days_since_epoch: i64) -> String {
    let (y, m, d) = civil_from_days(days_since_epoch);
    format!("{y:04}-{m:02}-{d:02}")
}

// Howard Hinnant's civil_from_days / days_from_civil algorithms (public
// domain), used to avoid a chrono dependency for simple UTC day bucketing.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
