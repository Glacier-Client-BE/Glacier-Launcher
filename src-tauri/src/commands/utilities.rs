//! Thin wrappers for the "Utilities" panel (ported from `Home.Utilities.cs`
//! / `BedrockOptionsService.cs` / `HardwareMonitorService.cs`). Three of its
//! four original sub-tabs are covered for real here:
//!
//! - **options**: full read/write of Bedrock's `options.txt`.
//! - **system**: RAM + disks via raw `kernel32` calls, CPU name/identifier/
//!   MHz, GPU list, and motherboard/BIOS info all read straight out of the
//!   registry (`winreg`). The C# original (`HardwareMonitorService.cs`)
//!   never actually used WMI for any of this — it's all registry reads
//!   (`HKLM\SYSTEM\...\Control\Class\{4d36e968-...}` for GPUs,
//!   `HKLM\HARDWARE\DESCRIPTION\System\BIOS` for board/BIOS) and raw
//!   `kernel32` calls (`GetSystemTimes`/`GetTickCount64` for live CPU% and
//!   uptime), so this port follows the same technique. Live CPU%/RAM%
//!   polling (`get_live_sample`) keeps a previous-sample baseline in
//!   `AppState` and computes the delta the same way `HardwareMonitorService
//!   .Sample()` did.
//! - **clients** (the user's own custom DLL/EXE library — distinct from the
//!   "Client Library" GitHub catalog panel): full list/add/rename/delete,
//!   built on `gl_clients::custom_client_library`'s pure helpers.
//!
//! The **shaders** sub-tab is wired for real in `commands::shaders` — see
//! that module's docs. `gl_bedrock::shader::ShaderService` already existed
//! as a straight port of `Services/ShaderService.cs`'s bulk file-copy logic;
//! this pass only needed to resolve the installed materials directory and
//! expose it as Tauri commands.

use std::path::{Path, PathBuf};

use gl_clients::custom_client_library as cc;
use gl_clients::models::CustomClientEntry;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

// ---------------------------------------------------------------------
// options.txt
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptionKind {
    Toggle,
    Integer,
    Decimal,
    Text,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptionEntry {
    pub key: String,
    pub value: String,
    pub kind: OptionKind,
    pub label: String,
    pub group: String,
    pub hint: String,
    pub known: bool,
}

struct Meta {
    label: &'static str,
    group: &'static str,
    hint: &'static str,
}

const fn m(label: &'static str, group: &'static str) -> Meta {
    Meta {
        label,
        group,
        hint: "",
    }
}
const fn mh(label: &'static str, group: &'static str, hint: &'static str) -> Meta {
    Meta { label, group, hint }
}

/// Ported from `BedrockOptionsService.KnownKeys`.
fn known_keys(key: &str) -> Option<Meta> {
    let k = key.to_ascii_lowercase();
    Some(match k.as_str() {
        "gfx_vsync" => mh(
            "V-Sync",
            "Video",
            "Caps framerate to your monitor. Off can reduce input lag.",
        ),
        "gfx_fullscreen" => m("Fullscreen", "Video"),
        "gfx_viewdistance" => mh(
            "Render distance",
            "Video",
            "Measured in blocks — 16 blocks = 1 chunk.",
        ),
        "gfx_gamma" => m("Brightness", "Video"),
        "gfx_particleviewdistance" => m("Particle render distance", "Video"),
        "gfx_fancygraphics" => m("Fancy graphics", "Video"),
        "gfx_fancyleaves" => m("Fancy leaves", "Video"),
        "gfx_transparentleaves" => m("Transparent leaves", "Video"),
        "gfx_smoothlighting" => m("Smooth lighting", "Video"),
        "gfx_texel_aa_enabled" => m("Texel anti-aliasing", "Video"),
        "gfx_multithreaded_renderer" => mh(
            "Multithreaded renderer",
            "Video",
            "Usually a win on 4+ core CPUs.",
        ),
        "gfx_msaa" => m("Anti-aliasing (MSAA)", "Video"),
        "gfx_hidehand" => m("Hide hand", "Video"),
        "gfx_hidepaperdoll" => m("Hide paper doll", "Video"),
        "gfx_hidegui" => m("Hide GUI", "Video"),
        "gfx_field_of_view" => m("Field of view", "Video"),
        "gfx_fov" => m("Field of view", "Video"),
        "gfx_gui_scale" => m("GUI scale", "Interface"),
        "gui_scale" => m("GUI scale", "Interface"),
        "gfx_gui_scale_offset" => m("GUI scale offset", "Interface"),
        "gui_scale_offset" => m("GUI scale offset", "Interface"),
        "ctrl_sensitivity" => m("Mouse sensitivity", "Controls"),
        "ctrl_rawinput" => m("Raw input", "Controls"),
        "ctrl_autojump" => m("Auto-jump", "Controls"),
        "ctrl_invertmouse" => m("Invert mouse", "Controls"),
        "audio_music" => m("Music volume", "Audio"),
        "audio_sound" => m("Sound volume", "Audio"),
        "dev_enable_debug_hud" => m("Debug HUD", "Advanced"),
        "dev_showhitboxes" => m("Show hitboxes", "Advanced"),
        "game_difficulty" => m("Difficulty", "Gameplay"),
        "game_hidehud" => m("Hide HUD", "Gameplay"),
        _ => return None,
    })
}

fn prettify(key: &str) -> String {
    let s = key.replace('_', " ");
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => key.to_string(),
    }
}

