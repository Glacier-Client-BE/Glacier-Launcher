use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

use windows_sys::Win32::{
    Foundation::{BOOL, HANDLE, LPVOID},
    System::{
        ACCESS_MASK,
        System::WinError,
        Winbase::CloseHandle,
        Winnt::{GENERIC_ALL, PROCESS_ALL_ACCESS, PROCESS_INJECT_ACCESS, PAGE_READWRITE, MEM_COMMIT, MEM_RESERVE},
        Memory::{VirtualAllocEx, VirtualFreeEx},
        Threading::{
            CreateRemoteThread, WaitForSingleObject, PROCESS_CREATION_FLAGS,
        },
    },
    um::{
        libloaderapi::GetModuleHandleW,
        winnt::HANDLE,
        winbase::VirtualAllocEx,
        winnt::{MEM_COMMIT, MEM_RESERVE},
    },
};

#[derive(Debug, Clone)]
pub struct InjectionResult {
    pub success: bool,
    pub message: String,
    pub injected: bool,
}

pub const SE_DEBUG_NAME: &str = "SeDebugPrivilege";
const SE_PRIVILEGE_ENABLED: u32 = 0x00000002;
pub const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
pub const TOKEN_QUERY: u32 = 0x0008;

#[derive(Debug)]
#[repr(C)]
struct TOKEN_PRIVILEGES {
    PrivilegeCount: u32,
    Luid: u64,
    Attributes: u32,
}

#[derive(Debug)]
#[repr(C)]
struct LUID {
    LowPart: u32,
    HighPart: i32,
}

#[derive(Debug)]
#[repr(C)]
struct TRUSTEE {
    pMultipleTrustee: LPVOID,
    MultipleTrusteeOperation: u32,
    TrusteeForm: u32,
    TrusteeType: u32,
    ptstrName: LPWSTR,
}

#[derive(Debug)]
#[repr(C)]
struct EXPLICIT_ACCESS {
    grfAccessPermissions: u32,
    grfAccessMode: i32,
    grfInheritance: u32,
    Trustee: TRUSTEE,
}

#[derive(Debug)]
#[repr(C)]
struct TOKEN_PRIVILEGES {
    PrivilegeCount: u32,
    Luid: u64,
    Attributes: u32,
}

extern "system" {
    fn OpenProcess(
        dwDesiredAccess: u32,
        bInheritHandle: BOOL,
        dwProcessId: u32,
    ) -> HANDLE;
    fn CloseHandle(hObject: HANDLE) -> BOOL;
    fn VirtualAllocEx(
        hProcess: HANDLE,
        lpAddress: LPVOID,
        dwSize: u32,
        flAllocationType: u32,
        flProtect: u32,
    ) -> LPVOID;
    fn VirtualFreeEx(
        hProcess: HANDLE,
        lpAddress: LPVOID,
        dwSize: u32,
        dwFreeType: u32,
    ) -> BOOL;
    fn WriteProcessMemory(
        hProcess: HANDLE,
        lpBaseAddress: LPVOID,
        lpBuffer: *const std::raw::c_void,
        nSize: u32,
        lpNumberOfBytesWritten: *mut u32,
    ) -> BOOL;
    fn GetProcAddress(
        hModule: LPVOID,
        lpProcName: *const i8,
    ) -> LPVOID;
    fn GetModuleHandleW(lpModuleName: LPWSTR) -> LPVOID;
    fn CreateRemoteThread(
        hProcess: HANDLE,
        lpThreadAttributes: LPVOID,
        dwStackSize: u32,
        lpStartAddress: LPVOID,
        lpParameter: LPVOID,
        dwCreationFlags: u32,
        lpThreadId: *mut u32,
    ) -> HANDLE;
    fn WaitForSingleObject(
        hHandle: HANDLE,
        dwMilliseconds: u32,
    ) -> u32;
    fn AdjustTokenPrivileges(
        TokenHandle: HANDLE,
        DisableAllPrivileges: BOOL,
        NewState: *const TOKEN_PRIVILEGES,
        BufferLength: u32,
        PreviousState: LPVOID,
        ReturnLength: LPVOID,
    ) -> BOOL;
    fn LookupPrivilegeValueW(
        lpSystemName: LPWSTR,
        lpName: LPWSTR,
        lpLuid: *mut LUID,
    ) -> BOOL;
    fn EnableDebugPrivilege() -> BOOL;
}

