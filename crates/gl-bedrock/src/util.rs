//! Small filesystem helpers shared by the Bedrock services.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{IoContext, Result};

/// Current UTC time as an ISO-8601 string, mirroring `DateTime.UtcNow.ToString("o")`.
/// We don't depend on `chrono` here; a `time_t`-based RFC3339-ish stamp is enough
/// for sorting and display purposes.
pub fn now_iso() -> String {
    iso_from_systemtime(SystemTime::now())
}

/// Current UTC time formatted with a 7-digit (100ns-tick) fractional second,
/// matching the `yyyy-MM-ddTHH:mm:ss.fffffffZ` shape .NET's `DateTime`
/// `"o"`/WCF timestamp formatting produces — the Windows Update SOAP
/// protocol expects this exact shape in its `<Created>`/`<Expires>`/
/// `<currentTime>` elements (see `wu::security_header`).
pub fn iso7_now() -> String {
    iso7_from_systemtime(SystemTime::now())
}

/// Like [`iso7_now`] but for an arbitrary instant (e.g. "now + 5 minutes").
pub fn iso7_from_systemtime(t: SystemTime) -> String {
    let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let ticks = dur.subsec_nanos() / 100; // 100ns ticks, 7 digits

    let (y, m_num, d, h, mi, s) = civil_from_secs(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:07}Z",
        y, m_num, d, h, mi, s, ticks
    )
}

fn civil_from_secs(secs: u64) -> (i64, u64, u64, u64, u64, u64) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m_num = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m_num <= 2 { y + 1 } else { y };
    (y, m_num, d, h, m, s)
}

/// Renders an arbitrary [`SystemTime`] as an ISO-8601-ish UTC string.
pub fn iso_from_systemtime(t: SystemTime) -> String {
    let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();

    // Minimal civil-from-days conversion (Howard Hinnant's algorithm) so we can
    // render a real calendar date without pulling in a datetime crate.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m_num = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m_num <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, m_num, d, h, m, s, millis
    )
}

/// Recursively sums the byte size of every file under `dir`. Mirrors the C#
/// `DirectorySize` helpers used by the world/pack listers — best-effort, skips
/// entries it can't stat rather than failing the whole listing.
pub fn directory_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(walker) = walk_files(dir) else {
        return 0;
    };
    for file in walker {
        if let Ok(meta) = std::fs::metadata(&file) {
            total += meta.len();
        }
    }
    total
}

/// Depth-first enumeration of every regular file under `dir`.
pub fn walk_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_files_into(dir, &mut out)?;
    Ok(out)
}

fn walk_files_into(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).ctx(dir)?;
    for entry in entries {
        let entry = entry.ctx(dir)?;
        let path = entry.path();
        let file_type = entry.file_type().ctx(&path)?;
        if file_type.is_dir() {
            walk_files_into(&path, out)?;
        } else if file_type.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// Recursively copies `source` into `target`, creating directories as needed.
/// Best-effort per file, mirroring the C# `CopyDirectory` helper which
/// swallows individual file failures (locked files, permissions, etc.).
pub fn copy_directory(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target).ctx(target)?;
    for file in walk_files(source)? {
        let relative = file.strip_prefix(source).unwrap_or(&file);
        let dest = target.join(relative);
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(&file, &dest);
    }
    Ok(())
}

/// Writes `contents` to `path` atomically via a sibling temp file + rename,
/// mirroring `JsonStore.WriteAtomic`.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).ctx(dir)?;
        }
    }
    let tmp = {
        let mut s = path.as_os_str().to_os_string();
        s.push(format!(".{}.tmp", std::process::id()));
        PathBuf::from(s)
    };
    std::fs::write(&tmp, contents).ctx(&tmp)?;
    let result = std::fs::rename(&tmp, path).ctx(path);
    if tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Slugifies a display name into a filesystem/id-safe token, mirroring the
/// C# `Slug` helper on `BedrockInstanceService`.
pub fn slug(value: &str) -> String {
    let raw = if value.trim().is_empty() {
        "instance".to_string()
    } else {
        value.trim().to_lowercase()
    };

    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else {
            out.push('-');
        }
    }

    while out.contains("--") {
        out = out.replace("--", "-");
    }

    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "instance".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Sanitizes a string for use as a file name component (mirrors the
/// `string.Join("_", label.Split(Path.GetInvalidFileNameChars()))` pattern
/// used throughout the C# services).
pub fn sanitize_file_component(value: &str) -> String {
    const INVALID: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    value
        .chars()
        .map(|c| {
            if INVALID.contains(&c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect()
}
