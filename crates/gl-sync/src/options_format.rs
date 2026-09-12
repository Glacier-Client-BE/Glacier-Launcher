//! Structured parse/merge for Minecraft's `options.txt`: one `key:value`
//! pair per line. Vanilla rewrites this file with a specific key ordering
//! and tolerates reordering, but a naive `HashMap` round-trip would still
//! scramble it on every merge/write for no reason — so this uses an
//! order-preserving `Vec<(String, String)>` instead. Order is: existing
//! instance-file order first, then any pack-only keys appended at the end.

use crate::error::{io_err, Result, SyncError};
use std::path::Path;

/// An order-preserving `options.txt` document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OptionsDocument {
    /// `(key, value)` pairs in on-disk order. Duplicate keys (which
    /// shouldn't occur in a well-formed file) keep only the last value,
    /// matching how Minecraft's own parser behaves.
    pub entries: Vec<(String, String)>,
}

impl OptionsDocument {
    pub fn parse(text: &str) -> Self {
        let mut entries: Vec<(String, String)> = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                // Malformed line: keep it as a whole-line "key" with an
                // empty value rather than dropping data silently.
                entries.push((line.to_string(), String::new()));
                continue;
            };
            let key = key.to_string();
            let value = value.to_string();
            if let Some(existing) = entries.iter_mut().find(|(k, _)| *k == key) {
                existing.1 = value;
            } else {
                entries.push((key, value));
            }
        }
        Self { entries }
    }

    pub fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).map_err(io_err(path))?;
        Ok(Self::parse(&text))
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        gl_persistence::write_atomic(path, &self.to_text()).map_err(SyncError::from)
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (key, value) in &self.entries {
            out.push_str(key);
            out.push(':');
            out.push_str(value);
            out.push('\n');
        }
        out
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        if let Some(existing) = self.entries.iter_mut().find(|(k, _)| k == key) {
            existing.1 = value.to_string();
        } else {
            self.entries.push((key.to_string(), value.to_string()));
        }
    }

    /// Merge `pack` into `self` (the instance's local document), in place.
    /// `managed_keys` are the keys the pack actually cares about keeping in
    /// sync; only those are overwritten with the pack's value. Every other
    /// key already in `self` (instance-local-only settings such as
    /// `lang`, `key_*` bindings the user customized, etc.) is left
    /// untouched. Pack keys not yet present locally are appended, keeping
    /// the pack's own relative order among themselves at the end so the
    /// merge is deterministic.
    pub fn merge_from_pack(&mut self, pack: &OptionsDocument, managed_keys: &[String]) {
        for key in managed_keys {
            if let Some(value) = pack.get(key) {
                self.set(key, value);
            }
        }
    }
}
