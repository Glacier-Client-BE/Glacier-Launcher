//! Modpack install plus CurseForge/Modrinth API clients (ModpackInstallService, CurseForgeService, ModrinthService)

pub mod curseforge;
pub mod dependencies;
pub mod error;
pub mod export;
pub mod hash;
pub mod jar_inspect;
pub mod loaders;
pub mod manifest;
pub mod models;
pub mod modrinth;
pub mod pack_preview;
pub mod unified;

pub use curseforge::CurseForgeClient;
pub use dependencies::{resolve_curseforge_plan, resolve_modrinth_plan, InstallPlan, PlannedFile};
pub use error::{ModpackError, Result};
pub use export::curseforge::export_curseforge;
pub use export::mrpack::export_mrpack;
pub use export::ExportOptions;
pub use hash::{sha1_and_sha512_hex_file, sha1_hex_file, sha512_hex_file};
pub use loaders::LoaderClient;
pub use modrinth::ModrinthClient;
pub use pack_preview::{preview_pack, PackPreview};
pub use unified::{fetch_versions, search_mods, Provider, UnifiedMod, UnifiedVersion};