fn infer_kind(key: &str, value: &str, meta: &Option<Meta>) -> OptionKind {
    if value == "0" || value == "1" {
        if let Some(meta) = meta {
            if matches!(
                meta.group,
                "Video" | "Controls" | "Interface" | "Advanced" | "Gameplay"
            ) {
                let kl = key.to_ascii_lowercase();
                if [
                    "distance",
                    "scale",
                    "gamma",
                    "sensitiv",
                    "msaa",
                    "fov",
                    "field_of_view",
                ]
                .iter()
                .any(|needle| kl.contains(needle))
                {
                    return OptionKind::Integer;
                }
            }
        }
        return OptionKind::Toggle;
    }
    if value.parse::<i64>().is_ok() {
        return OptionKind::Integer;
    }
    if value.parse::<f64>().is_ok() {
        return OptionKind::Decimal;
    }
    OptionKind::Text
}

/// `options.txt` for the selected instance, or for the live install when no
/// instance is selected.
///
/// C# `BedrockOptionsService.OptionsPath` is always
/// `CurseForgeService.MinecraftPeDir/options.txt` — i.e. the live,
/// per-account-resolved `minecraftpe` folder. Joining `"minecraftpe"` onto an
/// empty instance dir produced the *relative* path `minecraftpe/options.txt`,
/// which never matched the real file.
fn options_path(_instance_dir: &str) -> PathBuf {
    // Always the LIVE file. `BedrockOptionsService.OptionsPath` is a static
    // `CurseForgeService.MinecraftPeDir + "options.txt"` with no instance
    // parameter: the instance system swaps data in and out of the live folder,
    // so live is always the active instance's data and an instance directory
    // is a dormant snapshot. Resolving into the snapshot meant the Options tab
    // reported "options.txt isn't available" on a machine that has one.
    gl_bedrock::paths::minecraft_pe_dir().join("options.txt")
}

/// Port of C# `GameLauncher.IsMinecraftRunning()`.
fn is_minecraft_running() -> bool {
    gl_injection::find_process_by_name("Minecraft.Windows.exe").is_some()
}

fn backup_path(instance_dir: &str) -> PathBuf {
    let mut p = options_path(instance_dir).into_os_string();
    p.push(".glacier.bak");
    PathBuf::from(p)
}

/// Mirrors `BedrockOptionsService.Load`.
#[tauri::command(async)]
pub fn get_bedrock_options(instance_dir: String) -> Vec<OptionEntry> {
    let path = options_path(&instance_dir);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(sep) = line.find(':') else { continue };
        if sep == 0 {
            continue;
        }
        let key = line[..sep].trim().to_string();
        let value = line[sep + 1..].trim().to_string();
        if key.is_empty() {
            continue;
        }
        let meta = known_keys(&key);
        out.push(OptionEntry {
            kind: infer_kind(&key, &value, &meta),
            label: meta
                .as_ref()
                .map(|m| m.label.to_string())
                .unwrap_or_else(|| prettify(&key)),
            group: meta
                .as_ref()
                .map(|m| m.group.to_string())
                .unwrap_or_else(|| "Other".to_string()),
            hint: meta
                .as_ref()
                .map(|m| m.hint.to_string())
                .unwrap_or_default(),
            known: meta.is_some(),
            key,
            value,
        });
    }
    out
}

