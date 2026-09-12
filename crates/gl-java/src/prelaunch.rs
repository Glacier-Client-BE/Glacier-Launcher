//! Pre-launch, non-blocking health checks: Java-version compatibility for
//! the instance's pinned version, configured heap vs available system RAM,
//! and free disk space on the instance's drive. New feature — no C#
//! counterpart. Reuses `read_version_profile` / `resolve_java_runtime` /
//! `detect_java_major` (all already public in [`crate::launcher`]), so this
//! adds no I/O beyond what a normal launch already performs; the OS-level
//! facts (available RAM, free disk bytes) are gathered by the caller, which
//! already owns the platform-specific code for that (mirroring the existing
//! `commands::utilities`/`commands::versions` pattern) — this module only
//! turns those facts into advisory messages.

use crate::launcher::{detect_java_major, read_version_profile, resolve_java_runtime};
use crate::models::JavaInstance;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrelaunchWarning {
    /// "info" | "warning" — never "error": these never block Launch.
    pub level: String,
    pub message: String,
}

fn warn(message: impl Into<String>) -> PrelaunchWarning {
    PrelaunchWarning {
        level: "warning".to_string(),
        message: message.into(),
    }
}

/// Runs the checks and returns whatever warnings apply. Never returns an
/// `Err` and never blocks the launch — callers surface these as an advisory
/// banner only, alongside a still-enabled Launch button.
#[allow(clippy::too_many_arguments)]
pub fn check(
    instance: &JavaInstance,
    mc_dir: &Path,
    explicit_java_path: &str,
    runtimes_root: &Path,
    configured_max_ram_mb: i32,
    available_ram_mb: Option<u64>,
    free_disk_bytes: Option<u64>,
) -> Vec<PrelaunchWarning> {
    let mut warnings = Vec::new();

    // 1. Java version match for the pinned version, if one is set and
    // installed locally (read_version_profile needs the version's manifest
    // already on disk — an uninstalled/unset version has nothing to check).
    if !instance.version_id.trim().is_empty() {
        if let Ok(profile) = read_version_profile(&instance.version_id, mc_dir) {
            let required: u32 = profile.min_java_major.parse().unwrap_or(0);
            match resolve_java_runtime(explicit_java_path, runtimes_root, &profile.min_java_major)
            {
                Some(javaw) => {
                    let actual = detect_java_major(&javaw);
                    if required > 0 && actual > 0 && (actual as u32) < required {
                        warnings.push(warn(format!(
                            "This version needs Java {required}+, but the runtime found is Java {actual}. \
                             A matching runtime will be downloaded automatically if needed."
                        )));
                    }
                }
                None => warnings.push(warn(
                    "No Java runtime found yet for this version — one will be downloaded at launch.",
                )),
            }
        }
    }

    // 2. Configured heap vs available RAM.
    let ram = configured_max_ram_mb.clamp(512, 16384) as u64;
    if let Some(avail_mb) = available_ram_mb {
        if ram > avail_mb {
            warnings.push(warn(format!(
                "Configured max memory ({ram} MB) is more than the ~{avail_mb} MB currently \
                 available — the game may be slow to start or run out of memory."
            )));
        }
    }

    // 3. Free disk space on the instance's drive.
    if let Some(free) = free_disk_bytes {
        const MIN_FREE_BYTES: u64 = 1_073_741_824; // 1 GiB headroom for logs/saves/screenshots.
        if free < MIN_FREE_BYTES {
            let free_mb = free / 1_048_576;
            warnings.push(warn(format!(
                "Low disk space where this instance lives (~{free_mb} MB free) — \
                 saves or downloads may fail."
            )));
        }
    }

    warnings
}
