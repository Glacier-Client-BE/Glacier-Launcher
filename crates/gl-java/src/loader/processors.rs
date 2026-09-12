//! Executes a Forge/NeoForge `install_profile.json` `processors` array.
//!
//! Each processor is a maven-coordinate jar run via its own manifest
//! `Main-Class` (not a fixed entry point we hardcode), with a classpath and
//! argument list built from a mix of:
//! - `[group:artifact:version...]` — a maven coordinate, resolved to its path
//!   under the libraries dir;
//! - `{FOO}` — a fixed placeholder (`SIDE`, `ROOT`, `INSTALLER`,
//!   `MINECRAFT_JAR`, `LIBRARY_DIR`) or a key into the profile's `data` map;
//! - anything else — used verbatim.
//!
//! A `data` map entry's value is itself either a literal (wrapped in
//! `[...]`, already unwrapped by [`super::install_profile::DataValue`]) or an
//! artifact reference: either a bare maven coordinate, or a path beginning
//! with `/` that must first be extracted from the installer jar into
//! `ROOT/<path>` (this is how legacy-style single-file data like
//! `client.lzma` rides inside the installer).

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{io_err, JavaError, Result};
use crate::loader::install_profile::{DataValue, InstallProfile, Processor};
use crate::loader::maven::MavenCoordinate;

/// Everything a processor run needs to resolve placeholders and build a
/// classpath.
pub struct ProcessorContext {
    /// Working root the installer would have used (`{ROOT}`) — typically the
    /// instance's Minecraft directory root.
    pub root: PathBuf,
    /// `<mc_dir>/libraries`, where every maven-coordinate artifact is
    /// expected to already be downloaded.
    pub libraries_dir: PathBuf,
    /// The Forge/NeoForge installer jar itself (`{INSTALLER}`), also the
    /// source for any `/data/...`-style embedded files.
    pub installer_jar: PathBuf,
    /// The vanilla client jar being patched (`{MINECRAFT_JAR}`).
    pub minecraft_jar: PathBuf,
    /// `"client"` or `"server"` (`{SIDE}`); we only ever run `"client"`.
    pub side: String,
    /// Absolute path to the `java`/`java.exe` binary to run processors with.
    pub java_exe: PathBuf,
}

impl ProcessorContext {
    fn library_path(&self, coord: &str) -> Result<PathBuf> {
        let parsed = MavenCoordinate::parse(coord)?;
        Ok(self.libraries_dir.join(parsed.relative_path()))
    }

    /// Extracts `entry_path` (as it appears inside the installer jar, e.g.
    /// `/data/client.lzma`) to `root/<entry_path>`, returning the
    /// destination path. No-ops (and just returns the path) if it already
    /// exists, since processors may reference the same data file more than
    /// once.
    fn extract_installer_entry(&self, entry_path: &str) -> Result<PathBuf> {
        let relative = entry_path.trim_start_matches('/');
        let dest = self.root.join(relative);
        if dest.is_file() {
            return Ok(dest);
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(io_err(parent))?;
        }
        let file = std::fs::File::open(&self.installer_jar).map_err(io_err(&self.installer_jar))?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut entry = archive.by_name(relative).map_err(|_| {
            JavaError::Other(format!(
                "installer jar {} has no entry '{relative}'",
                self.installer_jar.display()
            ))
        })?;
        let mut out = std::fs::File::create(&dest).map_err(io_err(&dest))?;
        std::io::copy(&mut entry, &mut out).map_err(io_err(&dest))?;
        Ok(dest)
    }

    /// Resolves a `data` map entry (already picked for the current side) to
    /// a concrete string usable as a processor argument.
    fn resolve_data_value(&self, raw: &str) -> Result<String> {
        match DataValue::parse(raw) {
            DataValue::Literal(text) => Ok(text),
            DataValue::Artifact(reference) if reference.starts_with('/') => {
                Ok(self.extract_installer_entry(&reference)?.display().to_string())
            }
            DataValue::Artifact(coord) => Ok(self.library_path(&coord)?.display().to_string()),
        }
    }

