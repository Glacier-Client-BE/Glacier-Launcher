//! Port of `Services/JavaModLoaderService.cs`: installs Fabric/Quilt version
//! profiles, downloads Forge/NeoForge installer jars, and runs them
//! (mirrors `ModpackInstallService.RunLoaderInstallerAsync`).

use crate::download::{download_to_file, get_string};
use crate::error::{io_err, JavaError, Result};
use crate::loader::install_profile::InstallProfile;
use crate::loader::maven::MavenCoordinate;
use crate::loader::patcher;
use crate::loader::processors::{run_processors, ProcessorContext};
use crate::runtime_download::RuntimeDownloadService;
use gl_download::{DownloadQueue, DownloadRequest};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Run a downloaded Forge/NeoForge installer jar against `instance_dir`,
/// mirroring `ModpackInstallService.RunLoaderInstallerAsync`: resolves (or
/// downloads) a Java 17 runtime, then runs
/// `java -jar <installer_jar> --installClient <instance_dir>` with
/// `instance_dir` as the working directory, and waits for it to exit.
pub async fn run_loader_installer(
    installer_jar: &Path,
    instance_dir: &Path,
    runtimes_root: &Path,
) -> Result<()> {
    let runtime = RuntimeDownloadService::new(runtimes_root.to_path_buf());
    let javaw = match runtime.get_cached_javaw(17) {
        Some(p) => p,
        None => runtime.download(17, |_, _| {}).await?,
    };
    let java = {
        let candidate = PathBuf::from(javaw.to_string_lossy().replace("javaw.exe", "java.exe"));
        if candidate.is_file() {
            candidate
        } else {
            javaw
        }
    };

    std::fs::create_dir_all(instance_dir).map_err(io_err(instance_dir))?;

    let mut cmd = tokio::process::Command::new(&java);
    cmd.arg("-jar")
        .arg(installer_jar)
        .arg("--installClient")
        .arg(instance_dir)
        .current_dir(instance_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| JavaError::Other(format!("could not start the loader installer: {e}")))?;
    let status = child
        .wait()
        .await
        .map_err(|e| JavaError::Other(format!("loader installer failed: {e}")))?;
    if !status.success() {
        return Err(JavaError::Other(format!(
            "loader installer exited with status {status}"
        )));
    }
    Ok(())
}

/// Install a Fabric loader profile for `minecraft_version` into `versions_dir`
/// (mirrors `InstallFabricAsync`). Returns the resolved profile id.
pub async fn install_fabric(
    client: &reqwest::Client,
    versions_dir: &Path,
    minecraft_version: &str,
    loader_version: Option<&str>,
) -> Result<String> {
    let loader = match loader_version {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => latest_fabric_loader(client).await?,
    };
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        urlencode(minecraft_version),
        urlencode(&loader)
    );
    let id = format!("fabric-loader-{loader}-{minecraft_version}");
    install_profile(client, versions_dir, &id, &url, "Fabric", minecraft_version).await
}

/// Install a Quilt loader profile (mirrors `InstallQuiltAsync`).
pub async fn install_quilt(
    client: &reqwest::Client,
    versions_dir: &Path,
    minecraft_version: &str,
    loader_version: Option<&str>,
) -> Result<String> {
    let loader = match loader_version {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => latest_quilt_loader(client).await?,
    };
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
        urlencode(minecraft_version),
        urlencode(&loader)
    );
    let id = format!("quilt-loader-{loader}-{minecraft_version}");
    install_profile(client, versions_dir, &id, &url, "Quilt", minecraft_version).await
}

/// Download the Forge installer jar for `minecraft_version` into
/// `<root_dir>/loaders/forge/`, returning the local path (mirrors
/// `DownloadForgeInstallerAsync`).
pub async fn download_forge_installer(
    client: &reqwest::Client,
    root_dir: &Path,
    minecraft_version: &str,
    forge_version: Option<&str>,
) -> Result<PathBuf> {
    let build = match forge_version {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => latest_forge_version(client, minecraft_version).await?,
    };
    let artifact = resolve_forge_artifact(client, minecraft_version, &build).await;
    let file = format!("forge-{artifact}-installer.jar");
    let url =
        format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{artifact}/{file}");
    let dest = root_dir.join("loaders").join("forge").join(&file);
    download_to_file(client, &url, &dest).await?;
    Ok(dest)
}

/// Download the NeoForge installer jar (mirrors `DownloadNeoForgeInstallerAsync`).
pub async fn download_neoforge_installer(
    client: &reqwest::Client,
    root_dir: &Path,
    minecraft_version: &str,
) -> Result<PathBuf> {
    let ver = latest_neoforge_version(client, minecraft_version).await?;
    let file = format!("neoforge-{ver}-installer.jar");
    let url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{ver}/{file}");
    let dest = root_dir.join("loaders").join("neoforge").join(&file);
    download_to_file(client, &url, &dest).await?;
    Ok(dest)
}

