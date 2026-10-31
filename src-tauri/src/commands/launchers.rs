//! Backs the "Launchers" panel. Despite the panel title, the driving
//! code-behind method is `RefreshJavaClients` (see `Pages/Home.Panels.cs`),
//! which detects locally-installed Lunar Client / Badlion Client
//! (`LunarBadlionService.Detect`) and lets the user pick an offline Lunar
//! multiversion install to direct-launch. This is a separate concept from
//! both the Java Fabric/Forge mod loaders (`java_mods.rs`) and the Bedrock
//! third-party clients list.
//!
//! `LunarBadlionService.ReadUninstallExe`'s registry-uninstall-key fallback
//! is now ported too (see `gl_clients::lunar_badlion::read_uninstall_exe`),
//! used transparently by `detect_lunar_badlion` below.
//!
//! Not ported: the "Glacier custom client" manifest
//! (`EnsureGlacierManifestAsync`) referenced alongside this panel in the
//! original, which is a distinct feature covered by the existing custom
//! client tooling (`custom_client_library.rs` / `glacier_client.rs`) and not
//! duplicated here.

use gl_clients::lunar_badlion::{self, LunarBadlionInfo};
#[cfg(windows)]
use std::os::windows::process::CommandExt as _;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LunarBadlionStatus {
    pub lunar_installed: bool,
    pub badlion_installed: bool,
    pub can_direct_launch_lunar: bool,
    pub lunar_installed_versions: Vec<String>,
    pub lunar_exe_path: Option<String>,
    pub badlion_exe_path: Option<String>,
}

impl From<LunarBadlionInfo> for LunarBadlionStatus {
    fn from(info: LunarBadlionInfo) -> Self {
        Self {
            lunar_installed: info.lunar_installed(),
            badlion_installed: info.badlion_installed(),
            can_direct_launch_lunar: info.can_direct_launch_lunar(),
            lunar_installed_versions: info.lunar_installed_versions.clone(),
            lunar_exe_path: info
                .lunar_exe_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            badlion_exe_path: info
                .badlion_exe_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
        }
    }
}

/// Mirrors `RefreshJavaClients` / `OpenJavaClientsAsync`'s `LunarBadlion.Detect()` call.
#[tauri::command(async)]
pub fn detect_lunar_badlion() -> Result<LunarBadlionStatus, String> {
    let local_app_data = dirs_local_app_data();
    let user_profile = dirs_user_profile();
    Ok(lunar_badlion::detect(&local_app_data, &user_profile).into())
}