    /// Resolves one processor argument (or classpath/jar entry) against
    /// this context and the profile's `data` map.
    fn resolve_token(&self, profile: &InstallProfile, raw: &str) -> Result<String> {
        if let Some(coord) = raw.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            return Ok(self.library_path(coord)?.display().to_string());
        }
        if let Some(key) = raw.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            return match key {
                "SIDE" => Ok(self.side.clone()),
                "ROOT" => Ok(self.root.display().to_string()),
                "INSTALLER" => Ok(self.installer_jar.display().to_string()),
                "MINECRAFT_JAR" => Ok(self.minecraft_jar.display().to_string()),
                "LIBRARY_DIR" => Ok(self.libraries_dir.display().to_string()),
                _ => {
                    let sides = profile.data.get(key).ok_or_else(|| {
                        JavaError::Other(format!(
                            "install profile referenced unknown placeholder '{{{key}}}'"
                        ))
                    })?;
                    self.resolve_data_value(sides.for_side(&self.side))
                }
            };
        }
        Ok(raw.to_string())
    }
}

/// Reads `Main-Class` out of a jar's `META-INF/MANIFEST.MF`, handling the
/// manifest spec's line-continuation rule (a line starting with a single
/// space continues the previous line).
pub fn read_main_class(jar_path: &Path) -> Result<String> {
    let file = std::fs::File::open(jar_path).map_err(io_err(jar_path))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive.by_name("META-INF/MANIFEST.MF").map_err(|_| {
        JavaError::Other(format!(
            "processor jar {} has no META-INF/MANIFEST.MF",
            jar_path.display()
        ))
    })?;
    let mut raw = String::new();
    entry.read_to_string(&mut raw).map_err(io_err(jar_path))?;
    Ok(parse_main_class(&raw).ok_or_else(|| {
        JavaError::Other(format!(
            "processor jar {} manifest has no Main-Class",
            jar_path.display()
        ))
    })?)
}

fn parse_main_class(manifest: &str) -> Option<String> {
    // Unfold continuation lines (leading single space) before parsing.
    let mut unfolded = String::new();
    for line in manifest.split("\r\n").flat_map(|l| l.split('\n')) {
        if let Some(cont) = line.strip_prefix(' ') {
            unfolded.push_str(cont);
        } else {
            if !unfolded.is_empty() {
                unfolded.push('\n');
            }
            unfolded.push_str(line);
        }
    }
    for line in unfolded.lines() {
        if let Some(value) = line.strip_prefix("Main-Class:") {
            return Some(value.trim().to_string());
        }
    }
    None
}

/// Runs every processor in `profile.processors` that applies to
/// `ctx.side`, in order, stopping at the first failure. On failure, the
/// error names the failing processor jar and includes the tail of its
/// stderr so a broken install surfaces a real diagnosis instead of a bare
/// exit code (unlike the old `run_loader_installer` shell-out).
pub async fn run_processors(ctx: &ProcessorContext, profile: &InstallProfile) -> Result<()> {
    for processor in &profile.processors {
        if processor.applies_to_side(&ctx.side) {
            run_one(ctx, profile, processor).await?;
        }
    }
    Ok(())
}

