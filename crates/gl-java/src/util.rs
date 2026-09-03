use crate::error::{io_err, Result};
use std::path::Path;

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
    std::fs::write(&tmp, contents).map_err(io_err(&tmp))?;
    std::fs::rename(&tmp, path).map_err(io_err(path))?;
    Ok(())
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
