//! Built-in JVM-argument presets/templates offered when creating a new Java
//! instance (or switching a preset later from instance settings). Pure JVM
//! flags only — no mod/API downloads of any kind. Applied as
//! [`crate::models::JavaInstance::extra_jvm_args`], which
//! [`crate::launcher::build_jvm_args`] already appends verbatim.

/// A named preset: its display name, id (stored in
/// `JavaInstance::jvm_args_preset`), and the extra JVM args it applies.
#[derive(Debug, Clone, serde::Serialize)]
pub struct JvmPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub args: &'static [&'static str],
}

/// The full built-in preset list, in the order the UI should offer them.
pub const PRESETS: &[JvmPreset] = &[
    JvmPreset {
        id: "vanilla",
        label: "Vanilla",
        description: "No extra JVM flags beyond the version's own defaults.",
        args: &[],
    },
    JvmPreset {
        id: "balanced",
        label: "Balanced",
        description: "G1GC with modest tuning — a reasonable default for most machines.",
        args: &["-XX:+UseG1GC", "-XX:MaxGCPauseMillis=100"],
    },
    JvmPreset {
        id: "performance",
        label: "Performance",
        description: "Aikar's-flags-style G1GC tuning for smoother frame times on capable hardware.",
        args: &[
            "-XX:+UseG1GC",
            "-XX:MaxGCPauseMillis=100",
            "-XX:+UnlockExperimentalVMOptions",
            "-XX:G1NewSizePercent=30",
            "-XX:G1MaxNewSizePercent=40",
            "-XX:G1HeapRegionSize=8M",
            "-XX:G1ReservePercent=20",
            "-XX:InitiatingHeapOccupancyPercent=15",
        ],
    },
    JvmPreset {
        id: "low-end",
        label: "Low-End",
        description: "Serial GC with a small region size — trades peak throughput for lower memory/CPU overhead.",
        args: &[
            "-XX:+UseSerialGC",
            "-XX:TargetSurvivorRatio=90",
            "-XX:MaxTenuringThreshold=1",
        ],
    },
];

/// Look up a preset by id (case-insensitive). Returns `None` for an unknown
/// id (including "custom", which isn't a real preset).
pub fn find(id: &str) -> Option<&'static JvmPreset> {
    PRESETS.iter().find(|p| p.id.eq_ignore_ascii_case(id))
}

/// The JVM args for a preset id, or an empty list if the id isn't
/// recognized (e.g. "custom" or a typo) — never fails the caller.
pub fn args_for(id: &str) -> Vec<String> {
    find(id)
        .map(|p| p.args.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}
