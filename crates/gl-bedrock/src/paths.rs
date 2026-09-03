//! com.mojang root resolution — the port of the C# `CurseForgeService`
//! static path members (`UsersRoot`, `ResolveBestRootFor`, `ComMojangRoot`,
//! `LiveRootFor`, and the per-folder `*Dir` properties).
//!
//! # Why this lives here
//!
//! Modern Bedrock installs keep game data under a **per-Xbox-account** folder:
//!
//! ```text
//! %APPDATA%\Minecraft Bedrock\Users\<account>\games\com.mojang\<subfolder>
//! ```
//!
//! There can be several account folders — a signed-in account, a leftover
//! local one, plus `Shared` — and *different subfolders can be most-populated
//! under different accounts*. The original therefore resolves **each subfolder
//! independently**, picking the account whose copy of that specific subfolder
//! holds the most entries, and falling back to `Shared` when nothing matches.
//! Resolving one root for everything is precisely the bug the original's
//! own history had to hotfix, so this port keeps the per-subfolder behaviour.
//!
//! The services in this crate deliberately take their filesystem roots as
//! parameters (so they stay testable against temp dirs); this module is what
//! supplies those roots for the *live* game install.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `%APPDATA%\Minecraft Bedrock\Users` — the parent of the per-account
/// folders. Mirrors the C# `UsersRoot` property, which builds the same path
/// from `Environment.SpecialFolder.ApplicationData` (the *Roaming* AppData
/// folder, which is what `%APPDATA%` expands to).
pub fn users_root() -> PathBuf {
    let appdata = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_default();
    appdata.join("Minecraft Bedrock").join("Users")
}

/// The `Shared` fallback root, used when no account folder contains the
/// requested subfolder. Mirrors the C# `sharedRoot` local.
pub fn shared_root_in(users_root: &Path) -> PathBuf {
    users_root.join("Shared").join("games").join("com.mojang")
}

/// Pure, testable core of [`resolve_best_root_for`]: given an explicit
/// `users_root`, pick the best `com.mojang` root for `sub_folder_name`.
///
/// Faithful to the C# algorithm, including its edge cases:
/// - a candidate is only considered if `<candidate>/<sub_folder_name>`
///   **exists**;
/// - it wins on a **strictly greater** entry count (`count > best_count`),
///   so the first candidate encountered wins a tie, and a subfolder that
///   exists but is *empty* (count 0) never wins — matching `bestCount = 0`;
/// - the count is a non-recursive count of **all** entries (files *and*
///   directories), matching `Directory.EnumerateFileSystemEntries().Count()`;
/// - any I/O failure falls through to the `Shared` root rather than
///   propagating, matching the original's bare `catch { }`.
///
/// Deviation, deliberate: candidate account folders are visited in sorted
/// order. `Directory.EnumerateDirectories` returns NTFS index order (in
/// practice alphabetical) whereas `read_dir` guarantees no order at all, so
/// sorting is what actually reproduces the C# tie-breaking behaviour
/// deterministically instead of leaving it to the filesystem.
pub fn resolve_best_root_in(users_root: &Path, sub_folder_name: &str) -> PathBuf {
    let shared = shared_root_in(users_root);

    let Ok(entries) = std::fs::read_dir(users_root) else {
        return shared;
    };

    let mut user_dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    user_dirs.sort();

    let mut best: Option<PathBuf> = None;
    let mut best_count = 0usize;

    for user_dir in user_dirs {
        let candidate = user_dir.join("games").join("com.mojang");
        let target = candidate.join(sub_folder_name);
        if !target.is_dir() {
            continue;
        }

        let Ok(target_entries) = std::fs::read_dir(&target) else {
            continue;
        };
        let count = target_entries.filter_map(|e| e.ok()).count();

        if count > best_count {
            best_count = count;
            best = Some(candidate);
        }
    }

    best.unwrap_or(shared)
}

/// Resolves the best `com.mojang` root for `sub_folder_name` under the real
/// `%APPDATA%` users root. Port of C# `ResolveBestRootFor`.
pub fn resolve_best_root_for(sub_folder_name: &str) -> PathBuf {
    resolve_best_root_in(&users_root(), sub_folder_name)
}

/// The primary `com.mojang` root, resolved via the `minecraftWorlds`
/// subfolder. Port of the C# `public static readonly string ComMojangRoot`.
///
/// The C# field is a `static readonly` initialised once at type-load, so it
/// is resolved a single time per process; this uses a `OnceLock` to match
/// that caching exactly. Note the asymmetry in the original, preserved here:
/// this root is cached, while the per-folder `*_dir` helpers below re-resolve
/// on every call because they were C# *properties*.
pub fn com_mojang_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| resolve_best_root_for("minecraftWorlds"))
}

/// Port of C# `LiveRootFor`: pack folders resolve independently; everything
/// else uses the cached [`com_mojang_root`].
pub fn live_root_for(folder_name: &str) -> PathBuf {
    match folder_name {
        "resource_packs"
        | "behavior_packs"
        | "skin_packs"
        | "development_resource_packs"
        | "development_behavior_packs"
        | "development_skin_packs" => resolve_best_root_for(folder_name),
        _ => com_mojang_root().to_path_buf(),
    }
}

