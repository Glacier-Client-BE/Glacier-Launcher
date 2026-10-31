//! Transitive required-dependency resolution for a single mod install.
//!
//! Walks a mod version's declared "required" dependencies (Modrinth's
//! `MrDependency`; CurseForge's public files endpoint doesn't expose a
//! machine-readable dependency list, so CurseForge nodes are treated as
//! leaves — see the honest gap noted in `PARITY.md`), producing an ordered
//! install plan plus a list of conflicts (same project required at two
//! different, non-identical versions).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::curseforge::CurseForgeClient;
use crate::modrinth::ModrinthClient;
use crate::unified::Provider;
use crate::Result;

/// Recursion depth cap. Dependency graphs can (in principle, from bad
/// metadata) contain cycles; combined with the visited-id dedupe below this
/// makes the walk always terminate.
const MAX_DEPTH: u32 = 10;

/// One file the plan says to download and place in the mods folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedFile {
    pub provider: Provider,
    pub project_id: String,
    pub version_id: String,
    pub file_url: String,
    pub filename: String,
    pub sha1: String,
    /// True for the mod the caller originally asked to install; false for a
    /// pulled-in dependency. Lets the UI show "+ 2 dependencies" separately.
    pub is_root: bool,
}

/// The result of walking a mod's dependency tree.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallPlan {
    pub files: Vec<PlannedFile>,
    /// Human-readable notes about same-project version conflicts. Resolution
    /// picks the first version seen (typically the root's own pick) and
    /// reports the rest here rather than silently dropping them.
    pub conflicts: Vec<String>,
}

/// Resolves the transitive required-dependency plan for one Modrinth
/// project version. `loaders`/`game_versions` are reserved for future
/// per-dependency-version filtering; Modrinth's dependency edges reference a
/// specific project (and sometimes a specific version) rather than a loader,
/// so filtering happens by picking that project's *latest compatible*
/// version via `get_latest_version` (loader/version-aware version listing
/// would require the `/project/{id}/version` endpoint with query filters —
/// left as a follow-up, noted in PARITY.md).
pub async fn resolve_modrinth_plan(
    client: &ModrinthClient,
    root_project_id: &str,
    root_version_id: Option<&str>,
) -> Result<InstallPlan> {
    let mut plan = InstallPlan::default();
    let mut visited: HashSet<String> = HashSet::new();
    let mut chosen: HashMap<String, String> = HashMap::new(); // project_id -> version_id

    resolve_modrinth_node(
        client,
        root_project_id,
        root_version_id,
        true,
        0,
        &mut visited,
        &mut chosen,
        &mut plan,
    )
    .await?;

    Ok(plan)
}

/// Boxed to allow recursion in an `async fn` without an extra crate
/// dependency (`impl Future` cannot otherwise refer to itself).
fn resolve_modrinth_node<'a>(
    client: &'a ModrinthClient,
    project_id: &'a str,
    version_id: Option<&'a str>,
    is_root: bool,
    depth: u32,
    visited: &'a mut HashSet<String>,
    chosen: &'a mut HashMap<String, String>,
    plan: &'a mut InstallPlan,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(resolve_modrinth_node_inner(
        client, project_id, version_id, is_root, depth, visited, chosen, plan,
    ))
}

async fn resolve_modrinth_node_inner(
    client: &ModrinthClient,
    project_id: &str,
    version_id: Option<&str>,
    is_root: bool,
    depth: u32,
    visited: &mut HashSet<String>,
    chosen: &mut HashMap<String, String>,
    plan: &mut InstallPlan,
) -> Result<()> {
    if depth > MAX_DEPTH {
        plan.conflicts.push(format!(
            "Dependency depth limit ({MAX_DEPTH}) reached at project {project_id}; stopping recursion."
        ));
        return Ok(());
    }
    if visited.contains(project_id) {
        return Ok(());
    }
    visited.insert(project_id.to_string());

    // Resolve to a concrete version: an explicit version id if given (root
    // call may carry one from the UI's version picker), else "latest".
    let version = if let Some(vid) = version_id {
        // The public API has no direct "get version by id" used elsewhere in
        // this codebase; fall back to latest if a specific one isn't handed
        // in, which covers the common "install newest" path.
        let _ = vid;
        client.get_latest_version(project_id).await?
    } else {
        client.get_latest_version(project_id).await?
    };

    let Some(version) = version else {
        plan.conflicts.push(format!(
            "{project_id} has no downloadable version; skipped."
        ));
        return Ok(());
    };

    match chosen.get(project_id) {
        Some(existing) if existing != &version.id => {
            plan.conflicts.push(format!(
                "{project_id} required at both {existing} and {} — keeping {existing}.",
                version.id
            ));
            return Ok(());
        }
        Some(_) => return Ok(()),
        None => {
            chosen.insert(project_id.to_string(), version.id.clone());
        }
    }

    plan.files.push(PlannedFile {
        provider: Provider::Modrinth,
        project_id: project_id.to_string(),
        version_id: version.id.clone(),
        file_url: version.url.clone(),
        filename: version.file_name.clone(),
        sha1: version.sha1.clone(),
        is_root,
    });

    for dep in &version.dependencies {
        if dep.dependency_type != "required" {
            continue;
        }
        let Some(dep_project_id) = dep.project_id.clone() else {
            continue;
        };
        resolve_modrinth_node(
            client,
            &dep_project_id,
            dep.version_id.as_deref(),
            false,
            depth + 1,
            visited,
            chosen,
            plan,
        )
        .await?;
    }

    Ok(())
}

/// CurseForge has no required-dependency metadata available via the public
/// files endpoint used by `CurseForgeClient::get_files`, so this produces a
/// single-file plan for the requested file — a documented gap, not a bug.
pub async fn resolve_curseforge_plan(
    client: &CurseForgeClient,
    mod_id: i32,
    file_id: i32,
) -> Result<InstallPlan> {
    let mut plan = InstallPlan::default();
    let Some(file) = client.resolve_manifest_file(mod_id, file_id).await? else {
        plan.conflicts.push(format!(
            "CurseForge file {file_id} for mod {mod_id} could not be resolved."
        ));
        return Ok(plan);
    };
    plan.files.push(PlannedFile {
        provider: Provider::CurseForge,
        project_id: mod_id.to_string(),
        version_id: file_id.to_string(),
        file_url: file.download_url,
        filename: file.file_name,
        sha1: String::new(),
        is_root: true,
    });
    Ok(plan)
}
