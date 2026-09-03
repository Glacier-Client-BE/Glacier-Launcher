//! Discord RPC, announcements/news, VanillaTweaks, skins, server ping, misc GitHub-backed content catalogs.
//!
//! This crate ports the "integrations" services from the original C# WPF
//! launcher (`GlacierLauncher.Services`). Each module below corresponds
//! roughly 1:1 with a C# service file; see the module-level doc comments
//! for specific behavioral notes and deviations.

pub mod announcements;
pub mod discord;
pub mod error;
pub mod models;
pub mod news;
pub mod server_ping;
pub mod skin_library;
pub mod skins;
pub mod tweaks;

pub use error::{IntegrationError, Result};
