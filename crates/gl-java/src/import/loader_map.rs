//! Map each external launcher's own loader-naming convention to
//! [`ModLoaderKind`]. Every function is a best-effort string match over
//! well-known component/component-id names; an unrecognized name maps to
//! `None` (treated as vanilla) rather than guessing.

use crate::models::ModLoaderKind;

/// MultiMC/Prism `mmc-pack.json` `components[].uid` values, e.g.
/// `net.fabricmc.fabric-loader`, `org.quiltmc.quilt-loader`,
/// `net.minecraftforge`, `net.neoforged`.
pub fn from_mmc_component_uid(uid: &str) -> Option<ModLoaderKind> {
    match uid {
        "net.fabricmc.fabric-loader" => Some(ModLoaderKind::Fabric),
        "org.quiltmc.quilt-loader" => Some(ModLoaderKind::Quilt),
        "net.minecraftforge" => Some(ModLoaderKind::Forge),
        "net.neoforged" => Some(ModLoaderKind::NeoForge),
        _ => None,
    }
}

/// CurseForge app `minecraftinstance.json` `baseModLoader.name`, typically
/// something like `"forge-47.2.0"` or `"fabric-0.15.7"`.
pub fn from_curseforge_loader_name(name: &str) -> Option<ModLoaderKind> {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("neoforge") {
        Some(ModLoaderKind::NeoForge)
    } else if lower.starts_with("forge") {
        Some(ModLoaderKind::Forge)
    } else if lower.starts_with("fabric") {
        Some(ModLoaderKind::Fabric)
    } else if lower.starts_with("quilt") {
        Some(ModLoaderKind::Quilt)
    } else {
        None
    }
}

/// ATLauncher `instance.json` `launcher.loaderVersion.type`, one of
/// `"Forge"`, `"NeoForge"`, `"Fabric"`, `"Quilt"`.
pub fn from_atlauncher_loader_type(kind: &str) -> Option<ModLoaderKind> {
    match kind.to_ascii_lowercase().as_str() {
        "forge" => Some(ModLoaderKind::Forge),
        "neoforge" => Some(ModLoaderKind::NeoForge),
        "fabric" => Some(ModLoaderKind::Fabric),
        "quilt" => Some(ModLoaderKind::Quilt),
        _ => None,
    }
}

/// Modrinth App `profile.json` `loader` field: `"vanilla"`, `"fabric"`,
/// `"quilt"`, `"forge"`, `"neoforge"`.
pub fn from_modrinth_loader(loader: &str) -> Option<ModLoaderKind> {
    match loader.to_ascii_lowercase().as_str() {
        "fabric" => Some(ModLoaderKind::Fabric),
        "quilt" => Some(ModLoaderKind::Quilt),
        "forge" => Some(ModLoaderKind::Forge),
        "neoforge" => Some(ModLoaderKind::NeoForge),
        _ => None,
    }
}
