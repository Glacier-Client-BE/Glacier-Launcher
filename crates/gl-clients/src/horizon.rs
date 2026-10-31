//! Horizon (<https://github.com/horizon-rust/horizon>, MIT): a native Rust
//! rewrite of Minecraft Java Edition 26.3, shipped as the rolling `nightly`
//! GitHub release.
//!
//! This module is the pure half of the integration: which release asset to
//! download, how to read `SHA256SUMS`, where each build lives on disk, the
//! install-state bookkeeping (current build plus one previous build kept for
//! rollback), the `horizon-prepare` / `horizon-client` command lines, and the
//! session JSON the game reads from stdin. The Tauri layer owns the actual
//! network, extraction, process spawning and Java/Minecraft provisioning.
//!
//! Protocol facts come from Horizon's own docs and tool output:
//! - `horizon-prepare --prepare --root <build> --minecraft-dir <mc> --server-jar <jar>`
//!   builds resources into `<build>/.local/horizon-client/<version>` and needs
//!   a Java 25 runtime (found through `JAVA_HOME`, then `PATH`).
//! - `horizon-client --resources <dir> --game-dir <dir> --home <dir> --session-stdin`
//!   reads one JSON object `{name, uuid, access_token, user_type, xuid?}` from
//!   stdin, then starts. Exit codes: 0 quit, 1 error, 2 refused command line,
//!   101 panic (a crash report path is printed last on stderr).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ClientsError, Result};
use crate::github::GitHubAsset;
use crate::hash::hashes_equal;

pub const REPO_URL: &str = "https://github.com/horizon-rust/horizon";
pub const WEBSITE_URL: &str = "https://playhorizon.dev";
pub const RELEASE_API_URL: &str =
    "https://api.github.com/repos/horizon-rust/horizon/releases/tags/nightly";
pub const RELEASE_TAG: &str = "nightly";
pub const ZIP_ASSET_NAME: &str = "horizon-windows-x64.zip";
pub const SUMS_ASSET_NAME: &str = "SHA256SUMS";
pub const MINECRAFT_VERSION: &str = "26.3";
pub const JAVA_MAJOR: u32 = 25;
pub const CLIENT_EXE: &str = "horizon-client.exe";
pub const PREPARE_EXE: &str = "horizon-prepare.exe";
/// Instance id under which the running client is registered in
/// `gl_launch::ProcessRegistry`.
pub const PROCESS_KEY: &str = "horizon";

/// A GitHub release as Horizon publishes it, trimmed to what the card needs.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct HorizonRelease {
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    /// The branch or commit the release was cut from.
    #[serde(default)]
    pub target_commitish: String,
    /// The release notes ("patch notes"): Markdown with some HTML.
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
    /// Present on GitHub's error payload instead of a release.
    #[serde(default)]
    pub message: Option<String>,
}

/// Parses a `releases/tags/nightly` response body.
pub fn parse_release(body: &str) -> Result<HorizonRelease> {
    let release: HorizonRelease = serde_json::from_str(body)?;
    if release
        .message
        .as_deref()
        .is_some_and(|m| m.eq_ignore_ascii_case("not found"))
    {
        return Err(ClientsError::NotFound(format!(
            "Horizon release {RELEASE_TAG}"
        )));
    }
    Ok(release)
}

/// Recent releases, newest first, for finding the release an installed build came from.
pub const RELEASES_LIST_API_URL: &str =
    "https://api.github.com/repos/horizon-rust/horizon/releases?per_page=100";

/// Parses a `releases` list response body.
pub fn parse_releases(body: &str) -> Result<Vec<HorizonRelease>> {
    Ok(serde_json::from_str(body)?)
}

fn is_short_sha(s: &str) -> bool {
    (7..=40).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The short commit a build id ends with: `nightly-2026-10-30-587f81f` gives `587f81f`.
pub fn build_commit(build_id: &str) -> Option<String> {
    let last = build_id.rsplit('-').next()?;
    is_short_sha(last).then(|| last.to_ascii_lowercase())
}

/// The commit a release was built from, read from its name
/// ("Nightly 2026-10-30 · 1da354b" gives `1da354b`).
pub fn commit_of_release(release: &HorizonRelease) -> Option<String> {
    let last = release.name.as_deref()?.split_whitespace().last()?;
    is_short_sha(last).then(|| last.to_ascii_lowercase())
}

/// The release an installed build came from: the first (newest) one whose
/// name, tag or target commit carries the build's short commit.
pub fn release_for_build<'a>(
    releases: &'a [HorizonRelease],
    build_id: &str,
) -> Option<&'a HorizonRelease> {
    let commit = build_commit(build_id)?;
    releases.iter().find(|r| {
        r.name
            .as_deref()
            .is_some_and(|n| n.to_ascii_lowercase().contains(&commit))
            || r.tag_name.to_ascii_lowercase().contains(&commit)
            || r.target_commitish.to_ascii_lowercase().starts_with(&commit)
    })
}

