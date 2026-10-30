//! Shared application state: the long-lived service instances that Tauri
//! commands operate on. Held behind `Mutex` (these services do their own
//! synchronous, atomic-write-backed I/O — not `async`-heavy enough to
//! warrant `tokio::sync::Mutex`) and constructed once in `setup`.

use std::sync::Mutex;

use gl_bedrock::BedrockInstanceService;
use gl_integrations::discord::DiscordRpcService;
use gl_java::{InstanceService as JavaInstanceService, VersionService as JavaVersionService};
use gl_settings::{NotificationService, ThemeService};

pub struct AppState {
    pub java_instances: Mutex<JavaInstanceService>,
    /// `tokio::sync::Mutex`, not `std::sync::Mutex`: `VersionService::get_versions`
    /// is `async` and needs `&mut self` held across the `.await`, which a std
    /// `MutexGuard` can't do (it isn't `Send`).
    pub java_versions: tokio::sync::Mutex<JavaVersionService>,
    pub bedrock_instances: Mutex<BedrockInstanceService>,
    /// In-app notification feed (C# `NotificationService`). Holds its own
    /// interior mutability, so it is not wrapped in another `Mutex`.
    pub notifications: NotificationService,
    /// Saved custom themes (C# `ThemeService`). Interior mutability, so no
    /// outer `Mutex`.
    pub themes: ThemeService,
    pub discord: Mutex<DiscordRpcService>,
    pub http: reqwest::Client,
    pub downloads: gl_download::DownloadQueue,
    pub clients_dir: std::path::PathBuf,
    /// Previous-sample baseline for `commands::utilities::get_live_sample`'s
    /// delta-based CPU% calculation (ported from `HardwareMonitorService`'s
    /// `_prevIdle`/`_prevKernel`/`_prevUser`/`_haveBaseline` instance fields).
    pub live_sample_baseline: Mutex<crate::commands::utilities::LiveSampleBaseline>,
    /// Registry of currently-running Java child processes, keyed by instance
    /// id (see `commands::java_process` and `gl_launch::process_state`).
    pub java_processes: gl_launch::ProcessRegistry,
    /// Sync-pack engine (Phase 7): shared content (dir links, merged
    /// files, mod lists) kept consistent across Java instances. See
    /// `commands::sync_packs`.
    pub sync_engine: gl_sync::SyncEngine,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let root = gl_settings::launcher_root();

        let java_instances = JavaInstanceService::new(root.join("java"))
            .map_err(|e| format!("failed to initialize Java instances: {e}"))?;
        let java_versions = JavaVersionService::new(JavaVersionService::default_cache_file());
        // The live com.mojang root, resolved per-account (C#
        // `CurseForgeService.ComMojangRoot`). Passing `None` here left the
        // instance system unable to seed a default instance from — or swap
        // folders with — the real game install.
        let live_root = gl_bedrock::com_mojang_root();
        let bedrock_instances = BedrockInstanceService::new(root.join("bedrock"), Some(live_root))
            .map_err(|e| format!("failed to initialize Bedrock instances: {e}"))?;

        // Short timeouts here: this client is for API-shaped calls (auth
        // exchanges, CurseForge/Modrinth/GitHub lookups, Discord OAuth) that
        // should fail fast rather than hang the UI indefinitely on a stalled
        // connection. Actual file downloads use their own client
        // (`gl_download::DownloadClient::new()`, below) with a much longer
        // timeout suited to large transfers — sharing this one for both
        // would either make downloads time out early or leave API calls
        // able to hang for minutes.
        let http = reqwest::Client::builder()
            .user_agent(gl_download::USER_AGENT)
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("failed to build HTTP client: {e}"))?;

        // QOL: user-configurable concurrent-download limit (Settings ->
        // Downloads). Read once here rather than made live-adjustable -
        // shrinking a running `tokio::sync::Semaphore` safely mid-flight
        // isn't something the crate supports, so a change takes effect on
        // the next launch, same as e.g. `RememberWindowSize`'s window-size
        // fields already do.
        let max_concurrent_downloads = gl_settings::load()
            .map(|s| s.max_concurrent_downloads)
            .unwrap_or(4)
            .max(1) as usize;
        // Its own client, not `http.clone()`: downloads need a generous
        // multi-minute per-request timeout for large files, whereas `http`
        // above is deliberately short-timeout for API calls. See the
        // comment on `http`'s construction.
        let download_client = gl_download::DownloadClient::new()
            .map_err(|e| format!("failed to build download HTTP client: {e}"))?;
        let downloads = gl_download::DownloadQueue::new(download_client, max_concurrent_downloads);

        Ok(Self {
            java_instances: Mutex::new(java_instances),
            java_versions: tokio::sync::Mutex::new(java_versions),
            bedrock_instances: Mutex::new(bedrock_instances),
            notifications: NotificationService::new(),
            themes: ThemeService::new(),
            discord: Mutex::new(DiscordRpcService::new()),
            http,
            downloads,
            clients_dir: root.join("clients"),
            live_sample_baseline: Mutex::new(Default::default()),
            java_processes: gl_launch::ProcessRegistry::new(),
            sync_engine: gl_sync::SyncEngine::new(root.join("sync"))
                .map_err(|e| format!("failed to initialize sync packs: {e}"))?,
        })
    }
}
