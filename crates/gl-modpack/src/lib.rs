//! Modpack install plus CurseForge/Modrinth API clients (ModpackInstallService, CurseForgeService, ModrinthService)

pub mod curseforge;
pub mod error;
pub mod jar_inspect;
pub mod loaders;
pub mod manifest;
pub mod models;
pub mod modrinth;

pub use curseforge::CurseForgeClient;
pub use error::{ModpackError, Result};
pub use loaders::LoaderClient;
pub use modrinth::ModrinthClient;
