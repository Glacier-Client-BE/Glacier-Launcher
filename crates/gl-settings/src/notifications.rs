//! In-app notification feed — port of `Services/NotificationService.cs` and
//! `Models/AppNotification.cs`.
//!
//! Backs the top bar's bell badge, the notification panel's "Notifications"
//! section, and the toast container. None of that had a data source before
//! this module existed.
//!
//! # File format
//!
//! Stored at `%USERPROFILE%\Glacier Launcher\notifications.json` as a bare
//! JSON array with PascalCase keys — byte-compatible with what the C# app
//! wrote via `System.Text.Json` (which serializes property names as-is), so
//! an existing install's notification history survives the migration. That is
//! why this does *not* use `gl_persistence::save`, whose schema-versioned
//! `Envelope` wrapper would change the shape on disk; it uses the same
//! crate's atomic writer directly.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Cap on stored notifications; the oldest are dropped past this.
/// Mirrors the C# `MaxStored`.
const MAX_STORED: usize = 50;

/// One entry in the notification feed. Field names match the C#
/// `AppNotification` properties so the on-disk JSON is interchangeable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AppNotification {
    pub id: String,
    pub title: String,
    pub message: String,
    pub kind: String,
    pub icon: String,
    pub time_iso: String,
    pub read: bool,
    pub url: String,
}

impl AppNotification {
    /// The icon for a notification kind. Port of the C# `kind switch`.
    fn icon_for(kind: &str) -> &'static str {
        match kind {
            "success" => "fa-solid fa-circle-check",
            "error" => "fa-solid fa-circle-xmark",
            _ => "fa-solid fa-circle-info",
        }
    }

    pub fn new(title: &str, message: &str, kind: &str, url: &str) -> Self {
        Self {
            id: new_id(),
            title: title.to_string(),
            message: message.to_string(),
            kind: kind.to_string(),
            icon: Self::icon_for(kind).to_string(),
            time_iso: now_iso(),
            read: false,
            url: url.to_string(),
        }
    }
}

/// A 32-char lowercase hex id, matching C#'s `Guid.NewGuid().ToString("N")`.
///
/// Uniqueness here only has to hold within one launcher install, so this
/// derives from the system clock plus a per-process counter rather than
/// pulling in a UUID dependency for one call site.
fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:016x}{:016x}", nanos as u64, seq ^ (nanos >> 64) as u64)
}

/// UTC timestamp in .NET's round-trip ("o") shape, matching
/// `DateTime.UtcNow.ToString("o")` so entries sort against existing ones.
fn now_iso() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let ticks = dur.subsec_nanos() / 100;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ticks:07}Z")
}

pub fn notifications_path() -> PathBuf {
    crate::launcher_root().join("notifications.json")
}

/// The notification feed. Every mutation persists immediately, and — as in
/// the original — persistence failures are swallowed rather than surfaced:
/// losing a notification is never worth failing the operation that raised it.
#[derive(Debug, Default)]
pub struct NotificationService {
    items: Mutex<Vec<AppNotification>>,
}

impl NotificationService {
    /// Loads the stored feed, tolerating a missing or corrupt file.
    pub fn new() -> Self {
        let items = std::fs::read_to_string(notifications_path())
            .ok()
            .and_then(|json| serde_json::from_str::<Vec<AppNotification>>(&json).ok())
            .unwrap_or_default();
        Self {
            items: Mutex::new(items),
        }
    }

    fn with_items<T>(&self, f: impl FnOnce(&mut Vec<AppNotification>) -> T) -> T {
        // A poisoned lock must not take the UI's notification feed down with
        // it; recover the data and carry on.
        let mut guard = match self.items.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        f(&mut guard)
    }

    /// All notifications, newest first (C# orders by `TimeIso` descending).
    pub fn items(&self) -> Vec<AppNotification> {
        self.with_items(|items| {
            let mut out = items.clone();
            out.sort_by(|a, b| b.time_iso.cmp(&a.time_iso));
            out
        })
    }

    pub fn unread_count(&self) -> usize {
        self.with_items(|items| items.iter().filter(|n| !n.read).count())
    }

    /// Adds a notification at the front, trimming past [`MAX_STORED`].
    pub fn add(&self, title: &str, message: &str, kind: &str, url: &str) -> AppNotification {
        let entry = AppNotification::new(title, message, kind, url);
        self.with_items(|items| {
            items.insert(0, entry.clone());
            while items.len() > MAX_STORED {
                items.pop();
            }
            save(items);
        });
        entry
    }

    pub fn mark_read(&self, id: &str) {
        self.with_items(|items| {
            let Some(n) = items.iter_mut().find(|n| n.id == id) else {
                return;
            };
            if n.read {
                return;
            }
            n.read = true;
            save(items);
        });
    }

