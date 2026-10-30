//! Phase 4: querying/controlling the set of currently-running Java
//! instances (`gl_launch::ProcessRegistry`, fed by `commands::launch`'s
//! existing stdout/stderr streaming loop — see that module's
//! `stream_child_output`), plus local, pattern-based crash-log analysis.
//!
//! Crash analysis here is deliberately a flat ordered list of
//! (signature, category, fix) checks, not a rules engine — see the task
//! brief: this is the vendor-neutral idea worth taking from
//! NoRiskClient's crash handling (local categorization), not its
//! server-backed "fix database", which is out of scope.

use gl_launch::RunningProcess;
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

/// Every Java instance the process registry currently considers running.
/// Mirrors `commands::downloads::list_downloads`'s "just serialize shared
/// state" shape.
#[tauri::command]
pub fn list_running_instances(state: State<'_, AppState>) -> Vec<RunningProcess> {
    state.java_processes.list()
}

/// Force-kills a tracked instance's process tree (same `taskkill /T /F` /
/// `kill -9` approach `commands::launch::cancel_launch` already uses for the
/// single "last launched" PID) and removes it from the registry.
#[tauri::command]
pub async fn kill_instance(state: State<'_, AppState>, instance_id: String) -> Result<(), String> {
    state.java_processes.kill(&instance_id)
}

/// The buffered tail (up to `gl_launch::process_state::LOG_BUFFER_CAP` lines)
/// of a currently- (or just-)running instance's stdout/stderr. Returns an
/// empty list if the instance isn't tracked.
#[tauri::command]
pub fn get_instance_log_buffer(state: State<'_, AppState>, instance_id: String) -> Vec<String> {
    state.java_processes.get_log_buffer(&instance_id)
}

#[derive(Debug, Clone, Serialize)]
pub struct CrashAnalysis {
    pub category: String,
    pub message: String,
    pub suggested_fix: String,
}

/// One entry in the ordered signature table `analyze_crash` walks top to
/// bottom, first match wins. `find` returns the analysis if `needles` (all
/// case-sensitive substrings — crash logs consistently use exact exception/
/// class names) match, deriving any needed detail out of `text` itself.
type Matcher = fn(&str) -> Option<CrashAnalysis>;

const MATCHERS: &[Matcher] = &[
    match_oom,
    match_missing_dependency,
    match_wrong_java_version,
    match_mixin_failure,
    match_gpu_driver,
    match_corrupt_jar,
];

/// Reads `log_path` and runs it through the local pattern table. Returns a
/// generic "unrecognised" analysis (not an error) when nothing matches —
/// callers should still let the user open/share the raw log.
#[tauri::command]
pub async fn analyze_crash(log_path: String) -> Result<CrashAnalysis, String> {
    let text = std::fs::read_to_string(&log_path)
        .map_err(|e| format!("Couldn't read {log_path}: {e}"))?;

    for matcher in MATCHERS {
        if let Some(analysis) = matcher(&text) {
            return Ok(analysis);
        }
    }

    Ok(CrashAnalysis {
        category: "Unrecognised".to_string(),
        message: "This log doesn't match any known crash signature.".to_string(),
        suggested_fix:
            "Open the full log for details, or share it to mclo.gs and ask for help with the link."
                .to_string(),
    })
}

fn match_oom(text: &str) -> Option<CrashAnalysis> {
    if text.contains("OutOfMemoryError") {
        return Some(CrashAnalysis {
            category: "Out of memory".to_string(),
            message: "The JVM ran out of heap space (OutOfMemoryError).".to_string(),
            suggested_fix:
                "Raise the instance's max RAM in Settings → Java, or disable mods/resource packs that use a lot of memory."
                    .to_string(),
        });
    }
    None
}

fn match_missing_dependency(text: &str) -> Option<CrashAnalysis> {
    for needle in ["NoClassDefFoundError", "ClassNotFoundException"] {
        if let Some(pos) = text.find(needle) {
            let class = extract_after(&text[pos..], needle);
            let mod_hint = class
                .as_deref()
                .and_then(guess_mod_from_package)
                .unwrap_or_else(|| "a required mod".to_string());
            return Some(CrashAnalysis {
                category: "Missing dependency".to_string(),
                message: format!(
                    "{needle}{}",
                    class
                        .as_deref()
                        .map(|c| format!(": {c}"))
                        .unwrap_or_default()
                ),
                suggested_fix: format!(
                    "Looks like {mod_hint} is missing a dependency. Check the mod's page for required libraries (e.g. Fabric API / a core library mod) and install them."
                ),
            });
        }
    }
    None
}

