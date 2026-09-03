//! Port of `Services/JavaGameLauncher.cs`'s argument/classpath-construction
//! logic, plus a thin process-spawning wrapper.
//!
//! Deviation from the C# original: the original builds one big shell-quoted
//! `Arguments` string (because `ProcessStartInfo.Arguments` is a raw command
//! line on Windows). Here we build a `Vec<String>` of argument tokens instead
//! and hand them to `tokio::process::Command::args`, which performs its own
//! correct Windows argv-quoting per element — so no manual `Quote()` step is
//! needed for spawning. We keep a `quote_arg` helper (see `util.rs`) only for
//! producing a human-readable, shell-like command line for logging.

use crate::error::{io_err, JavaError, Result};
use crate::util::quote_arg;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};

#[derive(Debug, Clone)]
pub struct LibraryEntry {
    pub path: PathBuf,
    pub is_native: bool,
}

#[derive(Debug, Clone, Default)]
pub struct VersionProfile {
    pub id: String,
    pub main_class: String,
    pub assets_index: String,
    pub assets_dir: PathBuf,
    pub libraries: Vec<LibraryEntry>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub min_java_major: String,
    pub jar_version_id: String,
}

/// Read a version JSON (and any `inheritsFrom` ancestors) into a merged
/// `VersionProfile`. Mirrors `ReadVersionProfile`.
pub fn read_version_profile(id: &str, mc_dir: &Path) -> Result<VersionProfile> {
    let mut libraries = Vec::new();
    let mut jvm_args = Vec::new();
    let mut game_args = Vec::new();

    let mut main_class: Option<String> = None;
    let mut assets_idx: Option<String> = None;
    let mut jar_ver_id: Option<String> = None;
    let mut min_java = String::new();

    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut stack = vec![id.to_string()];

    while let Some(cur_id) = stack.pop() {
        let key = cur_id.to_lowercase();
        if !visited.insert(key) {
            continue;
        }

        let json_path = mc_dir
            .join("versions")
            .join(&cur_id)
            .join(format!("{cur_id}.json"));
        if !json_path.is_file() {
            return Err(JavaError::VersionNotInstalled(cur_id));
        }
        let text = std::fs::read_to_string(&json_path).map_err(io_err(&json_path))?;
        let root: Value = serde_json::from_str(&text)?;

        if main_class.is_none() {
            main_class = root
                .get("mainClass")
                .and_then(|v| v.as_str())
                .map(str::to_string);
        }
        if assets_idx.is_none() {
            assets_idx = root
                .get("assets")
                .and_then(|v| v.as_str())
                .map(str::to_string);
        }
        if jar_ver_id.is_none() {
            jar_ver_id = Some(
                root.get("jar")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| cur_id.clone()),
            );
        }

        if min_java.is_empty() {
            if let Some(mj) = root
                .get("javaVersion")
                .and_then(|v| v.get("majorVersion"))
                .and_then(|v| v.as_i64())
            {
                min_java = mj.to_string();
            }
        }

        if let Some(libs) = root.get("libraries").and_then(|v| v.as_array()) {
            for lib in libs {
                if !rules_allow(lib) {
                    continue;
                }

                if let Some(dl) = lib.get("downloads") {
                    if let Some(path) = dl
                        .get("artifact")
                        .and_then(|a| a.get("path"))
                        .and_then(|p| p.as_str())
                    {
                        let p = mc_dir
                            .join("libraries")
                            .join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
                        libraries.push(LibraryEntry {
                            path: p,
                            is_native: false,
                        });
                    }

                    if let (Some(classifiers), Some(win_key)) = (
                        dl.get("classifiers").filter(|c| c.is_object()),
                        lib.get("natives")
                            .and_then(|n| n.get("windows"))
                            .and_then(|w| w.as_str()),
                    ) {
                        let key = win_key.replace("${arch}", "64");
                        if let Some(npath) = classifiers
                            .get(&key)
                            .and_then(|n| n.get("path"))
                            .and_then(|p| p.as_str())
                        {
                            let p = mc_dir
                                .join("libraries")
                                .join(npath.replace('/', std::path::MAIN_SEPARATOR_STR));
                            libraries.push(LibraryEntry {
                                path: p,
                                is_native: true,
                            });
                        }
                    }
                } else if let Some(name) = lib.get("name").and_then(|v| v.as_str()) {
                    if let Some(p) = maven_coord_to_path(mc_dir, name) {
                        libraries.push(LibraryEntry {
                            path: p,
                            is_native: false,
                        });
                    }
                }
            }
        }

        if let Some(args) = root.get("arguments") {
            if let Some(j) = args.get("jvm") {
                collect_args(j, &mut jvm_args);
            }
            if let Some(g) = args.get("game") {
                collect_args(g, &mut game_args);
            }
        } else if let Some(legacy) = root.get("minecraftArguments").and_then(|v| v.as_str()) {
            game_args.extend(
                legacy
                    .split(' ')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        }

        if let Some(parent) = root.get("inheritsFrom").and_then(|v| v.as_str()) {
            if !parent.is_empty() {
                stack.push(parent.to_string());
            }
        }
    }

    let jar_id = jar_ver_id.unwrap_or_else(|| id.to_string());
    let primary = mc_dir
        .join("versions")
        .join(&jar_id)
        .join(format!("{jar_id}.jar"));
    if primary.is_file() {
        libraries.push(LibraryEntry {
            path: primary,
            is_native: false,
        });
    }

    let main_class = main_class.ok_or_else(|| JavaError::MissingMainClass(id.to_string()))?;

    Ok(VersionProfile {
        id: id.to_string(),
        main_class,
        assets_index: assets_idx.unwrap_or_else(|| "legacy".to_string()),
        assets_dir: mc_dir.join("assets"),
        libraries,
        jvm_args,
        game_args,
        min_java_major: min_java,
        jar_version_id: jar_id,
    })
}