/// The live directory for `folder_name`, i.e. its resolved root joined with
/// the folder itself. This is the shape every C# `*Dir` property takes
/// (`Path.Combine(ResolveBestRootFor(x), x)`), and the value the Bedrock
/// services expect to be handed.
pub fn live_dir_for(folder_name: &str) -> PathBuf {
    live_root_for(folder_name).join(folder_name)
}

/// `<com.mojang>/minecraftWorlds` — C# `WorldsDir`.
pub fn worlds_dir() -> PathBuf {
    com_mojang_root().join("minecraftWorlds")
}

/// `<com.mojang>/world_templates` — C# `WorldTemplatesDir`.
pub fn world_templates_dir() -> PathBuf {
    com_mojang_root().join("world_templates")
}

/// `<resolved>/minecraftpe` — C# `MinecraftPeDir`.
pub fn minecraft_pe_dir() -> PathBuf {
    live_dir_for("minecraftpe")
}

/// `<com.mojang>/Screenshots`. The original's screenshot service reads from
/// the primary root rather than resolving `Screenshots` independently.
pub fn screenshots_dir() -> PathBuf {
    com_mojang_root().join("Screenshots")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gl-paths-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Creates `<users>/<account>/games/com.mojang/<sub>` with `n` entries.
    fn seed(users: &Path, account: &str, sub: &str, n: usize) -> PathBuf {
        let root = users.join(account).join("games").join("com.mojang");
        let target = root.join(sub);
        fs::create_dir_all(&target).unwrap();
        for i in 0..n {
            fs::write(target.join(format!("e{i}.txt")), b"x").unwrap();
        }
        root
    }

    #[test]
    fn falls_back_to_shared_when_users_root_is_absent() {
        let users = scratch("absent").join("nope");
        assert_eq!(
            resolve_best_root_in(&users, "minecraftWorlds"),
            shared_root_in(&users)
        );
    }

    #[test]
    fn falls_back_to_shared_when_no_account_has_the_subfolder() {
        let users = scratch("nomatch");
        seed(&users, "AccountA", "behavior_packs", 3);
        assert_eq!(
            resolve_best_root_in(&users, "minecraftWorlds"),
            shared_root_in(&users)
        );
    }

    #[test]
    fn picks_the_account_with_the_most_entries() {
        let users = scratch("most");
        seed(&users, "AccountA", "minecraftWorlds", 1);
        let b = seed(&users, "AccountB", "minecraftWorlds", 7);
        seed(&users, "AccountC", "minecraftWorlds", 4);
        assert_eq!(resolve_best_root_in(&users, "minecraftWorlds"), b);
    }

    /// The bug this whole module exists to prevent: different subfolders can
    /// legitimately resolve to *different* account roots.
    #[test]
    fn resolves_each_subfolder_independently() {
        let users = scratch("independent");
        let a = seed(&users, "AccountA", "minecraftWorlds", 5);
        seed(&users, "AccountA", "resource_packs", 1);
        let b = seed(&users, "AccountB", "resource_packs", 9);
        seed(&users, "AccountB", "minecraftWorlds", 2);

        assert_eq!(resolve_best_root_in(&users, "minecraftWorlds"), a);
        assert_eq!(resolve_best_root_in(&users, "resource_packs"), b);
    }

    /// `count > best_count` with `best_count` starting at 0 means an existing
    /// but empty subfolder never wins.
    #[test]
    fn empty_subfolder_never_wins() {
        let users = scratch("empty");
        seed(&users, "AccountA", "skin_packs", 0);
        assert_eq!(
            resolve_best_root_in(&users, "skin_packs"),
            shared_root_in(&users)
        );
    }

    /// Strictly-greater comparison: on a tie the first (sorted) account wins.
    #[test]
    fn ties_go_to_the_first_account_in_sorted_order() {
        let users = scratch("tie");
        let a = seed(&users, "AccountA", "minecraftWorlds", 3);
        seed(&users, "AccountB", "minecraftWorlds", 3);
        assert_eq!(resolve_best_root_in(&users, "minecraftWorlds"), a);
    }

    /// Directories count toward the total, not just files.
    #[test]
    fn counts_directories_as_entries() {
        let users = scratch("dirs");
        seed(&users, "AccountA", "minecraftWorlds", 1);
        let b = users
            .join("AccountB")
            .join("games")
            .join("com.mojang")
            .join("minecraftWorlds");
        fs::create_dir_all(b.join("w1")).unwrap();
        fs::create_dir_all(b.join("w2")).unwrap();
        fs::create_dir_all(b.join("w3")).unwrap();

        assert_eq!(
            resolve_best_root_in(&users, "minecraftWorlds"),
            users.join("AccountB").join("games").join("com.mojang")
        );
    }

    #[test]
    fn live_root_for_routes_packs_independently_and_others_to_primary() {
        // Pure routing check that doesn't touch %APPDATA%: the pack arms must
        // be the six dev/non-dev pack folders and nothing else.
        for f in [
            "resource_packs",
            "behavior_packs",
            "skin_packs",
            "development_resource_packs",
            "development_behavior_packs",
            "development_skin_packs",
        ] {
            assert_eq!(live_dir_for(f), live_root_for(f).join(f));
        }
        assert_eq!(live_root_for("minecraftWorlds"), com_mojang_root());
        assert_eq!(live_root_for("Screenshots"), com_mojang_root());
    }
}
