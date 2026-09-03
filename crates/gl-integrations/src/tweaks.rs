//! VanillaTweaks (Java) + BedrockTweaks integration, ported from
//! `Services/TweaksService.cs`.
//!
//! Deviations from the C# service:
//! - Building the final resource pack (downloading the generated zip /
//!   downloading each selected BedrockTweaks blob and assembling the
//!   `.mcpack`) is a bulk-download concern. This module exposes the pure
//!   request/response logic (build the VanillaTweaks zip request, list the
//!   file blobs to fetch for a Bedrock selection, build the manifest.json)
//!   but does not perform the actual byte downloads — callers should wire
//!   those to `gl-download` once available (see `// TODO` markers below).
//! - Importing the finished `.mcpack` into CurseForge/Bedrock instances
//!   (`CurseForgeService.ImportBedrockPackFileAsync`) is out of scope for
//!   this crate; that belongs with the Bedrock pack management code.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{IntegrationError, Result};
use crate::models::{TweakCategory, TweakPack};

pub const JAVA_VERSIONS: &[&str] = &["1.21", "1.20", "1.19", "1.18", "1.17", "1.16"];

const BT_REPO: &str = "BedrockTweaks/Files";
const BT_BRANCH: &str = "devel";

/// Fetches and flattens the VanillaTweaks resource-pack category tree for a
/// given Java version (e.g. `"1.21"`).
pub async fn get_vanilla_resource_packs(
    client: &reqwest::Client,
    version: &str,
) -> Result<Vec<TweakCategory>> {
    let url =
        format!("https://vanillatweaks.net/assets/resources/json/{version}/rpcategories.json");
    let json: Value = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut result = Vec::new();
    if let Some(cats) = json.get("categories").and_then(Value::as_array) {
        for node in cats {
            flatten_vanilla(node, &mut result, version);
        }
    }
    Ok(result)
}

fn flatten_vanilla(node: &Value, out: &mut Vec<TweakCategory>, version: &str) {
    let name = node
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if let Some(packs) = node.get("packs").and_then(Value::as_array) {
        if !packs.is_empty() {
            let mut cat = TweakCategory {
                id: name.clone(),
                name: name.clone(),
                packs: Vec::new(),
            };
            for p in packs {
                let id = p
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                cat.packs.push(TweakPack {
                    id: id.clone(),
                    name: p.get("display").and_then(Value::as_str).unwrap_or("").to_string(),
                    description: strip_html(p.get("description").and_then(Value::as_str)),
                    category_id: String::new(),
                    category: name.clone(),
                    icon_url: format!(
                        "https://vanillatweaks.net/assets/resources/icons/resourcepacks/{version}/{id}.png"
                    ),
                });
            }
            if !cat.packs.is_empty() {
                out.push(cat);
            }
        }
    }

    if let Some(sub) = node.get("categories").and_then(Value::as_array) {
        for s in sub {
            flatten_vanilla(s, out, version);
        }
    }
}

#[derive(Debug, Deserialize)]
struct ZipResourcePacksResponse {
    status: Option<String>,
    link: Option<String>,
}

/// Calls the VanillaTweaks zip-generation endpoint for the given selection
/// (category id -> list of pack ids) and returns the full download URL for
/// the generated zip.
///
/// Mirrors `TweaksService.BuildVanillaResourcePackAsync` up through
/// obtaining the download link; the actual byte download + file write to
/// `<mc dir>/resourcepacks/<file>` is left to the caller (`gl-download`).
pub async fn request_vanilla_resource_pack_zip(
    client: &reqwest::Client,
    selection: &HashMap<String, Vec<String>>,
    version: &str,
) -> Result<String> {
    if selection.is_empty() {
        return Err(IntegrationError::Message(
            "Select at least one tweak first.".to_string(),
        ));
    }

    let packs_json = serde_json::to_string(selection)?;
    let form = [("version", version), ("packs", packs_json.as_str())];

    let resp = client
        .post("https://vanillatweaks.net/assets/server/zipresourcepacks.php")
        .form(&form)
        .send()
        .await?
        .error_for_status()?;

    let parsed: ZipResourcePacksResponse = resp.json().await?;
    if parsed.status.as_deref() != Some("success") {
        return Err(IntegrationError::Message(
            "VanillaTweaks couldn't build that selection. Try fewer / different tweaks."
                .to_string(),
        ));
    }
    let link = parsed.link.ok_or_else(|| {
        IntegrationError::Message("VanillaTweaks returned no download link.".to_string())
    })?;

    Ok(format!("https://vanillatweaks.net{link}"))
    // TODO: wire to gl-download once available — download `downloadUrl` and
    // write it to <minecraft_dir>/resourcepacks/<sanitized file name>.
}

