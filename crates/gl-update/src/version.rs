//! Semantic-ish version comparison, mirroring `AutoUpdateService.IsNewerVersion`.
//!
//! The C# code parses both strings with `System.Version` (which allows 2-4
//! numeric components: `major.minor[.build[.revision]]`) and falls back to an
//! ordinal string comparison if parsing fails. We mirror that: parse into up
//! to 4 numeric components, comparing missing components as 0; if either
//! string fails to parse as dotted numerics, fall back to an ordinal
//! (byte-wise) string comparison.

/// Parse a version string into up to 4 numeric components (`major.minor.build.revision`),
/// matching the permissive parsing behavior of .NET's `System.Version`.
fn parse_version(s: &str) -> Option<[i64; 4]> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.is_empty() || parts.len() > 4 {
        return None;
    }
    let mut out = [0i64; 4];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p.parse::<i64>().ok()?;
    }
    Some(out)
}

/// Returns true if `remote` is a newer version than `local`.
///
/// Mirrors `AutoUpdateService.IsNewerVersion`: attempt numeric dotted-version
/// comparison first, falling back to ordinal string comparison if either
/// side fails to parse.
pub fn is_newer_version(remote: &str, local: &str) -> bool {
    match (parse_version(remote), parse_version(local)) {
        (Some(r), Some(l)) => r > l,
        _ => remote > local,
    }
}

/// Strip a leading `v`/`V` from a tag name, matching `tag.TrimStart('v', 'V')`.
pub fn normalize_tag(tag: &str) -> String {
    tag.trim_start_matches(['v', 'V']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_comparison() {
        assert!(is_newer_version("1.2.0", "1.1.9"));
        assert!(!is_newer_version("1.1.9", "1.2.0"));
        assert!(!is_newer_version("1.2.0", "1.2.0"));
    }

    #[test]
    fn differing_component_counts() {
        assert!(is_newer_version("1.2.0.1", "1.2.0"));
        assert!(!is_newer_version("1.2", "1.2.0.0"));
        assert!(is_newer_version("2.0", "1.9.9.9"));
    }

    #[test]
    fn fallback_to_ordinal_on_unparsable() {
        // Non-numeric components can't be parsed as System.Version -> ordinal compare.
        assert!(is_newer_version("b", "a"));
        assert!(!is_newer_version("a", "b"));
        assert!(is_newer_version("1.2.0-beta", "1.2.0-alpha"));
    }

    #[test]
    fn normalize_tag_strips_v_prefix() {
        assert_eq!(normalize_tag("v1.2.3"), "1.2.3");
        assert_eq!(normalize_tag("V1.2.3"), "1.2.3");
        assert_eq!(normalize_tag("1.2.3"), "1.2.3");
        assert_eq!(normalize_tag("vV1.0.0"), "1.0.0");
    }
}
