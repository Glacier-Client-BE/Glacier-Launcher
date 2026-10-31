//! Bounded-concurrency download queue with live status tracking.
//!
//! Ports the `DownloadEntry` / `DownloadService.ActiveDownloads` concept from
//! `DownloadService.cs`: every submitted download gets a tracked entry (status,
//! progress, byte counts, timestamps) that observers can poll or subscribe to,
//! while a semaphore caps how many downloads run at once (the C# version relied on
//! the `HttpClient` connection pool cap; here we make the limit explicit).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{broadcast, watch, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::client::DownloadClient;
use crate::download::{download_file, DownloadOutcome, DownloadRequest};
use crate::error::Result;
use crate::progress::ThrottledProgress;

/// Lifecycle state of a queued/running download, mirroring `DownloadStatus` in
/// `DownloadService.cs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Completed,
    Failed,
    Cancelled,
    /// QOL pause/resume: stopped mid-transfer with its partial `.tmp` kept
    /// on disk (unlike `Cancelled`, which deletes it) so `resume` can
    /// continue via a Range request instead of restarting from zero.
    Paused,
}

/// Snapshot of a tracked download's state, mirroring `DownloadEntry` in the C#
/// source. Cheap to clone; produced on every state change.
#[derive(Debug, Clone)]
pub struct DownloadEntry {
    pub id: Uuid,
    pub label: String,
    pub status: DownloadStatus,
    pub progress: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
    /// Unix-ms timestamp of the last time `status` left `Queued`/`Downloading`
    /// (i.e. became `Completed`/`Failed`/`Cancelled`). `None` while still in
    /// flight. Backs QOL "auto-clear finished downloads after N days" -
    /// without a finish time there was nothing to age a finished entry
    /// against.
    pub finished_at_unix_ms: Option<i64>,
}

impl DownloadEntry {
    fn new(id: Uuid, label: String, total_bytes: u64) -> Self {
        Self {
            id,
            label,
            status: DownloadStatus::Queued,
            progress: 0.0,
            downloaded_bytes: 0,
            total_bytes,
            error: None,
            finished_at_unix_ms: None,
        }
    }
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A handle to a single queued download: lets the caller observe status updates
/// and cancel the download, and await its final outcome.
pub struct DownloadHandle {
    id: Uuid,
    watch: watch::Receiver<DownloadEntry>,
    cancel: CancellationToken,
    task: tokio::task::JoinHandle<Result<DownloadOutcome>>,
}

impl DownloadHandle {
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Current snapshot of this download's tracked entry.
    pub fn entry(&self) -> DownloadEntry {
        self.watch.borrow().clone()
    }

    /// A receiver that resolves on every status/progress change, for callers that
    /// want to subscribe rather than poll (analogous to `DownloadService.Changed`).
    pub fn watch(&self) -> watch::Receiver<DownloadEntry> {
        self.watch.clone()
    }

    /// Requests cancellation of this download; it will settle to `Cancelled`.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Waits for the download to finish and returns its outcome.
    pub async fn join(self) -> Result<DownloadOutcome> {
        match self.task.await {
            Ok(result) => result,
            Err(join_err) => {
                // The spawned task panicked or was aborted; surface a cancellation
                // rather than propagating a JoinError, since neither case leaves a
                // sensible download outcome.
                if join_err.is_cancelled() {
                    Err(crate::error::DownloadError::Cancelled)
                } else {
                    std::panic::resume_unwind(join_err.into_panic());
                }
            }
        }
    }
}

/// Bounded-concurrency queue for running many downloads without saturating the
/// network or disk. Analogous to `DownloadService`, but with an explicit
/// concurrency cap instead of relying on `HttpClient`'s connection pool alone.
#[derive(Clone)]
pub struct DownloadQueue {
    client: DownloadClient,
    permits: Arc<Semaphore>,
    progress_min_interval: Duration,
    /// Every submitted download's watch receiver + cancel token, kept around so
    /// UI layers (e.g. a Downloads panel) can list/cancel/clear in-flight and
    /// recently-finished downloads without each call site having to thread its
    /// own tracking through. Mirrors `DownloadService.ActiveDownloads` in the
    /// C# original, which was a single process-wide observable list.
    registry: Arc<Mutex<Vec<RegistryEntry>>>,
    /// Port of `DownloadService.Changed` (`DownloadService.cs:48`), the
    /// process-wide notification the C# Downloads panel re-rendered on. Kept as
    /// a broadcast so the Tauri layer can bridge it to a single frontend event
    /// instead of every panel polling `active_entries` on a timer.
    changed: broadcast::Sender<()>,
}

struct RegistryEntry {
    id: Uuid,
    watch: watch::Receiver<DownloadEntry>,
    cancel: CancellationToken,
    /// The request that started this entry, kept around so a failed
    /// download can be resubmitted (QOL: "Retry all failed") without the
    /// caller having to remember and rebuild the original URL/destination.
    req: DownloadRequest,
    /// Set just before `cancel`ling to request a pause rather than a real
    /// cancel - read by the spawned task once it observes the
    /// cancellation, to decide "keep the `.tmp` for `resume`" vs "delete
    /// it, this is a real cancel" and "settle to `Paused`" vs `Cancelled`.
    pause_requested: Arc<AtomicBool>,
}

impl DownloadQueue {
    /// Creates a queue that runs at most `max_concurrent` downloads at a time.
    pub fn new(client: DownloadClient, max_concurrent: usize) -> Self {
        Self {
            client,
            permits: Arc::new(Semaphore::new(max_concurrent.max(1))),
            progress_min_interval: Duration::from_millis(100),
            registry: Arc::new(Mutex::new(Vec::new())),
            changed: broadcast::channel(16).0,
        }
    }