#[derive(Debug, Default)]
pub struct BedrockTweaksIndex {
    pub categories: Vec<TweakCategory>,
    pub engine_version: [i32; 3],
    pub version_label: String,
}

/// Fetches and parses the BedrockTweaks `packs.json` category tree.
pub async fn get_bedrock_resource_packs(client: &reqwest::Client) -> Result<BedrockTweaksIndex> {
    let url = format!(
        "https://raw.githubusercontent.com/{BT_REPO}/{BT_BRANCH}/resource_packs/packs.json"
    );
    let root: Value = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut engine_version = [1, 20, 0];
    let mut version_label = String::new();
    if let Some(ver) = root.get("version").and_then(Value::as_array) {
        if ver.len() >= 2 {
            let maj = ver[0].as_i64().unwrap_or(1) as i32;
            let min = ver[1].as_i64().unwrap_or(0) as i32;
            engine_version = [maj, min.min(21), 0];
            version_label = ver
                .iter()
                .map(|v| v.as_i64().unwrap_or(0).to_string())
                .collect::<Vec<_>>()
                .join(".");
        }
    }

    let mut result = Vec::new();
    if let Some(cats) = root.get("categories").and_then(Value::as_array) {
        for c in cats {
            let cat_id = c
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let cat_name = c
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| cat_id.clone());
            let mut cat = TweakCategory {
                id: cat_id.clone(),
                name: cat_name.clone(),
                packs: Vec::new(),
            };
            if let Some(packs) = c.get("packs").and_then(Value::as_array) {
                for p in packs {
                    let id = p
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    cat.packs.push(TweakPack {
                        id: id.clone(),
                        name: p.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                        description: strip_html(p.get("description").and_then(Value::as_str)),
                        category_id: cat_id.clone(),
                        category: cat_name.clone(),
                        icon_url: format!(
                            "https://cdn.jsdelivr.net/gh/{BT_REPO}@{BT_BRANCH}/resource_packs/files/{cat_id}/{id}/pack_icon.png"
                        ),
                    });
                }
            }
            if !cat.packs.is_empty() {
                result.push(cat);
            }
        }
    }

    Ok(BedrockTweaksIndex {
        categories: result,
        engine_version,
        version_label,
    })
}

/// Given a git tree listing from the GitHub API (`{repo}/git/trees/{branch}?recursive=1`,
/// already fetched/parsed by the caller — GitHub API auth/caching is out of
/// scope here) and a selection of Bedrock tweak packs, returns the list of
/// blob paths that need to be downloaded, paired with the prefix that
/// should be stripped to get the relative in-pack path.
///
/// Mirrors the blob-matching loop in `TweaksService.BuildBedrockResourcePackAsync`.
pub fn resolve_bedrock_blobs(
    tree_entries: &[Value],
    selection: &[TweakPack],
) -> Vec<(String, String)> {
    let wanted: Vec<String> = selection
        .iter()
        .map(|p| format!("resource_packs/files/{}/{}/", p.category_id, p.id))
        .collect();

    let mut blobs = Vec::new();
    for e in tree_entries {
        let is_blob = e.get("type").and_then(Value::as_str) == Some("blob");
        let path = e.get("path").and_then(Value::as_str);
        if let (true, Some(path)) = (is_blob, path) {
            if let Some(prefix) = wanted.iter().find(|w| path.starts_with(w.as_str())) {
                blobs.push((prefix.clone(), path.to_string()));
            }
        }
    }
    blobs
}

#[derive(Debug, Serialize)]
struct BedrockManifestHeader {
    name: String,
    description: String,
    uuid: String,
    version: [i32; 3],
    min_engine_version: [i32; 3],
}