fn match_wrong_java_version(text: &str) -> Option<CrashAnalysis> {
    text.find("UnsupportedClassVersionError")?;
    // Typical message shape: "... has been compiled by a more recent version
    // of the Java Runtime (class file version 65.0), this version of the
    // Java Runtime only recognizes class file versions up to 61.0" — the
    // "class file version " marker can appear either before or after the
    // exception name itself depending on how the log wraps the message, so
    // search the whole text rather than only after the exception name.
    let required_major = text
        .find("class file version ")
        .and_then(|p| {
            let rest = &text[p + "class file version ".len()..];
            let major_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            major_str.parse::<u32>().ok()
        })
        // Class file major 44 == Java 1.0; Java N is class file major (N + 44).
        .map(|class_major| class_major.saturating_sub(44));

    Some(CrashAnalysis {
        category: "Wrong Java version".to_string(),
        message: match required_major {
            Some(major) => format!(
                "This version needs a newer Java (major version {major}) than the one currently selected."
            ),
            None => "This version needs a newer Java than the one currently selected.".to_string(),
        },
        suggested_fix: "Set a matching Java runtime in Settings → Java Runtime (or let Glacier auto-select one for this instance)."
            .to_string(),
    })
}

fn match_mixin_failure(text: &str) -> Option<CrashAnalysis> {
    if !text.contains("Mixin apply failed") && !text.contains("MixinApplicatorStandard") {
        return None;
    }
    // Best-effort: mixin failure lines commonly look like
    // "Mixin apply failed <modid>.mixins.json:SomeMixin -> ..." — grab the
    // modid prefix off the ".mixins.json" config name if present.
    let mod_name = text
        .find(".mixins.json")
        .and_then(|end| {
            let before = &text[..end];
            let start = before
                .rfind(|c: char| c.is_whitespace() || c == '[' || c == '(')
                .map(|i| i + 1)
                .unwrap_or(0);
            let candidate = before[start..].trim();
            if candidate.is_empty() {
                None
            } else {
                Some(candidate.to_string())
            }
        });

    Some(CrashAnalysis {
        category: "Mixin failure".to_string(),
        message: match &mod_name {
            Some(name) => format!("A Mixin transform failed to apply ({name})."),
            None => "A Mixin transform failed to apply.".to_string(),
        },
        suggested_fix: match &mod_name {
            Some(name) => format!(
                "{name} is likely incompatible with this Minecraft/loader version. Update or remove it, or check for a compatible build."
            ),
            None => "A mod's Mixin is incompatible with this Minecraft/loader version. Try updating mods, or remove the most recently added one."
                .to_string(),
        },
    })
}

fn match_gpu_driver(text: &str) -> Option<CrashAnalysis> {
    if text.contains("GLFW") || text.contains("LWJGL") || text.contains("OpenGL") {
        return Some(CrashAnalysis {
            category: "GPU / driver".to_string(),
            message: "The crash mentions GLFW/LWJGL/OpenGL — likely a graphics driver issue."
                .to_string(),
            suggested_fix:
                "Update your GPU driver, and if using a laptop with hybrid graphics, force Minecraft to run on the dedicated GPU rather than the integrated one."
                    .to_string(),
        });
    }
    None
}

fn match_corrupt_jar(text: &str) -> Option<CrashAnalysis> {
    for needle in ["zip END header not found", "Invalid CEN header"] {
        if let Some(pos) = text.find(needle) {
            let jar = find_nearby_jar_name(text, pos);
            return Some(CrashAnalysis {
                category: "Corrupt jar".to_string(),
                message: format!(
                    "{needle}{}",
                    jar.as_deref()
                        .map(|j| format!(" (likely {j})"))
                        .unwrap_or_default()
                ),
                suggested_fix: match &jar {
                    Some(j) => format!("{j} appears to be corrupt or truncated. Delete it and reinstall/redownload the mod or library it belongs to."),
                    None => "A jar on the classpath appears to be corrupt or truncated. Try reinstalling the version, or removing recently added mods one at a time.".to_string(),
                },
            });
        }
    }
    None
}