    /// Subscribes to change notifications. Port of subscribing to
    /// `DownloadService.Changed`; a lagged receiver is not an error here since
    /// every notification carries no payload and observers re-read
    /// [`active_entries`](Self::active_entries) anyway.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }

    /// Port of `DownloadService.RaiseChanged` (`DownloadService.cs:53`). Fails
    /// silently when nobody is subscribed, exactly as a null C# event does.
    fn raise_changed(&self) {
        let _ = self.changed.send(());
    }

    /// Snapshot of every tracked download (queued, running, or finished since
    /// the last `clear_finished` call), newest-submitted last.
    pub fn active_entries(&self) -> Vec<DownloadEntry> {
        let reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
        reg.iter().map(|r| r.watch.borrow().clone()).collect()
    }

    /// Cancels a tracked download by id, if it's still running. Unlike
    /// [`pause`](Self::pause), this deletes the partial `.tmp` - a real
    /// cancel means "I don't want this file," not "not right now."
    pub fn cancel(&self, id: Uuid) {
        let reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = reg.iter().find(|r| r.id == id) {
            r.pause_requested.store(false, Ordering::SeqCst);
            r.cancel.cancel();
        }
        drop(reg);
        self.raise_changed();
    }

    /// QOL pause/resume: stops a running download but keeps its partial
    /// `.tmp` on disk so [`resume`](Self::resume) can continue it via a
    /// Range request instead of restarting from zero. A no-op for an entry
    /// that isn't currently `Downloading`.
    pub fn pause(&self, id: Uuid) {
        let reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = reg.iter().find(|r| r.id == id) {
            if matches!(r.watch.borrow().status, DownloadStatus::Downloading) {
                r.pause_requested.store(true, Ordering::SeqCst);
                r.cancel.cancel();
            }
        }
        drop(reg);
        self.raise_changed();
    }

    /// String-id convenience wrapper around [`pause`](Self::pause).
    pub fn pause_str(&self, id: &str) -> std::result::Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.pause(uuid);
        Ok(())
    }

    /// Resumes a `Paused` entry: resubmits its original request, which
    /// picks the same deterministic `.tmp` path back up and continues it
    /// via Range request (see `download.rs::download_once`) rather than
    /// starting over. A no-op (returns `false`) for anything not currently
    /// `Paused`.
    pub fn resume(&self, id: Uuid) -> bool {
        let req = {
            let reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.iter()
                .find(|r| r.id == id && matches!(r.watch.borrow().status, DownloadStatus::Paused))
                .map(|r| r.req.clone())
        };
        let Some(req) = req else { return false };
        self.remove(id);
        self.submit(req);
        true
    }

