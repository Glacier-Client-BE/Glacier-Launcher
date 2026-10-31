//! Single-file download with retry, atomic commit, and optional SHA256 verification.
//!
//! Ports `DownloadService.cs`'s `DownloadAsync` / `DownloadLockedAsync` /
//! `DownloadToTempWithRetryAsync` pipeline: stream to a `.tmp` sibling file while
//! hashing incrementally, then atomically rename onto the destination, retrying
//! transient failures with exponential backoff.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;

use crate::client::DownloadClient;
use crate::error::{DownloadError, Result};
use crate::progress::ProgressValue;

/// Buffer size for both disk reads (re-hashing a partial) and buffered writes
/// of the streamed body.
const WRITE_BUFFER_BYTES: usize = 256 * 1024;

/// A "no real hash supplied" sentinel some callers pass (mirrors `HasRealHash` in
/// the C# source, which treated a leading run of zeros as "not a real checksum").
fn has_real_hash(sha256: &Option<String>) -> bool {
    match sha256 {
        Some(s) if !s.is_empty() => !s.starts_with("0000"),
        _ => false,
    }
}

/// Options for a single file download.
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub url: String,
    pub destination: PathBuf,
    pub expected_sha256: Option<String>,
    /// Total size in bytes if already known (e.g. from a manifest), used as a
    /// fallback when the server doesn't send `Content-Length`.
    pub known_total_bytes: Option<u64>,
    pub max_attempts: u32,
    pub label: Option<String>,
}

impl DownloadRequest {
    pub fn new(url: impl Into<String>, destination: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            destination: destination.into(),
            expected_sha256: None,
            known_total_bytes: None,
            max_attempts: 3,
            label: None,
        }
    }

    pub fn with_sha256(mut self, sha256: impl Into<String>) -> Self {
        self.expected_sha256 = Some(sha256.into());
        self
    }

    pub fn with_known_total_bytes(mut self, bytes: u64) -> Self {
        self.known_total_bytes = Some(bytes);
        self
    }

    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts.max(1);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn validate(&self) -> Result<()> {
        if self.url.trim().is_empty() {
            return Err(DownloadError::InvalidInput(
                "download URL is required".into(),
            ));
        }
        if self.destination.as_os_str().is_empty() {
            return Err(DownloadError::InvalidInput(
                "destination path is required".into(),
            ));
        }
        Ok(())
    }
}

/// Result of a successfully completed download.
#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub sha256: String,
    pub bytes: u64,
}

/// Downloads `req.url` to `req.destination`, retrying transient failures up to
/// `req.max_attempts` times with exponential backoff, and reports progress in
/// `[0.0, 1.0]` through `on_progress` (already throttled by the caller if desired).
///
/// `on_total` is called once per attempt with the response's total size as soon
/// as it is known, so callers can show real byte counts for downloads whose
/// size wasn't supplied up front via `with_known_total_bytes` - the C#
/// `DownloadEntry.TotalBytes` was likewise filled from the response's
/// `Content-Length`, not only from a caller-supplied value. It is NOT throttled
/// (it fires at most once per attempt) and reports 0 when the server sends no
/// length.
pub async fn download_file(
    client: &DownloadClient,
    req: &DownloadRequest,
    mut on_progress: impl FnMut(ProgressValue),
    mut on_total: impl FnMut(u64),
    cancel: &CancellationToken,
) -> Result<DownloadOutcome> {
    req.validate()?;

    if let Some(parent) = req.destination.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| DownloadError::io(parent, e))?;
        }
    }

    let tmp_path = tmp_sibling(&req.destination);
    let result = download_with_retry(
        client,
        req,
        &tmp_path,
        &mut on_progress,
        &mut on_total,
        cancel,
    )
    .await;

    match &result {
        Ok(_) => commit(&tmp_path, &req.destination).await?,
        // A paused download's `.tmp` is deliberately left in place (a real
        // cancel calls `try_delete` itself before this - see
        // `queue.rs::pause` vs `queue.rs::cancel`) so `resume` can pick up
        // where it left off via a Range request instead of restarting.
        Err(DownloadError::Cancelled) => {}
        Err(_) => try_delete(&tmp_path).await,
    }

    result
}

/// Deterministic (not per-attempt-random) so a paused-then-resumed download
/// finds the same partial file to continue from. Downloading two different
/// files to the same `destination` concurrently was never supported anyway
/// (they'd collide on the final `commit` rename regardless of tmp naming).
/// Public so `queue.rs` can find (and delete) the same path when a *real*
/// cancel - as opposed to a pause - needs to clean it up.
pub fn tmp_sibling(destination: &Path) -> PathBuf {
    let mut name = destination
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".partial.tmp");
    destination.with_file_name(name)
}

