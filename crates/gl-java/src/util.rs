use crate::error::{io_err, Result};
use std::path::{Path, PathBuf};

/// Joins a relative path taken from untrusted JSON (library `path`, asset
/// object name, ...) onto `base`. Returns `None` when the path could escape
/// `base`: any `..`, a root or drive prefix, or a `:` (drive/ADS syntax on
/// Windows). Empty and `.` segments are skipped.
pub fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut out = base.to_path_buf();
    let mut pushed = false;
    for seg in rel.split(['/', '\\']) {
        match seg {
            "" | "." => continue,
            s if s == ".." || s.contains(':') => return None,
            s => {
                out.push(s);
                pushed = true;
            }
        }
    }
    pushed.then_some(out)
}

/// Atomically write `contents` to `path` by writing to a sibling `.tmp` file
/// and renaming it over the destination. Mirrors `JsonStore.WriteAtomic` in the
/// C# codebase (best-effort port; the original also does file-lock retries which
/// aren't applicable here since we don't have concurrent WPF UI threads).
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_err(parent))?;
    }
    let tmp = path.with_extension(match path.extension() {
        Some(ext) => format!("{}.tmp", ext.to_string_lossy()),
        None => "tmp".to_string(),
    });
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp).map_err(io_err(&tmp))?;
        file.write_all(contents.as_bytes()).map_err(io_err(&tmp))?;
        file.sync_all().map_err(io_err(&tmp))?;
    }
    std::fs::rename(&tmp, path).map_err(io_err(path))?;
    Ok(())
}

/// Builds the HTTP client used for Java/runtime metadata and downloads.
/// `reqwest::Client::new()` has no timeouts, so a stalled connection could hang
/// a download forever. We bound connection setup and idle reads (not the whole
/// transfer, since runtimes are large). Falls back to the default client rather
/// than panicking if the builder fails.
pub(crate) fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(gl_download::USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(15))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Slugify a name the same way the C# `Slug` helper does: lowercase,
/// non-alphanumeric -> '-', collapse repeated dashes, trim dashes.
pub fn slug(value: &str) -> String {
    let raw = if value.trim().is_empty() {
        "default".to_string()
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
    let mut slug = out.trim_matches('-').to_string();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    if slug.is_empty() {
        "default".to_string()
    } else {
        slug
    }
}

/// Quote a command-line argument the same way the C# `Quote` helper does:
/// wrap in double quotes (escaping embedded quotes) if the value is empty or
/// contains whitespace/quotes.
pub fn quote_arg(v: &str) -> String {
    if v.is_empty() || v.contains([' ', '\t', '"']) {
        format!("\"{}\"", v.replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_keeps_normal_relative_paths_under_base() {
        let base = Path::new("base");
        assert_eq!(
            safe_join(base, "com/x/y-1.0.jar"),
            Some(base.join("com").join("x").join("y-1.0.jar"))
        );
        assert_eq!(
            safe_join(base, "a\\b.txt"),
            Some(base.join("a").join("b.txt"))
        );
    }

    #[test]
    fn safe_join_rejects_escaping_paths() {
        let base = Path::new("base");
        assert_eq!(safe_join(base, "../../evil.jar"), None);
        assert_eq!(safe_join(base, "a/../../evil"), None);
        assert_eq!(safe_join(base, "/etc/passwd"), Some(base.join("etc").join("passwd")));
        assert_eq!(safe_join(base, "C:/Windows/x"), None);
        assert_eq!(safe_join(base, "x:stream"), None);
        assert_eq!(safe_join(base, ""), None);
    }
}
