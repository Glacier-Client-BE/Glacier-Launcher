//! Client-jar binary patching for Forge/NeoForge.
//!
//! **Scope note (read before extending):** for every Forge/NeoForge release
//! this launcher targets (1.13+, i.e. every version with a `processors`-based
//! `install_profile.json`), the actual client-jar patch step is *not* special
//! -- it is just another entry in the `processors` array, typically running
//! `net.minecraftforge.binarypatcher.ConsoleTool` with `--clean`/`--output`/
//! `--apply` args pointing at a `[net.minecraftforge:forge:<ver>:clientdata@lzma]`
//! artifact. [`super::processors::run_processors`] executes that processor
//! like any other; this module does not need to (and does not) reimplement
//! the LZMA binpatch format itself.
//!
//! What this module *does* do is guard against the one case that generic
//! processor execution can't handle: legacy (pre-1.13) Forge installers,
//! which used a completely different `install_profile.json` shape (an
//! `install`/`versionInfo` pair, no `processors`/`data` map, and a
//! `binpatches.pack.lzma` applied by installer-internal Java code we do not
//! run at all). Installing those is explicitly **not implemented** — see
//! `PARITY.md` for the resulting version-range gap. Silently attempting to
//! run a legacy profile through the modern processor pipeline would fail
//! confusingly deep inside processor resolution instead of with a clear
//! message up front, so callers should check [`requires_legacy_patching`]
//! right after parsing `install_profile.json` and stop there if it returns
//! true.

use serde_json::Value;

/// Returns `true` if `install_profile_json` is the legacy (pre-1.13) Forge
/// installer format that this launcher does not support: no `processors`
/// array, but an `install` object naming a `filePath`/`path` patch archive.
pub fn requires_legacy_patching(install_profile_json: &Value) -> bool {
    let is_modern = install_profile_json.get("processors").is_some()
        && install_profile_json.get("data").is_some();
    if is_modern {
        return false;
    }
    install_profile_json.get("install").is_some() || install_profile_json.get("versionInfo").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn modern_profile_does_not_require_legacy_patching() {
        let modern = json!({
            "processors": [],
            "data": {},
            "version": "1.20.1-47.2.0",
            "minecraft": "1.20.1"
        });
        assert!(!requires_legacy_patching(&modern));
    }

    #[test]
    fn legacy_profile_is_detected() {
        // Synthetic fixture shaped like a pre-1.13 Forge install_profile.json
        // (real ones additionally nest a full vanilla `versionInfo` block,
        // which we don't need for the detection to work).
        let legacy = json!({
            "install": {
                "profileName": "forge",
                "filePath": "forge-1.12.2-14.23.5.2860-universal.jar"
            },
            "versionInfo": {
                "id": "1.12.2-forge1.12.2-14.23.5.2860"
            }
        });
        assert!(requires_legacy_patching(&legacy));
    }

    #[test]
    fn unrelated_json_is_not_flagged() {
        assert!(!requires_legacy_patching(&json!({"foo": "bar"})));
    }
}