#[derive(Debug, Serialize)]
struct BedrockManifestModule {
    #[serde(rename = "type")]
    kind: String,
    uuid: String,
    version: [i32; 3],
}

#[derive(Debug, Serialize)]
struct BedrockManifest {
    format_version: i32,
    header: BedrockManifestHeader,
    modules: Vec<BedrockManifestModule>,
}

/// Builds the `manifest.json` contents for an assembled BedrockTweaks pack.
/// Mirrors `TweaksService.WriteBedrockManifest`. UUIDs are supplied by the
/// caller (this crate does not depend on a UUID-generation crate).
pub fn build_bedrock_manifest(
    selection: &[TweakPack],
    engine_version: [i32; 3],
    header_uuid: String,
    module_uuid: String,
) -> Result<String> {
    let name = if selection.len() == 1 {
        selection[0].name.clone()
    } else {
        format!("BedrockTweaks ({})", selection.len())
    };
    let desc = if selection.len() == 1 {
        selection[0].description.clone()
    } else {
        "Merged BedrockTweaks selection built by Glacier Launcher.".to_string()
    };
    let description = if desc.trim().is_empty() {
        name.clone()
    } else {
        desc
    };

    let manifest = BedrockManifest {
        format_version: 2,
        header: BedrockManifestHeader {
            name,
            description,
            uuid: header_uuid,
            version: [1, 0, 0],
            min_engine_version: engine_version,
        },
        modules: vec![BedrockManifestModule {
            kind: "resources".to_string(),
            uuid: module_uuid,
            version: [1, 0, 0],
        }],
    };

    Ok(serde_json::to_string_pretty(&manifest)?)
}

fn strip_html(input: Option<&str>) -> String {
    let Some(s) = input else { return String::new() };
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    crate::news::html_decode(out.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_vanilla_categories_recursively() {
        let json: Value = serde_json::from_str(
            r#"{
                "categories": [
                    {
                        "category": "Parent",
                        "categories": [
                            {
                                "category": "Child",
                                "packs": [
                                    {"name": "foo", "display": "Foo", "description": "A <b>bold</b> pack"}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut result = Vec::new();
        for node in json["categories"].as_array().unwrap() {
            flatten_vanilla(node, &mut result, "1.21");
        }
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "Child");
        assert_eq!(result[0].packs[0].id, "foo");
        assert_eq!(result[0].packs[0].description, "A bold pack");
        assert_eq!(
            result[0].packs[0].icon_url,
            "https://vanillatweaks.net/assets/resources/icons/resourcepacks/1.21/foo.png"
        );
    }

    #[test]
    fn resolves_bedrock_blobs_by_prefix() {
        let entries: Vec<Value> = serde_json::from_str(
            r#"[
                {"type": "blob", "path": "resource_packs/files/cat1/pack1/manifest.json"},
                {"type": "blob", "path": "resource_packs/files/cat1/pack1/pack_icon.png"},
                {"type": "blob", "path": "resource_packs/files/other/pack2/manifest.json"},
                {"type": "tree", "path": "resource_packs/files/cat1/pack1"}
            ]"#,
        )
        .unwrap();
        let selection = vec![TweakPack {
            id: "pack1".to_string(),
            category_id: "cat1".to_string(),
            ..Default::default()
        }];
        let blobs = resolve_bedrock_blobs(&entries, &selection);
        assert_eq!(blobs.len(), 2);
        assert!(blobs
            .iter()
            .all(|(_, p)| p.starts_with("resource_packs/files/cat1/pack1/")));
    }

    #[test]
    fn builds_manifest_for_single_pack() {
        let selection = vec![TweakPack {
            name: "Cool Pack".to_string(),
            description: "Does cool things".to_string(),
            ..Default::default()
        }];
        let json = build_bedrock_manifest(
            &selection,
            [1, 20, 0],
            "header-uuid".to_string(),
            "module-uuid".to_string(),
        )
        .unwrap();
        let value: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["header"]["name"], "Cool Pack");
        assert_eq!(value["header"]["description"], "Does cool things");
        assert_eq!(value["modules"][0]["type"], "resources");
    }

    #[test]
    fn strips_html_tags() {
        assert_eq!(strip_html(Some("<b>Bold</b> &amp; plain")), "Bold & plain");
        assert_eq!(strip_html(None), "");
    }
}