/// Extracts the token immediately following `needle` in `text` (typically a
/// fully-qualified class name after "NoClassDefFoundError: " or similar).
fn extract_after(text: &str, needle: &str) -> Option<String> {
    let rest = text.strip_prefix(needle)?;
    let trimmed = rest.trim_start_matches([':', ' ']);
    let token: String = trimmed
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != ',' && *c != ')')
        .collect();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// Very small heuristic: turn a package-shaped class name into a guess at
/// which mod it belongs to (its first non-generic path segment), so the
/// suggested fix can name something more useful than "a required mod".
fn guess_mod_from_package(class_name: &str) -> Option<String> {
    let generic = ["java", "javax", "net", "com", "org", "sun", "jdk"];
    let mut parts = class_name.split('.');
    let mut first = parts.next()?;
    if generic.contains(&first) {
        first = parts.next()?;
    }
    Some(format!("'{first}'"))
}

/// Scans backward/forward a small window around `pos` for something that
/// looks like a `.jar` filename, for the corrupt-jar message.
fn find_nearby_jar_name(text: &str, pos: usize) -> Option<String> {
    let window_start = pos.saturating_sub(200);
    let window_end = (pos + 200).min(text.len());
    let window = &text[window_start..window_end];
    let jar_idx = window.find(".jar")?;
    let before = &window[..jar_idx];
    let start = before
        .rfind(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '(' || c == '[' || c == '\\')
        .map(|i| i + 1)
        .unwrap_or(0);
    let name = &window[start..jar_idx + 4];
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_text(text: &str) -> CrashAnalysis {
        for matcher in MATCHERS {
            if let Some(a) = matcher(text) {
                return a;
            }
        }
        CrashAnalysis {
            category: "Unrecognised".to_string(),
            message: String::new(),
            suggested_fix: String::new(),
        }
    }

    #[test]
    fn detects_oom() {
        let a = analyze_text("Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space");
        assert_eq!(a.category, "Out of memory");
    }

    #[test]
    fn detects_missing_class() {
        let a = analyze_text("Caused by: java.lang.NoClassDefFoundError: com/example/somemod/SomeClass");
        assert_eq!(a.category, "Missing dependency");
        assert!(a.suggested_fix.contains("com"));
    }

    #[test]
    fn detects_wrong_java_version() {
        let a = analyze_text(
            "has been compiled by a more recent version of the Java Runtime (class file version 65.0), java.lang.UnsupportedClassVersionError",
        );
        assert_eq!(a.category, "Wrong Java version");
        assert!(a.message.contains("21"));
    }

    #[test]
    fn detects_mixin_failure() {
        let a = analyze_text("org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError: Mixin apply failed examplemod.mixins.json:MyMixin -> net.minecraft.Foo");
        assert_eq!(a.category, "Mixin failure");
        assert!(a.message.contains("examplemod"));
    }

    #[test]
    fn detects_gpu_driver() {
        let a = analyze_text("org.lwjgl.LWJGLException: Pixel format not accelerated GLFW error 65542");
        assert_eq!(a.category, "GPU / driver");
    }

    #[test]
    fn detects_corrupt_jar() {
        let a = analyze_text("java.util.zip.ZipException: zip END header not found in libs/somemod-1.0.jar");
        assert_eq!(a.category, "Corrupt jar");
    }

    #[test]
    fn falls_back_to_unrecognised() {
        let a = analyze_text("everything is fine, nothing crashed here");
        assert_eq!(a.category, "Unrecognised");
    }

    #[test]
    fn matcher_order_prefers_oom_over_gpu_mentions() {
        // If a log mentions both OOM and LWJGL, OOM (listed first) should win.
        let a = analyze_text("java.lang.OutOfMemoryError: Java heap space\norg.lwjgl.LWJGLException: driver reset");
        assert_eq!(a.category, "Out of memory");
    }
}