async fn download_with_retry(
    client: &DownloadClient,
    req: &DownloadRequest,
    tmp_path: &Path,
    on_progress: &mut impl FnMut(ProgressValue),
    on_total: &mut impl FnMut(u64),
    cancel: &CancellationToken,
) -> Result<DownloadOutcome> {
    let mut attempt = 1u32;
    loop {
        if cancel.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }

        match download_once(client, req, tmp_path, on_progress, on_total, cancel).await {
            Ok(outcome) => return Ok(outcome),
            // A cancellation (pause or real cancel) mid-attempt must
            // propagate as a bare `Cancelled`, not get wrapped in
            // `RetriesExhausted` - `Cancelled::is_transient()` is `false`,
            // so without this early return it fell into the same "give up
            // and wrap" branch a real failure does, and the queue's
            // `matches!(err, DownloadError::Cancelled)` status check
            // (paused vs failed, cancelled vs failed) would never see a
            // plain `Cancelled` for anything cancelled mid-stream - only
            // for the (much rarer) case of cancelling between attempts.
            Err(DownloadError::Cancelled) => return Err(DownloadError::Cancelled),
            Err(err) => {
                let transient = err.is_transient();
                if attempt >= req.max_attempts || !transient {
                    return Err(DownloadError::RetriesExhausted {
                        url: req.url.clone(),
                        attempts: attempt,
                        source: Box::new(err),
                    });
                }
                on_progress(0.0);
                let backoff = backoff_for(attempt);
                tokio::select! {
                    _ = tokio::time::sleep(backoff) => {}
                    _ = cancel.cancelled() => return Err(DownloadError::Cancelled),
                }
                attempt += 1;
            }
        }
    }
}

fn backoff_for(attempt: u32) -> Duration {
    let millis = 400.0 * 2f64.powi((attempt as i32) - 1);
    Duration::from_millis(millis.min(u32::MAX as f64) as u64)
}

async fn download_once(
    client: &DownloadClient,
    req: &DownloadRequest,
    tmp_path: &Path,
    on_progress: &mut impl FnMut(ProgressValue),
    on_total: &mut impl FnMut(u64),
    cancel: &CancellationToken,
) -> Result<DownloadOutcome> {
    // QOL pause/resume: a `.tmp` left behind by a paused download (see
    // `download_file`'s doc comment) is resumed with a `Range` request
    // instead of restarting from zero. `existing_len` is only trusted once
    // the response actually confirms it with `206 Partial Content` -
    // servers are allowed to ignore `Range` and answer `200` with the full
    // body, in which case this falls back to a normal full download.
    let existing_len = fs::metadata(tmp_path).await.map(|m| m.len()).unwrap_or(0);

    let mut request_builder = client.inner().get(&req.url);
    if existing_len > 0 {
        request_builder =
            request_builder.header(reqwest::header::RANGE, format!("bytes={existing_len}-"));
    }
    let response = request_builder.send().await.map_err(DownloadError::Http)?;

    let status = response.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && existing_len > 0 {
        // The leftover partial is no longer a valid prefix (typically it was already
        // complete when the previous attempt died). Drop it so the transient-error
        // retry restarts from zero instead of re-sending the same bad Range.
        let _ = fs::remove_file(tmp_path).await;
    }
    if !status.is_success() {
        return Err(DownloadError::Status {
            status: status.as_u16(),
            url: req.url.clone(),
        });
    }

    let resuming = existing_len > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;

    let total = if resuming {
        // "bytes <start>-<end>/<total>" - the part after '/' is the full
        // size regardless of how much of the range is actually being sent.
        content_range_total(&response)
            .or(req.known_total_bytes)
            .unwrap_or(0)
    } else {
        response
            .content_length()
            .or(req.known_total_bytes)
            .unwrap_or(0)
    };
    on_total(total);

    let mut hasher = Sha256::new();
    let mut downloaded: u64;

    // Chunks from the network are small (often 8-16 KiB); without buffering,
    // every `write_all` on a `tokio::fs::File` is a separate blocking-pool
    // hop. A 256 KiB buffer turns that into a handful of large writes.
    let mut file = if resuming {
        // Re-hash what's already on disk so the final SHA256 covers the
        // whole file, not just the newly-appended tail - the alternative
        // (skip verification on a resumed download) would make
        // `expected_sha256` silently stop being checked for anything that
        // was ever paused.
        let mut existing = fs::File::open(tmp_path)
            .await
            .map_err(|e| DownloadError::io(tmp_path, e))?;
        let mut buf = vec![0u8; WRITE_BUFFER_BYTES];
        loop {
            let n = existing
                .read(&mut buf)
                .await
                .map_err(|e| DownloadError::io(tmp_path, e))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        downloaded = existing_len;
        let appending = fs::OpenOptions::new()
            .append(true)
            .open(tmp_path)
            .await
            .map_err(|e| DownloadError::io(tmp_path, e))?;
        BufWriter::with_capacity(WRITE_BUFFER_BYTES, appending)
    } else {
        downloaded = 0;
        let created = fs::File::create(tmp_path)
            .await
            .map_err(|e| DownloadError::io(tmp_path, e))?;
        BufWriter::with_capacity(WRITE_BUFFER_BYTES, created)
    };

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            file.flush()
                .await
                .map_err(|e| DownloadError::io(tmp_path, e))?;
            return Err(DownloadError::Cancelled);
        }
        let chunk = chunk.map_err(DownloadError::Http)?;
        file.write_all(&chunk)
            .await
            .map_err(|e| DownloadError::io(tmp_path, e))?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if total > 0 {
            on_progress((downloaded as f64 / total as f64).min(1.0));
        }
    }
    file.flush()
        .await
        .map_err(|e| DownloadError::io(tmp_path, e))?;

    let actual = hex::encode(hasher.finalize());

    if has_real_hash(&req.expected_sha256) {
        let expected = req
            .expected_sha256
            .as_ref()
            .expect("checked by has_real_hash");
        if !expected.eq_ignore_ascii_case(&actual) {
            return Err(DownloadError::HashMismatch {
                url: req.url.clone(),
                expected: expected.clone(),
                actual,
            });
        }
    }

    on_progress(1.0);

    Ok(DownloadOutcome {
        sha256: actual,
        bytes: downloaded,
    })
}