async fn run_one(ctx: &ProcessorContext, profile: &InstallProfile, processor: &Processor) -> Result<()> {
    let jar_path = ctx.library_path(&processor.jar)?;
    if !jar_path.is_file() {
        return Err(JavaError::Other(format!(
            "processor jar not found on disk: {} (expected at {})",
            processor.jar,
            jar_path.display()
        )));
    }
    let main_class = read_main_class(&jar_path)?;

    let mut classpath: Vec<PathBuf> = vec![jar_path.clone()];
    for entry in &processor.classpath {
        classpath.push(ctx.library_path(entry)?);
    }
    let classpath_str = std::env::join_paths(&classpath)
        .map_err(|e| JavaError::Other(format!("could not build processor classpath: {e}")))?;

    let mut args = Vec::with_capacity(processor.args.len());
    for raw in &processor.args {
        args.push(ctx.resolve_token(profile, raw)?);
    }

    let mut cmd = tokio::process::Command::new(&ctx.java_exe);
    cmd.arg("-cp").arg(&classpath_str).arg(&main_class).args(&args);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| JavaError::Other(format!("could not start processor {}: {e}", processor.jar)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr.chars().rev().take(2000).collect::<Vec<_>>().into_iter().rev().collect();
        return Err(JavaError::Other(format!(
            "processor {} ({main_class}) exited with {}: ...{tail}",
            processor.jar, output.status
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_jar_with_manifest(manifest: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let options: zip::write::FileOptions<()> =
                zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
            zip.start_file("META-INF/MANIFEST.MF", options).unwrap();
            zip.write_all(manifest.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn reads_main_class_from_manifest() {
        let jar_bytes = make_jar_with_manifest("Manifest-Version: 1.0\nMain-Class: com.example.Tool\n");
        let dir = tempfile::tempdir().unwrap();
        let jar_path = dir.path().join("tool.jar");
        std::fs::write(&jar_path, jar_bytes).unwrap();
        assert_eq!(read_main_class(&jar_path).unwrap(), "com.example.Tool");
    }

    #[test]
    fn unfolds_continuation_lines() {
        let manifest = "Manifest-Version: 1.0\nMain-Class: com.example.\n LongClassName\n";
        assert_eq!(
            parse_main_class(manifest).unwrap(),
            "com.example.LongClassName"
        );
    }

    #[test]
    fn resolve_token_handles_fixed_placeholders() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ProcessorContext {
            root: dir.path().to_path_buf(),
            libraries_dir: dir.path().join("libraries"),
            installer_jar: dir.path().join("installer.jar"),
            minecraft_jar: dir.path().join("client.jar"),
            side: "client".to_string(),
            java_exe: PathBuf::from("java"),
        };
        let profile = InstallProfile::parse(
            r#"{"version":"1.0","minecraft":"1.20.1","data":{},"processors":[],"libraries":[]}"#,
        )
        .unwrap();
        assert_eq!(ctx.resolve_token(&profile, "{SIDE}").unwrap(), "client");
        assert_eq!(
            ctx.resolve_token(&profile, "{MINECRAFT_JAR}").unwrap(),
            ctx.minecraft_jar.display().to_string()
        );
        assert_eq!(
            ctx.resolve_token(&profile, "[net.minecraftforge:forge:1.0]").unwrap(),
            ctx.libraries_dir
                .join("net/minecraftforge/forge/1.0/forge-1.0.jar")
                .display()
                .to_string()
        );
        assert_eq!(ctx.resolve_token(&profile, "plain-text").unwrap(), "plain-text");
    }

    #[test]
    fn resolve_token_reads_data_map_literal() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ProcessorContext {
            root: dir.path().to_path_buf(),
            libraries_dir: dir.path().join("libraries"),
            installer_jar: dir.path().join("installer.jar"),
            minecraft_jar: dir.path().join("client.jar"),
            side: "client".to_string(),
            java_exe: PathBuf::from("java"),
        };
        let profile = InstallProfile::parse(
            r#"{"version":"1.0","minecraft":"1.20.1","data":{"MAPPINGS":{"client":"[net.minecraftforge:mcp_config:1.0@zip]","server":"[net.minecraftforge:mcp_config:1.0@zip]"}},"processors":[],"libraries":[]}"#,
        )
        .unwrap();
        assert_eq!(
            ctx.resolve_token(&profile, "{MAPPINGS}").unwrap(),
            "net.minecraftforge:mcp_config:1.0@zip"
        );
    }

    #[test]
    fn resolve_token_extracts_installer_relative_data_file() {
        let dir = tempfile::tempdir().unwrap();
        let installer_jar = dir.path().join("installer.jar");
        let jar_bytes = {
            let mut buf = Vec::new();
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            zip.start_file("data/client.lzma", options).unwrap();
            zip.write_all(b"fake-binpatch-bytes").unwrap();
            zip.finish().unwrap();
            buf
        };
        std::fs::write(&installer_jar, jar_bytes).unwrap();

        let ctx = ProcessorContext {
            root: dir.path().join("root"),
            libraries_dir: dir.path().join("libraries"),
            installer_jar,
            minecraft_jar: dir.path().join("client.jar"),
            side: "client".to_string(),
            java_exe: PathBuf::from("java"),
        };
        let profile = InstallProfile::parse(
            r#"{"version":"1.0","minecraft":"1.20.1","data":{"BINPATCH":{"client":"/data/client.lzma","server":"/data/server.lzma"}},"processors":[],"libraries":[]}"#,
        )
        .unwrap();
        let resolved = ctx.resolve_token(&profile, "{BINPATCH}").unwrap();
        let extracted = PathBuf::from(&resolved);
        assert!(extracted.is_file());
        assert_eq!(std::fs::read(&extracted).unwrap(), b"fake-binpatch-bytes");
    }
}
