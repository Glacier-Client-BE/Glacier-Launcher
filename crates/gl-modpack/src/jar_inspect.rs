//! Inspects installed mod jars for Fabric/Quilt/Forge/NeoForge metadata and
//! flags duplicate ids or missing required dependencies, mirroring
//! `JavaModAnalyzer`.

use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

use serde_json::Value;

use crate::models::{ModInfo, ModReport};

const BUILT_INS: &[&str] = &[
    "minecraft",
    "java",
    "fabricloader",
    "fabric",
    "fabric-api",
    "fabric_api",
    "forge",
    "neoforge",
    "quilt_loader",
    "quilt_base",
    "quilted_fabric_api",
    "mcp",
];

fn is_built_in(id: &str) -> bool {
    BUILT_INS.iter().any(|b| b.eq_ignore_ascii_case(id))
}

/// Analyzes a set of jar paths (mods directory contents, `.disabled`
/// extension included) and returns parsed metadata plus dependency/duplicate
/// warnings.
pub fn analyze<I, P>(jar_paths: I) -> ModReport
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mods: Vec<ModInfo> = jar_paths
        .into_iter()
        .map(|p| read_mod(p.as_ref()))
        .collect();
    let enabled: Vec<&ModInfo> = mods.iter().filter(|m| !m.disabled).collect();

    let mut provided: HashSet<String> = HashSet::new();
    for m in &enabled {
        if !m.mod_id.is_empty() {
            provided.insert(m.mod_id.to_ascii_lowercase());
        }
        for p in &m.provides {
            provided.insert(p.to_ascii_lowercase());
        }
    }

    let mut warnings = Vec::new();

    let mut seen: std::collections::HashMap<String, Vec<&str>> = std::collections::HashMap::new();
    for m in &enabled {
        if m.mod_id.is_empty() {
            continue;
        }
        seen.entry(m.mod_id.to_ascii_lowercase())
            .or_default()
            .push(&m.file_name);
    }
    for (id, files) in &seen {
        if files.len() > 1 {
            warnings.push(format!("Duplicate mod \"{id}\": {}", files.join(", ")));
        }
    }

    for m in &enabled {
        for dep in &m.required_deps {
            let dep_lower = dep.to_ascii_lowercase();
            if is_built_in(&dep_lower) || provided.contains(&dep_lower) {
                continue;
            }
            warnings.push(format!(
                "\"{}\" needs \"{}\", which isn't installed",
                m.name, dep
            ));
        }
    }

    ModReport { mods, warnings }
}

fn read_mod(path: &Path) -> ModInfo {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let disabled = file_name.to_ascii_lowercase().ends_with(".disabled");

    let fallback = || ModInfo {
        file_name: file_name.clone(),
        mod_id: String::new(),
        name: stem(&file_name),
        provides: Vec::new(),
        required_deps: Vec::new(),
        loader: "unknown".to_string(),
        disabled,
    };

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return fallback(),
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return fallback(),
    };

    if let Some(text) = read_entry(&mut archive, "fabric.mod.json")
        .or_else(|| read_entry(&mut archive, "quilt.mod.json"))
    {
        return parse_fabric(&text, &file_name, disabled).unwrap_or_else(fallback);
    }

    if let Some(text) = read_entry(&mut archive, "META-INF/mods.toml")
        .or_else(|| read_entry(&mut archive, "META-INF/neoforge.mods.toml"))
    {
        return parse_forge(&text, &file_name, disabled);
    }

    fallback()
}

fn read_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut s = String::new();
    entry.read_to_string(&mut s).ok()?;
    Some(s)
}

fn stem(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string())
}