fn collect_args(arr: &Value, sink: &mut Vec<String>) {
    let Some(arr) = arr.as_array() else { return };
    for el in arr {
        if let Some(s) = el.as_str() {
            sink.push(s.to_string());
        } else if el.is_object() && rules_allow(el) {
            if let Some(v) = el.get("value") {
                if let Some(s) = v.as_str() {
                    sink.push(s.to_string());
                } else if let Some(arr) = v.as_array() {
                    for s in arr {
                        sink.push(s.as_str().unwrap_or("").to_string());
                    }
                }
            }
        }
    }
}

fn rules_allow(el: &Value) -> bool {
    let Some(rules) = el.get("rules").and_then(|v| v.as_array()) else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        let action = rule
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("allow");
        if !os_matches(rule) {
            continue;
        }
        if !features_match(rule) {
            continue;
        }
        allowed = action == "allow";
    }
    allowed
}

fn os_matches(rule: &Value) -> bool {
    let Some(os) = rule.get("os") else {
        return true;
    };
    if let Some(name) = os.get("name").and_then(|v| v.as_str()) {
        if !name.eq_ignore_ascii_case("windows") {
            return false;
        }
    }
    true
}

fn features_match(rule: &Value) -> bool {
    let Some(features) = rule.get("features") else {
        return true;
    };
    let Some(obj) = features.as_object() else {
        return true;
    };
    obj.is_empty()
}

fn maven_coord_to_path(mc_dir: &Path, coord: &str) -> Option<PathBuf> {
    let (coord, ext) = match coord.split_once('@') {
        Some((c, e)) => (c, e.to_string()),
        None => (coord, "jar".to_string()),
    };
    let parts: Vec<&str> = coord.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = if parts.len() > 3 {
        format!("-{}", parts[3])
    } else {
        String::new()
    };

    Some(
        mc_dir
            .join("libraries")
            .join(group.replace('/', std::path::MAIN_SEPARATOR_STR))
            .join(artifact)
            .join(version)
            .join(format!("{artifact}-{version}{classifier}.{ext}")),
    )
}

/// Non-native library paths, de-duplicated in first-seen order. Mirrors
/// `BuildClasspath`.
pub fn build_classpath(profile: &VersionProfile) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut cp = Vec::with_capacity(profile.libraries.len());
    for lib in &profile.libraries {
        if lib.is_native {
            continue;
        }
        let key = lib.path.to_string_lossy().to_lowercase();
        if seen.insert(key) {
            cp.push(lib.path.clone());
        }
    }
    cp
}

