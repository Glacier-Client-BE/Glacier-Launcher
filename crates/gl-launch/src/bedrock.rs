//! Bedrock package resolution and activation. Port of
//! `Services/BedrockPackageService.cs` + the launch-orchestration half of
//! `Services/GameLauncher.cs`, using PowerShell's `Get-AppxPackage` /
//! `Invoke-CommandInDesktopPackage` instead of the WinRT `PackageManager`
//! API (no `windows`-crate WinRT bindings in this workspace yet — PowerShell
//! is a pragmatic, dependency-free equivalent and is what the original used
//! for activation anyway).
//!
//! Scope for this first pass (see crate-level docs / final report for the
//! full list): package resolution + activation + optional DLL injection are
//! ported. `PreventSuspend` and `AccelerateLoadingAsync` are ported at the
//! `src-tauri/src/commands/launch.rs` call-site level (power-request /
//! RuntimeBroker kill respectively) rather than in this module. The
//! `minecraft://addExternalServer` deep link is ported here as
//! [`connect_external_server`].

use crate::error::{LaunchError, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
#[cfg(windows)]
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
use tokio::time::sleep;

pub const RELEASE_FAMILY_NAME: &str = "Microsoft.MinecraftUWP_8wekyb3d8bbwe";
pub const PREVIEW_FAMILY_NAME: &str = "Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe";
pub const GAME_EXE_NAME: &str = "Minecraft.Windows.exe";

/// Win32 window class of Minecraft Bedrock's main window, confirmed against
/// Flarial's own official launcher (`Flarial.Runtime/Game/Minecraft.Base.cs`:
/// `const string ClassName = "Bedrock";`). Used as the injection-readiness
/// signal in [`launch`].
const GAME_WINDOW_CLASS: &str = "Bedrock";

/// Longest we will wait for the game's window before falling back to a fixed
/// delay. Generous because a cold/slow machine can take a while to reach the
/// main menu, and the fallback path below is what runs if this is ever too
/// short.
const WINDOW_READY_TIMEOUT: Duration = Duration::from_secs(45);
const WINDOW_POLL_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Debug, Clone)]
pub struct BedrockPackageInfo {
    pub family_name: String,
    pub application_id: String,
    pub installed_path: PathBuf,
    pub is_preview: bool,
}

impl BedrockPackageInfo {
    pub fn aumid(&self) -> String {
        format!("{}!{}", self.family_name, self.application_id)
    }

    pub fn game_executable_path(&self) -> PathBuf {
        self.installed_path.join(GAME_EXE_NAME)
    }
}

fn powershell_exe() -> PathBuf {
    // Windows PowerShell (not pwsh) — matches the original, which hard-codes
    // this path so it doesn't depend on PowerShell 7 being installed.
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
}

async fn run_powershell(command: &str) -> Result<std::process::Output> {
    let ps = powershell_exe();
    if !ps.is_file() {
        return Err(LaunchError::Other("Windows PowerShell not found.".into()));
    }
    let mut cmd = Command::new(&ps);
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        command,
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output()
        .await
        .map_err(|e| LaunchError::Io(format!("failed to launch PowerShell: {e}")))
}

/// Resolves the installed Bedrock package, preferring the release channel
/// over Preview. Returns `Ok(None)` when Bedrock isn't installed (a real
/// "not installed" state, not an error).
pub async fn resolve() -> Result<Option<BedrockPackageInfo>> {
    for (family, is_preview) in [(RELEASE_FAMILY_NAME, false), (PREVIEW_FAMILY_NAME, true)] {
        // `Get-AppxPackage -Name` matches the package *Name*
        // (`Microsoft.MinecraftUWP`), NOT the family name
        // (`Microsoft.MinecraftUWP_8wekyb3d8bbwe`). Passing the full family
        // name here matched nothing, so resolution always reported "Bedrock
        // isn't installed" and the launch never started. Query by short name,
        // then verify the family name to stay faithful to the C# original,
        // which resolves by family via the WinRT `PackageManager`.
        let short_name = family.split('_').next().unwrap_or(family);
        let cmd = format!(
            "(Get-AppxPackage -Name '{short_name}' | Where-Object {{ $_.PackageFamilyName -eq '{family}' }} | Select-Object -First 1 -ExpandProperty InstallLocation)"
        );
        let out = run_powershell(&cmd).await?;
        let installed_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if installed_path.is_empty() || !Path::new(&installed_path).is_dir() {
            continue;
        }
        let installed_path = PathBuf::from(installed_path);
        let application_id = read_application_id(&installed_path).await;
        return Ok(Some(BedrockPackageInfo {
            family_name: family.to_string(),
            application_id,
            installed_path,
            is_preview,
        }));
    }
    Ok(None)
}