    /// String-id convenience wrapper around [`resume`](Self::resume).
    pub fn resume_str(&self, id: &str) -> std::result::Result<bool, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        Ok(self.resume(uuid))
    }

    /// String-id convenience wrapper around [`cancel`](Self::cancel), so
    /// callers that only have `uuid` transitively (e.g. `gl-download`'s
    /// consumers that don't depend on the `uuid` crate directly) can still
    /// cancel/remove by the id string handed back in [`DownloadEntry`].
    pub fn cancel_str(&self, id: &str) -> std::result::Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.cancel(uuid);
        Ok(())
    }

    /// Drops every tracked entry that isn't currently downloading. Mirrors
    /// `ClearFinishedDownloads` (`Home.Downloads.cs`), which removes every
    /// entry whose status `!= Downloading` - `Queued` included. Keeping queued
    /// entries here would leave the button unable to clear a row the original
    /// clears, and would disagree with `hasFinished` in the panel, which uses
    /// the same `!= Downloading` test to decide whether to show the button.
    pub fn clear_finished(&self) {
        let mut reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
        // `Paused` is deliberately not "finished" here alongside
        // Downloading/Queued - clearing it would drop the tracked `req`
        // `resume` needs, orphaning the `.tmp` it left on disk with no way
        // to continue it short of removing the file by hand.
        reg.retain(|r| {
            matches!(
                r.watch.borrow().status,
                DownloadStatus::Downloading | DownloadStatus::Paused
            )
        });
        drop(reg);
        self.raise_changed();
    }

    /// Drops every finished (non-Queued/Downloading) entry whose
    /// `finished_at_unix_ms` is older than `max_age_ms`. QOL "auto-clear
    /// finished downloads after N days" - a periodic caller (see
    /// `retry_failed_downloads`'s sibling command) sweeps with this instead
    /// of the panel accumulating every finished download forever until the
    /// user notices and hits "Clear finished" manually.
    pub fn clear_finished_older_than(&self, max_age_ms: i64) {
        let cutoff = now_unix_ms() - max_age_ms;
        let mut reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
        reg.retain(|r| {
            let entry = r.watch.borrow();
            if matches!(
                entry.status,
                DownloadStatus::Downloading | DownloadStatus::Queued
            ) {
                return true;
            }
            match entry.finished_at_unix_ms {
                Some(t) => t > cutoff,
                None => true,
            }
        });
        drop(reg);
        self.raise_changed();
    }

    /// Drops a single tracked entry by id, regardless of status.
    pub fn remove(&self, id: Uuid) {
        let mut reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
        reg.retain(|r| r.id != id);
        drop(reg);
        self.raise_changed();
    }

    /// String-id convenience wrapper around [`remove`](Self::remove).
    pub fn remove_str(&self, id: &str) -> std::result::Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.remove(uuid);
        Ok(())
    }

    /// Resubmits a single tracked entry using its original request,
    /// regardless of its current status. Returns `false` if `id` isn't
    /// tracked (already removed/cleared).
    pub fn retry(&self, id: Uuid) -> bool {
        let req = {
            let reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.iter().find(|r| r.id == id).map(|r| r.req.clone())
        };
        let Some(req) = req else { return false };
        self.remove(id);
        self.submit(req);
        true
    }

    /// String-id convenience wrapper around [`retry`](Self::retry).
    pub fn retry_str(&self, id: &str) -> std::result::Result<bool, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        Ok(self.retry(uuid))
    }

    /// How many download slots are currently free.
    pub fn available_permits(&self) -> usize {
        self.permits.available_permits()
    }

    /// Resubmits every tracked entry currently `Failed`, using each one's
    /// original request (URL, destination, label). QOL "Retry all failed":
    /// previously the only recovery from a failed download was to remove it
    /// and go re-trigger whatever action originally queued it, which for
    /// something like a batch mod install could mean re-finding every failed
    /// item's source individually. Returns the number of retries submitted.
    pub fn retry_failed(&self) -> usize {
        let failed: Vec<(Uuid, DownloadRequest)> = {
            let reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.iter()
                .filter(|r| matches!(r.watch.borrow().status, DownloadStatus::Failed))
                .map(|r| (r.id, r.req.clone()))
                .collect()
        };
        let count = failed.len();
        for (old_id, req) in failed {
            self.remove(old_id);
            self.submit(req);
        }
        count
    }

    /// Submits a download to the queue and returns immediately with a handle;
    /// the download starts once a concurrency slot is free.
    pub fn submit(&self, req: DownloadRequest) -> DownloadHandle {
        let id = Uuid::new_v4();
        let label = req.label.clone().unwrap_or_else(|| req.url.clone());
        let total = req.known_total_bytes.unwrap_or(0);
        let req_for_registry = req.clone();
        let (tx, rx) = watch::channel(DownloadEntry::new(id, label, total));
        let cancel = CancellationToken::new();
        let pause_requested = Arc::new(AtomicBool::new(false));

        let client = self.client.clone();
        let permits = self.permits.clone();
        let min_interval = self.progress_min_interval;
        let task_cancel = cancel.clone();
        let task_pause_requested = pause_requested.clone();
        let changed = self.changed.clone();
        let destination_for_cleanup = req.destination.clone();

        let task = tokio::spawn(async move {
            // Cancelling a download still waiting for a slot must settle it now, not
            // only once a slot frees up. A closed semaphore (queue dropped mid-flight)
            // also lands here and is treated as cancelled.
            let permit = tokio::select! {
                permit = permits.acquire_owned() => permit.ok(),
                _ = task_cancel.cancelled() => None,
            };
            let Some(_permit) = permit else {
                tx.send_modify(|e| {
                    e.status = DownloadStatus::Cancelled;
                    e.finished_at_unix_ms = Some(now_unix_ms());
                });
                let _ =
                    tokio::fs::remove_file(crate::download::tmp_sibling(&destination_for_cleanup))
                        .await;
                let _ = changed.send(());
                return Err(crate::error::DownloadError::Cancelled);
            };

            tx.send_modify(|e| e.status = DownloadStatus::Downloading);
            let _ = changed.send(());

            let progress_changed = changed.clone();
            let progress_tx = tx.clone();
            let mut throttled = ThrottledProgress::with_options(
                move |value: f64| {
                    progress_tx.send_modify(|e| {
                        e.progress = value;
                        // Read `total_bytes` off the entry rather than a value
                        // captured at submit time: it may only have been
                        // learned from the response's Content-Length after
                        // this closure was built.
                        if e.total_bytes > 0 {
                            e.downloaded_bytes = ((e.total_bytes as f64) * value) as u64;
                        }
                    });
                    // Already rate-limited to `min_interval` by ThrottledProgress,
                    // so this cannot flood the frontend.
                    let _ = progress_changed.send(());
                },
                min_interval,
                1.0,
                0.01,
            );

            let total_tx = tx.clone();
            let total_changed = changed.clone();

            let result = download_file(
                &client,
                &req,
                |value| {
                    throttled.report(value);
                },
                |learned| {
                    // Fires once per attempt, as soon as Content-Length is
                    // known, so a download submitted without
                    // `with_known_total_bytes` still shows real byte counts.
                    if learned > 0 {
                        total_tx.send_modify(|e| e.total_bytes = learned);
                        let _ = total_changed.send(());
                    }
                },
                &task_cancel,
            )
            .await;

            match &result {
                Ok(outcome) => tx.send_modify(|e| {
                    e.status = DownloadStatus::Completed;
                    e.progress = 1.0;
                    e.downloaded_bytes = outcome.bytes;
                    if e.total_bytes == 0 {
                        e.total_bytes = outcome.bytes;
                    }
                    e.finished_at_unix_ms = Some(now_unix_ms());
                }),
                Err(err) => {
                    let cancelled = matches!(err, crate::error::DownloadError::Cancelled);
                    let paused = cancelled && task_pause_requested.load(Ordering::SeqCst);
                    if cancelled && !paused {
                        // A real cancel (not a pause): `download_file` never
                        // deletes the `.tmp` on `Cancelled` any more (pause
                        // needs it kept), so that cleanup belongs here now.
                        let _ = tokio::fs::remove_file(crate::download::tmp_sibling(
                            &destination_for_cleanup,
                        ))
                        .await;
                    }
                    tx.send_modify(|e| {
                        e.status = if paused {
                            DownloadStatus::Paused
                        } else if cancelled {
                            DownloadStatus::Cancelled
                        } else {
                            DownloadStatus::Failed
                        };
                        if !paused {
                            e.error = Some(err.to_string());
                            e.finished_at_unix_ms = Some(now_unix_ms());
                        }
                    });
                }
            }
            let _ = changed.send(());

            result
        });

        {
            let mut reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.push(RegistryEntry {
                id,
                watch: rx.clone(),
                cancel: cancel.clone(),
                req: req_for_registry,
                pause_requested,
            });
        }
        self.raise_changed();

        DownloadHandle {
            id,
            watch: rx,
            cancel,
            task,
        }
    }

    /// Submits every request and waits for all of them to finish, returning the
    /// per-request results in the same order as `reqs`. Unlike a plain
    /// `join_all`, submission is queue-ordered so the first `max_concurrent`
    /// requests start immediately and the rest wait for a free slot (FIFO).
    pub async fn download_all(&self, reqs: Vec<DownloadRequest>) -> Vec<Result<DownloadOutcome>> {
        let handles: VecDeque<DownloadHandle> =
            reqs.into_iter().map(|req| self.submit(req)).collect();
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            results.push(handle.join().await);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_entry_starts_queued_with_zero_progress() {
        let entry = DownloadEntry::new(Uuid::nil(), "client.jar".into(), 100);
        assert_eq!(entry.status, DownloadStatus::Queued);
        assert_eq!(entry.progress, 0.0);
        assert_eq!(entry.total_bytes, 100);
        assert_eq!(entry.downloaded_bytes, 0);
    }

    #[tokio::test]
    async fn queue_caps_concurrency_at_configured_limit() {
        let queue = DownloadQueue::new(DownloadClient::from_client(reqwest::Client::new()), 2);
        assert_eq!(queue.available_permits(), 2);

        // Hold both permits manually to simulate two in-flight downloads.
        let p1 = queue.permits.clone().acquire_owned().await.unwrap();
        let p2 = queue.permits.clone().acquire_owned().await.unwrap();
        assert_eq!(queue.available_permits(), 0);

        drop(p1);
        assert_eq!(queue.available_permits(), 1);
        drop(p2);
        assert_eq!(queue.available_permits(), 2);
    }

    /// `ClearFinishedDownloads` (`Home.Downloads.cs`) removes every entry whose
    /// status `!= Downloading`, which includes `Queued`. Holding the only
    /// permit keeps a submitted download parked in `Queued` so we can assert
    /// it gets cleared rather than kept.
    #[tokio::test]
    async fn clear_finished_also_drops_queued_entries() {
        let queue = DownloadQueue::new(DownloadClient::from_client(reqwest::Client::new()), 1);
        let hold = queue.permits.clone().acquire_owned().await.unwrap();

        let dir = tempfile::tempdir().unwrap();
        let req = DownloadRequest::new("https://example.invalid/a.bin", dir.path().join("a.bin"));
        let _handle = queue.submit(req);

        let entries = queue.active_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, DownloadStatus::Queued);

        queue.clear_finished();
        assert!(
            queue.active_entries().is_empty(),
            "a Queued entry must be cleared, matching the C# `!= Downloading` test"
        );
        drop(hold);
    }

    /// Port of `DownloadService.Changed`: submitting must notify subscribers so
    /// the Tauri bridge can push a fresh snapshot to the frontend.
    #[tokio::test]
    async fn submit_notifies_subscribers() {
        let queue = DownloadQueue::new(DownloadClient::from_client(reqwest::Client::new()), 1);
        let mut rx = queue.subscribe();
        let hold = queue.permits.clone().acquire_owned().await.unwrap();

        let dir = tempfile::tempdir().unwrap();
        let req = DownloadRequest::new("https://example.invalid/b.bin", dir.path().join("b.bin"));
        let _handle = queue.submit(req);

        assert!(rx.try_recv().is_ok(), "submit must raise Changed");
        drop(hold);
    }

    /// Removing and clearing are both user-visible mutations, so they raise
    /// `Changed` too - otherwise the panel would keep showing a deleted row.
    #[tokio::test]
    async fn remove_and_clear_notify_subscribers() {
        let queue = DownloadQueue::new(DownloadClient::from_client(reqwest::Client::new()), 1);
        let hold = queue.permits.clone().acquire_owned().await.unwrap();

        let dir = tempfile::tempdir().unwrap();
        let req = DownloadRequest::new("https://example.invalid/c.bin", dir.path().join("c.bin"));
        let handle = queue.submit(req);
        let id = handle.id();

        let mut rx = queue.subscribe();
        queue.remove(id);
        assert!(rx.try_recv().is_ok(), "remove must raise Changed");

        queue.clear_finished();
        assert!(rx.try_recv().is_ok(), "clear_finished must raise Changed");
        drop(hold);
    }

    #[tokio::test]
    async fn submitting_nonexistent_url_reports_failed_status() {
        let queue = DownloadQueue::new(DownloadClient::from_client(reqwest::Client::new()), 1);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let req = DownloadRequest::new("not a valid url", &dest).with_max_attempts(1);

        let handle = queue.submit(req);
        let result = handle.join().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resume_is_a_no_op_for_an_entry_that_isnt_paused() {
        let queue = DownloadQueue::new(DownloadClient::from_client(reqwest::Client::new()), 1);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let req = DownloadRequest::new("not a valid url", &dest).with_max_attempts(1);

        let handle = queue.submit(req);
        let id = handle.id();
        let _ = handle.join().await; // settles to Failed, not Paused
        assert!(!queue.resume(id));
    }

    #[tokio::test]
    async fn pausing_a_download_that_finished_before_the_pause_request_is_a_no_op() {
        let queue = DownloadQueue::new(DownloadClient::from_client(reqwest::Client::new()), 1);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let req = DownloadRequest::new("not a valid url", &dest).with_max_attempts(1);

        let handle = queue.submit(req);
        let id = handle.id();
        let _ = handle.join().await;
        // Already Failed by the time pause() looks at it - must not
        // clobber that into a bogus Paused status.
        queue.pause(id);
        let entries = queue.active_entries();
        let entry = entries.iter().find(|e| e.id == id).unwrap();
        assert_eq!(entry.status, DownloadStatus::Failed);
    }
}