/// Options mirroring the relevant `LauncherSettings`/`JavaInstance` fields
/// consumed by `AppendJvmArgs`/`AppendGameArgs`.
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub max_ram_mb: i32,
    pub min_ram_mb: i32,
    pub custom_jvm_args: String,
    pub extra_jvm_args: Vec<String>,
    pub fullscreen: bool,
    pub use_custom_resolution: bool,
    pub window_width: i32,
    pub window_height: i32,
    pub server_address: String,
    pub server_port: i32,
    pub auth_xuid: String,
}

#[derive(Debug, Clone)]
pub struct AuthValues {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
}

/// Build the JVM-side argument tokens (natives dir, `${...}` substitutions,
/// -Xmx/-Xms, custom/extra args). Mirrors `AppendJvmArgs`, but returns tokens
/// rather than appending to a shell-quoted `StringBuilder`.
pub fn build_jvm_args(
    profile: &VersionProfile,
    mc_dir: &Path,
    version_id: &str,
    actual_java_major: i32,
    opts: &LaunchOptions,
) -> Result<Vec<String>> {
    let natives_dir = mc_dir.join("versions").join(version_id).join("natives");
    std::fs::create_dir_all(&natives_dir).map_err(io_err(&natives_dir))?;

    let mut subs = HashMap::new();
    subs.insert(
        "natives_directory".to_string(),
        natives_dir.to_string_lossy().into_owned(),
    );
    subs.insert("launcher_name".to_string(), "glacier".to_string());
    subs.insert("launcher_version".to_string(), "1.0".to_string());
    subs.insert("classpath".to_string(), "<set-by-cp-flag>".to_string());

    let mut out = Vec::new();

    if !profile.jvm_args.is_empty() {
        for raw in &profile.jvm_args {
            let v = substitute(raw, &subs);
            if v.contains("${classpath}") {
                continue;
            }
            if !is_jvm_arg_compatible(&v, actual_java_major) {
                continue;
            }
            out.push(v);
        }
    } else {
        out.push(format!(
            "-Djava.library.path={}",
            natives_dir.to_string_lossy()
        ));
    }

    let ram = opts.max_ram_mb.clamp(512, 16384);
    let min_ram = opts.min_ram_mb.clamp(256, ram);
    out.push(format!("-Xmx{ram}M"));
    out.push(format!("-Xms{min_ram}M"));

    if !opts.custom_jvm_args.trim().is_empty() {
        // The C# code appends the raw custom-args string verbatim (whitespace
        // separated) into the shell command line; here we split it into
        // discrete tokens since we're building an argv, not a command line.
        out.extend(opts.custom_jvm_args.split_whitespace().map(str::to_string));
    }

    out.extend(opts.extra_jvm_args.iter().cloned());

    Ok(out)
}

/// Build the game-side argument tokens (`${...}` substitutions,
/// fullscreen/resolution, server). Mirrors `AppendGameArgs`.
pub fn build_game_args(
    profile: &VersionProfile,
    mc_dir: &Path,
    version_id: &str,
    auth: &AuthValues,
    opts: &LaunchOptions,
) -> Vec<String> {
    let mut subs = HashMap::new();
    subs.insert("auth_player_name".to_string(), auth.name.clone());
    subs.insert("version_name".to_string(), version_id.to_string());
    subs.insert(
        "game_directory".to_string(),
        mc_dir.to_string_lossy().into_owned(),
    );
    subs.insert(
        "assets_root".to_string(),
        profile.assets_dir.to_string_lossy().into_owned(),
    );
    subs.insert(
        "assets_index_name".to_string(),
        profile.assets_index.clone(),
    );
    subs.insert("auth_uuid".to_string(), auth.uuid.clone());
    subs.insert("auth_access_token".to_string(), auth.access_token.clone());
    subs.insert("clientid".to_string(), "0".to_string());
    subs.insert("auth_xuid".to_string(), opts.auth_xuid.clone());
    subs.insert("user_type".to_string(), auth.user_type.clone());
    subs.insert("version_type".to_string(), "release".to_string());
    subs.insert("user_properties".to_string(), "{}".to_string());
    subs.insert(
        "auth_session".to_string(),
        format!("token:{}:{}", auth.access_token, auth.uuid),
    );
    subs.insert(
        "game_assets".to_string(),
        profile.assets_dir.to_string_lossy().into_owned(),
    );

    let mut out = Vec::new();
    for raw in &profile.game_args {
        let v = substitute(raw, &subs);
        if v.is_empty() {
            continue;
        }
        if v.starts_with("${") && v.ends_with('}') {
            continue;
        }
        out.push(v);
    }

    if opts.fullscreen {
        out.push("--fullscreen".to_string());
    } else if opts.use_custom_resolution {
        out.push("--width".to_string());
        out.push(opts.window_width.clamp(320, 7680).to_string());
        out.push("--height".to_string());
        out.push(opts.window_height.clamp(240, 4320).to_string());
    }

    if !opts.server_address.trim().is_empty() {
        out.push("--server".to_string());
        out.push(opts.server_address.trim().to_string());
        out.push("--port".to_string());
        out.push(opts.server_port.clamp(1, 65535).to_string());
    }

    out
}