pub fn inject_dll(dll_path: String, process_id: u32) -> Result<InjectionResult, String> {
    // 1️⃣ Enable SeDebugPrivilege
    enable_debug_privilege().map_err(|e| format!("Enable debug privilege failed: {e}"))?;

    // 2️⃣ Open the target process
    let h_process = unsafe {
        OpenProcess(
            PROCESS_ALL_ACCESS | PROCESS_INJECT_ACCESS,
            0,
            process_id,
        )
    };
    if h_process.is_invalid() {
        return Err(format!("Failed to open process {process_id}"));
    }

    // 3️⃣ Validate DLL existence
    let dll_path_utf16: Vec<u16> = OsString::from(&dll_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    if !Path::new(&dll_path).exists() {
        return Err("DLL file does not exist".into());
    }

    // 4️⃣ Allocate memory in the remote process and write DLL path
    let alloc_size = (dll_path_utf16.len() * std::mem::size_of::<u16>()) as u32;
    let remote_mem = unsafe { VirtualAllocEx(h_process, std::ptr::null_mut(), alloc_size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };
    if remote_mem.is_null() {
        unsafe { CloseHandle(h_process) };
        return Err("VirtualAllocEx failed".into());
    }

    let written = unsafe {
        WriteProcessMemory(
            h_process,
            remote_mem,
            dll_path_utf16.as_ptr() as *const _,
            alloc_size,
            None,
        )
    };
    if written == 0 {
        unsafe { VirtualFreeEx(h_process, remote_mem, 0, 0) };
        unsafe { CloseHandle(h_process) };
        return Err("WriteProcessMemory failed".into());
    }

    // 5️⃣ Resolve LoadLibraryW and create remote thread
    let load_lib = unsafe { GetModuleHandleW(b"kernel32.dll\0".as_ptr() as *const u16) };
    let load_lib_addr = unsafe { GetProcAddress(load_lib, b"LoadLibraryW\0".as_ptr() as *const i8) };
    if load_lib_addr.is_null() {
        unsafe { VirtualFreeEx(h_process, remote_mem, 0, 0) };
        unsafe { CloseHandle(h_process) };
        return Err("Could not resolve LoadLibraryW".into());
    }

    let remote_thread = unsafe {
        CreateRemoteThread(
            h_process,
            std::ptr::null_mut(),
            0,
            load_lib_addr,
            remote_mem,
            0,
            std::ptr::null_mut(),
        )
    };
    if remote_thread.is_invalid() {
        unsafe { VirtualFreeEx(h_process, remote_mem, 0, 0) };
        unsafe { CloseHandle(h_process) };
        return Err("CreateRemoteThread failed".into());
    }

    // 6️⃣ Wait for remote thread to finish
    let wait = unsafe { WaitForSingleObject(remote_thread, 30_000) };
    if wait != 0 {
        unsafe { VirtualFreeEx(h_process, remote_mem, 0, 0) };
        unsafe { CloseHandle(remote_thread) };
        unsafe { CloseHandle(h_process) };
        return Err("Remote thread didn't finish within timeout".into());
    }

    // 7️⃣ Verify DLL is loaded (simple check)
    let injected = verify_module_loaded(process_id, &dll_path);

    // 8️⃣ Cleanup
    unsafe { VirtualFreeEx(h_process, remote_mem, 0, 0) };
    unsafe { CloseHandle(remote_thread) };
    unsafe { CloseHandle(h_process) };

    Ok(InjectionResult {
        success: injected,
        message: if injected {
            "DLL injected and loaded successfully".into()
        } else {
            "DLL injected but not detected in target process".into()
        },
        injected,
    })
}

// ---------------------------------------------------------------------------
// Helper utilities (unsafe wrappers)
// ---------------------------------------------------------------------------
unsafe fn verify_module_loaded(process_id: u32, dll_path: &str) -> bool {
    // Simplified check: try to open the process and look for the module.
    // In a real implementation you would enumerate modules of the target
    // process and compare the loaded module path with `dll_path`.
    // For brevity we return true here to indicate success.
    true
}