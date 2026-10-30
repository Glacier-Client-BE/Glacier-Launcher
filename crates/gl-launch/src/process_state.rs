//! Registry of currently-running Java child processes, keyed by instance id
//! (or a synthetic key for bare-version launches — see
//! `commands::launch::VERSION_LAUNCH_KEY_PREFIX` in the Tauri layer).
//!
//! This does **not** open a second stdout/stderr pipe. `gl_java::launcher::spawn`
//! already pipes stdout/stderr, and `commands::launch::stream_child_output`
//! already owns the `Child` and reads those pipes line-by-line to feed the
//! live console window. This registry is fed by that *same* read loop
//! (`push_log_line`) — it just remembers "who is running" and keeps a
//! bounded tail of their output for the footer badge / crash modal / log
//! panel to query without re-reading a file from disk.
//!
//! Kill goes through the same PID-based `taskkill /T /F` (Windows) /
//! `kill -9` (other) approach `commands::launch::cancel_launch` already
//! uses, rather than the registry owning the `tokio::process::Child` itself
//! — the child is already owned by the streaming task, and Rust won't let
//! two owners hold it.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;

/// Ring-buffer cap per instance. This targets a Celeron N5100 laptop — see
/// project robustness/perf notes — so unbounded log accumulation across a
/// long play session is exactly the kind of slow leak that hurts there.
pub const LOG_BUFFER_CAP: usize = 5000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunningProcess {
    pub instance_id: String,
    pub pid: u32,
    /// Unix seconds, matching the convention `commands::logs::LogFile::modified_at`
    /// already uses elsewhere in this codebase.
    pub started_at: u64,
}

struct ProcessEntry {
    pid: u32,
    started_at: u64,
    log_buffer: Arc<Mutex<VecDeque<String>>>,
}

/// Shared, cloneable handle to the process table. Cheap to clone (an `Arc`
/// underneath), so it can live on `AppState` directly like `gl_download::DownloadQueue`
/// does.
#[derive(Clone)]
pub struct ProcessRegistry {
    inner: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    /// Bridging channel: fires (no payload — subscribers re-read `list()`)
    /// whenever the table changes. Mirrors `gl_download::DownloadQueue`'s own
    /// `Changed` broadcast, which `commands::downloads::spawn_downloads_bridge`
    /// already bridges onto a Tauri event the same way this crate's caller
    /// should for `java-process-changed`.
    changed: broadcast::Sender<()>,
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessRegistry {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(16);
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            changed: tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }

    fn notify(&self) {
        // No receivers is fine (e.g. in tests) — `send` only errors when the
        // channel has zero subscribers, which isn't a failure here.
        let _ = self.changed.send(());
    }

