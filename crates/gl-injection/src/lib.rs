//! Win32 DLL process injection and privilege elevation for mod clients,
//! ported from `InjectionService.cs`.
//!
//! # Safety-critical crate
//!
//! This crate opens a foreign process, allocates memory inside it, writes to
//! that memory, and creates a remote thread that calls `LoadLibraryW` on the
//! written path. Every Win32 handle acquired here is released on every exit
//! path (including early-return error paths) via explicit `Drop` guards, and
//! every fallible Win32 call maps `GetLastError` into a `Result::Err` rather
//! than panicking. No `unsafe` block here is exempt from that discipline.
//!
//! This is the direct Rust equivalent of the C# `InjectionService` static
//! class — see doc comments on each function for the exact Win32 call
//! sequence it mirrors.
//!
//! TODO(feature 28, client resource usage overlay): skipped. A per-process
//! CPU/memory readout is easy to bolt on via `GetProcessTimes` /
//! `GetProcessMemoryInfo` on the handles this crate already opens, but a
//! *polling loop* driving a live UI overlay is more surface area than "low
//! risk" — it means holding process handles open longer, a new Tauri event
//! stream, and periodic timer work, none of which touches injection safety
//! directly but all of which is easy to get wrong under review pressure.
//! Left for a dedicated, reviewed pass instead of squeezing it in here.
//!
//! Real-process injection cannot be meaningfully unit-tested (it requires an
//! actual running Win32 process and, for the full end-to-end path, a real
//! Minecraft Bedrock process). Per the rewrite plan's risk mitigations, this
//! crate is unit-tested only for the parts that don't require a live target
//! process (error mapping, path/name validation, UTF-16 buffer construction);
//! the actual `inject_dll` call must be exercised with a real repro (a
//! disposable test-harness process first, then the real game process) before
//! being considered verified.

#![cfg(windows)]

use std::path::Path;
use std::time::{Duration, Instant};

