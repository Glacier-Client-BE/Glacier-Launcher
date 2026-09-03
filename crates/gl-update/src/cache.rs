//! In-memory port of `GitHubApiCache.cs`.
//!
//! The original C# implementation persisted responses to disk (keyed by a
//! SHA-256 hash of the URL) so cached data survived process restarts and
//! could serve stale-but-usable results when GitHub's rate limit was hit.
//! Per the task instructions we port this as a simple in-memory cache with
//! TTL, keyed directly by the request URL (no persistence, no hashing —
//! there's no risk of collisions or filesystem-unsafe characters in an
//! in-memory `HashMap` key, so hashing the URL bought nothing at this
//! layer).
//!
//! Behavior mirrored from the C# version:
//! - A cached entry younger than [`FRESH_TTL`] is returned as-is (cache hit,
//!   no network call).
//! - On a cache miss / stale entry, the caller is expected to fetch fresh
//!   data and call [`GitHubApiCache::store`] to update the cache.
//! - If a fetch fails (e.g. rate limited) the caller can fall back to
//!   [`GitHubApiCache::get_stale`] to retrieve the last known-good body even
//!   though it's expired, matching the C# "Showing cached results" fallback.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a cached entry is considered fresh, mirroring `FreshTtlSeconds` (600s) in the C# source.
pub const FRESH_TTL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone)]
struct Entry {
    body: String,
    etag: Option<String>,
    fetched_at: Instant,
}

/// A cached GitHub API response body, along with whether it was served from cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedResponse {
    pub body: String,
    pub from_cache: bool,
}

/// Simple in-memory TTL cache for GitHub API responses, keyed by request URL.
///
/// Thread-safe via an internal `Mutex`; cheap to share behind an `Arc` or as
/// a long-lived struct field.
#[derive(Debug, Default)]
pub struct GitHubApiCache {
    entries: Mutex<HashMap<String, Entry>>,
}

impl GitHubApiCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Returns a fresh (within TTL) cached response for `url`, if any.
    pub fn get_fresh(&self, url: &str) -> Option<CachedResponse> {
        let entries = self.entries.lock().ok()?;
        let entry = entries.get(url)?;
        if entry.fetched_at.elapsed() < FRESH_TTL {
            Some(CachedResponse {
                body: entry.body.clone(),
                from_cache: true,
            })
        } else {
            None
        }
    }

    /// Returns the last cached response for `url` regardless of freshness,
    /// for use as a fallback when a live fetch fails (rate limit, network
    /// error) — mirrors the C# `catch` blocks that return `cached.Value`.
    pub fn get_stale(&self, url: &str) -> Option<CachedResponse> {
        let entries = self.entries.lock().ok()?;
        entries.get(url).map(|entry| CachedResponse {
            body: entry.body.clone(),
            from_cache: true,
        })
    }

    /// Returns the ETag stored for `url`, if any, for use in a conditional
    /// (`If-None-Match`) request.
    pub fn get_etag(&self, url: &str) -> Option<String> {
        let entries = self.entries.lock().ok()?;
        entries.get(url).and_then(|e| e.etag.clone())
    }

    /// Stores/refreshes a response body for `url`, resetting its fetch timestamp to now.
    pub fn store(&self, url: &str, body: String, etag: Option<String>) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(
                url.to_string(),
                Entry {
                    body,
                    etag,
                    fetched_at: Instant::now(),
                },
            );
        }
    }

    /// Refreshes only the fetch timestamp for `url` (used on HTTP 304 Not Modified),
    /// keeping the existing body/etag but marking it fresh again.
    pub fn touch(&self, url: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            if let Some(entry) = entries.get_mut(url) {
                entry.fetched_at = Instant::now();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miss_when_empty() {
        let cache = GitHubApiCache::new();
        assert!(cache.get_fresh("https://api.github.com/x").is_none());
        assert!(cache.get_stale("https://api.github.com/x").is_none());
    }

    #[test]
    fn hit_after_store() {
        let cache = GitHubApiCache::new();
        cache.store(
            "https://api.github.com/x",
            "{\"a\":1}".into(),
            Some("etag1".into()),
        );
        let hit = cache
            .get_fresh("https://api.github.com/x")
            .expect("should be fresh");
        assert_eq!(hit.body, "{\"a\":1}");
        assert!(hit.from_cache);
        assert_eq!(
            cache.get_etag("https://api.github.com/x").as_deref(),
            Some("etag1")
        );
    }

    #[test]
    fn expires_after_ttl() {
        let cache = GitHubApiCache::new();
        cache.store("https://api.github.com/x", "body".into(), None);
        // Simulate TTL expiry by directly manipulating the stored timestamp
        // via a zero-TTL comparison — we can't sleep 600s in a unit test, so
        // we verify the stale-fallback path instead, and separately verify
        // that a manually aged entry is no longer "fresh".
        {
            let mut entries = cache.entries.lock().unwrap();
            let entry = entries.get_mut("https://api.github.com/x").unwrap();
            entry.fetched_at = Instant::now() - FRESH_TTL - Duration::from_secs(1);
        }
        assert!(cache.get_fresh("https://api.github.com/x").is_none());
        let stale = cache
            .get_stale("https://api.github.com/x")
            .expect("stale entry still present");
        assert_eq!(stale.body, "body");
    }

    #[test]
    fn touch_refreshes_freshness() {
        let cache = GitHubApiCache::new();
        cache.store("https://api.github.com/x", "body".into(), None);
        {
            let mut entries = cache.entries.lock().unwrap();
            let entry = entries.get_mut("https://api.github.com/x").unwrap();
            entry.fetched_at = Instant::now() - FRESH_TTL - Duration::from_secs(1);
        }
        assert!(cache.get_fresh("https://api.github.com/x").is_none());
        cache.touch("https://api.github.com/x");
        assert!(cache.get_fresh("https://api.github.com/x").is_some());
    }
}