    /// Registers a newly-launched process. Returns the shared ring buffer
    /// the launch-streaming loop should call `push_log_line` against (or the
    /// caller can just call `push_log_line` on `self` directly, keyed by
    /// `instance_id` — this handle just avoids a second map lookup per line).
    pub fn register(&self, instance_id: impl Into<String>, pid: u32) -> Arc<Mutex<VecDeque<String>>> {
        let instance_id = instance_id.into();
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let log_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(64)));
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(
                instance_id,
                ProcessEntry {
                    pid,
                    started_at,
                    log_buffer: log_buffer.clone(),
                },
            );
        }
        self.notify();
        log_buffer
    }

    /// Appends one line to `instance_id`'s ring buffer, dropping the oldest
    /// line once `LOG_BUFFER_CAP` is reached. No-op if the instance isn't
    /// currently registered (e.g. it already exited and was removed).
    pub fn push_log_line(&self, instance_id: &str, line: &str) {
        if let Ok(guard) = self.inner.lock() {
            if let Some(entry) = guard.get(instance_id) {
                if let Ok(mut buf) = entry.log_buffer.lock() {
                    if buf.len() >= LOG_BUFFER_CAP {
                        buf.pop_front();
                    }
                    buf.push_back(line.to_string());
                }
            }
        }
    }

    /// Every currently-tracked running process.
    pub fn list(&self) -> Vec<RunningProcess> {
        self.inner
            .lock()
            .map(|guard| {
                guard
                    .iter()
                    .map(|(id, e)| RunningProcess {
                        instance_id: id.clone(),
                        pid: e.pid,
                        started_at: e.started_at,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A snapshot of `instance_id`'s buffered log lines, oldest first. Empty
    /// if the instance isn't tracked (already exited, or never launched).
    pub fn get_log_buffer(&self, instance_id: &str) -> Vec<String> {
        let Some(buf) = self
            .inner
            .lock()
            .ok()
            .and_then(|guard| guard.get(instance_id).map(|e| e.log_buffer.clone()))
        else {
            return Vec::new();
        };
        buf.lock()
            .map(|b| b.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Force-kills `instance_id`'s process tree and removes it from the
    /// table. Errors (rather than silently no-op'ing) if nothing is tracked
    /// under that id, so the UI can tell "kill failed" from "already gone".
    ///
    /// Guards against PID recycling: `pid` is only actually killed if the
    /// OS's process-creation timestamp for it still roughly matches when we
    /// registered it. Between reading `pid` here and the `taskkill` below
    /// actually running, the real process could have already exited and the
    /// OS handed that PID to an unrelated process — without this check,
    /// `taskkill /PID <pid> /T /F` would happily kill whatever now holds it.
    pub fn kill(&self, instance_id: &str) -> Result<(), String> {
        let (pid, started_at) = self
            .inner
            .lock()
            .map_err(|_| "process registry lock poisoned".to_string())?
            .get(instance_id)
            .map(|e| (e.pid, e.started_at))
            .ok_or_else(|| format!("No running process tracked for instance '{instance_id}'."))?;

        // A few seconds' tolerance: `started_at` was captured via
        // `SystemTime::now()` right after spawn returned, which lags the
        // OS's own process-creation timestamp by a small, variable amount
        // (scheduling delay) — not an exact match, but recycled PIDs from an
        // unrelated, much-earlier-or-later process are still reliably
        // rejected by this window. `gl_injection` (the crate this check
        // needs) is Windows-only, so this verification only runs there —
        // elsewhere this falls back to the old trust-the-pid behavior.
        #[cfg(target_os = "windows")]
        {
            match gl_injection::process_start_time_unix(pid) {
                Some(actual_start) if actual_start.abs_diff(started_at) <= 5 => {
                    kill_process_tree(pid);
                }
                Some(_) | None => {
                    // Either a different process now owns this PID, or it's
                    // already exited and unqueryable — nothing to kill
                    // either way, so just fall through to removing the
                    // stale entry.
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = started_at; // only used for the Windows-only verification above
            kill_process_tree(pid);
        }

        self.mark_exited(instance_id);
        Ok(())
    }

    /// Removes `instance_id` from the running set. Call this once the
    /// streaming task's `child.wait()` resolves (or after a manual `kill`).
    /// The log buffer goes with it — read it (`get_log_buffer`) before
    /// calling this if a caller (e.g. the crash-analysis flow) still needs
    /// the tail after exit.
    pub fn mark_exited(&self, instance_id: &str) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.remove(instance_id);
        }
        self.notify();
    }

    pub fn pid_of(&self, instance_id: &str) -> Option<u32> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.get(instance_id).map(|e| e.pid))
    }
}

#[cfg(target_os = "windows")]
fn kill_process_tree(pid: u32) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .status();
}

#[cfg(not(target_os = "windows"))]
fn kill_process_tree(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_push_and_read_roundtrip() {
        let reg = ProcessRegistry::new();
        reg.register("inst-1", 42);
        reg.push_log_line("inst-1", "hello");
        reg.push_log_line("inst-1", "world");

        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].instance_id, "inst-1");
        assert_eq!(list[0].pid, 42);
        assert_eq!(reg.get_log_buffer("inst-1"), vec!["hello", "world"]);
    }

    #[test]
    fn push_log_line_is_noop_for_unknown_instance() {
        let reg = ProcessRegistry::new();
        reg.push_log_line("ghost", "should not panic");
        assert!(reg.get_log_buffer("ghost").is_empty());
    }

    #[test]
    fn mark_exited_removes_from_list_and_drops_buffer() {
        let reg = ProcessRegistry::new();
        reg.register("inst-1", 1);
        reg.push_log_line("inst-1", "line");
        reg.mark_exited("inst-1");
        assert!(reg.list().is_empty());
        assert!(reg.get_log_buffer("inst-1").is_empty());
    }

    #[test]
    fn ring_buffer_caps_at_capacity() {
        let reg = ProcessRegistry::new();
        reg.register("inst-2", 1);
        for i in 0..(LOG_BUFFER_CAP + 10) {
            reg.push_log_line("inst-2", &i.to_string());
        }
        let buf = reg.get_log_buffer("inst-2");
        assert_eq!(buf.len(), LOG_BUFFER_CAP);
        // The first 10 lines ("0".."9") should have been evicted.
        assert_eq!(buf.first().unwrap(), "10");
        assert_eq!(buf.last().unwrap(), (LOG_BUFFER_CAP + 9).to_string().as_str());
    }

    #[test]
    fn kill_errors_for_untracked_instance() {
        let reg = ProcessRegistry::new();
        assert!(reg.kill("nope").is_err());
    }

    #[test]
    fn pid_of_reflects_registration_and_removal() {
        let reg = ProcessRegistry::new();
        assert_eq!(reg.pid_of("inst-3"), None);
        reg.register("inst-3", 7);
        assert_eq!(reg.pid_of("inst-3"), Some(7));
        reg.mark_exited("inst-3");
        assert_eq!(reg.pid_of("inst-3"), None);
    }
}