use windows::core::{Result as WinResult, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::GENERIC_ALL;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, HLOCAL, LUID};
use windows::Win32::Security::Authorization::{
    BuildTrusteeWithNameW, GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW,
    EXPLICIT_ACCESS_W, SET_ACCESS,
    SE_FILE_OBJECT, TRUSTEE_IS_NAME, TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::Security::{DACL_SECURITY_INFORMATION, SUB_CONTAINERS_AND_OBJECTS_INHERIT};
use windows::Win32::Storage::Packaging::Appx::GetPackageFamilyName;
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
};
use windows::Win32::System::Threading::{
    CreateRemoteThread, GetCurrentProcess, GetExitCodeThread, GetProcessTimes, OpenProcess,
    OpenProcessToken, WaitForSingleObject, PROCESS_ACCESS_RIGHTS, PROCESS_ALL_ACCESS,
    PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

/// Errors that can occur while enabling privileges, injecting a DLL, or
/// probing whether it loaded.
#[derive(Debug, thiserror::Error)]
pub enum InjectionError {
    #[error("DLL not found: {0}")]
    DllNotFound(std::path::PathBuf),
    #[error("could not open the target process (Win32 error {0}); try running the launcher as administrator")]
    OpenProcessFailed(u32),
    #[error("VirtualAllocEx failed (Win32 error {0})")]
    VirtualAllocFailed(u32),
    #[error("WriteProcessMemory failed (Win32 error {0})")]
    WriteProcessMemoryFailed(u32),
    #[error("could not resolve LoadLibraryW in the target process's kernel32.dll")]
    LoadLibraryWUnresolved,
    #[error("CreateRemoteThread failed (Win32 error {0})")]
    CreateRemoteThreadFailed(u32),
    #[error("injection thread did not finish within the timeout (the DLL may be hung in DllMain)")]
    ThreadTimedOut,
    #[error("LoadLibraryW returned NULL inside the target process — the DLL was rejected (common causes: architecture mismatch, missing dependency, or the DLL crashed in DllMain)")]
    LoadLibraryRejected,
    #[error("the DLL was loaded but never appeared in the target process's module list — it likely unloaded itself or failed during initialization")]
    ModuleNeverAppeared,
    #[error("win32 error: {0}")]
    Win32(#[from] windows::core::Error),
}

pub type Result<T> = std::result::Result<T, InjectionError>;

/// RAII wrapper around a Win32 `HANDLE` that always closes it on drop,
/// including on early-return error paths. This is what guarantees the "every
/// handle closed on every exit path" property called out in the rewrite
/// plan's risk list for this crate.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // Best-effort: nothing meaningful to do if CloseHandle itself fails.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// RAII wrapper around memory allocated in a foreign process via
/// `VirtualAllocEx`, freed via `VirtualFreeEx` on drop.
struct RemoteAlloc {
    process: HANDLE,
    address: *mut core::ffi::c_void,
}

impl Drop for RemoteAlloc {
    fn drop(&mut self) {
        if !self.address.is_null() {
            let _ = unsafe { VirtualFreeEx(self.process, self.address, 0, MEM_RELEASE) };
        }
    }
}

/// Enables `SeDebugPrivilege` on the current process token, mirroring
/// `InjectionService.EnableDebugPrivilege`. This is required to open
/// higher-privileged processes (e.g. a UWP/GDK-hosted Minecraft) for
/// injection. Best-effort: matches the C# code's `catch {}` — failure here is
/// not fatal, it just means a later `OpenProcess` may fail instead.
pub fn enable_debug_privilege() {
    let _ = try_enable_debug_privilege();
}

fn try_enable_debug_privilege() -> WinResult<()> {
    unsafe {
        let current_process = GetCurrentProcess();
        let mut token = HANDLE::default();
        OpenProcessToken(
            current_process,
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )?;
        let _token_guard = OwnedHandle(token);

        let mut luid = LUID::default();
        LookupPrivilegeValueW(
            PCWSTR::null(),
            windows::core::w!("SeDebugPrivilege"),
            &mut luid,
        )?;

        let mut privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [windows::Win32::Security::LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        // AdjustTokenPrivileges can return success while still reporting
        // ERROR_NOT_ALL_ASSIGNED via GetLastError; matching the C# code, we
        // treat that as "best effort" too and don't surface it as an error.
        AdjustTokenPrivileges(token, false, Some(&mut privileges as *mut _), 0, None, None)?;
        Ok(())
    }
}

const PROCESS_INJECT_ACCESS: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(
    PROCESS_VM_OPERATION.0
        | PROCESS_VM_READ.0
        | PROCESS_VM_WRITE.0
        | PROCESS_QUERY_INFORMATION.0
        | PROCESS_CREATE_THREAD.0,
);

/// `Path::canonicalize` on Windows returns a `\\?\`-prefixed ("verbatim")
/// path. `LoadLibraryW` accepts these, but strip the prefix anyway for a
/// path that reads naturally in logs/errors and matches what
/// `Process.Modules[i].FileName` reports when we later verify the DLL
/// loaded.
fn strip_verbatim_prefix(path: std::path::PathBuf) -> std::path::PathBuf {
    match path.to_str() {
        Some(s) if s.starts_with(r"\\?\") => std::path::PathBuf::from(&s[4..]),
        _ => path,
    }
}

fn open_target_process(process_id: u32) -> Result<OwnedHandle> {
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_ALL_ACCESS, false, process_id) {
            if !h.is_invalid() {
                return Ok(OwnedHandle(h));
            }
        }
        // Capture the wider attempt's error before trying the narrower
        // fallback — if both fail for *different* reasons (e.g. the process
        // is a protected process, so the wide request is denied for that,
        // while the narrower request fails for an unrelated reason), the
        // wide attempt's code is almost always the more diagnostic one to
        // surface, since it's the more common failure path for a real
        // target and the one users' error messages should match.
        let broad_access_error = windows::Win32::Foundation::GetLastError().0;
        match OpenProcess(PROCESS_INJECT_ACCESS, false, process_id) {
            Ok(h) if !h.is_invalid() => Ok(OwnedHandle(h)),
            _ => Err(InjectionError::OpenProcessFailed(broad_access_error)),
        }
    }
}

/// Injects `dll_path` into the process identified by `process_id` via the
/// classic `CreateRemoteThread` + `LoadLibraryW` technique, mirroring
/// `InjectionService.InjectDll`.
///
/// Sequence (matches the C# implementation exactly):
/// 1. Grant the "ALL APPLICATION PACKAGES" group access to the DLL and its
///    parent directory, so a UWP/GDK-sandboxed target process can read it.
/// 2. Enable `SeDebugPrivilege` on our own token.
/// 3. Open the target process (`PROCESS_ALL_ACCESS`, falling back to a
///    narrower injection-only access mask).
/// 4. Allocate a buffer in the target process and write the DLL's absolute
///    path into it as a UTF-16, NUL-terminated string.
/// 5. Resolve `LoadLibraryW` in our own `kernel32.dll` (its address is the
///    same in every process on a given Windows install, so resolving it
///    locally and calling it remotely is valid) and start a remote thread at
///    that address with the allocated buffer as its argument.
/// 6. Wait up to 30 seconds for the thread to finish, then check its exit
///    code (the return value of `LoadLibraryW`, i.e. the loaded module
///    handle) — zero means the target rejected the DLL.
/// 7. If `verify` is set, poll the target's module list for up to 5 seconds
///    to confirm the DLL is still loaded (it may have unloaded itself after
///    a failed `DllMain`).
pub fn inject_dll(process_id: u32, dll_path: &Path, verify: bool) -> Result<()> {
    if !dll_path.exists() {
        return Err(InjectionError::DllNotFound(dll_path.to_path_buf()));
    }

    // Resolve to an absolute, `\\?\`-free path before handing it to the
    // remote LoadLibraryW call: the target process has its own working
    // directory (and, for a UWP/GDK-hosted Minecraft, its own filesystem
    // view), so a relative path that happens to resolve correctly from our
    // own CWD would silently fail to resolve — or resolve to the wrong file
    // — once read back inside the target process.
    let dll_path = dll_path
        .canonicalize()
        .map(strip_verbatim_prefix)
        .unwrap_or_else(|_| dll_path.to_path_buf());
    let dll_path = dll_path.as_path();

    grant_uwp_access(dll_path);
    if let Some(parent) = dll_path.parent() {
        grant_uwp_access(parent);
    }

    enable_debug_privilege();

    let process = open_target_process(process_id)?;

    let path_str = dll_path.to_string_lossy();
    let mut path_utf16: Vec<u16> = path_str.encode_utf16().collect();
    path_utf16.push(0);
    let byte_len = path_utf16.len() * std::mem::size_of::<u16>();

    let remote_addr = unsafe {
        VirtualAllocEx(
            process.0,
            None,
            byte_len,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };
    if remote_addr.is_null() {
        return Err(InjectionError::VirtualAllocFailed(unsafe {
            windows::Win32::Foundation::GetLastError().0
        }));
    }
    let remote_alloc = RemoteAlloc {
        process: process.0,
        address: remote_addr,
    };

    let bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(path_utf16.as_ptr() as *const u8, byte_len) };
    let mut written = 0usize;
    let write_ok = unsafe {
        WriteProcessMemory(
            process.0,
            remote_addr,
            bytes.as_ptr() as *const core::ffi::c_void,
            byte_len,
            Some(&mut written),
        )
    };
    if write_ok.is_err() || written != byte_len {
        return Err(InjectionError::WriteProcessMemoryFailed(unsafe {
            windows::Win32::Foundation::GetLastError().0
        }));
    }

    let kernel32 = unsafe { GetModuleHandleW(windows::core::w!("kernel32.dll")) }
        .map_err(|_| InjectionError::LoadLibraryWUnresolved)?;
    let load_library = unsafe { GetProcAddress(kernel32, windows::core::s!("LoadLibraryW")) }
        .ok_or(InjectionError::LoadLibraryWUnresolved)?;
    let load_library_fn: extern "system" fn(*mut core::ffi::c_void) -> u32 =
        unsafe { std::mem::transmute(load_library) };

    let mut thread_id: u32 = 0;
    let thread = unsafe {
        CreateRemoteThread(
            process.0,
            None,
            0,
            Some(std::mem::transmute::<
                extern "system" fn(*mut core::ffi::c_void) -> u32,
                unsafe extern "system" fn(*mut core::ffi::c_void) -> u32,
            >(load_library_fn)),
            Some(remote_addr),
            0,
            Some(&mut thread_id),
        )
    };
    let thread = match thread {
        Ok(h) if !h.is_invalid() => OwnedHandle(h),
        _ => {
            return Err(InjectionError::CreateRemoteThreadFailed(unsafe {
                windows::Win32::Foundation::GetLastError().0
            }))
        }
    };

    const THIRTY_SECONDS_MS: u32 = 30_000;
    let wait_result = unsafe { WaitForSingleObject(thread.0, THIRTY_SECONDS_MS) };
    if wait_result.0 != 0 {
        return Err(InjectionError::ThreadTimedOut);
    }

    let mut exit_code: u32 = 0;
    let got_exit_code = unsafe { GetExitCodeThread(thread.0, &mut exit_code) };
    if got_exit_code.is_ok() && exit_code == 0 {
        return Err(InjectionError::LoadLibraryRejected);
    }

    drop(remote_alloc);
    drop(thread);
    drop(process);

    if verify && !wait_for_module(process_id, dll_path, Duration::from_secs(5)) {
        return Err(InjectionError::ModuleNeverAppeared);
    }

    Ok(())
}

#[derive(PartialEq, Eq)]
enum ModuleProbe {
    Confirmed,
    Absent,
    Unavailable,
}

/// Enumerates the loaded modules of `process_id` via a toolhelp snapshot
/// (equivalent to iterating `Process.Modules` in the C# code) and checks
/// whether `dll_path` is among them.
fn probe_module(process_id: u32, dll_path: &Path) -> ModuleProbe {
    let target = match dll_path.canonicalize() {
        Ok(p) => p,
        Err(_) => dll_path.to_path_buf(),
    };

    let snapshot =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, process_id) };
    let snapshot = match snapshot {
        Ok(h) if !h.is_invalid() => OwnedHandle(h),
        _ => return ModuleProbe::Unavailable,
    };

    let mut entry = MODULEENTRY32W {
        dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
        ..Default::default()
    };

    let target_str = target.to_string_lossy();
    let mut found_any = false;
    let mut ok = unsafe { Module32FirstW(snapshot.0, &mut entry) }.is_ok();
    while ok {
        found_any = true;
        let name = pwstr_buf_to_string(&entry.szExePath);
        // Cheap string match first: canonicalize() is a filesystem call per module, and this
        // runs for every module on every poll.
        if name.eq_ignore_ascii_case(&target_str) {
            return ModuleProbe::Confirmed;
        }
        if let Ok(module_path) = Path::new(&name).canonicalize() {
            if module_path == target {
                return ModuleProbe::Confirmed;
            }
        } else if name.eq_ignore_ascii_case(&target.to_string_lossy()) {
            return ModuleProbe::Confirmed;
        }
        ok = unsafe { Module32NextW(snapshot.0, &mut entry) }.is_ok();
    }

    if found_any {
        ModuleProbe::Absent
    } else {
        ModuleProbe::Unavailable
    }
}