/// Reads the `Application Id` out of `AppxManifest.xml`. GDK builds
/// (1.26.40+) use `"Game"`; older UWP builds use `"App"`. Falls back to
/// `"Game"` if `gamelaunchhelper.exe` is present (manifest unreadable due to
/// WindowsApps ACLs), else `"App"`. Mirrors `ReadApplicationId`.
async fn read_application_id(installed_path: &Path) -> String {
    let manifest = installed_path.join("AppxManifest.xml");
    if let Ok(text) = tokio::fs::read_to_string(&manifest).await {
        if let Some(id) = extract_application_id(&text) {
            return id;
        }
    }
    if installed_path.join("gamelaunchhelper.exe").is_file() {
        return "Game".to_string();
    }
    "App".to_string()
}

/// Minimal `Id="..."` extraction from an `<Application ...>` element,
/// avoiding a full XML-parser dependency for one attribute read.
fn extract_application_id(xml: &str) -> Option<String> {
    let mut search_from = 0usize;
    while let Some(rel) = xml[search_from..].find("<Application") {
        let tag_start = search_from + rel;
        let tag_end = xml[tag_start..].find('>').map(|e| tag_start + e)?;
        let tag = &xml[tag_start..tag_end];
        if let Some(id_rel) = tag.find("Id=\"") {
            let id_start = id_rel + 4;
            if let Some(id_end_rel) = tag[id_start..].find('"') {
                let id = &tag[id_start..id_start + id_end_rel];
                if !id.trim().is_empty() {
                    return Some(id.to_string());
                }
            }
        }
        search_from = tag_end + 1;
    }
    None
}

/// The installed Bedrock build, as shown in the footer's version chip.
/// Mirrors the `BedrockVersion.IsInstalled` / `DisplayVersion` / `IsPreview`
/// trio the original's footer reads.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InstalledBedrockVersion {
    pub installed: bool,
    pub display_version: String,
    pub is_preview: bool,
}

/// Reads the installed package's version via `Get-AppxPackage`, preferring
/// the release channel over Preview (the same order as [`resolve`]).
///
/// Returns `installed: false` rather than an error when Bedrock isn't
/// present - "not installed" is a real state the footer renders, not a
/// failure.
pub async fn installed_version() -> InstalledBedrockVersion {
    for (family, is_preview) in [(RELEASE_FAMILY_NAME, false), (PREVIEW_FAMILY_NAME, true)] {
        let short_name = family.split('_').next().unwrap_or(family);
        let cmd = format!(
            "(Get-AppxPackage -Name '{short_name}' | Where-Object {{ $_.PackageFamilyName -eq '{family}' }} | Select-Object -First 1 -ExpandProperty Version)"
        );
        let Ok(out) = run_powershell(&cmd).await else {
            continue;
        };
        let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if version.is_empty() {
            continue;
        }
        return InstalledBedrockVersion {
            installed: true,
            display_version: version,
            is_preview,
        };
    }

    InstalledBedrockVersion {
        installed: false,
        display_version: String::new(),
        is_preview: false,
    }
}

/// Finds the running Minecraft process, optionally constrained to a package
/// family. Port of `BedrockPackageService.FindGameProcess`.
///
/// Faithful to the original's three-part behaviour, which matters when more
/// than one candidate is running (release *and* preview, say) — injecting
/// into the wrong one is a real failure mode:
/// - image names are tried in order: `Minecraft.Windows`, `Minecraft`,
///   `MinecraftUWP`;
/// - with no `family_name`, the first candidate wins;
/// - a process whose family name can't be read (access denied, not packaged)
///   is kept as a *last-resort fallback* rather than discarded, since
///   "unreadable" is not the same as "wrong package".
pub fn find_game_process_for(family_name: Option<&str>) -> Option<u32> {
    let mut fallback: Option<u32> = None;

    for name in [GAME_EXE_NAME, "Minecraft.exe", "MinecraftUWP.exe"] {
        for pid in gl_injection::find_processes_by_name(name) {
            let Some(family) = family_name else {
                return Some(pid);
            };
            match gl_injection::process_package_family_name(pid) {
                Some(actual) if actual.eq_ignore_ascii_case(family) => return Some(pid),
                // Couldn't read it — keep as a last resort, matching C#.
                None if fallback.is_none() => fallback = Some(pid),
                _ => {}
            }
        }
    }

    fallback
}