#[tauri::command(async)]
pub fn bedrock_options_exist(instance_dir: String) -> bool {
    options_path(&instance_dir).is_file()
}

#[tauri::command(async)]
pub fn bedrock_options_has_backup(instance_dir: String) -> bool {
    backup_path(&instance_dir).is_file()
}

/// Port of `BedrockOptionsService.Save`, including its two refusal cases
/// (game running, settings folder absent) and its case-insensitive key
/// matching.
#[tauri::command(async)]
pub fn save_bedrock_options(
    instance_dir: String,
    changes: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    if changes.is_empty() {
        return Ok(());
    }

    if is_minecraft_running() {
        return Err(
            "Close Minecraft first — it rewrites options.txt on exit and would undo these changes."
                .to_string(),
        );
    }

    let path = options_path(&instance_dir);
    let dir = path.parent().ok_or("Invalid options.txt path")?;
    // C# throws here rather than creating the folder: if `minecraftpe` does
    // not exist, the game has never run and there is nothing to configure.
    if !dir.is_dir() {
        return Err(
            "Minecraft's settings folder doesn't exist yet. Launch the game once, then try again."
                .to_string(),
        );
    }

    let existing: Vec<String> = if path.is_file() {
        std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())?
            .lines()
            .map(str::to_string)
            .collect()
    } else {
        Vec::new()
    };

    let bak = backup_path(&instance_dir);
    if path.is_file() && !bak.is_file() {
        let _ = std::fs::copy(&path, &bak);
    }

    // C# builds `pending` with `StringComparer.OrdinalIgnoreCase`, so a key
    // whose case differs from the one already in options.txt *updates* that
    // line. Matching case-sensitively appended a duplicate key instead,
    // leaving two conflicting lines for the same setting.
    let mut pending = changes;
    let mut ci_index: std::collections::HashMap<String, String> = pending
        .keys()
        .map(|k| (k.to_lowercase(), k.clone()))
        .collect();

    let mut lines = Vec::with_capacity(existing.len());
    for line in existing {
        // `sep <= 0` in C# skips both "no colon" and a colon in column 0.
        if let Some(sep) = line.find(':').filter(|&i| i > 0) {
            let key = line[..sep].trim().to_string();
            if let Some(orig_key) = ci_index.remove(&key.to_lowercase()) {
                if let Some(new_value) = pending.remove(&orig_key) {
                    lines.push(format!("{key}:{new_value}"));
                    continue;
                }
            }
        }
        lines.push(line);
    }
    // Deterministic order for keys that weren't already present.
    let mut leftover: Vec<(String, String)> = pending.into_iter().collect();
    leftover.sort();
    for (key, value) in leftover {
        lines.push(format!("{key}:{value}"));
    }

    let mut text = lines.join("\r\n");
    text.push_str("\r\n");

    let tmp = path.with_extension("txt.tmp");
    std::fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn restore_bedrock_options_backup(instance_dir: String) -> Result<bool, String> {
    let bak = backup_path(&instance_dir);
    if !bak.is_file() {
        return Ok(false);
    }
    if is_minecraft_running() {
        return Err("Close Minecraft first.".to_string());
    }
    let text = std::fs::read_to_string(&bak).map_err(|e| e.to_string())?;
    let path = options_path(&instance_dir);
    let tmp = path.with_extension("txt.tmp");
    std::fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(true)
}