fn pwstr_buf_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Returns whether `dll_path` currently appears in `process_id`'s module
/// list, equivalent to `InjectionService.IsModuleLoaded`.
pub fn is_module_loaded(process_id: u32, dll_path: &Path) -> bool {
    probe_module(process_id, dll_path) == ModuleProbe::Confirmed
}

/// Finds the process id of the first running process whose image name
/// matches `exe_name` (case-insensitive, e.g. `"Minecraft.Windows.exe"` or
/// `"javaw.exe"`), via a `TH32CS_SNAPPROCESS` toolhelp snapshot. Returns
/// `None` if no matching process is running.
pub fn find_process_by_name(exe_name: &str) -> Option<u32> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let snapshot = match snapshot {
        Ok(h) if !h.is_invalid() => OwnedHandle(h),
        _ => return None,
    };

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    let mut ok = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
    while ok {
        let name = pwstr_buf_to_string(&entry.szExeFile);
        if name.eq_ignore_ascii_case(exe_name) {
            return Some(entry.th32ProcessID);
        }
        ok = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
    }
    None
}

/// Every running process id whose image name matches `exe_name`
/// (case-insensitive), via a `TH32CS_SNAPPROCESS` toolhelp snapshot.
///
/// The singular [`find_process_by_name`] stops at the first hit, which is
/// wrong when several candidates exist and only one belongs to the package
/// we care about — see [`process_package_family_name`].
pub fn find_processes_by_name(exe_name: &str) -> Vec<u32> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let snapshot = match snapshot {
        Ok(h) if !h.is_invalid() => OwnedHandle(h),
        _ => return Vec::new(),
    };

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    let mut found = Vec::new();
    let mut ok = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
    while ok {
        let name = pwstr_buf_to_string(&entry.szExeFile);
        if name.eq_ignore_ascii_case(exe_name) {
            found.push(entry.th32ProcessID);
        }
        ok = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
    }
    found
}