/// Launches the standalone Lunar Client or Badlion Client executable.
/// Mirrors the exe-spawn half of `LaunchLunar`/`LaunchBadlion` (the original
/// just starts the installed launcher exe for the user to click Play in).
#[tauri::command(async)]
pub fn launch_client_exe(exe_path: String) -> Result<(), String> {
    // Only ever start the Lunar/Badlion exe we detected ourselves, never an
    // arbitrary path handed over from the webview.
    let info = lunar_badlion::detect(&dirs_local_app_data(), &dirs_user_profile());
    let requested = std::path::Path::new(&exe_path);
    let allowed = [
        info.lunar_exe_path.as_deref(),
        info.badlion_exe_path.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|p| p == requested);
    if !allowed {
        return Err("That launcher isn't a detected Lunar or Badlion install.".into());
    }
    std::process::Command::new(&exe_path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Resolves the jars/natives/JRE needed to direct-launch a specific Lunar
/// offline multiversion install, without spawning anything. Mirrors the
/// file-discovery half of `LunarBadlionService.TryDirectLaunch`; used by the
/// frontend to check availability/list versions before calling
/// [`launch_lunar_direct`], which does the actual spawn.
#[tauri::command(async)]
pub fn resolve_lunar_direct_launch(
    version_id: String,
) -> Result<Option<DirectLaunchFilesDto>, String> {
    let local_app_data = dirs_local_app_data();
    let user_profile = dirs_user_profile();
    let info = lunar_badlion::detect(&local_app_data, &user_profile);
    Ok(
        lunar_badlion::resolve_direct_launch(&info, &version_id).map(|f| DirectLaunchFilesDto {
            jre_path: f.jre_path.to_string_lossy().into_owned(),
            version_dir: f.version_dir.to_string_lossy().into_owned(),
            natives_dir: f.natives_dir.to_string_lossy().into_owned(),
            jars: f
                .jars
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        }),
    )
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectLaunchFilesDto {
    pub jre_path: String,
    pub version_dir: String,
    pub natives_dir: String,
    pub jars: Vec<String>,
}

/// Spawns the JVM directly against a resolved Lunar offline multiversion
/// install. Mirrors `LunarBadlionService.TryDirectLaunch`'s exact
/// classpath/args/main-class construction — this is Lunar's own
/// `com.moonsworth.lunar.genesis.Genesis` entry point, not the vanilla
/// launch pipeline, so it deliberately does not go through
/// `gl_launch::java`. Returns the spawned PID.
#[tauri::command]
pub fn launch_lunar_direct(version_id: String) -> Result<u32, String> {
    let local_app_data = dirs_local_app_data();
    let user_profile = dirs_user_profile();
    let info = lunar_badlion::detect(&local_app_data, &user_profile);
    let files = lunar_badlion::resolve_direct_launch(&info, &version_id)
        .ok_or_else(|| format!("Lunar version {version_id} isn't available for direct launch."))?;

    if files.jars.is_empty() {
        return Err(format!("No jars found for Lunar version {version_id}."));
    }

    let classpath = files
        .jars
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(";");

    let mut lunar_cmd = std::process::Command::new(&files.jre_path);
    lunar_cmd
        .arg("-Xms512m")
        .arg("-Xmx2048m")
        .arg(format!(
            "-Djava.library.path={}",
            files.natives_dir.display()
        ))
        .arg("-cp")
        .arg(&classpath)
        .arg("com.moonsworth.lunar.genesis.Genesis")
        .arg("--version")
        .arg(&version_id)
        .arg("--launcherVersion")
        .arg("glacier")
        .current_dir(&files.version_dir);
    #[cfg(windows)]
    lunar_cmd.creation_flags(CREATE_NO_WINDOW);
    let child = lunar_cmd
        .spawn()
        .map_err(|e| format!("Failed to start Lunar: {e}"))?;

    Ok(child.id())
}

fn dirs_local_app_data() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
}

fn dirs_user_profile() -> std::path::PathBuf {
    std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Horizon (horizon-rust/horizon): a native Rust rewrite of Minecraft Java
// Edition 26.3, shipped as the rolling `nightly` release. The pure logic
// (asset selection, SHA256SUMS, state, command lines) lives in
// `gl_clients::horizon`; this section does the network, extraction,
// provisioning (Minecraft 26.3, Java 25, the server jar), preparation and
// process handling.
// ---------------------------------------------------------------------------

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use gl_clients::hash::hashes_equal;
use gl_clients::horizon::{self as hz, BuildRecord, HorizonLayout, InstallState};
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

const HORIZON_PROGRESS_EVENT: &str = "horizon-progress";
const HORIZON_PROCESS_EVENT: &str = "java-process-changed";
/// Lines of prepare's stderr kept for the error message.
const PREPARE_STDERR_TAIL: usize = 12;

static HORIZON_BUSY: AtomicBool = AtomicBool::new(false);

/// Holds the single Horizon operation slot (install, update, rollback,
/// uninstall, or the short launch step). Released on drop.
struct HorizonBusy;

impl Drop for HorizonBusy {
    fn drop(&mut self) {
        HORIZON_BUSY.store(false, Ordering::SeqCst);
    }
}

fn begin_horizon_op() -> Result<HorizonBusy, String> {
    if HORIZON_BUSY.swap(true, Ordering::SeqCst) {
        return Err("Horizon is busy with another step. Wait for it to finish.".into());
    }
    Ok(HorizonBusy)
}

#[derive(Clone, serde::Serialize)]
struct HorizonProgressPayload {
    stage: String,
    percent: f64,
}

fn emit_horizon_progress(app: &AppHandle, stage: impl Into<String>, percent: f64) {
    let _ = app.emit(
        HORIZON_PROGRESS_EVENT,
        HorizonProgressPayload {
            stage: stage.into(),
            percent,
        },
    );
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HorizonBuildInfo {
    id: String,
    name: String,
    zip_sha256: String,
    installed_at: i64,
    published_at: String,
    notes: String,
    playable: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HorizonLatestInfo {
    name: String,
    published_at: String,
    notes: String,
    zip_sha256: String,
    update_available: bool,
}

/// The patch notes of the release an official-launcher build came from.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HorizonOfficialNotes {
    name: String,
    notes: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HorizonStatus {
    installed: bool,
    /// Build id when Horizon's own launcher already has a playable install,
    /// which Glacier launches directly instead of downloading its own.
    official_build: Option<String>,
    /// Notes for `official_build`, from the release it was built from.
    official_notes: Option<HorizonOfficialNotes>,
    official_notes_error: Option<String>,
    current: Option<HorizonBuildInfo>,
    previous: Option<HorizonBuildInfo>,
    can_rollback: bool,
    latest: Option<HorizonLatestInfo>,
    latest_error: Option<String>,
    state_error: Option<String>,
    busy: bool,
    running: bool,
    repo_url: &'static str,
    website_url: &'static str,
}

fn horizon_layout(state: &AppState) -> HorizonLayout {
    HorizonLayout::new(state.clients_dir.join("horizon"))
}

fn load_horizon_state(layout: &HorizonLayout) -> Result<InstallState, String> {
    match std::fs::read_to_string(layout.state_file()) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| {
            format!(
                "Horizon's state file is unreadable ({e}). Uninstall Horizon and install it again."
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(InstallState::default()),
        Err(e) => Err(format!("Could not read Horizon's state: {e}")),
    }
}

/// Writes `state.json` through a temporary file, so a crash mid-write can't
/// leave a truncated state behind.
fn save_horizon_state(layout: &HorizonLayout, install: &InstallState) -> Result<(), String> {
    std::fs::create_dir_all(&layout.root).map_err(|e| e.to_string())?;
    let path = layout.state_file();
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(install).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

fn build_info(layout: &HorizonLayout, record: &BuildRecord) -> HorizonBuildInfo {
    HorizonBuildInfo {
        id: record.id.clone(),
        name: record.name.clone(),
        zip_sha256: record.zip_sha256.clone(),
        installed_at: record.installed_at,
        published_at: record.published_at.clone(),
        notes: hz::release_notes_text(&record.notes),
        playable: hz::is_playable(&layout.build_dir(&record.id)),
    }
}

/// What the published nightly is, resolved from the release JSON and its
/// SHA256SUMS. Nothing is downloaded yet.
#[derive(Clone)]
struct LatestBuild {
    release: hz::HorizonRelease,
    zip_url: String,
    zip_sha256: String,
}

/// Last successful nightly lookup. `horizon_status` runs on every Horizon card
/// render, so it reuses this for `LATEST_BUILD_TTL` instead of hitting GitHub.
static LATEST_BUILD_CACHE: std::sync::Mutex<Option<(std::time::Instant, LatestBuild)>> =
    std::sync::Mutex::new(None);
const LATEST_BUILD_TTL: std::time::Duration = std::time::Duration::from_secs(600);

async fn fetch_latest_build_cached(http: &reqwest::Client) -> Result<LatestBuild, String> {
    if let Ok(guard) = LATEST_BUILD_CACHE.lock() {
        if let Some((at, build)) = guard.as_ref() {
            if at.elapsed() < LATEST_BUILD_TTL {
                return Ok(build.clone());
            }
        }
    }
    fetch_latest_build(http).await
}

/// Recent Horizon releases, cached like `LATEST_BUILD_CACHE`. Used to find the
/// patch notes of a build installed by Horizon's own launcher.
static RELEASES_CACHE: std::sync::Mutex<Option<(std::time::Instant, Vec<hz::HorizonRelease>)>> =
    std::sync::Mutex::new(None);

async fn fetch_releases_cached(http: &reqwest::Client) -> Result<Vec<hz::HorizonRelease>, String> {
    if let Ok(guard) = RELEASES_CACHE.lock() {
        if let Some((at, releases)) = guard.as_ref() {
            if at.elapsed() < LATEST_BUILD_TTL {
                return Ok(releases.clone());
            }
        }
    }
    let body = http
        .get(hz::RELEASES_LIST_API_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not reach Horizon's release list: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Could not read Horizon's release list: {e}"))?;
    let releases = hz::parse_releases(&body).map_err(|e| e.to_string())?;
    if let Ok(mut guard) = RELEASES_CACHE.lock() {
        *guard = Some((std::time::Instant::now(), releases.clone()));
    }
    Ok(releases)
}

async fn fetch_latest_build(http: &reqwest::Client) -> Result<LatestBuild, String> {
    let body = http
        .get(hz::RELEASE_API_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not reach Horizon's release page: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Could not read Horizon's release page: {e}"))?;
    let release = hz::parse_release(&body).map_err(|e| e.to_string())?;

    let sums_url = hz::pick_sums_asset(&release)
        .map_err(|e| e.to_string())?
        .browser_download_url
        .clone();
    let sums_text = http
        .get(&sums_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not download Horizon's checksums: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Could not read Horizon's checksums: {e}"))?;
    let sums = hz::parse_sha256sums(&sums_text);

    let zip = hz::pick_zip_asset(&release).map_err(|e| e.to_string())?;
    let zip_sha256 = hz::expected_sha256(&sums, &zip.name).map_err(|e| e.to_string())?;
    let build = LatestBuild {
        zip_url: zip.browser_download_url.clone(),
        zip_sha256,
        release,
    };
    // Every successful lookup (including the install path, which must stay
    // fresh) refreshes the cache that `horizon_status` reads.
    if let Ok(mut guard) = LATEST_BUILD_CACHE.lock() {
        *guard = Some((std::time::Instant::now(), build.clone()));
    }
    Ok(build)
}

async fn build_status(state: &AppState) -> HorizonStatus {
    let layout = horizon_layout(state);
    let (install, state_error) = match load_horizon_state(&layout) {
        Ok(install) => (install, None),
        Err(e) => (InstallState::default(), Some(e)),
    };
    let official = official_horizon_install();
    // With Horizon's launcher install, "update available" means the rolling
    // nightly is a different commit from the one installed there.
    let official_commit = official.as_ref().and_then(|o| hz::build_commit(&o.id));
    let (latest, latest_error) = match fetch_latest_build_cached(&state.http).await {
        Ok(found) => {
            let update_available = match (&official, &official_commit) {
                (Some(_), Some(commit)) => {
                    hz::commit_of_release(&found.release).is_some_and(|c| c != *commit)
                }
                (Some(_), None) => false,
                (None, _) => !install.is_current_digest(&found.zip_sha256),
            };
            (
                Some(HorizonLatestInfo {
                    name: found
                        .release
                        .name
                        .clone()
                        .unwrap_or_else(|| "Nightly".to_string()),
                    published_at: found.release.published_at.clone().unwrap_or_default(),
                    notes: hz::release_notes_text(found.release.body.as_deref().unwrap_or("")),
                    zip_sha256: found.zip_sha256,
                    update_available,
                }),
                None,
            )
        }
        Err(e) => (None, Some(e)),
    };
    let current = install.current.as_ref().map(|r| build_info(&layout, r));
    let (official_notes, official_notes_error) = match &official {
        None => (None, None),
        Some(o) => match fetch_releases_cached(&state.http).await {
            Ok(releases) => (
                hz::release_for_build(&releases, &o.id).map(|r| HorizonOfficialNotes {
                    name: r.name.clone().unwrap_or_else(|| o.id.clone()),
                    notes: hz::release_notes_text(r.body.as_deref().unwrap_or("")),
                }),
                None,
            ),
            Err(e) => (None, Some(e)),
        },
    };
    HorizonStatus {
        installed: official.is_some() || current.as_ref().is_some_and(|c| c.playable),
        official_build: official.as_ref().map(|o| o.id.clone()),
        official_notes,
        official_notes_error,
        current,
        previous: install.previous.as_ref().map(|r| build_info(&layout, r)),
        can_rollback: install.previous.is_some(),
        latest,
        latest_error,
        state_error,
        busy: HORIZON_BUSY.load(Ordering::SeqCst),
        running: state.java_processes.pid_of(hz::PROCESS_KEY).is_some(),
        repo_url: hz::REPO_URL,
        website_url: hz::WEBSITE_URL,
    }
}

fn official_horizon_install() -> Option<hz::OfficialInstall> {
    std::env::var_os("APPDATA").and_then(|a| hz::find_official_install(Path::new(&a)))
}

/// The official launcher's `.minecraft`, if the player has one.
fn official_minecraft_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(".minecraft"))
}

/// Finds a usable 26.3 client: the official launcher's install first (no
/// second download), then Glacier's own Java versions folder, downloading
/// 26.3 from Mojang into it when neither has it.
async fn ensure_minecraft_26_3(app: &AppHandle, state: &AppState) -> Result<PathBuf, String> {
    if let Some(official) = official_minecraft_dir() {
        if hz::has_vanilla_26_3(&official) {
            emit_horizon_progress(
                app,
                "Using your Minecraft 26.3 files from the official launcher",
                5.0,
            );
            return Ok(official);
        }
    }
    let root = {
        let instances = state
            .java_instances
            .lock()
            .map_err(|_| "java instance service lock poisoned".to_string())?;
        instances.root_dir().to_path_buf()
    };
    if hz::has_vanilla_26_3(&root) {
        return Ok(root);
    }

    emit_horizon_progress(
        app,
        "Downloading Minecraft 26.3 from Mojang for your account",
        3.0,
    );
    let versions_dir = root.join("versions");
    let versions = {
        let mut guard = state.java_versions.lock().await;
        guard
            .get_versions(&versions_dir, None)
            .await
            .map_err(|e| e.to_string())?
    };
    let version = versions
        .into_iter()
        .find(|v| v.id == hz::MINECRAFT_VERSION)
        .ok_or_else(|| {
            format!(
                "Minecraft {} is not in Mojang's version list",
                hz::MINECRAFT_VERSION
            )
        })?;
    gl_java::InstallService::new()
        .install(&version, &root, |p: &gl_java::InstallProgress| {
            emit_horizon_progress(app, p.stage.clone(), p.percent);
        })
        .await
        .map_err(|e| format!("Could not install Minecraft 26.3: {e}"))?;
    Ok(root)
}

/// Java 25, downloaded into Glacier's runtimes folder when not cached.
/// Returns the JAVA_HOME that `horizon-prepare` should use.
async fn ensure_java_25(app: &AppHandle) -> Result<PathBuf, String> {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or_else(|| "could not resolve home directory (USERPROFILE)".to_string())?;
    let service = gl_java::RuntimeDownloadService::new(
        gl_java::RuntimeDownloadService::default_runtimes_root(&home),
    );
    let javaw = service
        .download(hz::JAVA_MAJOR, |stage: &str, percent: f64| {
            emit_horizon_progress(app, stage.to_string(), percent);
        })
        .await
        .map_err(|e| format!("Could not get Java {}: {e}", hz::JAVA_MAJOR))?;
    hz::java_home_from_javaw(&javaw).ok_or_else(|| {
        format!(
            "Unexpected Java {} location: {}",
            hz::JAVA_MAJOR,
            javaw.display()
        )
    })
}

/// The 26.3 server jar that `horizon-prepare` needs (first preparation),
/// taken from the version JSON's `downloads.server` and checked by its SHA-1.
async fn ensure_server_jar(
    app: &AppHandle,
    state: &AppState,
    layout: &HorizonLayout,
    mc_dir: &Path,
) -> Result<PathBuf, String> {
    let json_path = mc_dir
        .join("versions")
        .join(hz::MINECRAFT_VERSION)
        .join(format!("{}.json", hz::MINECRAFT_VERSION));
    let text = std::fs::read_to_string(&json_path)
        .map_err(|e| format!("Could not read {}: {e}", json_path.display()))?;
    let (url, sha1) = hz::server_jar_from_version_json(&text).map_err(|e| e.to_string())?;

    let dest = layout.server_jar_path();
    if hz::file_has_sha1(&dest, &sha1) {
        return Ok(dest);
    }
    emit_horizon_progress(
        app,
        "Downloading the 26.3 server jar (used once, for preparing)",
        6.0,
    );
    let bytes = state
        .http
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not download the server jar: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Could not read the server jar: {e}"))?;
    if !hashes_equal(&hz::sha1_hex_bytes(&bytes), &sha1) {
        return Err("The server jar does not match Mojang's checksum; not using it.".into());
    }
    std::fs::create_dir_all(layout.downloads_dir()).map_err(|e| e.to_string())?;
    let tmp = dest.with_extension("jar.tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// Extracts the release zip into `staging`, moves the folder holding
/// `horizon-client.exe` to `dest`, and removes the staging copy. The zip is
/// one top-level folder in the release, so `dest` ends up being that folder.
fn extract_horizon_build(zip: &Path, staging: &Path, dest: &Path) -> Result<(), String> {
    if staging.exists() {
        std::fs::remove_dir_all(staging).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(staging).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("The Horizon download is not a zip archive: {e}"))?;
    archive
        .extract(staging)
        .map_err(|e| format!("Could not unpack Horizon: {e}"))?;

    let root = hz::find_client_root(staging)
        .ok_or_else(|| format!("The Horizon archive has no {}", hz::CLIENT_EXE))?;
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| e.to_string())?;
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if root == staging {
        return std::fs::rename(staging, dest).map_err(|e| e.to_string());
    }
    std::fs::rename(&root, dest).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(staging);
    Ok(())
}

/// Runs `horizon-prepare --prepare` on a build, streaming its output as
/// progress. Its output is `<build>/.local/horizon-client/<version>`.
async fn run_horizon_prepare(
    app: AppHandle,
    build_dir: PathBuf,
    mc_dir: PathBuf,
    server_jar: PathBuf,
    java_home: PathBuf,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let exe = build_dir.join(hz::PREPARE_EXE);
        if !exe.is_file() {
            return Err(format!(
                "{} is missing from the Horizon build",
                hz::PREPARE_EXE
            ));
        }
        let mut cmd = Command::new(&exe);
        cmd.arg("--prepare")
            .arg("--root")
            .arg(&build_dir)
            .arg("--minecraft-dir")
            .arg(&mc_dir)
            .arg("--server-jar")
            .arg(&server_jar)
            .current_dir(&build_dir)
            .env("JAVA_HOME", &java_home)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Could not start horizon-prepare: {e}"))?;

        let stderr_handle = child.stderr.take().map(|stderr| {
            std::thread::spawn(move || {
                let mut tail: VecDeque<String> = VecDeque::with_capacity(PREPARE_STDERR_TAIL);
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    if tail.len() == PREPARE_STDERR_TAIL {
                        tail.pop_front();
                    }
                    tail.push_back(line);
                }
                tail
            })
        });
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let line = line.trim().to_string();
                if !line.is_empty() {
                    emit_horizon_progress(&app, line, 50.0);
                }
            }
        }
        let tail = stderr_handle
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        let status = child
            .wait()
            .map_err(|e| format!("horizon-prepare did not finish: {e}"))?;
        if status.success() {
            return Ok(());
        }
        let reason = tail
            .back()
            .cloned()
            .unwrap_or_else(|| "no error output".to_string());
        Err(format!("Horizon could not prepare its resources: {reason}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Downloads, verifies, unpacks and prepares the latest nightly, then makes it
/// the current build and keeps the old current one for rollback. Used by both
/// install (`must_be_installed = false`) and update (`true`).
async fn install_latest_horizon(
    app: &AppHandle,
    state: &AppState,
    must_be_installed: bool,
) -> Result<(), String> {
    let _busy = begin_horizon_op()?;
    if state.java_processes.pid_of(hz::PROCESS_KEY).is_some() {
        return Err("Close Horizon before installing or updating it.".into());
    }
    let layout = horizon_layout(state);
    let mut install = load_horizon_state(&layout)?;
    if must_be_installed && install.current.is_none() {
        return Err("Horizon is not installed yet. Use Install instead.".into());
    }

    emit_horizon_progress(app, "Checking the latest Horizon nightly", 1.0);
    let latest = fetch_latest_build(&state.http).await?;
    let already_playable = install
        .current
        .as_ref()
        .is_some_and(|c| hz::is_playable(&layout.build_dir(&c.id)));
    if install.is_current_digest(&latest.zip_sha256) && already_playable {
        emit_horizon_progress(app, "Horizon is already up to date", 100.0);
        return Ok(());
    }

    let id = hz::build_id(latest.release.name.as_deref(), &latest.zip_sha256);
    let mc_dir = ensure_minecraft_26_3(app, state).await?;
    let java_home = ensure_java_25(app).await?;
    let server_jar = ensure_server_jar(app, state, &layout, &mc_dir).await?;

    emit_horizon_progress(app, "Downloading Horizon nightly", 10.0);
    let zip_path = layout.zip_path(&id);
    std::fs::create_dir_all(layout.downloads_dir()).map_err(|e| e.to_string())?;
    let request = gl_download::DownloadRequest::new(&latest.zip_url, &zip_path)
        .with_sha256(latest.zip_sha256.clone())
        .with_label("Horizon");
    state
        .downloads
        .submit(request)
        .join()
        .await
        .map_err(|e| format!("Horizon download failed: {e}"))?;

    let build_dir = layout.build_dir(&id);
    {
        let zip = zip_path.clone();
        let staging = layout.staging_dir(&id);
        let dest = build_dir.clone();
        tauri::async_runtime::spawn_blocking(move || extract_horizon_build(&zip, &staging, &dest))
            .await
            .map_err(|e| e.to_string())??;
    }
    let _ = std::fs::remove_file(&zip_path);

    emit_horizon_progress(
        app,
        "Preparing Horizon's resources (a few minutes the first time)",
        40.0,
    );
    if let Err(e) = run_horizon_prepare(
        app.clone(),
        build_dir.clone(),
        mc_dir,
        server_jar,
        java_home,
    )
    .await
    {
        let _ = std::fs::remove_dir_all(&build_dir);
        return Err(e);
    }
    if !hz::is_playable(&build_dir) {
        let _ = std::fs::remove_dir_all(&build_dir);
        return Err("Horizon's resources were not created".into());
    }

    let record = BuildRecord {
        id,
        name: latest
            .release
            .name
            .clone()
            .unwrap_or_else(|| "Nightly".to_string()),
        zip_sha256: latest.zip_sha256,
        installed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        published_at: latest.release.published_at.clone().unwrap_or_default(),
        notes: latest
            .release
            .body
            .clone()
            .unwrap_or_default()
            .trim()
            .to_string(),
    };
    let dropped = install.promote(record);
    save_horizon_state(&layout, &install)?;
    if let Some(old) = dropped {
        let _ = std::fs::remove_dir_all(layout.build_dir(&old.id));
    }
    emit_horizon_progress(app, "Horizon is ready", 100.0);
    Ok(())
}

/// The account JSON for `horizon-client --session-stdin`: the Microsoft
/// account the launcher is signed in with, its token refreshed first when it
/// has a refresh token. Online-mode servers refuse the player without it.
async fn horizon_session(http: &reqwest::Client) -> Result<String, String> {
    let mut settings = gl_settings::load().map_err(|e| e.to_string())?;
    if !settings.xbox_live_refresh_token.is_empty() {
        if let Ok(result) = gl_auth::refresh(http, &settings.xbox_live_refresh_token).await {
            settings.java_username = result.minecraft_profile.name.clone();
            settings.java_uuid = result.minecraft_profile.uuid.clone();
            settings.java_access_token = result.minecraft_access_token.clone();
            if let Some(refresh_token) = result.live_refresh_token.clone() {
                settings.xbox_live_refresh_token = refresh_token;
            }
            // A failed save shouldn't block this launch; the next one refreshes again.
            let _ = gl_settings::save(&settings);
        }
    }
    hz::session_json(
        &settings.java_username,
        &settings.java_uuid,
        &settings.java_access_token,
        Some(settings.xbox_xuid.as_str()),
    )
    .map_err(|_| {
        "Horizon needs a signed-in Microsoft account that owns Minecraft: Java Edition. Sign in under Accounts first."
            .to_string()
    })
}

#[derive(Clone, serde::Serialize)]
struct HorizonProcessPayload {
    instance_id: &'static str,
    status: &'static str,
    exit_code: Option<i32>,
}

/// The Horizon card's state: the installed builds, plus the published nightly
/// (best effort; `latest_error` says why it could not be read).
#[tauri::command]
pub async fn horizon_status(state: tauri::State<'_, AppState>) -> Result<HorizonStatus, String> {
    Ok(build_status(&state).await)
}

#[tauri::command]
pub async fn horizon_install(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<HorizonStatus, String> {
    if official_horizon_install().is_none() {
        install_latest_horizon(&app, &state, false).await?;
    }
    Ok(build_status(&state).await)
}

#[tauri::command]
pub async fn horizon_update(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<HorizonStatus, String> {
    install_latest_horizon(&app, &state, true).await?;
    Ok(build_status(&state).await)
}

/// Makes the previous build current again (the current one is kept as the
/// previous). Only the two most recent builds are kept on disk.
#[tauri::command]
pub async fn horizon_rollback(state: tauri::State<'_, AppState>) -> Result<HorizonStatus, String> {
    {
        let _busy = begin_horizon_op()?;
        if state.java_processes.pid_of(hz::PROCESS_KEY).is_some() {
            return Err("Close Horizon before switching builds.".into());
        }
        let layout = horizon_layout(&state);
        let mut install = load_horizon_state(&layout)?;
        let previous = install
            .previous
            .clone()
            .ok_or_else(|| "There is no previous build to roll back to.".to_string())?;
        if !hz::is_playable(&layout.build_dir(&previous.id)) {
            return Err(
                "The previous build is missing or incomplete. Update Horizon instead.".into(),
            );
        }
        install.rollback();
        save_horizon_state(&layout, &install)?;
    }
    Ok(build_status(&state).await)
}

/// Starts the current build, with the signed-in account on stdin. The process
/// is tracked in the Java process registry under `horizon`, and its output
/// goes to that registry's log buffer.
#[tauri::command]
pub async fn horizon_launch(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<u32, String> {
    let _busy = begin_horizon_op()?;
    if state.java_processes.pid_of(hz::PROCESS_KEY).is_some() {
        return Err("Horizon is already running.".into());
    }
    let layout = horizon_layout(&state);
    // Horizon's own launcher install wins: its build, worlds and settings are
    // used as-is. Glacier's downloaded build is the fallback.
    let (build, game_dir, home_dir) = match official_horizon_install() {
        Some(o) => (o.build_dir, o.data_dir.clone(), o.data_dir),
        None => {
            let install = load_horizon_state(&layout)?;
            let current = install
                .current
                .clone()
                .ok_or_else(|| "Horizon is not installed. Install it first.".to_string())?;
            let build = layout.build_dir(&current.id);
            if !hz::is_playable(&build) {
                return Err("Horizon's files are incomplete. Update Horizon.".into());
            }
            (build, layout.game_dir(), layout.home_dir())
        }
    };
    let resources = hz::resources_dir(&build)
        .ok_or_else(|| "Horizon's prepared resources are missing. Update Horizon.".to_string())?;

    let session = horizon_session(&state.http).await?;
    std::fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&home_dir).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(build.join(hz::CLIENT_EXE));
    cmd.args(hz::launch_args(&resources, &game_dir, &home_dir))
        .current_dir(&build)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Could not start Horizon: {e}"))?;
    let pid = child.id();

    // The session is written once and stdin closed, so it never touches disk.
    let wrote = match child.stdin.take() {
        Some(mut stdin) => stdin
            .write_all(session.as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|e| e.to_string()),
        None => Err("stdin unavailable".to_string()),
    };
    if let Err(e) = wrote {
        let _ = child.kill();
        return Err(format!("Could not hand the account to Horizon: {e}"));
    }

    let registry = state.java_processes.clone();
    let _ = registry.register(hz::PROCESS_KEY, pid);
    let _ = app.emit(
        HORIZON_PROCESS_EVENT,
        HorizonProcessPayload {
            instance_id: hz::PROCESS_KEY,
            status: "started",
            exit_code: None,
        },
    );

    if let Some(stdout) = child.stdout.take() {
        let registry = registry.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                registry.push_log_line(hz::PROCESS_KEY, &line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let registry = registry.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                registry.push_log_line(hz::PROCESS_KEY, &line);
            }
        });
    }

    // A client that dies right after starting (bad session, missing resources,
    // a crash on startup) must not be reported as launched. Give it a moment,
    // then surface its last output as the error the frontend toasts.
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
    if let Ok(Some(status)) = child.try_wait() {
        registry.mark_exited(hz::PROCESS_KEY);
        let _ = app.emit(
            HORIZON_PROCESS_EVENT,
            HorizonProcessPayload {
                instance_id: hz::PROCESS_KEY,
                status: "exited",
                exit_code: status.code(),
            },
        );
        let tail = registry.get_log_buffer(hz::PROCESS_KEY);
        let last = tail
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| "no output".to_string());
        return Err(format!(
            "Horizon closed right after starting (exit code {}): {last}",
            status
                .code()
                .map_or("unknown".to_string(), |c| c.to_string())
        ));
    }

    tauri::async_runtime::spawn_blocking(move || {
        let code = child.wait().ok().and_then(|s| s.code());
        registry.mark_exited(hz::PROCESS_KEY);
        let _ = app.emit(
            HORIZON_PROCESS_EVENT,
            HorizonProcessPayload {
                instance_id: hz::PROCESS_KEY,
                status: "exited",
                exit_code: code,
            },
        );
    });

    Ok(pid)
}

/// Removes the builds, the staging and download folders, and the state. The
/// game folder (saves, options, resource packs) is kept, so worlds survive.
#[tauri::command(async)]
pub fn horizon_uninstall(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let _busy = begin_horizon_op()?;
    if state.java_processes.pid_of(hz::PROCESS_KEY).is_some() {
        return Err("Close Horizon before uninstalling it.".into());
    }
    let layout = horizon_layout(&state);
    for dir in [
        layout.builds_dir(),
        layout.root.join(".staging"),
        layout.downloads_dir(),
    ] {
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        }
    }
    let state_file = layout.state_file();
    if state_file.exists() {
        std::fs::remove_file(&state_file).map_err(|e| e.to_string())?;
    }
    Ok(())
}