// ---------------------------------------------------------------------
// system snapshot (RAM + disks + CPU + GPU + board/BIOS)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DiskSnapshot {
    pub name: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub vram_bytes: u64,
    pub driver_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSnapshot {
    pub cpu_name: String,
    pub cpu_cores: usize,
    pub cpu_identifier: String,
    pub cpu_mhz: u32,
    pub gpus: Vec<GpuInfo>,
    pub board_vendor: String,
    pub board_product: String,
    pub bios_version: String,
    pub total_ram_bytes: u64,
    pub used_ram_bytes: u64,
    pub os: String,
    pub arch: String,
    pub disks: Vec<DiskSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveSample {
    pub cpu_percent: f64,
    pub ram_percent: f64,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub uptime_ms: u64,
}

/// Reads `ProcessorNameString` from the registry — matches
/// `HardwareMonitorService.ReadCpu`.
#[cfg(windows)]
fn read_cpu_name() -> String {
    read_cpu_string("ProcessorNameString").unwrap_or_else(|| "Unknown CPU".to_string())
}

#[cfg(not(windows))]
fn read_cpu_name() -> String {
    "Unknown CPU".to_string()
}

#[cfg(windows)]
fn read_cpu_string(value_name: &str) -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0")
        .and_then(|key| key.get_value::<String, _>(value_name))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(windows)]
fn read_cpu_mhz() -> u32 {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0")
        .and_then(|key| key.get_value::<u32, _>("~MHz"))
        .unwrap_or(0)
}

