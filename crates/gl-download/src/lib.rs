//! Resumable/parallel download engine shared by Java, Bedrock, mods, and auto-update (DownloadService.cs)

mod client;
mod download;
mod error;
mod progress;
mod queue;

pub use client::{DownloadClient, USER_AGENT};
pub use download::{download_file, tmp_sibling, DownloadOutcome, DownloadRequest};
pub use error::{DownloadError, Result};
pub use progress::{ProgressValue, ThrottledProgress};
pub use queue::{DownloadEntry, DownloadHandle, DownloadQueue, DownloadStatus};