fn substitute(template: &str, subs: &HashMap<String, String>) -> String {
    if !template.contains("${") {
        return template.to_string();
    }
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(end_rel) = template[i + 2..].find('}') {
                let key = &template[i + 2..i + 2 + end_rel];
                if let Some(val) = subs.get(key) {
                    out.push_str(val);
                }
                i = i + 2 + end_rel + 1;
                continue;
            }
        }
        // Advance by one char (not byte) to stay UTF-8 safe.
        let ch = template[i..].chars().next().expect("i < len");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn is_jvm_arg_compatible(arg: &str, java_major: i32) -> bool {
    if java_major <= 0 {
        return true;
    }
    if java_major < 23 && arg.starts_with("--sun-misc-unsafe-memory-access") {
        return false;
    }
    true
}

/// Best-effort detection of a JDK/JRE's major version from its `release`
/// file, falling back to a filename heuristic. Mirrors `DetectJavaMajor`.
pub fn detect_java_major(javaw_path: &Path) -> i32 {
    let mut dir = javaw_path.parent().map(Path::to_path_buf);
    for _ in 0..6 {
        let Some(d) = dir else { break };
        let Some(parent) = d.parent() else { break };
        let release = parent.join("release");
        if let Ok(text) = std::fs::read_to_string(&release) {
            for line in text.lines() {
                if !line.to_uppercase().starts_with("JAVA_VERSION") {
                    continue;
                }
                let Some(eq) = line.find('=') else { continue };
                let ver = line[eq + 1..].trim().trim_matches('"');
                if let Some(rest) = ver.strip_prefix("1.") {
                    if let Some(legacy) = rest.split('.').next().and_then(|s| s.parse::<i32>().ok())
                    {
                        return legacy;
                    }
                } else if let Some(modern) =
                    ver.split('.').next().and_then(|s| s.parse::<i32>().ok())
                {
                    return modern;
                }
            }
        }
        dir = Some(parent.to_path_buf());
    }

    heuristic_major_from_path(javaw_path).unwrap_or(0)
}