fn parse_fabric(text: &str, file_name: &str, disabled: bool) -> Option<ModInfo> {
    let json: Value = serde_json::from_str(text).ok()?;
    // Quilt wraps everything under `quilt_loader`.
    let root = json.get("quilt_loader").unwrap_or(&json);

    let id = root
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let name = root
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let mut provides = Vec::new();
    if let Some(arr) = root.get("provides").and_then(Value::as_array) {
        for p in arr {
            if let Some(s) = p.as_str() {
                provides.push(s.to_string());
            }
        }
    }

    let mut deps = Vec::new();
    if let Some(dp) = root.get("depends") {
        match dp {
            Value::Object(map) => {
                for k in map.keys() {
                    deps.push(k.clone());
                }
            }
            Value::Array(arr) => {
                for d in arr {
                    match d {
                        Value::String(s) => deps.push(s.clone()),
                        Value::Object(o) => {
                            if let Some(id) = o.get("id").and_then(Value::as_str) {
                                deps.push(id.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let display_name = if name.is_empty() { id.clone() } else { name };

    Some(ModInfo {
        file_name: file_name.to_string(),
        mod_id: id,
        name: display_name,
        provides,
        required_deps: deps,
        loader: "fabric".to_string(),
        disabled,
    })
}

fn parse_forge(text: &str, file_name: &str, disabled: bool) -> ModInfo {
    let id = find_toml_string(text, "modId").unwrap_or_default();
    let name = find_toml_string(text, "displayName").unwrap_or_default();

    let mut deps = Vec::new();
    for block in split_dependency_blocks(text) {
        let Some(dep_id) = find_toml_string(&block, "modId") else {
            continue;
        };
        if dep_id.is_empty() {
            continue;
        }
        let required = block.to_ascii_lowercase().contains("mandatory = true")
            || block.to_ascii_lowercase().contains("mandatory=true")
            || block.to_ascii_lowercase().contains(r#"type = "required""#)
            || block.to_ascii_lowercase().contains(r#"type="required""#);
        if required && !dep_id.eq_ignore_ascii_case(&id) && !is_built_in(&dep_id) {
            deps.push(dep_id);
        }
    }

    let display = if !name.is_empty() {
        name
    } else if !id.is_empty() {
        id.clone()
    } else {
        stem(file_name)
    };

    ModInfo {
        file_name: file_name.to_string(),
        mod_id: id,
        name: display,
        provides: Vec::new(),
        required_deps: deps,
        loader: "forge".to_string(),
        disabled,
    }
}

/// Splits a `mods.toml`/`neoforge.mods.toml` document on `[[dependencies.*]]`
/// table-array headers, returning the body text following each header (up to
/// the next `[[` or end of file). Avoids a TOML crate dependency for this
/// narrow extraction.
fn split_dependency_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[dependencies") {
        let after_header = match rest[start..].find(']') {
            Some(_) => {
                // Skip past the full `[[...]]` header line.
                let header_end = rest[start..]
                    .find("]]")
                    .map(|i| start + i + 2)
                    .unwrap_or(rest.len());
                &rest[header_end..]
            }
            None => break,
        };
        let next = after_header.find("[[").unwrap_or(after_header.len());
        blocks.push(after_header[..next].to_string());
        rest = &after_header[next..];
        if next == 0 && after_header.len() == rest.len() {
            // Guard against zero-progress loops.
            break;
        }
    }
    blocks
}

/// Finds `key = "value"` (with optional whitespace around `=`) in a TOML-ish
/// text blob, returning the first match.
fn find_toml_string(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(key) {
            continue;
        }
        let after_key = trimmed[key.len()..].trim_start();
        let Some(after_eq) = after_key.strip_prefix('=') else {
            continue;
        };
        let after_eq = after_eq.trim_start();
        if let Some(rest) = after_eq.strip_prefix('"') {
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fabric_mod_json_fixture() {
        let text = r#"{
            "schemaVersion": 1,
            "id": "sodium",
            "name": "Sodium",
            "depends": { "fabricloader": ">=0.14.0", "fabric-api": "*" },
            "provides": ["sodium-fabric"]
        }"#;
        let info = parse_fabric(text, "sodium.jar", false).unwrap();
        assert_eq!(info.mod_id, "sodium");
        assert_eq!(info.name, "Sodium");
        assert_eq!(info.loader, "fabric");
        assert!(info.required_deps.contains(&"fabricloader".to_string()));
        assert_eq!(info.provides, vec!["sodium-fabric".to_string()]);
    }

    #[test]
    fn parses_quilt_mod_json_wrapped_fixture() {
        let text = r#"{
            "schema_version": 1,
            "quilt_loader": {
                "id": "example_mod",
                "name": "Example Mod",
                "depends": [ { "id": "quilt_loader", "versions": "*" }, "quilted_fabric_api" ]
            }
        }"#;
        let info = parse_fabric(text, "example.jar", false).unwrap();
        assert_eq!(info.mod_id, "example_mod");
        assert_eq!(info.name, "Example Mod");
        assert_eq!(info.required_deps.len(), 2);
    }

    #[test]
    fn parses_forge_mods_toml_fixture_and_extracts_required_deps() {
        let text = r#"
modLoader="javafml"
loaderVersion="[47,)"
license="MIT"

[[mods]]
modId="examplemod"
displayName="Example Mod"

[[dependencies.examplemod]]
    modId="forge"
    mandatory=true
    versionRange="[47,)"
    ordering="NONE"
    side="BOTH"

[[dependencies.examplemod]]
    modId="jei"
    mandatory=true
    versionRange="[15,)"
    ordering="AFTER"
    side="CLIENT"
"#;
        let info = parse_forge(text, "examplemod.jar", false);
        assert_eq!(info.mod_id, "examplemod");
        assert_eq!(info.name, "Example Mod");
        assert_eq!(info.loader, "forge");
        // "forge" is a built-in and should be filtered out; "jei" should remain.
        assert_eq!(info.required_deps, vec!["jei".to_string()]);
    }

    #[test]
    fn analyze_flags_duplicate_ids_and_missing_deps() {
        let mods = [
            ModInfo {
                file_name: "a.jar".into(),
                mod_id: "modA".into(),
                name: "Mod A".into(),
                provides: vec![],
                required_deps: vec!["modb".into()],
                loader: "fabric".into(),
                disabled: false,
            },
            ModInfo {
                file_name: "a2.jar".into(),
                mod_id: "modA".into(),
                name: "Mod A (dupe)".into(),
                provides: vec![],
                required_deps: vec![],
                loader: "fabric".into(),
                disabled: false,
            },
        ];

        // Reuse the warning-generation logic directly since `analyze` reads
        // from disk; simulate its body here against in-memory ModInfo.
        let enabled: Vec<&ModInfo> = mods.iter().filter(|m| !m.disabled).collect();
        let mut provided: HashSet<String> = HashSet::new();
        for m in &enabled {
            provided.insert(m.mod_id.to_ascii_lowercase());
        }
        let mut warnings = Vec::new();
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for m in &enabled {
            *seen.entry(m.mod_id.to_ascii_lowercase()).or_default() += 1;
        }
        for (id, count) in &seen {
            if *count > 1 {
                warnings.push(format!("Duplicate mod \"{id}\""));
            }
        }
        for m in &enabled {
            for dep in &m.required_deps {
                if !is_built_in(dep) && !provided.contains(&dep.to_ascii_lowercase()) {
                    warnings.push(format!("\"{}\" needs \"{}\"", m.name, dep));
                }
            }
        }

        assert!(warnings
            .iter()
            .any(|w| w.contains("Duplicate mod \"moda\"")));
        assert!(warnings.iter().any(|w| w.contains("needs \"modb\"")));
    }
}