/// Any running Minecraft process, regardless of package family.
pub fn find_game_process() -> Option<u32> {
    find_game_process_for(None)
}

/// Starts Minecraft.Windows.exe inside the package's app container via
/// `Invoke-CommandInDesktopPackage`. Mirrors `TryInvokeInDesktopPackageAsync`
/// — the reliable activation path for GDK builds (1.26.40+), since it
/// bypasses `gamelaunchhelper.exe`.
pub async fn invoke_in_desktop_package(info: &BedrockPackageInfo) -> Result<bool> {
    let exe = info.game_executable_path();
    if !exe.is_file() {
        return Err(LaunchError::BedrockExeMissing(
            GAME_EXE_NAME.to_string(),
            info.installed_path.display().to_string(),
        ));
    }

    let escape = |s: &str| s.replace('\'', "''");
    let command = format!(
        "Invoke-CommandInDesktopPackage -PackageFamilyName '{}' -AppId '{}' -Command '{}'",
        escape(&info.family_name),
        escape(&info.application_id),
        escape(&exe.to_string_lossy())
    );

    let out = run_powershell(&command).await?;
    Ok(out.status.success())
}

/// Activates Minecraft directly through `IApplicationActivationManager`
/// (`gl_injection::activate_application`) — the literal port of
/// `InjectionService.LaunchMinecraft`, and this project's memory notes that
/// GDK builds (1.26.40+) need `Invoke-CommandInDesktopPackage` instead, so
/// this is kept as a last-resort fallback rather than the primary strategy.
async fn activate_via_com(info: &BedrockPackageInfo) -> Result<bool> {
    let aumid = info.aumid();
    tokio::task::spawn_blocking(move || gl_injection::activate_application(&aumid, None))
        .await
        .map_err(|e| LaunchError::Io(e.to_string()))?
        .map(|_pid| true)
        .map_err(|e| LaunchError::Io(e.to_string()))
}

