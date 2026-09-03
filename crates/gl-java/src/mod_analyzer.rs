//! Port of `Services/JavaModAnalyzer.cs`: reads installed mod jars in an
//! instance's `mods/` folder and reports basic metadata plus dependency /
//! duplicate-id warnings. Also provides the enable/disable/delete file
//! operations the original wired up via `.disabled` renaming.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModInfo {
    pub file_name: String,
    pub mod_id: String,
    pub name: String,
    pub provides: Vec<String>,
    pub required_deps: Vec<String>,
    pub loader: String,
    pub disabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModReport {
    pub mods: Vec<ModInfo>,
    pub warnings: Vec<String>,
}

fn built_ins() -> HashSet<&'static str> {
    [
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
    ]
    .into_iter()
    .collect()
}

/// List `.jar`/`.jar.disabled` files directly under `mods_dir` and analyze them.
pub fn analyze_dir(mods_dir: &Path) -> ModReport {
    let Ok(entries) = std::fs::read_dir(mods_dir) else {
        return ModReport::default();
    };
    let paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && (matches!(p.extension().and_then(|e| e.to_str()), Some("jar"))
                    || is_disabled_jar(p))
        })
        .collect();
    analyze(&paths)
}

fn is_disabled_jar(p: &Path) -> bool {
    p.to_string_lossy()
        .to_lowercase()
        .ends_with(".jar.disabled")
}