fn heuristic_major_from_path(path: &Path) -> Option<i32> {
    let s = path.to_string_lossy().to_lowercase();
    // Look for jdk/jre/java/openjdk optionally followed by '-' then digits.
    for marker in ["openjdk", "jdk", "jre", "java"] {
        if let Some(idx) = s.find(marker) {
            let rest = &s[idx + marker.len()..];
            let rest = rest.strip_prefix('-').unwrap_or(rest);
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                if let Ok(n) = digits.parse::<i32>() {
                    if n == 1 {
                        // "...1.NN..." legacy form
                        let after = &rest[digits.len()..];
                        if let Some(after) = after.strip_prefix('.') {
                            let d2: String =
                                after.chars().take_while(|c| c.is_ascii_digit()).collect();
                            if let Ok(legacy) = d2.parse::<i32>() {
                                return Some(legacy);
                            }
                        }
                    }
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Compute the MD5-derived offline UUID for `name`, matching the C#
/// `OfflinePlayer:<name>` MD5-hash-based UUID (variant 3).
pub fn offline_uuid(name: &str) -> String {
    let mut input = Vec::with_capacity(14 + name.len());
    input.extend_from_slice(b"OfflinePlayer:");
    input.extend_from_slice(name.as_bytes());

    let mut hash = md5(&input);
    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;

    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    hex
}

/// Render a human-readable, shell-like command line for logging purposes
/// only (never used for actual process spawning — see module docs).
pub fn render_command_line(
    javaw: &Path,
    jvm_args: &[String],
    classpath: &[PathBuf],
    main_class: &str,
    game_args: &[String],
) -> String {
    let mut parts = vec![quote_arg(&javaw.to_string_lossy())];
    parts.extend(jvm_args.iter().map(|a| quote_arg(a)));
    parts.push("-cp".to_string());
    let cp = classpath
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(";");
    parts.push(quote_arg(&cp));
    parts.push(main_class.to_string());
    parts.extend(game_args.iter().map(|a| quote_arg(a)));
    parts.join(" ")
}

/// Resolve a `javaw.exe` to launch with, mirroring `ResolveJavaRuntime`.
///
/// Deviation from the C# original: the Windows-registry probe (Eclipse
/// Adoptium / Microsoft JDK / JavaSoft uninstall keys) is not ported here —
/// this crate has no registry dependency, and the explicit-path/cached
/// runtime/`JAVA_HOME`/`PATH` checks below cover the common cases. See the
/// crate's top-level launch report for this as a known gap.
///
/// Order: explicit `explicit_path` setting -> cached Glacier-downloaded
/// runtime for `min_java_major` -> any javaw.exe under `runtimes_root` ->
/// `JAVA_HOME` -> `PATH`.
pub fn resolve_java_runtime(
    explicit_path: &str,
    runtimes_root: &Path,
    min_java_major: &str,
) -> Option<PathBuf> {
    if !explicit_path.trim().is_empty() {
        let p = PathBuf::from(explicit_path.trim());
        if p.is_file() {
            return Some(p);
        }
    }

    if let Ok(major) = min_java_major.parse::<u32>() {
        if major > 0 {
            let svc = crate::RuntimeDownloadService::new(runtimes_root.to_path_buf());
            if let Some(p) = svc.get_cached_javaw(major) {
                return Some(p);
            }
        }
    }

    if runtimes_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(runtimes_root) {
            for entry in entries.flatten() {
                if let Some(p) = find_first_javaw_under(&entry.path()) {
                    return Some(p);
                }
            }
        }
    }

    if let Ok(home) = std::env::var("JAVA_HOME") {
        let p = Path::new(&home).join("bin").join("javaw.exe");
        if p.is_file() {
            return Some(p);
        }
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for seg in path_env.split(';') {
            let seg = seg.trim();
            if seg.is_empty() {
                continue;
            }
            let p = Path::new(seg).join("javaw.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }

    None
}

fn find_first_javaw_under(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    let mut stack = vec![dir.to_path_buf()];
    // Depth-unbounded but directory-count-bounded in practice (a runtime
    // component dir), unlike the recursive `EnumerateFiles` in the C#
    // original which relies on the OS to short-circuit — fine for our sizes.
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.eq_ignore_ascii_case("javaw.exe"))
                .unwrap_or(false)
            {
                return Some(p);
            }
        }
    }
    None
}

/// Spawn the game process with piped stdout/stderr. Caller owns streaming
/// the output; this function does not build any UI.
pub fn spawn(
    javaw: &Path,
    jvm_args: &[String],
    classpath: &[PathBuf],
    main_class: &str,
    game_args: &[String],
    working_dir: &Path,
) -> Result<Child> {
    let cp = classpath
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(";");

    let mut cmd = Command::new(javaw);
    cmd.args(jvm_args)
        .arg("-cp")
        .arg(cp)
        .arg(main_class)
        .args(game_args)
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    cmd.spawn()
        .map_err(|e| JavaError::Other(format!("failed to spawn {}: {e}", javaw.display())))
}

/// Minimal MD5 implementation (RFC 1321) so this crate doesn't need an extra
/// dependency purely for the offline-UUID computation.
fn md5(input: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    let mut msg = input.to_vec();
    let orig_len_bits = (input.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&orig_len_bits.to_le_bytes());

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, word) in chunk.chunks(4).enumerate() {
            m[i] = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
        }

        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(K[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}
