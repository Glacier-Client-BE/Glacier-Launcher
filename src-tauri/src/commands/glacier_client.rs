//! Glacier Client (the launcher's own bundled Java-agent client) — ports the
//! four commands `LauncherClientsView.svelte` documented as missing:
//! `get_glacier_manifest`, `install_glacier_client`, `uninstall_glacier_client`,
//! `launch_glacier_client`. Mirrors `Services/GlacierClientService.cs` +
//! `Pages/Home.Glacier.cs`.
//!
//! `gl_clients::glacier_client` already had the manifest fetch/parse and
//! path/size helpers fully ported and tested; this file is the missing
//! Tauri-command layer around it, plus the download/launch steps that crate
//! deliberately left to the caller (`gl-download`/`gl-launch` are the
//! caller's dependencies, not `gl-clients`'s).

use gl_clients::models::{GlacierClientVersion, GlacierManifest};
use tauri::State;

use crate::state::AppState;

fn user_profile() -> std::path::PathBuf {
    std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
}

/// Fetches the Glacier Client version manifest. Mirrors
/// `GlacierClientService.GetManifestAsync` — the C# caches the manifest
/// in-process; this doesn't (a fresh fetch per open is cheap and simpler),
/// so `force_refresh` is accepted but unused, kept only to match the
/// original call shape the frontend already expects.
#[tauri::command]
pub async fn get_glacier_manifest(
    state: State<'_, AppState>,
    #[allow(unused_variables)] force_refresh: Option<bool>,
) -> Result<GlacierManifest, String> {
    gl_clients::glacier_client::fetch_manifest(&state.http)
        .await
        .map_err(|e| e.to_string())
}

/// Fetches the live per-version download counts from the same Cloudflare
/// Worker `glacierclient.xyz` itself calls (`GET /counts`), keyed by the
/// manifest's own `tag` string (e.g. `"v1.2.0"`). Read-only — never
/// increments anything, so this can't inflate the public count just by
/// opening the panel. Best-effort: returns an empty map on any failure
/// (network down, worker unreachable) rather than erroring the whole panel
/// over what's a "nice to have" stat.
#[tauri::command]
pub async fn get_glacier_download_counts(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, u64>, String> {
    Ok(gl_clients::glacier_client::fetch_download_counts(&state.http)
        .await
        .unwrap_or_default())
}

/// Downloads and SHA-256-verifies one Glacier Client version's jar into
/// `~/.glacier/versions/<id>/`. Mirrors `GlacierClientService.InstallAsync`.
#[tauri::command]
pub async fn install_glacier_client(
    state: State<'_, AppState>,
    version: GlacierClientVersion,
) -> Result<String, String> {
    let profile = user_profile();
    let dest = version.jar_path(&profile);

    let mut req = gl_download::DownloadRequest::new(&version.url, &dest)
        .with_label(format!("Glacier {}", version.name));
    if !version.sha256.is_empty() {
        req = req.with_sha256(version.sha256.clone());
    }
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().into_owned())
}

/// Deletes an installed Glacier Client version's jar. Mirrors
/// `GlacierClientService.Uninstall`.
#[tauri::command]
pub async fn uninstall_glacier_client(version: GlacierClientVersion) -> Result<(), String> {
    // `id` is joined into the install path, so it must be one plain component.
    super::safe_file_name(&version.id)?;
    let path = version.jar_path(&user_profile());
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Total size on disk of every installed Glacier Client jar. Mirrors
/// `GlacierClientService.GetInstalledSizeBytes`.
#[tauri::command]
pub async fn glacier_installed_size_bytes() -> u64 {
    gl_clients::glacier_client::installed_size_bytes(&user_profile())
}

/// Launches the active Java instance with the Glacier jar attached as a
/// `-javaagent:`. Mirrors `GlacierClientService.LaunchAsync`, adapted to
/// this app's instance model: the original launches a bare version id
/// (`JavaGameLauncher.LaunchAsync(mcVersionId, ...)` — Java launches here
/// are instance-scoped, not version-scoped, matching `launch_java_instance`
/// everywhere else in this app; the caller is expected to have the desired
/// version active on the target instance already, same as every other
/// Java launch path).
#[tauri::command]
pub async fn launch_glacier_client(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    version: GlacierClientVersion,
    instance_id: Option<String>,
) -> Result<u32, String> {
    let profile = user_profile();
    gl_clients::glacier_client::require_installed(&version, &profile).map_err(|e| e.to_string())?;

    let jar_path = version.jar_path(&profile);
    let javaagent = format!("-javaagent:{}", jar_path.display());

    crate::commands::launch::launch_java_instance(app, state, instance_id, Some(vec![javaagent]))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn javaagent_arg_points_at_the_installed_jar() {
        let v = GlacierClientVersion {
            id: "1.2.0".into(),
            ..Default::default()
        };
        let profile = std::path::Path::new("C:/Users/Test");
        let jar_path = v.jar_path(profile);
        let arg = format!("-javaagent:{}", jar_path.display());
        assert!(arg.starts_with("-javaagent:"));
        assert!(arg.ends_with("Glacier-1.2.0.jar"));
    }
}
