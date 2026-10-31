//! Maven coordinate parsing, used to resolve Forge/NeoForge `install_profile.json`
//! library and processor-classpath entries to on-disk paths / download URLs.
//!
//! Coordinate shape: `group:artifact:version[:classifier][@extension]`, e.g.
//! `net.minecraftforge:forge:1.20.1-47.2.0:installer@jar` or the more common
//! bare `de.oceanlabs.mcp:mcp_config:1.20.1`.

use crate::error::{JavaError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenCoordinate {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub extension: String,
}

impl MavenCoordinate {
    /// Parses a maven coordinate string. Returns an error (not a panic) on a
    /// malformed coordinate so a bad `install_profile.json` fails loudly
    /// instead of silently resolving to a garbage path.
    pub fn parse(coord: &str) -> Result<Self> {
        let (coord, extension) = match coord.split_once('@') {
            Some((c, ext)) if !ext.is_empty() => (c, ext.to_string()),
            _ => (coord, "jar".to_string()),
        };
        let parts: Vec<&str> = coord.split(':').collect();
        if parts.len() < 3 || parts.iter().any(|p| p.is_empty()) {
            return Err(JavaError::Other(format!(
                "invalid maven coordinate: '{coord}'"
            )));
        }
        let classifier = parts.get(3).map(|s| s.to_string());
        Ok(Self {
            group: parts[0].to_string(),
            artifact: parts[1].to_string(),
            version: parts[2].to_string(),
            classifier,
            extension,
        })
    }

    /// Relative path under a maven repository root, e.g.
    /// `net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar`.
    pub fn relative_path(&self) -> String {
        let group_path = self.group.replace('.', "/");
        let file = match &self.classifier {
            Some(c) if !c.is_empty() => format!(
                "{}-{}-{}.{}",
                self.artifact, self.version, c, self.extension
            ),
            _ => format!("{}-{}.{}", self.artifact, self.version, self.extension),
        };
        format!("{group_path}/{}/{}/{file}", self.artifact, self.version)
    }

    /// Full download URL against a maven repository base (no trailing slash
    /// required on `base_repo`).
    pub fn url(&self, base_repo: &str) -> String {
        format!(
            "{}/{}",
            base_repo.trim_end_matches('/'),
            self.relative_path()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_coordinate() {
        let c = MavenCoordinate::parse("net.minecraftforge:forge:1.20.1-47.2.0").unwrap();
        assert_eq!(c.group, "net.minecraftforge");
        assert_eq!(c.artifact, "forge");
        assert_eq!(c.version, "1.20.1-47.2.0");
        assert_eq!(c.classifier, None);
        assert_eq!(c.extension, "jar");
        assert_eq!(
            c.relative_path(),
            "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0.jar"
        );
    }

    #[test]
    fn parses_classifier_and_extension() {
        let c =
            MavenCoordinate::parse("net.minecraftforge:forge:1.20.1-47.2.0:installer@jar").unwrap();
        assert_eq!(c.classifier.as_deref(), Some("installer"));
        assert_eq!(c.extension, "jar");
        assert_eq!(
            c.relative_path(),
            "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar"
        );
    }

    #[test]
    fn parses_extension_without_classifier() {
        let c = MavenCoordinate::parse("de.oceanlabs.mcp:mcp_config:1.20.1@zip").unwrap();
        assert_eq!(c.extension, "zip");
        assert_eq!(c.classifier, None);
        assert_eq!(
            c.relative_path(),
            "de/oceanlabs/mcp/mcp_config/1.20.1/mcp_config-1.20.1.zip"
        );
    }

    #[test]
    fn url_joins_base_repo() {
        let c = MavenCoordinate::parse("net.minecraftforge:forge:1.20.1-47.2.0").unwrap();
        assert_eq!(
            c.url("https://maven.minecraftforge.net/"),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0.jar"
        );
    }

    #[test]
    fn rejects_malformed_coordinate() {
        assert!(MavenCoordinate::parse("not-a-coordinate").is_err());
        assert!(MavenCoordinate::parse("a::c").is_err());
    }
}