/// Reads a value under `HKLM\HARDWARE\DESCRIPTION\System\BIOS`. Some
/// values (notably `BIOSVersion` on many boards) come back as a
/// `REG_MULTI_SZ` string array rather than a plain string — try both,
/// matching `HardwareMonitorService.ReadBios`.
#[cfg(windows)]
fn read_bios(value_name: &str) -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"HARDWARE\DESCRIPTION\System\BIOS")
        .ok()?;

    if let Ok(s) = key.get_value::<String, _>(value_name) {
        let s = s.trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    if let Ok(arr) = key.get_value::<Vec<String>, _>(value_name) {
        let s = arr.join(" ").trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

#[cfg(windows)]
const DISPLAY_CLASS_KEY: &str =
    r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";

/// Enumerates installed GPUs from the display adapter class registry key —
/// matches `HardwareMonitorService.ReadGpus`. Best-effort: any single
/// unreadable subkey/value is skipped rather than failing the whole scan.
#[cfg(windows)]
fn read_gpus() -> Vec<GpuInfo> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, RegType};
    use winreg::RegKey;

    let mut out: Vec<GpuInfo> = Vec::new();
    let root = match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(DISPLAY_CLASS_KEY) {
        Ok(k) => k,
        Err(_) => return out,
    };

    for sub_name in root.enum_keys().flatten() {
        if sub_name.is_empty() || !sub_name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let sub = match root.open_subkey(&sub_name) {
            Ok(k) => k,
            Err(_) => continue,
        };

        let name = match sub.get_value::<String, _>("DriverDesc") {
            Ok(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => continue,
        };
        if out.iter().any(|g| g.name == name) {
            continue;
        }

        let vram_bytes = sub
            .get_raw_value("HardwareInformation.qwMemorySize")
            .ok()
            .map(|raw| match raw.vtype {
                RegType::REG_QWORD if raw.bytes.len() >= 8 => {
                    u64::from_le_bytes(raw.bytes[..8].try_into().unwrap_or([0; 8]))
                }
                RegType::REG_DWORD if raw.bytes.len() >= 4 => {
                    u32::from_le_bytes(raw.bytes[..4].try_into().unwrap_or([0; 4])) as u64
                }
                _ if raw.bytes.len() >= 8 => {
                    u64::from_le_bytes(raw.bytes[..8].try_into().unwrap_or([0; 8]))
                }
                _ if raw.bytes.len() >= 4 => {
                    u32::from_le_bytes(raw.bytes[..4].try_into().unwrap_or([0; 4])) as u64
                }
                _ => 0,
            })
            .unwrap_or(0);

        let driver_version = sub
            .get_value::<String, _>("DriverVersion")
            .unwrap_or_default();

        out.push(GpuInfo {
            name,
            vram_bytes,
            driver_version,
        });
    }
    out
}

#[cfg(not(windows))]
fn read_gpus() -> Vec<GpuInfo> {
    Vec::new()
}

#[cfg(windows)]
mod win {
    // Raw kernel32 declarations — deliberately avoids adding a `windows`/
    // `sysinfo` crate dependency to src-tauri/Cargo.toml (out of scope for
    // this change) for just two calls. kernel32 is implicitly linked on
    // every Windows Rust target, so `#[link]` here needs no new dependency.
    #[repr(C)]
    #[allow(non_snake_case)]
    // Name kept exactly as Win32 documents it so this can be checked
    // against the API reference; renaming it to `Memorystatusex` would
    // only obscure that.
    #[allow(clippy::upper_case_acronyms)]
    pub struct MEMORYSTATUSEX {
        pub dwLength: u32,
        pub dwMemoryLoad: u32,
        pub ullTotalPhys: u64,
        pub ullAvailPhys: u64,
        pub ullTotalPageFile: u64,
        pub ullAvailPageFile: u64,
        pub ullTotalVirtual: u64,
        pub ullAvailVirtual: u64,
        pub ullAvailExtendedVirtual: u64,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MEMORYSTATUSEX) -> i32;
        fn GetLogicalDrives() -> u32;
        fn GetDiskFreeSpaceExW(
            dir_name: *const u16,
            free_bytes_avail: *mut u64,
            total_bytes: *mut u64,
            total_free_bytes: *mut u64,
        ) -> i32;
        fn GetSystemTimes(idle_time: *mut i64, kernel_time: *mut i64, user_time: *mut i64) -> i32;
        fn GetTickCount64() -> u64;
    }

    /// Returns (idle, kernel, user) as raw FILETIME 100ns ticks, matching
    /// `HardwareMonitorService.GetSystemTimes`.
    pub fn system_times() -> Option<(i64, i64, i64)> {
        let mut idle = 0i64;
        let mut kernel = 0i64;
        let mut user = 0i64;
        let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
        if ok == 0 {
            return None;
        }
        Some((idle, kernel, user))
    }

    pub fn uptime_ms() -> u64 {
        unsafe { GetTickCount64() }
    }

    pub fn memory() -> Option<(u64, u64)> {
        let mut status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            dwMemoryLoad: 0,
            ullTotalPhys: 0,
            ullAvailPhys: 0,
            ullTotalPageFile: 0,
            ullAvailPageFile: 0,
            ullTotalVirtual: 0,
            ullAvailVirtual: 0,
            ullAvailExtendedVirtual: 0,
        };
        let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
        if ok == 0 {
            return None;
        }
        let used = status.ullTotalPhys.saturating_sub(status.ullAvailPhys);
        Some((status.ullTotalPhys, used))
    }

    pub fn disks() -> Vec<super::DiskSnapshot> {
        let mut out = Vec::new();
        let mask = unsafe { GetLogicalDrives() };
        for i in 0..26u32 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let letter = (b'A' + i as u8) as char;
            let root = format!("{letter}:\\");
            let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
            let mut free = 0u64;
            let mut total = 0u64;
            let mut total_free = 0u64;
            let ok = unsafe {
                GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free)
            };
            if ok != 0 && total > 0 {
                out.push(super::DiskSnapshot {
                    name: letter.to_string(),
                    total_bytes: total,
                    free_bytes: total_free,
                });
            }
        }
        out
    }
}

/// The registry and GPU enumeration never change while the launcher runs, so
/// they are read once. Only RAM and disk usage are refreshed per call.
#[tauri::command(async)]
pub fn get_system_snapshot() -> SystemSnapshot {
    static STATIC_SNAPSHOT: std::sync::OnceLock<SystemSnapshot> = std::sync::OnceLock::new();
    let mut snap = STATIC_SNAPSHOT.get_or_init(read_system_snapshot).clone();
    #[cfg(windows)]
    {
        let (total, used) = win::memory().unwrap_or((0, 0));
        snap.total_ram_bytes = total;
        snap.used_ram_bytes = used;
        snap.disks = win::disks();
    }
    snap
}

