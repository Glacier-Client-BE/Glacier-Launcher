//! Find likely install roots for each supported external launcher on
//! Windows, checking real candidate paths rather than guessing blindly.
//!
//! Verification status (see also `PARITY.md`):
//! - **MultiMC/Prism Launcher**: `%APPDATA%\PrismLauncher` (and the older
//!   `%APPDATA%\MultiMC`) with an `instances\` subdirectory is the
//!   documented default for the installed build; both are also commonly run
//!   portable (self-contained folder anywhere), which this detector cannot
//!   find without a user-chosen root — best-effort, unverified against a
//!   real install.
//! - **CurseForge app**: the retail Overwolf-based app stores instances
//!   under `%APPDATA%\CurseForge\Instances` per widely-documented user
//!   reports; there is no public spec for this (closed-source app) — marked
//!   unverified.
//! - **ATLauncher**: `%APPDATA%\ATLauncher\instances` is documented in
//!   ATLauncher's own configuration docs — best-effort, unverified in this
//!   sandbox.
//! - **Modrinth App**: `%APPDATA%\ModrinthApp\profiles` per the Modrinth App
//!   source (`theseus` data dir layout) — best-effort, unverified.

use super::{DetectedLauncher, SourceLauncher};
use std::path::PathBuf;

fn appdata() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

/// Detect every supported external launcher's install root that appears to
/// exist on disk. Returns an empty vec (never errors) when `%APPDATA%` can't
/// be resolved or nothing is found — detection failure is not fatal to the
/// rest of the app.
pub fn detect_all() -> Vec<DetectedLauncher> {
    let Some(appdata) = appdata() else {
        return Vec::new();
    };
    let mut out = Vec::new();

    for candidate in ["PrismLauncher", "MultiMC"] {
        let root = appdata.join(candidate);
        if root.join("instances").is_dir() {
            out.push(DetectedLauncher {
                launcher: SourceLauncher::MultiMcOrPrism,
                root_path: root,
            });
        }
    }

    let cf_root = appdata.join("CurseForge").join("Instances");
    if cf_root.is_dir() {
        out.push(DetectedLauncher {
            launcher: SourceLauncher::CurseForgeApp,
            root_path: cf_root,
        });
    }

    let at_root = appdata.join("ATLauncher");
    if at_root.join("instances").is_dir() {
        out.push(DetectedLauncher {
            launcher: SourceLauncher::ATLauncher,
            root_path: at_root,
        });
    }

    let mr_root = appdata.join("ModrinthApp");
    if mr_root.join("profiles").is_dir() {
        out.push(DetectedLauncher {
            launcher: SourceLauncher::ModrinthApp,
            root_path: mr_root,
        });
    }

    out
}

/// Instances subdirectory for a detected launcher root, mirroring each
/// adapter's expected layout.
pub fn instances_dir(launcher: SourceLauncher, root: &std::path::Path) -> PathBuf {
    match launcher {
        SourceLauncher::MultiMcOrPrism => root.join("instances"),
        SourceLauncher::CurseForgeApp => root.to_path_buf(),
        SourceLauncher::ATLauncher => root.join("instances"),
        SourceLauncher::ModrinthApp => root.join("profiles"),
    }
}