    pub fn mark_all_read(&self) {
        self.with_items(|items| {
            if items.iter().all(|n| n.read) {
                return;
            }
            for n in items.iter_mut() {
                n.read = true;
            }
            save(items);
        });
    }

    pub fn remove(&self, id: &str) {
        self.with_items(|items| {
            let before = items.len();
            items.retain(|n| n.id != id);
            if items.len() != before {
                save(items);
            }
        });
    }

    pub fn clear(&self) {
        self.with_items(|items| {
            if items.is_empty() {
                return;
            }
            items.clear();
            save(items);
        });
    }
}

/// Unit tests mutate detached services through the same methods, so saving is
/// a no-op under `cfg(test)`: otherwise every test run wrote its fixtures
/// (e.g. an unread "b" from 2026-01-02) into the player's real bell.
#[cfg(test)]
fn save(_items: &[AppNotification]) {}

#[cfg(not(test))]
fn save(items: &[AppNotification]) {
    let path = notifications_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(items) {
        let _ = gl_persistence::write_atomic(&path, &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a service without touching the real notifications file.
    fn detached(items: Vec<AppNotification>) -> NotificationService {
        NotificationService {
            items: Mutex::new(items),
        }
    }

    fn note(title: &str, time: &str, read: bool) -> AppNotification {
        AppNotification {
            id: format!("id-{title}"),
            title: title.into(),
            message: String::new(),
            kind: "info".into(),
            icon: "fa-solid fa-circle-info".into(),
            time_iso: time.into(),
            read,
            url: String::new(),
        }
    }

    #[test]
    fn icon_is_chosen_by_kind() {
        assert_eq!(
            AppNotification::icon_for("success"),
            "fa-solid fa-circle-check"
        );
        assert_eq!(
            AppNotification::icon_for("error"),
            "fa-solid fa-circle-xmark"
        );
        assert_eq!(AppNotification::icon_for("info"), "fa-solid fa-circle-info");
        // Unknown kinds fall through to info, as in the C# `_ =>` arm.
        assert_eq!(
            AppNotification::icon_for("whatever"),
            "fa-solid fa-circle-info"
        );
    }

    #[test]
    fn items_come_back_newest_first() {
        let svc = detached(vec![
            note("old", "2024-01-01T00:00:00.0000000Z", false),
            note("new", "2026-01-01T00:00:00.0000000Z", false),
        ]);
        let titles: Vec<String> = svc.items().into_iter().map(|n| n.title).collect();
        assert_eq!(titles, vec!["new", "old"]);
    }

    #[test]
    fn unread_count_ignores_read_entries() {
        let svc = detached(vec![
            note("a", "2026-01-01T00:00:00.0000000Z", false),
            note("b", "2026-01-02T00:00:00.0000000Z", true),
            note("c", "2026-01-03T00:00:00.0000000Z", false),
        ]);
        assert_eq!(svc.unread_count(), 2);
    }

    #[test]
    fn mark_all_read_clears_the_count() {
        let svc = detached(vec![
            note("a", "2026-01-01T00:00:00.0000000Z", false),
            note("b", "2026-01-02T00:00:00.0000000Z", false),
        ]);
        svc.mark_all_read();
        assert_eq!(svc.unread_count(), 0);
    }

    #[test]
    fn remove_deletes_only_the_named_entry() {
        let svc = detached(vec![
            note("a", "2026-01-01T00:00:00.0000000Z", false),
            note("b", "2026-01-02T00:00:00.0000000Z", false),
        ]);
        svc.remove("id-a");
        let titles: Vec<String> = svc.items().into_iter().map(|n| n.title).collect();
        assert_eq!(titles, vec!["b"]);
    }

    #[test]
    fn ids_are_unique_and_guid_n_shaped() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn timestamps_match_the_dotnet_round_trip_shape() {
        let s = now_iso();
        assert_eq!(
            s.len(),
            28,
            "expected yyyy-MM-ddTHH:mm:ss.fffffffZ, got {s}"
        );
        assert!(s.ends_with('Z'));
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[10..11], "T");
        assert_eq!(&s[19..20], ".");
    }

    /// The on-disk shape must stay PascalCase to remain readable by (and
    /// interchangeable with) what the C# app wrote.
    #[test]
    fn serializes_with_pascal_case_keys() {
        let json =
            serde_json::to_string(&note("t", "2026-01-01T00:00:00.0000000Z", false)).unwrap();
        for key in [
            "\"Id\"",
            "\"Title\"",
            "\"Message\"",
            "\"Kind\"",
            "\"Icon\"",
            "\"TimeIso\"",
            "\"Read\"",
            "\"Url\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
    }
}
