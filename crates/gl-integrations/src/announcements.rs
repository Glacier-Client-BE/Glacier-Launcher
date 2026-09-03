//! Announcement banner fetcher, ported from `Services/AnnouncementService.cs`.

use crate::models::Announcement;

const ANNOUNCEMENT_URL: &str = "https://glacierclient.xyz/announcement.json";

/// Fetches the current announcement banner. Returns `None` on any network
/// or parse failure, or if the response has a blank id — mirroring the C#
/// service, which swallows all errors and treats them as "no announcement".
///
/// Deviation: the C# version also writes an on-disk cache
/// (`LauncherUtilityService.CacheDir/announcement.json`) via `JsonStore`.
/// That local caching/atomic-write concern belongs to `gl-persistence`
/// (`JsonStore` port), not this HTTP-fetch crate, so callers that want the
/// cache-then-fetch behavior should pair this with `gl-persistence`.
pub async fn get_announcement(client: &reqwest::Client) -> Option<Announcement> {
    let resp = client.get(ANNOUNCEMENT_URL).send().await.ok()?;
    let resp = resp.error_for_status().ok()?;
    let announcement: Announcement = resp.json().await.ok()?;
    if announcement.id.trim().is_empty() {
        return None;
    }
    Some(announcement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixture_response() {
        let body = r#"{
            "id": "abc123",
            "title": "Server maintenance",
            "message": "We'll be down for a bit.",
            "kind": "warning",
            "url": "https://glacierclient.xyz"
        }"#;
        let announcement: Announcement = serde_json::from_str(body).expect("valid json");
        assert_eq!(announcement.id, "abc123");
        assert_eq!(announcement.title, "Server maintenance");
        assert_eq!(announcement.kind, "warning");
    }

    #[test]
    fn rejects_empty_id_semantics() {
        // Mirrors the C# `!string.IsNullOrWhiteSpace(a.Id)` guard: an
        // announcement with a blank id should be treated as "none" by callers.
        let body = r#"{"id":"  ","title":"","message":"","kind":"info","url":""}"#;
        let announcement: Announcement = serde_json::from_str(body).expect("valid json");
        assert!(announcement.id.trim().is_empty());
    }
}