/// Turns a release body (GitHub Markdown mixed with HTML) into plain display
/// text. Tags are removed (block tags become line breaks, list items bullets),
/// links become `text (url)`, images are dropped, entities are decoded once.
/// The result never contains markup, so it is safe to show as text.
pub fn release_notes_text(raw: &str) -> String {
    let text = strip_html_comments(raw);
    let text = strip_markdown_images(&text);
    let text = replace_html_links(&text);
    let text = html_tags_to_text(&text);
    let text = decode_entities(&text);
    tidy_text(&text)
}

fn strip_html_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = match rest[start..].find("-->") {
            Some(end) => &rest[start + end + 3..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// Removes `![alt](url)`.
fn strip_markdown_images(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("![") {
        let after = &rest[start + 2..];
        let end = after.find(']').and_then(|close| {
            let tail = &after[close + 1..];
            tail.strip_prefix('(')
                .and_then(|t| t.find(')'))
                .map(|p| start + 2 + close + 1 + 1 + p + 1)
        });
        match end {
            Some(end) => {
                out.push_str(&rest[..start]);
                rest = &rest[end..];
            }
            None => {
                out.push_str(&rest[..start + 2]);
                rest = &rest[start + 2..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Rewrites `<a href="X">text</a>` as `text (X)`, or just `X` when there is
/// no distinct text.
fn replace_html_links(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut pos = 0;
    while let Some(rel) = lower[pos..].find("<a") {
        let start = pos + rel;
        let is_anchor = matches!(
            lower.as_bytes().get(start + 2).copied(),
            Some(b' ' | b'\t' | b'\n' | b'\r' | b'>')
        );
        if !is_anchor {
            out.push_str(&s[pos..start + 2]);
            pos = start + 2;
            continue;
        }
        let Some(open_rel) = lower[start..].find('>') else {
            break;
        };
        let open_end = start + open_rel;
        let Some(close_rel) = lower[open_end..].find("</a>") else {
            break;
        };
        let close = open_end + close_rel;
        let href = attr_value(&s[start..open_end], "href").unwrap_or_default();
        let label = strip_tags(&s[open_end + 1..close]);
        let label = label.trim();
        out.push_str(&s[pos..start]);
        if label.is_empty() {
            out.push_str(&href);
        } else if href.is_empty() || label == href {
            out.push_str(label);
        } else {
            out.push_str(&format!("{label} ({href})"));
        }
        pos = close + "</a>".len();
    }
    out.push_str(&s[pos..]);
    out
}

/// The value of attribute `name` in an opening tag, quoted or bare.
fn attr_value(tag: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=");
    let idx = tag.to_ascii_lowercase().find(&key)? + key.len();
    let value = &tag[idx..];
    let (quote, body) = match value.chars().next()? {
        q @ ('"' | '\'') => (Some(q), &value[1..]),
        _ => (None, value),
    };
    let end = match quote {
        Some(q) => body.find(q)?,
        None => body
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(body.len()),
    };
    Some(body[..end].trim().to_string())
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Replaces tags with line breaks where they start or end a block, and drops
/// the rest. A `<` not followed by a tag name is kept as text.
fn html_tags_to_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(lt) = rest.find('<') {
        let next = rest[lt + 1..].chars().next();
        if !matches!(next, Some(c) if c.is_ascii_alphabetic() || c == '/') {
            out.push_str(&rest[..lt + 1]);
            rest = &rest[lt + 1..];
            continue;
        }
        let Some(gt) = rest[lt..].find('>') else {
            break;
        };
        out.push_str(&rest[..lt]);
        out.push_str(tag_break(&rest[lt + 1..lt + gt]));
        rest = &rest[lt + gt + 1..];
    }
    out.push_str(rest);
    out
}

fn tag_break(tag: &str) -> &'static str {
    let closing = tag.starts_with('/');
    let name = tag
        .trim_start_matches('/')
        .split(|c: char| c.is_whitespace() || c == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match name.as_str() {
        "br" | "hr" => "\n",
        "li" if closing => "",
        "li" => "\n- ",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if closing => "\n",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "\n\n",
        "p" if closing => "\n\n",
        "p" | "ul" | "ol" | "div" | "tr" | "table" | "section" | "pre" | "blockquote" => "\n",
        _ => "",
    }
}

/// Decodes `&name;`, `&#NN;` and `&#xHH;` in one pass, so `&amp;lt;` becomes `&lt;`.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let decoded = tail
            .find(';')
            .filter(|&semi| semi <= 10)
            .and_then(|semi| decode_entity(&tail[1..semi]).map(|c| (c, semi)));
        match decoded {
            Some((c, semi)) => {
                out.push(c);
                rest = &tail[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let num = name.strip_prefix('#')?;
            let code = match num.strip_prefix(|c: char| c == 'x' || c == 'X') {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => num.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

/// Trims lines, drops Markdown heading marks (`### Fixed` becomes `Fixed`),
/// and collapses runs of blank lines to one.
fn tidy_text(s: &str) -> String {
    let lines: Vec<String> = s
        .lines()
        .map(|line| {
            let line = line.trim_end();
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                let heading = trimmed.trim_start_matches('#');
                if heading.is_empty() || heading.starts_with(' ') {
                    return heading.trim().to_string();
                }
            }
            line.to_string()
        })
        .collect();
    let mut joined = lines.join("\n");
    while joined.contains("\n\n\n") {
        joined = joined.replace("\n\n\n", "\n\n");
    }
    joined.trim().to_string()
}

/// The Windows x64 build zip. Matched by exact name so the Linux, macOS and
/// server archives in the same release are never picked by mistake.
pub fn pick_zip_asset(release: &HorizonRelease) -> Result<&GitHubAsset> {
    release
        .assets
        .iter()
        .find(|a| a.name == ZIP_ASSET_NAME)
        .ok_or_else(|| ClientsError::NoAssetFound(ZIP_ASSET_NAME.to_string()))
}

/// The checksum list published next to the assets.
pub fn pick_sums_asset(release: &HorizonRelease) -> Result<&GitHubAsset> {
    release
        .assets
        .iter()
        .find(|a| a.name == SUMS_ASSET_NAME)
        .ok_or_else(|| ClientsError::NoAssetFound(SUMS_ASSET_NAME.to_string()))
}

/// Parses `sha256sum`-style lines (`<hex>  <name>` or `<hex> *<name>`) into a
/// map from file name to lower-case hex digest. Blank lines, comments and
/// lines whose first token is not a 64-digit hex string are skipped.
pub fn parse_sha256sums(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((hash, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        if !is_sha256_hex(hash) {
            continue;
        }
        let name = rest.trim().trim_start_matches('*').trim();
        if name.is_empty() {
            continue;
        }
        out.insert(name.to_string(), hash.to_ascii_lowercase());
    }
    out
}

/// The expected digest of `asset_name` from a parsed `SHA256SUMS`.
pub fn expected_sha256(sums: &HashMap<String, String>, asset_name: &str) -> Result<String> {
    sums.get(asset_name)
        .cloned()
        .ok_or_else(|| ClientsError::NotFound(format!("{asset_name} in {SUMS_ASSET_NAME}")))
}

/// `true` when `actual` (a file's digest) matches the published `expected`.
pub fn digest_matches(actual: &str, expected: &str) -> bool {
    is_sha256_hex(expected) && hashes_equal(actual, expected)
}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Lower-case hex SHA-1 of a byte slice (the Mojang server jar is checked
/// against the `sha1` in its version JSON).
pub fn sha1_hex_bytes(bytes: &[u8]) -> String {
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Stable folder/record id for a build: a slug of the release name plus the
/// first eight digits of the zip's SHA-256, so two uploads with the same
/// name never collide and the id changes whenever the bytes do.
pub fn build_id(release_name: Option<&str>, zip_sha256: &str) -> String {
    let mut slug = String::new();
    for ch in release_name.unwrap_or(RELEASE_TAG).chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_matches('-').chars().take(40).collect();
    let slug = if slug.is_empty() {
        RELEASE_TAG.to_string()
    } else {
        slug
    };
    let short: String = zip_sha256.chars().take(8).collect();
    format!("{slug}-{short}")
}

/// One installed build as recorded in `state.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRecord {
    pub id: String,
    /// The release name shown to the player, e.g. "Nightly 2026-10-30 · 587f81f".
    pub name: String,
    pub zip_sha256: String,
    /// Unix seconds when the build was installed.
    pub installed_at: i64,
    #[serde(default)]
    pub published_at: String,
    /// The release's patch notes at install time, kept so they survive
    /// the release being replaced upstream.
    #[serde(default)]
    pub notes: String,
}

/// The install state: the build that plays, and the one before it, kept so
/// the player can roll back.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallState {
    #[serde(default)]
    pub current: Option<BuildRecord>,
    #[serde(default)]
    pub previous: Option<BuildRecord>,
}

impl InstallState {
    /// Makes `new` the current build. The old current becomes the previous
    /// one. Returns the build that fell out of the two-slot history, whose
    /// folder the caller should delete.
    pub fn promote(&mut self, new: BuildRecord) -> Option<BuildRecord> {
        let old_previous = self.previous.take();
        let old_current = self.current.take();
        self.previous = old_current.filter(|c| c.id != new.id);
        self.current = Some(new.clone());
        old_previous.filter(|p| {
            p.id != new.id && self.previous.as_ref().map(|q| q.id.as_str()) != Some(p.id.as_str())
        })
    }

    /// Swaps current and previous. Returns `false` when there is no previous
    /// build to return to.
    pub fn rollback(&mut self) -> bool {
        let Some(previous) = self.previous.take() else {
            return false;
        };
        self.previous = self.current.take();
        self.current = Some(previous);
        true
    }

    /// `true` when `zip_sha256` is the digest of the build now playing.
    pub fn is_current_digest(&self, zip_sha256: &str) -> bool {
        self.current
            .as_ref()
            .is_some_and(|c| digest_matches(&c.zip_sha256, zip_sha256))
    }
}

/// On-disk layout under `<clients>/horizon`:
///
/// ```text
/// state.json                 InstallState
/// builds/<id>/               a Horizon build with its prepared .local/ folder
/// .staging/<id>/             temporary extraction, removed after use
/// downloads/                 the zip in flight and the cached server jar
/// game/                      --game-dir: saves, options, resource packs (kept on uninstall)
/// home/                      --home: logs and crash reports
/// ```
#[derive(Debug, Clone)]
pub struct HorizonLayout {
    pub root: PathBuf,
}

impl HorizonLayout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn state_file(&self) -> PathBuf {
        self.root.join("state.json")
    }

    pub fn builds_dir(&self) -> PathBuf {
        self.root.join("builds")
    }

    pub fn build_dir(&self, id: &str) -> PathBuf {
        self.builds_dir().join(id)
    }

    pub fn staging_dir(&self, id: &str) -> PathBuf {
        self.root.join(".staging").join(id)
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join("downloads")
    }

    pub fn zip_path(&self, id: &str) -> PathBuf {
        self.downloads_dir().join(format!("{id}.zip"))
    }

    pub fn server_jar_path(&self) -> PathBuf {
        self.downloads_dir()
            .join(format!("server-{MINECRAFT_VERSION}.jar"))
    }

    pub fn game_dir(&self) -> PathBuf {
        self.root.join("game")
    }

    pub fn home_dir(&self) -> PathBuf {
        self.root.join("home")
    }
}

/// A build installed by Horizon's own launcher, reused instead of downloading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficialInstall {
    pub id: String,
    pub build_dir: PathBuf,
    /// `%APPDATA%\horizon`: saves, options and mods shared with that launcher.
    pub data_dir: PathBuf,
}

/// Finds a playable build under `<appdata>/horizon-launcher/versions`: the one
/// that launcher has selected if it is playable, else the newest playable one.
pub fn find_official_install(appdata: &Path) -> Option<OfficialInstall> {
    let launcher = appdata.join("horizon-launcher");
    let versions = launcher.join("versions");
    let selected = std::fs::read_to_string(launcher.join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("selected")?.as_str().map(str::to_owned));
    let mut ids: Vec<String> = std::fs::read_dir(&versions)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|id| is_playable(&versions.join(id)))
        .collect();
    ids.sort();
    let id = match selected {
        Some(s) if ids.contains(&s) => s,
        _ => ids.pop()?,
    };
    Some(OfficialInstall {
        build_dir: versions.join(&id),
        id,
        data_dir: appdata.join("horizon"),
    })
}

/// `<build>/horizon-client.exe` exists.
pub fn is_built(build_dir: &Path) -> bool {
    build_dir.join(CLIENT_EXE).is_file()
}

/// The prepared resources folder: `<build>/.local/horizon-client/<version>`.
/// When several exist (a rebuilt version), the last in name order wins.
pub fn resources_dir(build_dir: &Path) -> Option<PathBuf> {
    let parent = build_dir.join(".local").join("horizon-client");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&parent)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs.pop()
}

/// A build can be played: its executable and prepared resources are present.
pub fn is_playable(build_dir: &Path) -> bool {
    is_built(build_dir) && resources_dir(build_dir).is_some()
}

/// Finds the folder that holds `horizon-client.exe` in an extracted archive:
/// the folder itself, or the single child that contains it.
pub fn find_client_root(extracted: &Path) -> Option<PathBuf> {
    if is_built(extracted) {
        return Some(extracted.to_path_buf());
    }
    let mut children: Vec<PathBuf> = std::fs::read_dir(extracted)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    children.sort();
    children.into_iter().find(|p| is_built(p))
}

/// The argument list for `horizon-client`. The session itself is not an
/// argument: it goes on stdin (see [`session_json`]).
pub fn launch_args(resources: &Path, game_dir: &Path, home_dir: &Path) -> Vec<String> {
    vec![
        "--resources".into(),
        resources.to_string_lossy().into_owned(),
        "--game-dir".into(),
        game_dir.to_string_lossy().into_owned(),
        "--home".into(),
        home_dir.to_string_lossy().into_owned(),
        "--width".into(),
        "1280".into(),
        "--height".into(),
        "720".into(),
        "--session-stdin".into(),
    ]
}

/// The account JSON `horizon-client --session-stdin` reads. Name, uuid and
/// token are required; `user_type` is always `msa`.
pub fn session_json(
    name: &str,
    uuid: &str,
    access_token: &str,
    xuid: Option<&str>,
) -> Result<String> {
    if name.trim().is_empty() || uuid.trim().is_empty() || access_token.trim().is_empty() {
        return Err(ClientsError::InvalidInput(
            "Horizon needs a signed-in Microsoft account that owns Minecraft: Java Edition".into(),
        ));
    }
    let mut value = serde_json::json!({
        "name": name.trim(),
        "uuid": uuid.trim(),
        "access_token": access_token.trim(),
        "user_type": "msa",
    });
    if let Some(xuid) = xuid.map(str::trim).filter(|x| !x.is_empty()) {
        value["xuid"] = serde_json::Value::String(xuid.to_string());
    }
    Ok(value.to_string())
}

/// `downloads.server.url` and `downloads.server.sha1` from a Mojang version
/// JSON, the source of the 26.3 server jar that `horizon-prepare` needs.
pub fn server_jar_from_version_json(text: &str) -> Result<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let server = value
        .get("downloads")
        .and_then(|d| d.get("server"))
        .ok_or_else(|| ClientsError::NotFound("downloads.server in the version JSON".into()))?;
    let url = server
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let sha1 = server
        .get("sha1")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if url.is_empty() || sha1.is_empty() {
        return Err(ClientsError::NotFound(
            "server url and sha1 in the version JSON".into(),
        ));
    }
    Ok((url.to_string(), sha1.to_ascii_lowercase()))
}

/// `true` when `mc_dir` already holds a complete 26.3 client: the version
/// JSON, the client jar, and the libraries and assets folders that
/// `horizon-prepare` imports from. Used to reuse an official-launcher
/// install instead of downloading a second copy.
pub fn has_vanilla_26_3(mc_dir: &Path) -> bool {
    let version = mc_dir.join("versions").join(MINECRAFT_VERSION);
    version.join(format!("{MINECRAFT_VERSION}.json")).is_file()
        && version.join(format!("{MINECRAFT_VERSION}.jar")).is_file()
        && mc_dir.join("libraries").is_dir()
        && mc_dir.join("assets").is_dir()
}

/// The Java home (the folder above `bin`) for a `javaw.exe` path. Prepare
/// reads `JAVA_HOME/bin/java`, so this is what it needs.
pub fn java_home_from_javaw(javaw: &Path) -> Option<PathBuf> {
    javaw.parent()?.parent().map(Path::to_path_buf)
}

/// Lower-case hex SHA-1 of a file on disk.
pub fn sha1_hex_file(path: &Path) -> Result<String> {
    use sha1::Digest;
    use std::io::Read;
    let mut file = std::io::BufReader::with_capacity(64 * 1024, std::fs::File::open(path)?);
    let mut hasher = sha1::Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Whether the file at `path` exists and matches `sha1`.
pub fn file_has_sha1(path: &Path, sha1: &str) -> bool {
    path.is_file()
        && sha1_hex_file(path)
            .map(|h| hashes_equal(&h, sha1))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_A: &str = "11d7d7f35c23b65cb07f2519e3c98b2278c8328a259c1bbf083934d46e6ba964";
    const HASH_B: &str = "0d6d44800079acf2514c5e0e6a60c7c6e3c8ca070c6990b2457a925838b24d4e";

    const RELEASE: &str = r#"{
        "tag_name": "nightly",
        "name": "Nightly 2026-10-30 · 587f81f",
        "published_at": "2026-10-30T15:12:05Z",
        "body": "Fixed: riders move their horses",
        "assets": [
            {"name": "horizon-linux-x86_64.tar.gz", "browser_download_url": "https://x/linux.tar.gz"},
            {"name": "horizon-windows-x64.zip", "browser_download_url": "https://x/windows.zip"},
            {"name": "horizon-macos-arm64.zip", "browser_download_url": "https://x/macos.zip"},
            {"name": "SHA256SUMS", "browser_download_url": "https://x/SHA256SUMS"}
        ]
    }"#;

    const SUMS: &str = "0d6d44800079acf2514c5e0e6a60c7c6e3c8ca070c6990b2457a925838b24d4e  horizon-linux-x86_64.tar.gz
11d7d7f35c23b65cb07f2519e3c98b2278c8328a259c1bbf083934d46e6ba964 *horizon-windows-x64.zip

# comment line
not-a-hash  junk.txt
";

    #[test]
    fn picks_exact_windows_zip_not_other_platforms() {
        let release = parse_release(RELEASE).unwrap();
        let asset = pick_zip_asset(&release).unwrap();
        assert_eq!(asset.name, ZIP_ASSET_NAME);
        assert_eq!(asset.browser_download_url, "https://x/windows.zip");
    }

    #[test]
    fn missing_windows_zip_is_an_error() {
        let json = r#"{"tag_name":"nightly","assets":[
            {"name":"horizon-linux-x86_64.tar.gz","browser_download_url":"https://x/a"}]}"#;
        let release = parse_release(json).unwrap();
        assert!(matches!(
            pick_zip_asset(&release),
            Err(ClientsError::NoAssetFound(_))
        ));
    }

    #[test]
    fn picks_sums_asset() {
        let release = parse_release(RELEASE).unwrap();
        assert_eq!(pick_sums_asset(&release).unwrap().name, SUMS_ASSET_NAME);
    }

    #[test]
    fn not_found_payload_is_reported() {
        let err = parse_release(r#"{"message":"Not Found"}"#).unwrap_err();
        assert!(matches!(err, ClientsError::NotFound(_)));
    }

    #[test]
    fn parses_sha256sums_in_both_formats() {
        let sums = parse_sha256sums(SUMS);
        assert_eq!(sums.len(), 2);
        assert_eq!(expected_sha256(&sums, ZIP_ASSET_NAME).unwrap(), HASH_A);
        assert_eq!(
            expected_sha256(&sums, "horizon-linux-x86_64.tar.gz").unwrap(),
            HASH_B
        );
        assert!(expected_sha256(&sums, "junk.txt").is_err());
    }

    #[test]
    fn sums_lowercases_and_rejects_short_digests() {
        let upper = HASH_A.to_ascii_uppercase();
        let sums = parse_sha256sums(&format!("{upper}  {ZIP_ASSET_NAME}\nabc  short.zip\n"));
        assert_eq!(sums.get(ZIP_ASSET_NAME).map(String::as_str), Some(HASH_A));
        assert!(!sums.contains_key("short.zip"));
    }

    #[test]
    fn digest_comparison_requires_a_real_digest() {
        assert!(digest_matches(HASH_A, &HASH_A.to_ascii_uppercase()));
        assert!(!digest_matches(HASH_A, HASH_B));
        assert!(!digest_matches("", ""));
    }

    #[test]
    fn build_id_is_slugged_and_digest_suffixed() {
        let id = build_id(Some("Nightly 2026-10-30 · 587f81f"), HASH_A);
        assert_eq!(id, "nightly-2026-10-30-587f81f-11d7d7f3");
        assert_eq!(build_id(None, HASH_A), "nightly-11d7d7f3");
        assert_ne!(build_id(Some("x"), HASH_A), build_id(Some("x"), HASH_B));
    }

    #[test]
    fn promote_keeps_previous_and_returns_dropped_build() {
        let rec = |id: &str| BuildRecord {
            id: id.into(),
            name: id.into(),
            zip_sha256: HASH_A.into(),
            installed_at: 0,
            published_at: String::new(),
            notes: String::new(),
        };
        let mut st = InstallState::default();
        assert!(st.promote(rec("a")).is_none());
        assert!(st.promote(rec("b")).is_none());
        assert_eq!(st.current.as_ref().unwrap().id, "b");
        assert_eq!(st.previous.as_ref().unwrap().id, "a");
        let dropped = st.promote(rec("c")).unwrap();
        assert_eq!(dropped.id, "a");
        assert_eq!(st.previous.as_ref().unwrap().id, "b");
    }

    #[test]
    fn rollback_swaps_and_needs_a_previous_build() {
        let rec = |id: &str| BuildRecord {
            id: id.into(),
            name: id.into(),
            zip_sha256: HASH_A.into(),
            installed_at: 0,
            published_at: String::new(),
            notes: String::new(),
        };
        let mut st = InstallState::default();
        assert!(!st.rollback());
        st.promote(rec("a"));
        assert!(!st.rollback());
        st.promote(rec("b"));
        assert!(st.rollback());
        assert_eq!(st.current.as_ref().unwrap().id, "a");
        assert_eq!(st.previous.as_ref().unwrap().id, "b");
    }

    #[test]
    fn layout_paths_are_under_root() {
        let l = HorizonLayout::new("/x/horizon");
        assert_eq!(
            l.build_dir("abc"),
            PathBuf::from("/x/horizon").join("builds").join("abc")
        );
        assert!(l.game_dir().ends_with("game"));
        assert!(l.server_jar_path().ends_with("server-26.3.jar"));
    }

    #[test]
    fn session_json_has_msa_user_and_optional_xuid() {
        let json = session_json(
            "Alex",
            "069a79f444e94726a5befca90e38aaf5",
            "tok",
            Some("123"),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["name"], "Alex");
        assert_eq!(v["user_type"], "msa");
        assert_eq!(v["access_token"], "tok");
        assert_eq!(v["xuid"], "123");
        let no_xuid: serde_json::Value =
            serde_json::from_str(&session_json("Alex", "u", "t", Some("  ")).unwrap()).unwrap();
        assert!(no_xuid.get("xuid").is_none());
    }

    #[test]
    fn session_json_refuses_missing_fields() {
        assert!(session_json("", "u", "t", None).is_err());
        assert!(session_json("a", "u", "  ", None).is_err());
    }

    #[test]
    fn launch_args_match_the_documented_flags() {
        let args = launch_args(Path::new("R"), Path::new("G"), Path::new("H"));
        assert_eq!(args[0], "--resources");
        assert!(args.windows(2).any(|w| w == ["--game-dir", "G"]));
        assert!(args.windows(2).any(|w| w == ["--home", "H"]));
        assert_eq!(args.last().map(String::as_str), Some("--session-stdin"));
    }

    #[test]
    fn server_jar_is_read_from_version_json() {
        let json = r#"{"downloads":{"server":{"sha1":"ABCDEF","url":"https://piston-data/server.jar","size":1}}}"#;
        let (url, sha1) = server_jar_from_version_json(json).unwrap();
        assert_eq!(url, "https://piston-data/server.jar");
        assert_eq!(sha1, "abcdef");
        assert!(server_jar_from_version_json(r#"{"downloads":{}}"#).is_err());
    }

    #[test]
    fn finds_client_root_flat_or_nested() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("horizon-windows-x64");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join(CLIENT_EXE), b"").unwrap();
        assert_eq!(find_client_root(dir.path()), Some(nested.clone()));
        assert_eq!(find_client_root(&nested), Some(nested));
        assert!(find_client_root(&dir.path().join("nope")).is_none());
    }

    #[test]
    fn official_install_prefers_selected_playable_build() {
        let tmp = tempfile::tempdir().unwrap();
        let versions = tmp.path().join("horizon-launcher").join("versions");
        let make = |id: &str, prepared: bool| {
            let b = versions.join(id);
            std::fs::create_dir_all(&b).unwrap();
            std::fs::write(b.join(CLIENT_EXE), b"").unwrap();
            if prepared {
                std::fs::create_dir_all(b.join(".local/horizon-client/26.3")).unwrap();
            }
        };
        make("nightly-a", true);
        make("nightly-b", true);
        make("nightly-c", false);
        assert_eq!(find_official_install(tmp.path()).unwrap().id, "nightly-b");
        std::fs::write(
            tmp.path().join("horizon-launcher/settings.json"),
            r#"{"selected":"nightly-a"}"#,
        )
        .unwrap();
        let found = find_official_install(tmp.path()).unwrap();
        assert_eq!(found.id, "nightly-a");
        assert_eq!(found.data_dir, tmp.path().join("horizon"));
        assert!(find_official_install(&tmp.path().join("none")).is_none());
    }

    #[test]
    fn resources_dir_and_playable_checks() {
        let dir = tempfile::tempdir().unwrap();
        let build = dir.path();
        assert!(resources_dir(build).is_none());
        assert!(!is_playable(build));
        std::fs::write(build.join(CLIENT_EXE), b"").unwrap();
        std::fs::create_dir_all(build.join(".local").join("horizon-client").join("26.3")).unwrap();
        assert!(resources_dir(build).unwrap().ends_with("26.3"));
        assert!(is_playable(build));
    }

    #[test]
    fn vanilla_check_needs_json_jar_libraries_and_assets() {
        let dir = tempfile::tempdir().unwrap();
        let mc = dir.path();
        assert!(!has_vanilla_26_3(mc));
        let v = mc.join("versions").join("26.3");
        std::fs::create_dir_all(&v).unwrap();
        std::fs::write(v.join("26.3.json"), b"{}").unwrap();
        std::fs::write(v.join("26.3.jar"), b"").unwrap();
        std::fs::create_dir_all(mc.join("libraries")).unwrap();
        assert!(!has_vanilla_26_3(mc));
        std::fs::create_dir_all(mc.join("assets")).unwrap();
        assert!(has_vanilla_26_3(mc));
    }

    #[test]
    fn java_home_is_the_folder_above_bin() {
        let home = java_home_from_javaw(Path::new("/jdk/bin/javaw.exe")).unwrap();
        assert_eq!(home, PathBuf::from("/jdk"));
    }

    /// Head and tail of the real `nightly` release body (gh api, 2026-10-30).
    const NIGHTLY_BODY: &str = r##"<h3>Want to play? Download the <a href="https://playhorizon.dev">Horizon launcher</a>, not the files below</h3><p>It installs this version and keeps it up to date. It is also on GitHub: <a href="https://github.com/horizon-rust/horizon/releases?q=launcher&amp;expanded=true">the launcher releases</a>. You need your own copy of Minecraft: Java Edition.</p>

Nightly 2026-10-30 · 1da354b: main (1da354b), built from `nightly/build-2026-10-30-9` (1da354b67a509812c4141b67de2affd6de65c342).

### Fixed
- Fix detail requests for retiring fine LOD leaves (#4048, fixes #4039)
- Thrown tridents: clip blocks with vanilla's centre ray, not the 0.5 box (#4406, fixes #4365)

### Known issues
- Shader packs have been removed for now. They will come back later, written again from scratch.

Thanks to @1038april, @CocofireHD

Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft."##;

    #[test]
    fn release_body_becomes_plain_text() {
        let text = release_notes_text(NIGHTLY_BODY);
        assert!(
            !text.contains('<') && !text.contains('>'),
            "markup left: {text}"
        );
        assert!(!text.contains("&amp;"), "entity left: {text}");
        assert!(!text.contains("\n\n\n"));
        assert!(text.starts_with("Want to play? Download the Horizon launcher (https://playhorizon.dev), not the files below"));
        assert!(text.contains("the launcher releases (https://github.com/horizon-rust/horizon/releases?q=launcher&expanded=true)"));
        assert!(text.contains(
            "\nFixed\n- Fix detail requests for retiring fine LOD leaves (#4048, fixes #4039)\n"
        ));
        assert!(text
            .contains("- Thrown tridents: clip blocks with vanilla's centre ray, not the 0.5 box"));
        assert!(text.contains("\nKnown issues\n"));
        assert!(text.ends_with("Mojang or Microsoft."));
    }

    #[test]
    fn entities_decode_once_and_markup_is_dropped() {
        assert_eq!(release_notes_text("a &amp;lt; b"), "a &lt; b");
        assert_eq!(release_notes_text("&lt;b&gt;bold&lt;/b&gt;"), "<b>bold</b>");
        assert_eq!(
            release_notes_text("it&#39;s &quot;x&quot; &#x41;&nbsp;&copy;"),
            "it's \"x\" A &copy;"
        );
        assert_eq!(release_notes_text("<b>bold</b> <code>x</code>"), "bold x");
        assert_eq!(release_notes_text("1 < 2 and 3 > 2"), "1 < 2 and 3 > 2");
    }

    #[test]
    fn blocks_become_lines_and_items_bullets() {
        let text =
            release_notes_text("<h3>Fixed</h3><p>One<br>two</p><ul><li>a</li><li>b</li></ul>");
        assert_eq!(text, "Fixed\n\nOne\ntwo\n\n- a\n- b");
    }

    #[test]
    fn links_and_images_are_rewritten() {
        assert_eq!(
            release_notes_text(r#"<a href="https://x.y/z">https://x.y/z</a>"#),
            "https://x.y/z"
        );
        assert_eq!(
            release_notes_text(r#"<a href='https://x.y'>site</a>"#),
            "site (https://x.y)"
        );
        assert_eq!(
            release_notes_text(r#"<a href="https://x.y"></a>"#),
            "https://x.y"
        );
        assert_eq!(
            release_notes_text("see ![shot](https://img/x.png) here"),
            "see  here"
        );
        assert_eq!(release_notes_text("<!-- hidden -->shown"), "shown");
        assert_eq!(
            release_notes_text("### Heading\n\n\n\nbody"),
            "Heading\n\nbody"
        );
    }

    #[test]
    fn release_for_build_matches_the_short_commit() {
        let json = r#"[
            {"tag_name":"nightly","name":"Nightly 2026-10-30 · 1da354b","target_commitish":"main"},
            {"tag_name":"nightly-2026-10-29","name":"Nightly 2026-10-29 · 587f81f","target_commitish":"main"}
        ]"#;
        let releases = parse_releases(json).unwrap();
        let hit = release_for_build(&releases, "nightly-2026-10-30-587f81f").unwrap();
        assert_eq!(hit.name.as_deref(), Some("Nightly 2026-10-29 · 587f81f"));
        assert!(release_for_build(&releases, "nightly-2026-10-30-abcdef1").is_none());
        assert!(release_for_build(&releases, "nightly").is_none());
        assert_eq!(commit_of_release(&releases[0]).as_deref(), Some("1da354b"));
        assert_eq!(
            build_commit("nightly-2026-10-30-587F81F").as_deref(),
            Some("587f81f")
        );
    }

    #[test]
    fn sha1_of_abc() {
        assert_eq!(
            sha1_hex_bytes(b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }
}
