//! End-to-end injection test against a throwaway host process.
//!
//! This is the step the rewrite plan calls non-negotiable: prove the injection
//! pipeline against a disposable process and a known-good DLL *before* ever
//! pointing it at Minecraft. Everything `inject_dll` does — `OpenProcess`,
//! `VirtualAllocEx`, `WriteProcessMemory`, resolving `LoadLibraryW`,
//! `CreateRemoteThread`, waiting on the thread, and the module probe — is
//! exercised here, so a regression in the highest-risk code in the repo shows
//! up as a failing test rather than as a crashed game.
//!
//! `#[ignore]` by default: it spawns and kills a real process, which is not
//! something CI should do implicitly. Run it deliberately:
//!
//! ```text
//! cargo test -p gl-injection --test throwaway_host -- --ignored --nocapture
//! ```
//!
//! The host is `cmd /c timeout 30`: present on every Windows install, does
//! nothing, and exits on its own within 30s so a leaked process is harmless.
//! The payload is `version.dll` from System32 — a real, signed,
//! side-effect-free library, so no compiler is needed to produce a test DLL.

#![cfg(windows)]

use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// Spawns a host process that stays alive long enough to be injected into,
/// and exits by itself if anything goes wrong before the guard can kill it.
fn spawn_host() -> std::io::Result<Child> {
    Command::new("cmd")
        .args(["/c", "timeout", "/t", "30", "/nobreak"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
}

fn system32(name: &str) -> PathBuf {
    PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into()))
        .join("System32")
        .join(name)
}

/// Kills the host on every exit path, including a panicking assertion.
struct HostGuard(Child);
impl Drop for HostGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "spawns and kills a real process; run explicitly with --ignored"]
fn injects_a_real_dll_into_a_throwaway_process() {
    let payload = system32("version.dll");
    assert!(payload.is_file(), "expected {} to exist", payload.display());

    gl_injection::enable_debug_privilege();

    let host = HostGuard(spawn_host().expect("failed to spawn throwaway host"));
    let pid = host.0.id();

    // Give the process a moment to finish initialising before injecting;
    // injecting into a process still setting up its loader is a real source
    // of flakiness and is not what this test is trying to measure.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline
        && !gl_injection::find_processes_by_name("cmd.exe").contains(&pid)
    {
        std::thread::sleep(Duration::from_millis(100));
    }

    assert!(
        !gl_injection::is_module_loaded(pid, &payload),
        "payload must not already be loaded in a fresh host"
    );

    gl_injection::inject_dll(pid, &payload, true)
        .expect("injection into the throwaway host failed");

    assert!(
        gl_injection::is_module_loaded(pid, &payload),
        "module probe should see the payload after a verified injection"
    );

    drop(host);
}

#[test]
#[ignore = "spawns and kills a real process; run explicitly with --ignored"]
fn missing_dll_is_reported_before_touching_the_process() {
    let host = HostGuard(spawn_host().expect("failed to spawn throwaway host"));
    let pid = host.0.id();

    let err = gl_injection::inject_dll(pid, &system32("definitely-not-here.dll"), true)
        .expect_err("a missing DLL must be an error");
    let msg = err.to_string();
    assert!(
        msg.contains("DLL not found"),
        "expected an actionable message, got: {msg}"
    );

    drop(host);
}

#[test]
fn injecting_into_a_dead_process_fails_cleanly() {
    // PID 0 is the System Idle Process and can never be opened for injection.
    // The point is that this returns an error rather than panicking or
    // silently reporting success.
    let payload = system32("version.dll");
    if !payload.is_file() {
        return;
    }
    let result = gl_injection::inject_dll(0, &payload, false);
    assert!(result.is_err(), "injecting into pid 0 must fail");
}

#[test]
fn module_probe_on_a_nonexistent_process_is_false_not_a_panic() {
    // A pid that has almost certainly never existed in this session.
    assert!(!gl_injection::is_module_loaded(
        0xFFFF_FFF0,
        &system32("version.dll")
    ));
}

#[test]
fn process_lookup_finds_a_spawned_host_and_loses_it_after_exit() {
    let host = HostGuard(spawn_host().expect("failed to spawn throwaway host"));
    let pid = host.0.id();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = false;
    while Instant::now() < deadline {
        if gl_injection::find_processes_by_name("cmd.exe").contains(&pid) {
            seen = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(seen, "find_processes_by_name should see the spawned host");

    drop(host);

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !gl_injection::find_processes_by_name("cmd.exe").contains(&pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("host pid {pid} still listed after the process was killed");
}

#[test]
fn window_visibility_is_false_for_a_class_that_does_not_exist() {
    // No real window ever has this class name, so this must return false
    // rather than hang or panic - the loop in `is_target_window_visible`
    // must terminate on FindWindowExW's "not found" error.
    assert!(!gl_injection::is_target_window_visible(
        std::process::id(),
        "GlacierTestNonexistentWindowClass"
    ));
}

#[test]
fn wait_for_target_window_gives_up_when_the_process_has_already_exited() {
    // A pid that is already dead: the poll loop must notice and return
    // false immediately rather than spinning for the full timeout.
    let host = spawn_host().expect("failed to spawn throwaway host");
    let pid = host.id();
    let mut host = host;
    host.kill().ok();
    host.wait().ok();

    let start = Instant::now();
    let ready = gl_injection::wait_for_target_window(
        pid,
        "Bedrock",
        Duration::from_secs(10),
        Duration::from_millis(50),
    );
    assert!(!ready);
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "should give up quickly once the process is gone, took {:?}",
        start.elapsed()
    );
}