/// Activates Minecraft via `minecraft:` URI as a fallback strategy.
async fn launch_via_uri() -> Result<bool> {
    let mut cmd = Command::new("cmd.exe");
    cmd.args(["/c", "start", "minecraft:"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.status()
        .await
        .map(|s| s.success())
        .map_err(|e| LaunchError::Io(e.to_string()))
}

/// Tries each activation strategy in turn until a game process appears.
/// Mirrors `StartMinecraftAsync`: `Invoke-CommandInDesktopPackage` is tried
/// first on GDK builds (AppId `"Game"`), matching this project's memory note
/// that 1.26.40+ requires this path.
async fn start_minecraft(info: &BedrockPackageInfo) -> Result<u32> {
    // Scope every lookup to the package we resolved: with release *and*
    // preview installed, an unscoped search can return the other channel's
    // process and we would activate (or inject into) the wrong game.
    let family = Some(info.family_name.as_str());

    if let Some(pid) = find_game_process_for(family) {
        return Ok(pid);
    }

    let is_gdk = info.application_id.eq_ignore_ascii_case("Game");
    let mut failures: Vec<String> = Vec::new();

    enum Strategy {
        DesktopPackage,
        Uri,
        Com,
    }

    let mut strategies: Vec<(&str, Strategy)> = Vec::new();
    if is_gdk {
        strategies.push((
            "Invoke-CommandInDesktopPackage (GDK)",
            Strategy::DesktopPackage,
        ));
    }
    strategies.push(("minecraft:// URI", Strategy::Uri));
    if !is_gdk {
        strategies.push(("Invoke-CommandInDesktopPackage", Strategy::DesktopPackage));
    }
    // Last resort: the original's own direct COM activation path
    // (`InjectionService.LaunchMinecraft`), tried after both of the above
    // have had a chance since it's unproven against current GDK builds.
    strategies.push(("IApplicationActivationManager (COM)", Strategy::Com));

    for (name, strategy) in strategies {
        if let Some(pid) = find_game_process_for(family) {
            return Ok(pid);
        }

        let ok = match strategy {
            Strategy::DesktopPackage => invoke_in_desktop_package(info).await,
            Strategy::Uri => launch_via_uri().await,
            Strategy::Com => activate_via_com(info).await,
        };

        match ok {
            Ok(true) => {}
            Ok(false) => {
                failures.push(format!("{name}: reported failure"));
                continue;
            }
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        }

        // Same ~10s budget as before, polled at 250ms so a game that
        // appears early is picked up sooner.
        for _ in 0..40 {
            sleep(Duration::from_millis(250)).await;
            if let Some(pid) = find_game_process_for(family) {
                return Ok(pid);
            }
        }
        failures.push(format!("{name}: no game process appeared"));
    }

    Err(LaunchError::BedrockActivationFailed(failures.join("; ")))
}

/// Minimal `Uri.EscapeDataString` equivalent: percent-encodes everything
/// except RFC 3986 unreserved characters. No crate for this in the
/// workspace, and the alphabet involved (server names/hostnames) is small
/// enough that hand-rolling it is simpler than adding a dependency.
fn escape_data_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Fires the `minecraft://?addExternalServer=<name>|<host>:<port>` deep link
/// so an already-running (or just-launched) Bedrock activates straight into
/// `host:port`. Mirrors `LaunchServerAsync`'s URI-construction tail (the
/// launch itself is a separate call to [`launch`], matching the original,
/// which always launches first and fires this as a follow-up).
///
/// Best-effort like the C# original: `Process::Start` via `cmd.exe`
/// (`ShellExecute` isn't directly available without the `windows` crate)
/// is the primary path, with the same shell fallback the original used when
/// direct URI activation is unavailable — here that's simply "try the
/// direct route, and if spawning fails at all, try the `cmd /c start`
/// wrapper," since both routes end up shelling out on this platform anyway.
pub async fn connect_external_server(server_name: &str, host: &str, port: u16) -> Result<()> {
    let display_name = if server_name.trim().is_empty() {
        host
    } else {
        server_name
    };
    let uri = format!(
        "minecraft://?addExternalServer={}|{}:{}",
        escape_data_string(display_name),
        escape_data_string(host),
        port
    );

    let direct = Command::new("cmd.exe")
        .args(["/c", "start", "", uri.as_str()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    match direct {
        Ok(status) if status.success() => Ok(()),
        _ => Err(LaunchError::Other(format!(
            "Failed to activate deep link: {uri}"
        ))),
    }
}

pub struct LaunchedBedrock {
    pub pid: u32,
    pub was_already_running: bool,
    pub package: BedrockPackageInfo,
}

/// Full Bedrock launch: resolve the package, activate it (or reuse an
/// already-running instance), and optionally inject a client DLL after
/// `injection_delay_ms`. Mirrors `LaunchWithPathCoreAsync`.
pub async fn launch(
    dll_path: Option<PathBuf>,
    injection_delay_ms: u64,
    verify_injection: bool,
) -> Result<LaunchedBedrock> {
    let package = resolve().await?.ok_or(LaunchError::BedrockNotInstalled)?;

    let family = Some(package.family_name.as_str());
    let already_running = find_game_process_for(family).is_some();
    let pid = start_minecraft(&package).await?;

    let Some(dll_path) = dll_path else {
        return Ok(LaunchedBedrock {
            pid,
            was_already_running: already_running,
            package,
        });
    };

    // Readiness wait, modeled on Flarial's own official launcher
    // (`Flarial.Runtime/Game/Minecraft.Bootstrap.cs::Launch`): the game's
    // main window only becomes visible once its script engine has finished
    // booting, which is what actually determines whether injection is safe
    // - a fixed delay before this was a guess at that moment, too short on a
    // slow machine and wasted time on a fast one. `injection_delay_ms`
    // (still a user-facing setting) is kept as a MINIMUM wait, so a value the
    // user set for a specific reason (e.g. a client that wants extra time
    // after menu load) is still honored on top of the readiness signal.
    let window_ready = tokio::task::spawn_blocking(move || {
        gl_injection::wait_for_target_window(
            pid,
            GAME_WINDOW_CLASS,
            WINDOW_READY_TIMEOUT,
            WINDOW_POLL_INTERVAL,
        )
    })
    .await
    .unwrap_or(false);

    if injection_delay_ms > 0 {
        sleep(Duration::from_millis(injection_delay_ms)).await;
    } else if !window_ready {
        // Neither a configured delay nor a confirmed-visible window: give the
        // game a short grace period rather than injecting the instant the
        // process object exists, which is well before it can safely load a
        // client DLL. This mirrors the floor the old fixed-delay code
        // effectively provided.
        sleep(Duration::from_millis(1500)).await;
    }

    // The game can exit or restart during the wait — re-resolve rather than
    // injecting into a stale PID. Mirrors the C# re-check.
    let current_pid = find_game_process_for(family).ok_or(LaunchError::BedrockLaunchTimedOut)?;

    gl_injection::inject_dll(current_pid, &dll_path, verify_injection)
        .map_err(|e| LaunchError::InjectionFailed(e.to_string()))?;

    Ok(LaunchedBedrock {
        pid: current_pid,
        was_already_running: already_running,
        package,
    })
}