pub fn analyze(jar_paths: &[PathBuf]) -> ModReport {
    let mods: Vec<ModInfo> = jar_paths.iter().map(|p| try_read(p)).collect();
    let enabled: Vec<&ModInfo> = mods.iter().filter(|m| !m.disabled).collect();

    let mut provided: HashSet<String> = HashSet::new();
    for m in &enabled {
        if !m.mod_id.is_empty() {
            provided.insert(m.mod_id.to_lowercase());
        }
        for p in &m.provides {
            provided.insert(p.to_lowercase());
        }
    }

    let mut warnings = Vec::new();
    let builtins = built_ins();

    let mut by_id: HashMap<String, Vec<&str>> = HashMap::new();
    for m in &enabled {
        if m.mod_id.is_empty() {
            continue;
        }
        by_id
            .entry(m.mod_id.to_lowercase())
            .or_default()
            .push(&m.file_name);
    }
    for (id, files) in &by_id {
        if files.len() > 1 {
            warnings.push(format!("Duplicate mod \"{id}\": {}", files.join(", ")));
        }
    }

    for m in &enabled {
        for dep in &m.required_deps {
            let dep_lower = dep.to_lowercase();
            if builtins.contains(dep_lower.as_str()) || provided.contains(&dep_lower) {
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

fn try_read(path: &Path) -> ModInfo {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let disabled = file_name.to_lowercase().ends_with(".disabled");

    let fallback = || ModInfo {
        file_name: file_name.clone(),
        mod_id: String::new(),
        name: stem(&file_name),
        provides: Vec::new(),
        required_deps: Vec::new(),
        loader: "unknown".to_string(),
        disabled,
    };

    let Ok(file) = std::fs::File::open(path) else {
        return fallback();
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return fallback();
    };

    let fabric_name = if zip.by_name("fabric.mod.json").is_ok() {
        Some("fabric.mod.json")
    } else if zip.by_name("quilt.mod.json").is_ok() {
        Some("quilt.mod.json")
    } else {
        None
    };
    if let Some(name) = fabric_name {
        if let Ok(mut entry) = zip.by_name(name) {
            let mut text = String::new();
            if entry.read_to_string(&mut text).is_ok() {
                return read_fabric(&text, &file_name, disabled);
            }
        }
    }

    let forge_name = if zip.by_name("META-INF/mods.toml").is_ok() {
        Some("META-INF/mods.toml")
    } else if zip.by_name("META-INF/neoforge.mods.toml").is_ok() {
        Some("META-INF/neoforge.mods.toml")
    } else {
        None
    };
    if let Some(name) = forge_name {
        if let Ok(mut entry) = zip.by_name(name) {
            let mut text = String::new();
            if entry.read_to_string(&mut text).is_ok() {
                return read_forge(&text, &file_name, disabled);
            }
        }
    }

    fallback()
}

fn stem(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.to_string())
}

fn read_fabric(text: &str, file_name: &str, disabled: bool) -> ModInfo {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(text) else {
        return ModInfo {
            file_name: file_name.to_string(),
            mod_id: String::new(),
            name: stem(file_name),
            provides: Vec::new(),
            required_deps: Vec::new(),
            loader: "fabric".to_string(),
            disabled,
        };
    };
    // quilt.mod.json nests everything under "quilt_loader".
    let root = root.get("quilt_loader").unwrap_or(&root);

    let id = root
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = root
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(&id)
        .to_string();

    let mut provides = Vec::new();
    if let Some(arr) = root.get("provides").and_then(|v| v.as_array()) {
        for p in arr {
            if let Some(s) = p.as_str() {
                provides.push(s.to_string());
            }
        }
    }

    let mut deps = Vec::new();
    if let Some(dp) = root.get("depends") {
        if let Some(obj) = dp.as_object() {
            deps.extend(obj.keys().cloned());
        } else if let Some(arr) = dp.as_array() {
            for d in arr {
                if let Some(s) = d.as_str() {
                    deps.push(s.to_string());
                } else if let Some(id) = d.get("id").and_then(|v| v.as_str()) {
                    deps.push(id.to_string());
                }
            }
        }
    }

    ModInfo {
        file_name: file_name.to_string(),
        mod_id: id,
        name: if name.is_empty() {
            stem(file_name)
        } else {
            name
        },
        provides,
        required_deps: deps,
        loader: "fabric".to_string(),
        disabled,
    }
}

fn read_forge(text: &str, file_name: &str, disabled: bool) -> ModInfo {
    let id = capture(text, "modId");
    let name = capture(text, "displayName");

    let mut deps = Vec::new();
    for block in split_dependency_blocks(text) {
        let dep_id = capture(&block, "modId");
        if dep_id.is_empty() {
            continue;
        }
        let required = block.to_lowercase().contains("mandatory")
            && block.to_lowercase().contains("true")
            || block.to_lowercase().contains(r#"type="required""#)
            || block.to_lowercase().contains("type = \"required\"");
        if required
            && !dep_id.eq_ignore_ascii_case(&id)
            && !built_ins().contains(dep_id.to_lowercase().as_str())
        {
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

/// Very small stand-in for the C# `Regex.Match(text, "key\\s*=\\s*\"([^\"]+)\"")`.
fn capture(text: &str, key: &str) -> String {
    let Some(pos) = text.find(key) else {
        return String::new();
    };
    let rest = &text[pos + key.len()..];
    let Some(eq) = rest.find('=') else {
        return String::new();
    };
    let after_eq = &rest[eq + 1..];
    let Some(start) = after_eq.find('"') else {
        return String::new();
    };
    let after_quote = &after_eq[start + 1..];
    let Some(end) = after_quote.find('"') else {
        return String::new();
    };
    after_quote[..end].to_string()
}

/// Split `[[dependencies.*]] ... [[` blocks from a mods.toml document.
fn split_dependency_blocks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find("[[dependencies") {
        let after = &rest[pos..];
        let Some(header_end) = after.find("]]") else {
            break;
        };
        let body_start = header_end + 2;
        let body = &after[body_start..];
        let next = body.find("[[").unwrap_or(body.len());
        out.push(body[..next].to_string());
        rest = &body[next..];
    }
    out
}

/// Toggle a mod file between enabled (`.jar`) and disabled (`.jar.disabled`).
/// Idiot-proof: no-ops if the source doesn't exist, refuses to overwrite an
/// existing destination.
pub fn set_mod_enabled(mods_dir: &Path, file_name: &str, enabled: bool) -> std::io::Result<String> {
    let src = mods_dir.join(file_name);
    let dest_name = if enabled {
        file_name
            .strip_suffix(".disabled")
            .unwrap_or(file_name)
            .to_string()
    } else if file_name.to_lowercase().ends_with(".disabled") {
        file_name.to_string()
    } else {
        format!("{file_name}.disabled")
    };
    let dest = mods_dir.join(&dest_name);
    if src == dest {
        return Ok(dest_name);
    }
    if dest.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{} already exists", dest.display()),
        ));
    }
    std::fs::rename(&src, &dest)?;
    Ok(dest_name)
}

pub fn delete_mod(mods_dir: &Path, file_name: &str) -> std::io::Result<()> {
    let path = mods_dir.join(file_name);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_zip(path: &Path, entries: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn analyzes_fabric_mod_json() {
        let tmp = tempfile::tempdir().unwrap();
        let jar = tmp.path().join("test.jar");
        write_zip(
            &jar,
            &[(
                "fabric.mod.json",
                r#"{"id":"testmod","name":"Test Mod","depends":{"fabric-api":"*"}}"#,
            )],
        );
        let info = try_read(&jar);
        assert_eq!(info.mod_id, "testmod");
        assert_eq!(info.name, "Test Mod");
        assert_eq!(info.loader, "fabric");
        assert!(info.required_deps.contains(&"fabric-api".to_string()));
        assert!(!info.disabled);
    }

    #[test]
    fn analyzes_forge_mods_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let jar = tmp.path().join("forgemod.jar");
        let toml = r#"
modId="forgemod"
displayName="Forge Mod"
[[dependencies.forgemod]]
    modId="needsthis"
    mandatory=true
    type="required"
"#;
        write_zip(&jar, &[("META-INF/mods.toml", toml)]);
        let info = try_read(&jar);
        assert_eq!(info.mod_id, "forgemod");
        assert_eq!(info.name, "Forge Mod");
        assert_eq!(info.loader, "forge");
        assert!(info.required_deps.contains(&"needsthis".to_string()));
    }

    #[test]
    fn warns_on_missing_dependency_and_duplicate_id() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.jar");
        let b = tmp.path().join("b.jar");
        write_zip(
            &a,
            &[(
                "fabric.mod.json",
                r#"{"id":"dup","name":"A","depends":{"missing-mod":"*"}}"#,
            )],
        );
        write_zip(&b, &[("fabric.mod.json", r#"{"id":"dup","name":"B"}"#)]);
        let report = analyze(&[a, b]);
        assert_eq!(report.mods.len(), 2);
        assert!(report.warnings.iter().any(|w| w.contains("Duplicate mod")));
        assert!(report.warnings.iter().any(|w| w.contains("missing-mod")));
    }

    #[test]
    fn set_mod_enabled_toggles_extension() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mymod.jar"), b"fake").unwrap();
        let disabled = set_mod_enabled(tmp.path(), "mymod.jar", false).unwrap();
        assert_eq!(disabled, "mymod.jar.disabled");
        assert!(tmp.path().join("mymod.jar.disabled").exists());

        let enabled = set_mod_enabled(tmp.path(), &disabled, true).unwrap();
        assert_eq!(enabled, "mymod.jar");
        assert!(tmp.path().join("mymod.jar").exists());
    }

    #[test]
    fn set_mod_enabled_refuses_to_overwrite_existing() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mymod.jar"), b"fake").unwrap();
        std::fs::write(tmp.path().join("mymod.jar.disabled"), b"other").unwrap();
        let err = set_mod_enabled(tmp.path(), "mymod.jar", false).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    }
}
