//! News feed fetcher. Originally ported from `Services/NewsService.cs`
//! (`https://glacierclient.xyz/news.json`), but that endpoint no longer
//! exists on the live site (confirmed 404 — the site was restructured and
//! never re-added a news JSON). Replaced with the Glacier Client website's
//! `assets/data/downloads.json`, whose `clients.working[].changelog` array
//! is the client's real, live changelog feed. The launcher's own GitHub
//! Releases changelog is a separate feed (`list_launcher_releases` in
//! `src-tauri/src/commands/update.rs`, rendered as NewsView's dedicated
//! "Changelog" section) — kept apart here so the two don't duplicate.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::models::NewsPost;

const DOWNLOADS_JSON_URL: &str = "https://glacierclient.xyz/assets/data/downloads.json";

/// Fetches the Glacier Client website's changelog feed and HTML-decodes
/// each post's title/subtitle (mirroring the original `Decode` helper).
pub async fn get_news(client: &reqwest::Client) -> Option<Vec<NewsPost>> {
    if let Some(cached) = cached_news() {
        return Some(cached);
    }
    let mut posts = fetch_client_changelog(client).await?;
    if posts.is_empty() {
        return None;
    }
    posts.sort_by(|a, b| b.date.cmp(&a.date));
    let decoded = decode(posts);
    store_news(&decoded);
    Some(decoded)
}

/// How long a successfully fetched changelog is reused before re-fetching.
/// The feed changes rarely, and the panel can be opened many times per
/// session, so this avoids a network round trip on every open. Failures are
/// never cached, so a transient error retries on the next call.
const NEWS_TTL: Duration = Duration::from_secs(10 * 60);

static NEWS_CACHE: Mutex<Option<(Instant, Vec<NewsPost>)>> = Mutex::new(None);

fn cached_news() -> Option<Vec<NewsPost>> {
    let guard = NEWS_CACHE.lock().unwrap_or_else(|p| p.into_inner());
    match guard.as_ref() {
        Some((at, posts)) if at.elapsed() < NEWS_TTL => Some(posts.clone()),
        _ => None,
    }
}

fn store_news(posts: &[NewsPost]) {
    let mut guard = NEWS_CACHE.lock().unwrap_or_else(|p| p.into_inner());
    *guard = Some((Instant::now(), posts.to_vec()));
}

async fn fetch_client_changelog(client: &reqwest::Client) -> Option<Vec<NewsPost>> {
    let json: Value = client
        .get(DOWNLOADS_JSON_URL)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;

    let entries = json
        .get("clients")?
        .get("working")?
        .as_array()?
        .iter()
        .flat_map(|pack| {
            pack.get("changelog")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|entry| {
            let version = entry.get("version")?.as_str()?.to_string();
            let date = entry
                .get("date")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let title = entry
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let first_note = entry
                .get("notes")
                .and_then(Value::as_array)
                .and_then(|notes| notes.first())
                .and_then(Value::as_str)
                .unwrap_or("");
            Some(NewsPost {
                icon: "fa-solid fa-download".to_string(),
                title: format!("Client {version}"),
                subtitle: if title.is_empty() {
                    first_note.to_string()
                } else {
                    title
                },
                url: "https://glacierclient.xyz/#downloads".to_string(),
                date,
            })
        })
        .collect();

    Some(entries)
}

/// HTML-decodes title/subtitle fields in place, matching the C# `Decode`
/// static helper so cached/loaded posts can be re-decoded consistently.
pub fn decode(mut posts: Vec<NewsPost>) -> Vec<NewsPost> {
    for post in &mut posts {
        post.title = html_decode(&post.title);
        post.subtitle = html_decode(&post.subtitle);
    }
    posts
}

/// Minimal HTML entity decoder covering the named/numeric entities that
/// realistically show up in short news blurbs. Not a full HTML5 decoder.
pub(crate) fn html_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        let mut entity = String::new();
        let mut consumed = Vec::new();
        while let Some(&next) = chars.peek() {
            if next == ';' || entity.len() > 10 {
                break;
            }
            entity.push(next);
            consumed.push(next);
            chars.next();
        }
        if chars.peek() == Some(&';') {
            chars.next();
            match decode_entity(&entity) {
                Some(decoded) => out.push_str(&decoded),
                None => {
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
            }
        } else {
            out.push('&');
            out.push_str(&entity);
        }
    }
    out
}

fn decode_entity(entity: &str) -> Option<String> {
    Some(match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" | "#39" => "'".to_string(),
        "nbsp" => "\u{00A0}".to_string(),
        "mdash" => "\u{2014}".to_string(),
        "ndash" => "\u{2013}".to_string(),
        "hellip" => "\u{2026}".to_string(),
        e if e.starts_with('#') => {
            let rest = &e[1..];
            let code = if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X'))
            {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                rest.parse::<u32>().ok()?
            };
            char::from_u32(code)?.to_string()
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixture_response() {
        let body = r#"[
            {"icon":"fa-star","title":"New &amp; improved","subtitle":"Ships &lt;now&gt;","url":"https://x","date":"2026-01-01"}
        ]"#;
        let posts: Vec<NewsPost> = serde_json::from_str(body).expect("valid json");
        let decoded = decode(posts);
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].title, "New & improved");
        assert_eq!(decoded[0].subtitle, "Ships <now>");
    }

    #[test]
    fn html_decode_handles_common_entities() {
        assert_eq!(html_decode("Tom &amp; Jerry"), "Tom & Jerry");
        assert_eq!(html_decode("caf&#233;"), "caf\u{e9}");
        assert_eq!(html_decode("no entities here"), "no entities here");
    }

    #[test]
    fn html_decode_is_single_pass() {
        // A double-encoded entity must only lose one layer of escaping.
        assert_eq!(html_decode("&amp;lt;"), "&lt;");
        assert_eq!(html_decode("&amp;amp;"), "&amp;");
    }

    #[test]
    fn decode_posts_decodes_exactly_once() {
        let post = NewsPost {
            icon: String::new(),
            title: "&amp;lt;b&amp;gt;".to_string(),
            subtitle: "&amp;lt;".to_string(),
            url: String::new(),
            date: String::new(),
        };
        let decoded = decode(vec![post]);
        assert_eq!(decoded[0].title, "&lt;b&gt;");
        assert_eq!(decoded[0].subtitle, "&lt;");
    }
}