/// The package family name of `process_id`, or `None` when it can't be read
/// (the process is not packaged, has exited, or we lack access).
///
/// Port of the C# `BedrockPackageService.GetProcessPackageFamilyName`:
/// `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` then `GetPackageFamilyName`,
/// called once for the buffer length and again to fill it. A `None` result is
/// explicitly *not* the same as "different package" — the caller keeps such a
/// process as a last-resort fallback, exactly as the original does.
pub fn process_package_family_name(process_id: u32) -> Option<String> {
    let handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    if handle.is_invalid() {
        return None;
    }
    let handle = OwnedHandle(handle);

    let mut length: u32 = 0;
    // First call reports the required buffer length (in WCHARs, including the
    // terminating null). A zero length means the process isn't packaged.
    let _ = unsafe { GetPackageFamilyName(handle.0, &mut length, None) };
    if length == 0 {
        return None;
    }

    let mut buffer = vec![0u16; length as usize];
    let rc =
        unsafe { GetPackageFamilyName(handle.0, &mut length, Some(PWSTR(buffer.as_mut_ptr()))) };
    if rc.is_err() {
        return None;
    }

    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..end]))
}

/// Polls the target process's module list until `dll_path` appears or
/// `timeout` elapses, equivalent to `InjectionService.WaitForModule`.
///
/// Mirrors a subtlety in the C# implementation: if the module is *never*
/// successfully read (`Unavailable` the whole time — e.g. the process
/// exited, or we lack access), that's treated as "can't tell, assume okay"
/// rather than a hard failure — only an module list we *could* read but that
/// never contained the DLL counts as a genuine failure.
fn wait_for_module(process_id: u32, dll_path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let mut ever_read_it = false;

    while Instant::now() < deadline {
        match probe_module(process_id, dll_path) {
            ModuleProbe::Confirmed => return true,
            ModuleProbe::Absent => ever_read_it = true,
            ModuleProbe::Unavailable => {}
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    !ever_read_it
}

/// Grants the "ALL APPLICATION PACKAGES" well-known group full access to
/// `path`, so a UWP/GDK-sandboxed process (like Minecraft Bedrock) can read a
/// DLL living outside its package. Best-effort, matching the C# code's
/// `catch {}` — a failure here just means injection will likely fail later
/// with a more specific error, but isn't itself fatal.
fn grant_uwp_access(path: &Path) {
    let _ = try_grant_uwp_access(path);
}

fn try_grant_uwp_access(path: &Path) -> WinResult<()> {
    unsafe {
        let mut group_name: Vec<u16> = "ALL APPLICATION PACKAGES\0".encode_utf16().collect();

        let mut trustee = TRUSTEE_W::default();
        BuildTrusteeWithNameW(&mut trustee, PWSTR(group_name.as_mut_ptr()));
        trustee.TrusteeForm = TRUSTEE_IS_NAME;
        trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;

        let access = EXPLICIT_ACCESS_W {
            grfAccessPermissions: GENERIC_ALL.0,
            grfAccessMode: SET_ACCESS,
            grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            Trustee: trustee,
        };

        // Merge into the existing DACL. Building a fresh ACL and applying it would
        // drop every other entry on the file/folder, including the owner's own access.
        let path_hstring = HSTRING::from(path.as_os_str());
        let mut old_acl: *mut windows::Win32::Security::ACL = std::ptr::null_mut();
        let mut security_descriptor = windows::Win32::Security::PSECURITY_DESCRIPTOR::default();
        let status = GetNamedSecurityInfoW(
            PCWSTR(path_hstring.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut old_acl as *mut _),
            None,
            &mut security_descriptor as *mut _,
        );
        if status.0 != 0 {
            return Err(windows::core::Error::from_hresult(
                windows::core::HRESULT::from_win32(status.0),
            ));
        }

        let mut new_acl: *mut windows::Win32::Security::ACL = std::ptr::null_mut();
        let status = SetEntriesInAclW(Some(&[access]), Some(old_acl), &mut new_acl);
        let result = if status.0 != 0 {
            Err(windows::core::Error::from_hresult(
                windows::core::HRESULT::from_win32(status.0),
            ))
        } else {
            let status = SetNamedSecurityInfoW(
                PWSTR(path_hstring.as_ptr() as *mut u16),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                Some(new_acl as *const _),
                None,
            );
            if status.0 == 0 {
                Ok(())
            } else {
                Err(windows::core::Error::from_hresult(
                    windows::core::HRESULT::from_win32(status.0),
                ))
            }
        };

        if !new_acl.is_null() {
            let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(new_acl as *mut _)));
        }
        if !security_descriptor.0.is_null() {
            let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(security_descriptor.0)));
        }

        result
    }
}