/// Parses the total size out of a `Content-Range: bytes 1000-1999/2000`
/// response header. `None` for a missing/malformed header or a server that
/// doesn't know the total (`.../*`).
fn content_range_total(response: &reqwest::Response) -> Option<u64> {
    let value = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)?
        .to_str()
        .ok()?;
    let total_part = value.rsplit('/').next()?;
    total_part.parse::<u64>().ok()
}

async fn commit(tmp_path: &Path, destination: &Path) -> Result<()> {
    const COMMIT_RETRIES: u32 = 5;
    let mut attempt = 1u32;
    loop {
        match fs::rename(tmp_path, destination).await {
            Ok(()) => return Ok(()),
            Err(e) if attempt < COMMIT_RETRIES && is_destination_busy(&e) => {
                tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                attempt += 1;
            }
            Err(e) => return Err(DownloadError::io(destination, e)),
        }
    }
}

fn is_destination_busy(err: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    matches!(
        err.kind(),
        ErrorKind::PermissionDenied | ErrorKind::AlreadyExists
    ) || err
        .raw_os_error()
        .is_some_and(|code| code == 32 || code == 33)
}

async fn try_delete(path: &Path) {
    if fs::metadata(path).await.is_ok() {
        let _ = fs::remove_file(path).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_real_hash_rejects_placeholder_and_empty() {
        assert!(!has_real_hash(&None));
        assert!(!has_real_hash(&Some(String::new())));
        assert!(!has_real_hash(&Some("0000000000".into())));
        assert!(has_real_hash(&Some("deadbeef".into())));
    }

    #[test]
    fn tmp_sibling_stays_alongside_destination_with_suffix() {
        let dest = PathBuf::from("/downloads/client.jar");
        let tmp = tmp_sibling(&dest);
        assert_eq!(tmp.parent(), dest.parent());
        let name = tmp.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("client.jar."));
        assert!(name.ends_with(".tmp"));
    }

    #[test]
    fn tmp_sibling_is_deterministic_for_resume() {
        // Two calls for the same destination must agree on the path - a
        // random-per-call name (the old behavior) would make a paused
        // download's resume attempt look for a file that was never written.
        let dest = PathBuf::from("/downloads/client.jar");
        assert_eq!(tmp_sibling(&dest), tmp_sibling(&dest));
    }

    #[test]
    fn validate_rejects_empty_url() {
        let req = DownloadRequest::new("", "/tmp/x");
        assert!(req.validate().is_err());
    }

    #[test]
    fn backoff_grows_exponentially() {
        assert_eq!(backoff_for(1), Duration::from_millis(400));
        assert_eq!(backoff_for(2), Duration::from_millis(800));
        assert_eq!(backoff_for(3), Duration::from_millis(1600));
    }
}
