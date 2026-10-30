//! Third-party mod client catalogs and installers: LeviLamina, Flarial,
//! Lunar/Badlion, custom/GitHub-backed client libraries.
//!
//! This crate ports the *catalog / metadata / download-target* half of the
//! original C# services (which GitHub release, which asset, what hash to
//! expect, where an install should live). It intentionally does **not**
//! own:
//!
//! - Actual chunked/resumable file downloading — that belongs to
//!   `gl-download`. Anywhere this crate needs bytes off the network for a
//!   real install, it's marked `// TODO: wire to gl-download once
//!   available` and uses a plain `reqwest` call in the meantime.
//! - DLL injection (`InjectionService.cs`) — that's `gl-injection`'s
//!   responsibility and is explicitly out of scope for this crate.

pub mod client_library;
pub mod custom_client_library;
pub mod error;
pub mod flarial;
pub mod github;
pub mod glacier_client;
pub mod hash;
pub mod horizon;
pub mod latite;
pub mod levilamina;
pub mod levilamina_mods;
pub mod lunar_badlion;
pub mod models;
pub mod oderso;

pub use error::{ClientsError, Result};
