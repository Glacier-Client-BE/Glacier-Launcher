//! Launcher self-update (AutoUpdateService.cs port / tauri-plugin-updater integration)

mod cache;
mod error;
mod models;
mod releases;
mod resume;
mod version;

pub use cache::{CachedResponse, GitHubApiCache, FRESH_TTL};
pub use error::{Result, UpdateError};
pub use models::{GitHubAsset, GitHubRelease, LauncherRelease, LauncherUpdateInfo};
pub use releases::{pick_asset, pick_update, recent_releases, UpdateChecker};
pub use resume::{clear_pending_marker, marker_path, take_pending_update, write_pending_marker};
pub use version::{is_newer_version, normalize_tag};
