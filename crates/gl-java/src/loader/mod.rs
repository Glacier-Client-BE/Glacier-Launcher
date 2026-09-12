//! Real Forge/NeoForge install-profile execution (Phase 2 of the NoRiskClient
//! -> Glacier port): parses a downloaded installer jar's `install_profile.json`,
//! resolves and downloads every referenced library, and runs the declared
//! `processors` for real instead of shelling out to the installer jar's own
//! (Swing) installer UI/CLI as `mod_loader::run_loader_installer` did.

pub mod install_profile;
pub mod maven;
pub mod patcher;
pub mod processors;