/// Whether a top-level window of class `class_name` belonging to `process_id`
/// is currently visible.
///
/// Modeled directly on Flarial's own official launcher
/// (`Flarial.Runtime/Game/Minecraft.Bootstrap.cs::Launch`,
/// `Minecraft.Lookup.cs::GetWindow`): Minecraft Bedrock's main window (Win32
/// class `"Bedrock"`) only becomes visible once the game's script engine has
/// finished booting, which is what actually determines whether injecting a
/// client DLL is safe. A fixed post-launch delay is a guess at that moment;
/// this checks it directly.
///
/// Enumerates every top-level window of the given class (there is normally at
/// most one, but a stale window from a previous instance could still be
/// closing) and returns `true` for the first one owned by `process_id` that
/// is visible.
pub fn is_target_window_visible(process_id: u32, class_name: &str) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowExW, GetWindowThreadProcessId, IsWindowVisible,
    };

    let class_wide: Vec<u16> = class_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let class_pcwstr = PCWSTR(class_wide.as_ptr());

    let mut after: Option<HWND> = None;
    loop {
        let hwnd = match unsafe { FindWindowExW(None, after, class_pcwstr, PCWSTR::null()) } {
            Ok(h) => h,
            Err(_) => return false,
        };

        let mut owner_pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut owner_pid)) };

        if owner_pid == process_id {
            return unsafe { IsWindowVisible(hwnd) }.as_bool();
        }

        after = Some(hwnd);
    }
}