async fn install_profile(
    client: &reqwest::Client,
    versions_dir: &Path,
    id: &str,
    url: &str,
    loader_name: &str,
    minecraft_version: &str,
) -> Result<String> {
    let resp = client.get(url).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND
        || resp.status() == reqwest::StatusCode::BAD_REQUEST
    {
        return Err(JavaError::Other(format!(
            "{loader_name} does not support Minecraft {minecraft_version}."
        )));
    }
    let resp = resp.error_for_status().map_err(JavaError::Http)?;
    let json = resp.text().await?;
    let parsed: Value = serde_json::from_str(&json)?;
    let profile_id = parsed
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(id)
        .to_string();

    let dir = versions_dir.join(&profile_id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(io_err(&dir))?;
    let dest = dir.join(format!("{profile_id}.json"));
    crate::util::write_atomic(&dest, &json)?;
    Ok(profile_id)
}

async fn latest_fabric_loader(client: &reqwest::Client) -> Result<String> {
    let json = get_string(client, "https://meta.fabricmc.net/v2/versions/loader").await?;
    let parsed: Value = serde_json::from_str(&json)?;
    let arr = parsed.as_array().cloned().unwrap_or_default();
    for item in &arr {
        if item.get("stable").and_then(Value::as_bool) == Some(true) {
            if let Some(v) = item.get("version").and_then(Value::as_str) {
                return Ok(v.to_string());
            }
        }
    }
    arr.first()
        .and_then(|v| v.get("version"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| JavaError::Other("no Fabric loader versions found".to_string()))
}

async fn latest_quilt_loader(client: &reqwest::Client) -> Result<String> {
    let json = get_string(client, "https://meta.quiltmc.org/v3/versions/loader").await?;
    let parsed: Value = serde_json::from_str(&json)?;
    let arr = parsed.as_array().cloned().unwrap_or_default();
    let mut newest: Option<String> = None;
    for item in &arr {
        let version = item
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if newest.is_none() {
            newest = Some(version.clone());
        }
        if !is_pre_release(&version) {
            return Ok(version);
        }
    }
    newest.ok_or_else(|| JavaError::Other("no Quilt loader versions found".to_string()))
}

fn is_pre_release(version: &str) -> bool {
    let lower = version.to_lowercase();
    lower.contains("-beta") || lower.contains("-pre") || lower.contains("-rc")
}

async fn resolve_forge_artifact(
    client: &reqwest::Client,
    minecraft_version: &str,
    build: &str,
) -> String {
    let coordinate = format!("{minecraft_version}-{build}");
    let Ok(xml) = get_string(
        client,
        "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml",
    )
    .await
    else {
        return coordinate;
    };
    let matches: Vec<&str> = xml
        .split("<version>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</version>").next())
        .filter(|v| *v == coordinate || v.starts_with(&format!("{coordinate}-")))
        .collect();
    matches
        .iter()
        .find(|v| **v == coordinate)
        .or_else(|| matches.first())
        .map(|v| v.to_string())
        .unwrap_or(coordinate)
}

async fn latest_forge_version(client: &reqwest::Client, minecraft_version: &str) -> Result<String> {
    let json = get_string(
        client,
        "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
    )
    .await?;
    let parsed: Value = serde_json::from_str(&json)?;
    if let Some(promos) = parsed.get("promos") {
        if let Some(rec) = promos
            .get(format!("{minecraft_version}-recommended"))
            .and_then(Value::as_str)
        {
            return Ok(rec.to_string());
        }
        if let Some(lat) = promos
            .get(format!("{minecraft_version}-latest"))
            .and_then(Value::as_str)
        {
            return Ok(lat.to_string());
        }
    }
    Err(JavaError::Other(format!(
        "No Forge version found for Minecraft {minecraft_version}. You can enter a version manually from files.minecraftforge.net."
    )))
}

async fn latest_neoforge_version(
    client: &reqwest::Client,
    minecraft_version: &str,
) -> Result<String> {
    let json = get_string(
        client,
        "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge",
    )
    .await?;
    let parsed: Value = serde_json::from_str(&json)?;
    let versions = parsed
        .get("versions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let parts: Vec<&str> = minecraft_version.split('.').collect();
    let prefix = if parts.len() >= 3 {
        format!("{}.{}.", parts[1], parts[2])
    } else if parts.len() >= 2 {
        format!("{}.0.", parts[1])
    } else {
        return Err(JavaError::Other(format!(
            "No NeoForge version found for Minecraft {minecraft_version}. NeoForge supports 1.20.2+."
        )));
    };

    let mut best: Option<String> = None;
    for v in &versions {
        if let Some(ver) = v.as_str() {
            if ver.starts_with(&prefix) {
                best = Some(ver.to_string());
            }
        }
    }
    best.ok_or_else(|| {
        JavaError::Other(format!(
            "No NeoForge version found for Minecraft {minecraft_version}. NeoForge supports 1.20.2+."
        ))
    })
}

fn urlencode(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for b in v.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// ── Forge / NeoForge: real install-profile execution (Phase 2) ────────────
//
// Unlike Fabric/Quilt (a static version-profile JSON fetched and written
// straight into `versions/`), Forge/NeoForge ship a self-extracting
// installer jar containing an `install_profile.json` that names a
// `processors` pipeline which must actually be *run* (each a maven-coordinate
// jar invoked via its own manifest `Main-Class`) to produce the loader's
// launchable artifacts. See `crate::loader` for the DTOs/executor.

/// Installs Forge for `minecraft_version` into `mc_dir` (mirrors
/// `install_fabric`/`install_quilt`'s role, but for the processors-based
/// installer format). `mc_dir` is the Java root containing `versions/`,
/// `libraries/`, etc. Requires the vanilla client jar for
/// `minecraft_version` to already be installed under
/// `versions/<minecraft_version>/<minecraft_version>.jar`. Returns the
/// resolved loader version id to persist via
/// `InstanceService::set_instance_loader`.
pub async fn install_forge(
    client: &reqwest::Client,
    downloads: &DownloadQueue,
    mc_dir: &Path,
    runtimes_root: &Path,
    minecraft_version: &str,
    loader_version: Option<&str>,
    mut on_progress: impl FnMut(&str),
) -> Result<String> {
    let build = match loader_version {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => latest_forge_version(client, minecraft_version).await?,
    };
    let artifact = resolve_forge_artifact(client, minecraft_version, &build).await;
    let file = format!("forge-{artifact}-installer.jar");
    let url =
        format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{artifact}/{file}");
    let installer_jar = mc_dir.join("loaders").join("forge").join(&file);

    install_from_installer_jar(
        downloads,
        mc_dir,
        runtimes_root,
        minecraft_version,
        &url,
        &installer_jar,
        "Forge",
        &mut on_progress,
    )
    .await
}

/// Installs NeoForge (mirrors [`install_forge`]).
pub async fn install_neoforge(
    client: &reqwest::Client,
    downloads: &DownloadQueue,
    mc_dir: &Path,
    runtimes_root: &Path,
    minecraft_version: &str,
    loader_version: Option<&str>,
    mut on_progress: impl FnMut(&str),
) -> Result<String> {
    let ver = match loader_version {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => latest_neoforge_version(client, minecraft_version).await?,
    };
    let file = format!("neoforge-{ver}-installer.jar");
    let url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{ver}/{file}");
    let installer_jar = mc_dir.join("loaders").join("neoforge").join(&file);

    install_from_installer_jar(
        downloads,
        mc_dir,
        runtimes_root,
        minecraft_version,
        &url,
        &installer_jar,
        "NeoForge",
        &mut on_progress,
    )
    .await
}

async fn install_from_installer_jar(
    downloads: &DownloadQueue,
    mc_dir: &Path,
    runtimes_root: &Path,
    minecraft_version: &str,
    installer_url: &str,
    installer_jar: &Path,
    loader_name: &str,
    on_progress: &mut impl FnMut(&str),
) -> Result<String> {
    on_progress(&format!("Downloading {loader_name} installer..."));
    download_via_queue(downloads, installer_url, installer_jar, "installer").await?;

    on_progress("Reading install profile...");
    let raw_profile = read_jar_entry_string(installer_jar, "install_profile.json")?;
    let profile_json: Value = serde_json::from_str(&raw_profile)?;
    if patcher::requires_legacy_patching(&profile_json) {
        return Err(JavaError::Other(format!(
            "{loader_name} {minecraft_version} uses the legacy (pre-1.13) installer format, \
             which this launcher does not support. Only the modern processors-based installer \
             (Minecraft 1.13+) is implemented."
        )));
    }
    let profile: InstallProfile = InstallProfile::parse(&raw_profile).map_err(JavaError::Json)?;

    let libraries_dir = mc_dir.join("libraries");
    let vanilla_jar = mc_dir
        .join("versions")
        .join(minecraft_version)
        .join(format!("{minecraft_version}.jar"));
    if !vanilla_jar.is_file() {
        return Err(JavaError::Other(format!(
            "Minecraft {minecraft_version} must be installed before installing {loader_name} \
             (missing {})",
            vanilla_jar.display()
        )));
    }

    on_progress("Downloading loader libraries...");
    download_install_libraries(downloads, &profile, &libraries_dir).await?;

    on_progress("Resolving a Java runtime for the installer processors...");
    let java_major = required_java_major(minecraft_version);
    let runtime = RuntimeDownloadService::new(runtimes_root.to_path_buf());
    let javaw = match runtime.get_cached_javaw(java_major) {
        Some(p) => p,
        None => runtime.download(java_major, |_, _| {}).await?,
    };
    let java_exe = {
        let candidate = PathBuf::from(javaw.to_string_lossy().replace("javaw.exe", "java.exe"));
        if candidate.is_file() {
            candidate
        } else {
            javaw
        }
    };

    on_progress("Running installer processors...");
    let ctx = ProcessorContext {
        root: mc_dir.to_path_buf(),
        libraries_dir,
        installer_jar: installer_jar.to_path_buf(),
        minecraft_jar: vanilla_jar,
        side: "client".to_string(),
        java_exe,
    };
    run_processors(&ctx, &profile).await.map_err(|e| {
        JavaError::Other(format!(
            "{loader_name} {minecraft_version} install failed while running installer processors: {e}"
        ))
    })?;

    on_progress("Writing version profile...");
    let version_json_entry = profile
        .json
        .clone()
        .unwrap_or_else(|| "/version.json".to_string());
    let version_json = read_jar_entry_string(installer_jar, version_json_entry.trim_start_matches('/'))?;
    let versions_dir = mc_dir.join("versions").join(&profile.version);
    tokio::fs::create_dir_all(&versions_dir)
        .await
        .map_err(io_err(&versions_dir))?;
    let dest = versions_dir.join(format!("{}.json", profile.version));
    crate::util::write_atomic(&dest, &version_json)?;

    on_progress(&format!("{loader_name} {} installed.", profile.version));
    Ok(profile.version)
}

/// Downloads every `install_profile.json` library entry that declares a
/// direct download URL, via `gl-download`'s queue so it shows in the
/// Downloads panel; entries with no `downloads.artifact` (generated/output
/// artifacts a processor will produce) just get their parent directory
/// created so the processor can write into it.
async fn download_install_libraries(
    downloads: &DownloadQueue,
    profile: &InstallProfile,
    libraries_dir: &Path,
) -> Result<()> {
    let mut handles = Vec::new();
    for lib in &profile.libraries {
        let coord = MavenCoordinate::parse(&lib.name)?;
        let dest = libraries_dir.join(coord.relative_path());
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(io_err(parent))?;
        }
        let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) else {
            continue;
        };
        if artifact.url.trim().is_empty() || dest.is_file() {
            continue;
        }
        let mut req = DownloadRequest::new(artifact.url.clone(), dest).with_label(lib.name.clone());
        if let Some(size) = artifact.size {
            req = req.with_known_total_bytes(size);
        }
        handles.push(downloads.submit(req));
    }
    for handle in handles {
        handle
            .join()
            .await
            .map_err(|e| JavaError::Other(format!("failed to download a loader library: {e}")))?;
    }
    Ok(())
}

async fn download_via_queue(
    downloads: &DownloadQueue,
    url: &str,
    dest: &Path,
    label: &str,
) -> Result<()> {
    if dest.is_file() {
        return Ok(());
    }
    let req = DownloadRequest::new(url.to_string(), dest.to_path_buf()).with_label(label.to_string());
    downloads
        .submit(req)
        .join()
        .await
        .map_err(|e| JavaError::Other(format!("failed to download {label}: {e}")))?;
    Ok(())
}

fn read_jar_entry_string(jar_path: &Path, entry: &str) -> Result<String> {
    let file = std::fs::File::open(jar_path).map_err(io_err(jar_path))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut zf = archive.by_name(entry).map_err(|_| {
        JavaError::Other(format!(
            "{} has no entry '{entry}'",
            jar_path.display()
        ))
    })?;
    let mut out = String::new();
    zf.read_to_string(&mut out).map_err(io_err(jar_path))?;
    Ok(out)
}

/// The Java major version Forge/NeoForge require to run their installer
/// processors, by Minecraft version bracket (matches Mojang's own bundled-
/// runtime brackets, which Forge/NeoForge track).
fn required_java_major(minecraft_version: &str) -> u32 {
    let parts: Vec<u32> = minecraft_version
        .split(['.', '-'])
        .map(|p| p.parse::<u32>().unwrap_or(0))
        .collect();
    let major = parts.first().copied().unwrap_or(1);
    let minor = parts.get(1).copied().unwrap_or(0);
    let patch = parts.get(2).copied().unwrap_or(0);
    if major == 1 && (minor > 20 || (minor == 20 && patch >= 5)) {
        21
    } else if major == 1 && minor >= 18 {
        17
    } else if major == 1 && minor == 17 {
        16
    } else {
        8
    }
}