fn read_system_snapshot() -> SystemSnapshot {
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    #[cfg(windows)]
    let (total_ram_bytes, used_ram_bytes, disks) = {
        let (total, used) = win::memory().unwrap_or((0, 0));
        (total, used, win::disks())
    };
    #[cfg(not(windows))]
    let (total_ram_bytes, used_ram_bytes, disks): (u64, u64, Vec<DiskSnapshot>) =
        (0, 0, Vec::new());

    #[cfg(windows)]
    let cpu_identifier = read_cpu_string("Identifier").unwrap_or_default();
    #[cfg(not(windows))]
    let cpu_identifier = String::new();

    #[cfg(windows)]
    let cpu_mhz = read_cpu_mhz();
    #[cfg(not(windows))]
    let cpu_mhz = 0u32;

    #[cfg(windows)]
    let (board_vendor, board_product, bios_version) = (
        read_bios("BaseBoardManufacturer")
            .or_else(|| read_bios("SystemManufacturer"))
            .unwrap_or_default(),
        read_bios("BaseBoardProduct")
            .or_else(|| read_bios("SystemProductName"))
            .unwrap_or_default(),
        read_bios("BIOSVersion").unwrap_or_default(),
    );
    #[cfg(not(windows))]
    let (board_vendor, board_product, bios_version) =
        (String::new(), String::new(), String::new());

    SystemSnapshot {
        cpu_name: read_cpu_name(),
        cpu_cores,
        cpu_identifier,
        cpu_mhz,
        gpus: read_gpus(),
        board_vendor,
        board_product,
        bios_version,
        total_ram_bytes,
        used_ram_bytes,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        disks,
    }
}

/// Baseline for delta-based live CPU% sampling — holds the previous
/// `GetSystemTimes` sample, matching `HardwareMonitorService`'s
/// `_prevIdle/_prevKernel/_prevUser/_haveBaseline` instance fields.
#[derive(Debug, Clone, Copy, Default)]
pub struct LiveSampleBaseline {
    pub prev_idle: i64,
    pub prev_kernel: i64,
    pub prev_user: i64,
    pub have_baseline: bool,
}

#[tauri::command(async)]
pub fn get_live_sample(state: State<'_, AppState>) -> LiveSample {
    #[cfg(windows)]
    let cpu_percent = {
        let mut baseline = state.live_sample_baseline.lock().unwrap();
        let mut cpu = 0.0f64;
        if let Some((idle, kernel, user)) = win::system_times() {
            if baseline.have_baseline {
                let idle_delta = idle - baseline.prev_idle;
                let kernel_delta = kernel - baseline.prev_kernel;
                let user_delta = user - baseline.prev_user;
                let total = kernel_delta + user_delta;
                if total > 0 {
                    cpu = ((total - idle_delta) as f64 * 100.0 / total as f64).clamp(0.0, 100.0);
                }
            }
            baseline.prev_idle = idle;
            baseline.prev_kernel = kernel;
            baseline.prev_user = user;
            baseline.have_baseline = true;
        }
        cpu
    };
    #[cfg(not(windows))]
    let cpu_percent = 0.0f64;

    #[cfg(windows)]
    let (ram_total_bytes, ram_used_bytes) = win::memory().unwrap_or((0, 0));
    #[cfg(not(windows))]
    let (ram_total_bytes, ram_used_bytes): (u64, u64) = (0, 0);

    let ram_percent = if ram_total_bytes > 0 {
        (ram_used_bytes as f64 * 100.0 / ram_total_bytes as f64).clamp(0.0, 100.0)
    } else {
        0.0
    };

    #[cfg(windows)]
    let uptime_ms = win::uptime_ms();
    #[cfg(not(windows))]
    let uptime_ms = 0u64;

    LiveSample {
        cpu_percent,
        ram_percent,
        ram_used_bytes,
        ram_total_bytes,
        uptime_ms,
    }
}

// ---------------------------------------------------------------------
// custom client library (the user's own .dll/.exe clients)
// ---------------------------------------------------------------------

fn user_profile() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn library_dir() -> PathBuf {
    cc::library_dir(&user_profile())
}

fn load_entries() -> Result<Vec<CustomClientEntry>, String> {
    let dir = library_dir();
    let index = cc::index_path(&dir);
    let mut entries = if index.is_file() {
        let body = std::fs::read_to_string(&index).map_err(|e| e.to_string())?;
        cc::parse_index(&body).map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };
    if cc::prune_missing_files(&mut entries, &dir) {
        save_entries(&entries)?;
    }
    Ok(entries)
}