/// Polls [`is_target_window_visible`] until it is `true`, the process exits,
/// or `timeout` elapses. Returns `true` only on the visible-window case.
///
/// This is the readiness wait Flarial's launcher performs before injecting
/// (a `FileSystemWatcher`/`WaitForMultipleObjects` pair there; a poll loop
/// here, which needs no extra Windows API surface and is precise enough at a
/// short interval).
///
/// Liveness is checked via a single `HANDLE` opened once at the start, not by
/// re-opening `process_id` by number on every iteration: Windows recycles
/// process ids as soon as they're freed, so a numeric re-check can easily
/// find some unrelated, unrelated-but-alive process that was just handed the
/// same id and wrongly conclude the original process is still running. A
/// `HANDLE` refers to the specific kernel object regardless of what happens
/// to the number afterward, so `WaitForSingleObject` on it can't be fooled
/// that way. If the handle can't be opened at all, liveness can't be
/// determined and the loop falls back to timing out rather than spinning
/// forever.
pub fn wait_for_target_window(
    process_id: u32,
    class_name: &str,
    timeout: Duration,
    poll_interval: Duration,
) -> bool {
    let handle = match unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            process_id,
        )
    } {
        Ok(h) if !h.is_invalid() => OwnedHandle(h),
        _ => return false,
    };

    let deadline = Instant::now() + timeout;
    loop {
        if is_target_window_visible(process_id, class_name) {
            return true;
        }
        // A 0ms wait just polls the signaled state without blocking; `0`
        // (`WAIT_OBJECT_0`) means the process has already exited.
        if unsafe { WaitForSingleObject(handle.0, 0) }.0 == 0 {
            return false;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(poll_interval);
    }
}

