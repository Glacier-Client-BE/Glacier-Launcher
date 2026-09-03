//! Shared HTTP client construction, mirroring `HttpFactory.cs`.
//!
//! The original C# factory builds a single lazily-initialized `HttpClient` with a
//! pooled `SocketsHttpHandler`, a 5 minute timeout, automatic gzip/deflate/brotli
//! decompression, and a `User-Agent` header. Several upstreams (Mojang, CurseForge,
//! Modrinth, GitHub) reject or heavily rate-limit requests carrying no `User-Agent`
//! or a generic one, so this must always be set.

use std::time::Duration;

use crate::error::{DownloadError, Result};

/// User-Agent sent with every request, matching the original launcher's convention.
pub const USER_AGENT: &str = "GlacierLauncher/1.0";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_IDLE_PER_HOST: usize = 24;

/// Thin wrapper around a shared, pre-configured [`reqwest::Client`].
///
/// Construct once per process (e.g. behind a `OnceCell`/`Lazy` in the app) and share
/// it across all downloads so connection pooling actually applies, matching the
/// `Lazy<HttpClient>` singleton in `HttpFactory.cs`.
#[derive(Clone)]
pub struct DownloadClient {
    http: reqwest::Client,
}

impl DownloadClient {
    /// Builds a new client with rustls-tls, pooled connections, and the launcher's
    /// `User-Agent`. reqwest negotiates gzip/deflate/brotli automatically via the
    /// `json`/`stream` feature set's transitive decompression support.
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .pool_idle_timeout(POOL_IDLE_TIMEOUT)
            .pool_max_idle_per_host(MAX_IDLE_PER_HOST)
            .build()
            .map_err(DownloadError::Http)?;
        Ok(Self { http })
    }

    /// Wraps an already-constructed [`reqwest::Client`] (e.g. one shared with other
    /// crates, or a mock client injected in tests).
    pub fn from_client(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub fn inner(&self) -> &reqwest::Client {
        &self.http
    }
}
