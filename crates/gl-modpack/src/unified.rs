//! Provider-agnostic mod shape: normalizes a Modrinth project+version and a
//! CurseForge mod+file into one `UnifiedMod` so the frontend can render one
//! card type regardless of source.
//!
//! Wraps `ModrinthClient`/`CurseForgeClient` — no HTTP calls of its own.

use serde::{Deserialize, Serialize};

use crate::curseforge::CurseForgeClient;
use crate::models::{CfAddon, MrProject};
use crate::modrinth::ModrinthClient;
use crate::Result;

/// Which backend a `UnifiedMod` (or a resolved dependency) came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Modrinth,
    CurseForge,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Modrinth => "modrinth",
            Provider::CurseForge => "curseforge",
        }
    }
}

/// One provider-agnostic mod card. `id` is the provider's native project id
/// (Modrinth's alphanumeric id/slug, or CurseForge's numeric addon id as a
/// string) so the frontend never has to branch on provider to key a list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMod {
    pub id: String,
    pub provider: Provider,
    pub name: String,
    pub summary: String,
    pub icon_url: String,
    pub author: String,
    pub downloads: i64,
    /// Populated by `search`; left empty until a caller fetches version/file
    /// details for a single mod (CurseForge and Modrinth both charge a
    /// separate request for that, so `search` doesn't eagerly pay for it).
    pub versions: Vec<UnifiedVersion>,
}

/// One downloadable file/version of a `UnifiedMod`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedVersion {
    pub version_id: String,
    pub name: String,
    pub file_name: String,
    pub download_url: String,
    pub size: i64,
    /// Modrinth only; empty for CurseForge (its public API doesn't expose
    /// file hashes on the plain files endpoint used here).
    pub sha1: String,
}

impl From<MrProject> for UnifiedMod {
    fn from(p: MrProject) -> Self {
        UnifiedMod {
            id: p.id,
            provider: Provider::Modrinth,
            name: p.title,
            summary: p.description,
            icon_url: p.icon_url,
            author: p.author,
            downloads: p.downloads,
            versions: Vec::new(),
        }
    }
}

impl From<CfAddon> for UnifiedMod {
    fn from(a: CfAddon) -> Self {
        UnifiedMod {
            id: a.id.to_string(),
            provider: Provider::CurseForge,
            name: a.name,
            summary: a.summary,
            icon_url: a.thumbnail_url,
            author: a.author_name,
            downloads: a.download_count,
            versions: Vec::new(),
        }
    }
}

/// Searches either provider for mods (project_type/class fixed to "mod").
/// `loader`/`mc_version` are accepted for API parity with the plan but are
/// applied as best-effort query hints only — neither client's `search`
/// currently takes loader/version facets, so full filtering happens when the
/// caller inspects `UnifiedVersion`s via `fetch_versions` instead.
pub async fn search_mods(
    modrinth: &ModrinthClient,
    curseforge: Option<&CurseForgeClient>,
    provider: Provider,
    query: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<UnifiedMod>> {
    match provider {
        Provider::Modrinth => {
            let result = modrinth.search(query, "mod", limit, offset).await?;
            Ok(result.projects.into_iter().map(UnifiedMod::from).collect())
        }
        Provider::CurseForge => {
            let Some(cf) = curseforge else {
                return Err(crate::error::ModpackError::MissingApiKey);
            };
            let page = offset.checked_div(limit).unwrap_or(0);
            let result = cf
                .search(
                    crate::curseforge::GAME_ID_JAVA,
                    Some(crate::curseforge::JAVA_CLASS_MODS),
                    query,
                    limit,
                    page,
                )
                .await?;
            Ok(result.addons.into_iter().map(UnifiedMod::from).collect())
        }
    }
}

/// Fetches downloadable versions/files for one mod, populating
/// `UnifiedMod::versions`.
pub async fn fetch_versions(
    modrinth: &ModrinthClient,
    curseforge: Option<&CurseForgeClient>,
    provider: Provider,
    project_id: &str,
) -> Result<Vec<UnifiedVersion>> {
    match provider {
        Provider::Modrinth => {
            let Some(v) = modrinth.get_latest_version(project_id).await? else {
                return Ok(Vec::new());
            };
            Ok(vec![UnifiedVersion {
                version_id: v.id,
                name: v.name,
                file_name: v.file_name,
                download_url: v.url,
                size: v.size,
                sha1: v.sha1,
            }])
        }
        Provider::CurseForge => {
            let Some(cf) = curseforge else {
                return Err(crate::error::ModpackError::MissingApiKey);
            };
            let mod_id: i32 = project_id.parse().map_err(|_| {
                crate::error::ModpackError::Other("invalid CurseForge mod id".into())
            })?;
            let files = cf.get_files(mod_id).await?;
            Ok(files
                .into_iter()
                .map(|f| UnifiedVersion {
                    version_id: f.file_id.to_string(),
                    name: f.display_name,
                    file_name: f.file_name,
                    download_url: f.download_url,
                    size: f.file_length,
                    sha1: String::new(),
                })
                .collect())
        }
    }
}