/// Non-blocking check for whether `process_id` is still running. Uses the
/// same zero-timeout `WaitForSingleObject` probe as [`wait_for_process_exit`]
/// — `WAIT_OBJECT_0` (0) means the process has already signaled (exited);
/// anything else (typically `WAIT_TIMEOUT`) means it's still alive. A PID
/// that can no longer be opened (already exited and recycled) reports false.
pub fn process_is_alive(process_id: u32) -> bool {
    let handle = match unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            process_id,
        )
    } {
        Ok(h) if !h.is_invalid() => OwnedHandle(h),
        _ => return false,
    };
    unsafe { WaitForSingleObject(handle.0, 0) }.0 != 0
}

/// The process's creation time as Unix seconds, or `None` if it can't be
/// opened/queried (already exited, or a permissions issue). Used to guard
/// against PID recycling: a tracker that remembers "PID 1234 is my launched
/// process" can otherwise end up killing a completely unrelated process if
/// the original already exited and the OS handed that PID to something else
/// before the kill runs — see [`ProcessRegistry::kill`] in `gl_launch`,
/// which compares this against the PID's remembered start time before
/// trusting it.
pub fn process_start_time_unix(process_id: u32) -> Option<u64> {
    let handle = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
    {
        Ok(h) if !h.is_invalid() => OwnedHandle(h),
        _ => return None,
    };

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let ok = unsafe {
        GetProcessTimes(handle.0, &mut creation, &mut exit, &mut kernel, &mut user)
    };
    if ok.is_err() {
        return None;
    }

    // FILETIME is 100ns intervals since 1601-01-01; Unix epoch is
    // 1970-01-01, which is 11644473600 seconds later.
    let ticks_100ns = ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64;
    let unix_100ns = ticks_100ns.checked_sub(116_444_736_000_000_000)?;
    Some(unix_100ns / 10_000_000)
}

/// Blocks the calling thread until the process `process_id` exits (or is
/// already gone). Used to know when to release a [`set_prevent_suspend`]
/// request — run this on a blocking thread, never the async executor.
pub fn wait_for_process_exit(process_id: u32) {
    let handle = match unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            process_id,
        )
    } {
        Ok(h) if !h.is_invalid() => OwnedHandle(h),
        _ => return, // already gone, or inaccessible — nothing to wait on
    };
    unsafe {
        WaitForSingleObject(handle.0, u32::MAX);
    }
}