fn save_entries(entries: &[CustomClientEntry]) -> Result<(), String> {
    let dir = library_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = cc::serialize_index(entries).map_err(|e| e.to_string())?;
    std::fs::write(cc::index_path(&dir), json).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn list_custom_clients() -> Result<Vec<CustomClientEntry>, String> {
    load_entries()
}

fn register_new_entry(
    src: &Path,
    desired_name: &str,
    display_name: &str,
    source_url: &str,
) -> Result<CustomClientEntry, String> {
    let dir = library_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let kind = cc::kind_for_extension(desired_name).map_err(|e| e.to_string())?;
    let file_name = cc::unique_file_name(&dir, desired_name);
    let dest = dir.join(&file_name);
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    let (sha256, size_bytes) = cc::hash_and_size(&dest).map_err(|e| e.to_string())?;

    let now = now_iso();
    let entry = CustomClientEntry {
        id: uuid_v4_lite(),
        name: display_name.to_string(),
        file_name,
        kind: kind.to_string(),
        source_url: source_url.to_string(),
        sha256,
        size_bytes: size_bytes as i64,
        added_iso: now.clone(),
        updated_iso: now,
        auto_update: !source_url.is_empty(),
    };

    let mut entries = load_entries()?;
    entries.push(entry.clone());
    save_entries(&entries)?;
    Ok(entry)
}

/// Minimal dependency-free v4-ish id: not RFC 4122 strict, just unique
/// enough for this local index (matches the C# `Guid.NewGuid()` role only
/// as an opaque stable identifier — nothing parses this id's structure).
fn now_iso() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let (h, mi, s) = (
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60,
    );
    // Days-since-epoch -> Y/M/D (civil_from_days, Howard Hinnant's algorithm).
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn uuid_v4_lite() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:032x}")
}

#[tauri::command(async)]
pub fn add_custom_client_from_file(path: String) -> Result<CustomClientEntry, String> {
    let src = PathBuf::from(&path);
    if !src.is_file() {
        return Err("File not found.".to_string());
    }
    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file name")?;
    let stem = src
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or(file_name)
        .to_string();
    register_new_entry(&src, file_name, &stem, "")
}

#[tauri::command]
pub async fn add_custom_client_from_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<CustomClientEntry, String> {
    let file_name = url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("client.dll")
        .to_string();
    cc::kind_for_extension(&file_name).map_err(|e| e.to_string())?;

    let dir = library_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let staging = dir.join(format!("_download_{file_name}"));

    let req = gl_download::DownloadRequest::new(&url, &staging).with_label("Custom client");
    let handle = state.downloads.submit(req);
    handle.join().await.map_err(|e| e.to_string())?;

    let stem = Path::new(&file_name)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or(&file_name)
        .to_string();
    let result = register_new_entry(&staging, &file_name, &stem, &url);
    let _ = std::fs::remove_file(&staging);
    result
}

#[tauri::command(async)]
pub fn rename_custom_client(id: String, new_name: String) -> Result<(), String> {
    let mut entries = load_entries()?;
    let entry = entries
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("Client not found")?;
    let trimmed = new_name.trim();
    if !trimmed.is_empty() {
        entry.name = trimmed.to_string();
    }
    save_entries(&entries)
}

#[tauri::command(async)]
pub fn toggle_custom_client_auto_update(id: String) -> Result<(), String> {
    let mut entries = load_entries()?;
    let entry = entries
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("Client not found")?;
    entry.auto_update = !entry.auto_update;
    save_entries(&entries)
}

#[tauri::command(async)]
pub fn delete_custom_client(id: String) -> Result<(), String> {
    let dir = library_dir();
    let mut entries = load_entries()?;
    if let Some(pos) = entries.iter().position(|e| e.id == id) {
        let entry = entries.remove(pos);
        let path = cc::path_of(&dir, &entry);
        if path.is_file() {
            let _ = std::fs::remove_file(&path);
        }
    }
    save_entries(&entries)
}

#[tauri::command(async)]
pub fn open_custom_client_library_folder() -> Result<(), String> {
    let dir = library_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