/// Prevents (or restores) Windows from suspending the process/display while
/// a launched Minecraft process is running, via `SetThreadExecutionState`.
/// `ES_CONTINUOUS` makes the state persist until explicitly cleared, rather
/// than being a one-shot "stay awake for the next call" request.
///
/// Call with `prevent = true` once the game is running, and again with
/// `prevent = false` when it exits to release the request — otherwise the
/// last-set state stays in effect for the whole launcher process lifetime.
pub fn set_prevent_suspend(prevent: bool) {
    let flags = if prevent {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    unsafe {
        SetThreadExecutionState(flags);
    }
}

/// Kills every running process named `exe_name` (e.g. `"RuntimeBroker.exe"`)
/// via `taskkill`. Best-effort: RuntimeBroker is a system process that Windows
/// respawns on demand, so killing it briefly is safe and is the documented
/// trick behind Bedrock's "accelerate loading" option (stopping RuntimeBroker
/// mid-boot shortens the UWP app's loading-screen throttle).
pub fn kill_processes_by_name(exe_name: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/IM", exe_name, "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}

/// Activates a UWP/GDK app by AUMID via `IApplicationActivationManager`,
/// mirroring `InjectionService.LaunchMinecraft`. This is the direct COM
/// activation path the original uses; `gl_launch::bedrock` normally
/// activates through `Invoke-CommandInDesktopPackage` instead (a
/// PowerShell-based equivalent chosen to avoid a WinRT/COM dependency), so
/// this exists as the literal port for parity and as a fallback that
/// doesn't need to shell out to `powershell.exe`. Returns the new process's
/// PID.
pub fn activate_application(aumid: &str, arguments: Option<&str>) -> Result<u32> {
    use windows::core::HSTRING;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_LOCAL_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        ApplicationActivationManager, IApplicationActivationManager, AO_NONE,
    };

    // COM requires the apartment to be initialized on the calling thread.
    // `CoInitializeEx` returns S_FALSE (a non-error HRESULT `Ok`) if already
    // initialized on this thread, matching how the C# runtime lazily
    // initializes COM the first time it's touched — safe to call every time.
    let init_hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let we_initialized = init_hr.is_ok();

    let result = (|| -> Result<u32> {
        let manager: IApplicationActivationManager =
            unsafe { CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER)? };

        let aumid_h = HSTRING::from(aumid);
        let args_h = arguments.map(HSTRING::from).unwrap_or_default();

        let pid = unsafe { manager.ActivateApplication(&aumid_h, &args_h, AO_NONE)? };
        // Interface is released when `manager` drops; explicit for clarity.
        drop(manager);
        Ok(pid)
    })();

    if we_initialized {
        unsafe { CoUninitialize() };
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_verbatim_prefix_removes_win32_file_namespace() {
        let verbatim = std::path::PathBuf::from(r"\\?\C:\Games\mod.dll");
        assert_eq!(
            strip_verbatim_prefix(verbatim),
            std::path::PathBuf::from(r"C:\Games\mod.dll")
        );
    }

    #[test]
    fn strip_verbatim_prefix_leaves_normal_paths_alone() {
        let normal = std::path::PathBuf::from(r"C:\Games\mod.dll");
        assert_eq!(strip_verbatim_prefix(normal.clone()), normal);
    }

    #[test]
    fn inject_rejects_missing_dll_before_touching_win32() {
        let missing = Path::new(r"C:\this\path\definitely\does\not\exist\fake.dll");
        let result = inject_dll(u32::MAX, missing, false);
        assert!(matches!(result, Err(InjectionError::DllNotFound(_))));
    }

    #[test]
    fn pwstr_buf_to_string_stops_at_nul() {
        let mut buf = [0u16; 8];
        for (i, c) in "abc".encode_utf16().enumerate() {
            buf[i] = c;
        }
        assert_eq!(pwstr_buf_to_string(&buf), "abc");
    }

    #[test]
    fn pwstr_buf_to_string_handles_full_buffer_without_nul() {
        let buf: [u16; 3] = ['x' as u16, 'y' as u16, 'z' as u16];
        assert_eq!(pwstr_buf_to_string(&buf), "xyz");
    }

    #[test]
    fn error_messages_are_actionable() {
        let err = InjectionError::OpenProcessFailed(5);
        assert!(err.to_string().contains("administrator"));
        let err = InjectionError::LoadLibraryRejected;
        assert!(err.to_string().contains("architecture mismatch"));
    }
}
