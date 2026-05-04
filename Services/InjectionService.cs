using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

public static class InjectionService
{
    // ── COM: IApplicationActivationManager ─────────────────────

    [ComImport]
    [Guid("2e941141-7f97-4756-ba1d-9decde894a3d")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IApplicationActivationManager
    {
        uint ActivateApplication(
            [MarshalAs(UnmanagedType.LPWStr)] string appUserModelId,
            [MarshalAs(UnmanagedType.LPWStr)] string? arguments,
            int options,
            out uint processId);

        uint ActivateForFile(
            [MarshalAs(UnmanagedType.LPWStr)] string appUserModelId,
            IntPtr itemArray,
            [MarshalAs(UnmanagedType.LPWStr)] string? verb,
            out uint processId);

        uint ActivateForProtocol(
            [MarshalAs(UnmanagedType.LPWStr)] string appUserModelId,
            IntPtr itemArray,
            out uint processId);
    }

    [ComImport]
    [Guid("45ba127d-10a8-46ea-8ab7-56ea9078943c")]
    private class ApplicationActivationManager { }

    // ── P/Invoke ────────────────────────────────────────────────

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr OpenProcess(uint dwDesiredAccess, bool bInheritHandle, uint dwProcessId);

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr VirtualAllocEx(IntPtr hProcess, IntPtr lpAddress, nuint dwSize, uint flAllocationType, uint flProtect);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool WriteProcessMemory(IntPtr hProcess, IntPtr lpBaseAddress, byte[] lpBuffer, nuint nSize, out nuint lpNumberOfBytesWritten);

    // GetProcAddress takes LPCSTR (ANSI) — there is no Unicode "GetProcAddressW".
    // Marshalling the function name as wide chars looks up "L\0o\0a\0d\0…" and
    // returns NULL, which is the bug we're fixing here.
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Ansi, ExactSpelling = true, BestFitMapping = false)]
    private static extern IntPtr GetProcAddress(IntPtr hModule, [MarshalAs(UnmanagedType.LPStr)] string lpProcName);

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode, EntryPoint = "GetModuleHandleW")]
    private static extern IntPtr GetModuleHandle(string lpModuleName);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr CreateRemoteThread(IntPtr hProcess, IntPtr lpThreadAttributes, nuint dwStackSize, IntPtr lpStartAddress, IntPtr lpParameter, uint dwCreationFlags, out uint lpThreadId);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint WaitForSingleObject(IntPtr hHandle, uint dwMilliseconds);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool VirtualFreeEx(IntPtr hProcess, IntPtr lpAddress, nuint dwSize, uint dwFreeType);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr hObject);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetExitCodeThread(IntPtr hThread, out uint lpExitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GetCurrentProcess();

    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern bool OpenProcessToken(IntPtr ProcessHandle, uint DesiredAccess, out IntPtr TokenHandle);

    [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern bool LookupPrivilegeValue(string? lpSystemName, string lpName, out long lpLuid);

    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern bool AdjustTokenPrivileges(IntPtr TokenHandle, bool DisableAllPrivileges, ref TOKEN_PRIVILEGES NewState, uint BufferLength, IntPtr PreviousState, IntPtr ReturnLength);

    [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern uint SetNamedSecurityInfo(string pObjectName, uint ObjectType, uint SecurityInfo, IntPtr psidOwner, IntPtr psidGroup, IntPtr pDacl, IntPtr pSacl);

    [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern uint BuildTrusteeWithName(ref TRUSTEE pTrustee, string pName);

    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern uint SetEntriesInAcl(uint cCountOfExplicitEntries, ref EXPLICIT_ACCESS pListOfExplicitEntries, IntPtr OldAcl, out IntPtr NewAcl);

    [DllImport("kernel32.dll")]
    private static extern IntPtr LocalFree(IntPtr hMem);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct TRUSTEE
    {
        public IntPtr pMultipleTrustee;
        public int MultipleTrusteeOperation;
        public int TrusteeForm;
        public int TrusteeType;
        [MarshalAs(UnmanagedType.LPWStr)] public string ptstrName;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct EXPLICIT_ACCESS
    {
        public uint grfAccessPermissions;
        public int  grfAccessMode;
        public uint grfInheritance;
        public TRUSTEE Trustee;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TOKEN_PRIVILEGES
    {
        public uint PrivilegeCount;
        public long Luid;
        public uint Attributes;
    }

    private const uint TOKEN_ADJUST_PRIVILEGES = 0x0020;
    private const uint TOKEN_QUERY             = 0x0008;
    private const uint SE_PRIVILEGE_ENABLED    = 0x00000002;
    private const string SE_DEBUG_NAME         = "SeDebugPrivilege";

    private const uint PROCESS_ALL_ACCESS       = 0x1F0FFF;
    // Just the access rights actually needed for DLL injection — used as a fallback when
    // PROCESS_ALL_ACCESS is denied (e.g. running as a non-admin against a UWP process).
    private const uint PROCESS_INJECT_ACCESS    = 0x0002 /*CREATE_THREAD*/
                                                | 0x0008 /*VM_OPERATION*/
                                                | 0x0010 /*VM_READ*/
                                                | 0x0020 /*VM_WRITE*/
                                                | 0x0400 /*QUERY_INFORMATION*/;
    private const uint MEM_COMMIT               = 0x1000;
    private const uint MEM_RESERVE              = 0x2000;
    private const uint MEM_RELEASE              = 0x8000;
    private const uint PAGE_READWRITE           = 0x04;
    private const uint INFINITE                 = 0xFFFFFFFF;
    private const uint SE_FILE_OBJECT           = 1;
    private const uint DACL_SECURITY_INFORMATION = 4;
    private const uint SUB_CONTAINERS_AND_OBJECTS_INHERIT = 3;
    private const uint SET_ACCESS              = 2;
    private const uint GENERIC_ALL             = 0x10000000;
    private const int  TRUSTEE_IS_NAME         = 1;
    private const int  TRUSTEE_IS_WELL_KNOWN_GROUP = 5;

    // ── Public API ──────────────────────────────────────────────

    public static uint LaunchMinecraft()
    {
        const string appId = "Microsoft.MinecraftUWP_8wekyb3d8bbwe!App";
        var manager = (IApplicationActivationManager)new ApplicationActivationManager();
        manager.ActivateApplication(appId, null, 0, out var pid);
        return pid;
    }

    /// <summary>
    /// Enable SeDebugPrivilege so we can OpenProcess(PROCESS_ALL_ACCESS) on a UWP/AppContainer process.
    /// Silently no-ops if the launcher token can't grant it.
    /// </summary>
    public static void EnableDebugPrivilege()
    {
        try
        {
            if (!OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, out var hToken))
                return;
            try
            {
                if (!LookupPrivilegeValue(null, SE_DEBUG_NAME, out var luid)) return;
                var tp = new TOKEN_PRIVILEGES
                {
                    PrivilegeCount = 1,
                    Luid           = luid,
                    Attributes     = SE_PRIVILEGE_ENABLED
                };
                AdjustTokenPrivileges(hToken, false, ref tp, 0, IntPtr.Zero, IntPtr.Zero);
            }
            finally { CloseHandle(hToken); }
        }
        catch { /* best effort */ }
    }

    public static void InjectDll(uint processId, string dllPath)
    {
        if (!System.IO.File.Exists(dllPath))
            throw new System.IO.FileNotFoundException($"DLL not found: {dllPath}");

        // 1. Grant "ALL APPLICATION PACKAGES" full access to the DLL and its parent
        //    directory — some DLLs load dependencies from the same folder, and the
        //    UWP AppContainer blocks access without explicit permissions.
        GrantUwpAccess(dllPath);
        var parentDir = System.IO.Path.GetDirectoryName(dllPath);
        if (!string.IsNullOrEmpty(parentDir))
            GrantUwpAccess(parentDir);

        // 2. Make sure SeDebugPrivilege is enabled before opening a UWP process at PROCESS_ALL_ACCESS.
        EnableDebugPrivilege();

        // 3. Open the target process. Try the full access mask first; fall back to the
        //    narrower injection-only mask if PROCESS_ALL_ACCESS is denied (typical when
        //    the launcher is not elevated against a UWP/AppContainer process).
        var hProcess = OpenProcess(PROCESS_ALL_ACCESS, false, processId);
        if (hProcess == IntPtr.Zero)
            hProcess = OpenProcess(PROCESS_INJECT_ACCESS, false, processId);
        if (hProcess == IntPtr.Zero)
        {
            var err = Marshal.GetLastWin32Error();
            throw new InvalidOperationException(
                $"Could not open Minecraft process (Win32 error {err}). Try running the launcher as administrator.");
        }

        var hAlloc   = IntPtr.Zero;
        var hThread  = IntPtr.Zero;

        try
        {
            // 4. Allocate memory for the DLL path inside the target process
            var pathBytes = System.Text.Encoding.Unicode.GetBytes(dllPath + "\0");
            hAlloc = VirtualAllocEx(hProcess, IntPtr.Zero, (nuint)pathBytes.Length, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
            if (hAlloc == IntPtr.Zero)
                throw new InvalidOperationException($"VirtualAllocEx failed (Win32 error {Marshal.GetLastWin32Error()}).");

            if (!WriteProcessMemory(hProcess, hAlloc, pathBytes, (nuint)pathBytes.Length, out _))
                throw new InvalidOperationException($"WriteProcessMemory failed (Win32 error {Marshal.GetLastWin32Error()}).");

            // 5. Spawn a remote thread calling LoadLibraryW. kernel32.dll lives at the
            //    same address in every process on this boot, so we can use our copy's address.
            var loadLibrary = GetProcAddress(GetModuleHandle("kernel32.dll"), "LoadLibraryW");
            if (loadLibrary == IntPtr.Zero)
                throw new InvalidOperationException("Could not resolve LoadLibraryW.");

            hThread = CreateRemoteThread(hProcess, IntPtr.Zero, 0, loadLibrary, hAlloc, 0, out _);
            if (hThread == IntPtr.Zero)
                throw new InvalidOperationException($"CreateRemoteThread failed (Win32 error {Marshal.GetLastWin32Error()}).");

            // 6. Wait for LoadLibraryW to return inside Minecraft. 30s is generous; if it
            //    never returns, the DLL is hung in DllMain — bail rather than block forever.
            const uint thirtySeconds = 30_000;
            var wait = WaitForSingleObject(hThread, thirtySeconds);
            if (wait != 0 /* WAIT_OBJECT_0 */)
                throw new InvalidOperationException("Injection thread did not finish (the DLL may be hung in DllMain).");

            // 7. The thread's exit code is the HMODULE returned by LoadLibraryW. Zero means
            //    the DLL was rejected — usually a bad signature/architecture or a missing dependency.
            //    Note: on x64 the HMODULE is 64 bits but the thread exit code is only 32 bits,
            //    so we can't recover the real handle, only check for zero/non-zero.
            if (GetExitCodeThread(hThread, out var exitCode) && exitCode == 0)
            {
                throw new InvalidOperationException(
                    "LoadLibraryW returned NULL inside Minecraft — the DLL was rejected. " +
                    "Common causes: x86 DLL injected into x64 Minecraft, missing dependency, " +
                    "or the DLL crashed in DllMain.");
            }
        }
        finally
        {
            if (hAlloc  != IntPtr.Zero) VirtualFreeEx(hProcess, hAlloc, 0, MEM_RELEASE);
            if (hThread != IntPtr.Zero) CloseHandle(hThread);
            CloseHandle(hProcess);
        }
    }

    private static void GrantUwpAccess(string filePath)
    {
        try
        {
            var trustee = new TRUSTEE
            {
                pMultipleTrustee      = IntPtr.Zero,
                MultipleTrusteeOperation = 0,
                TrusteeForm           = TRUSTEE_IS_NAME,
                TrusteeType           = TRUSTEE_IS_WELL_KNOWN_GROUP,
                ptstrName             = "ALL APPLICATION PACKAGES"
            };

            var access = new EXPLICIT_ACCESS
            {
                grfAccessPermissions = GENERIC_ALL,
                grfAccessMode        = (int)SET_ACCESS,
                grfInheritance       = SUB_CONTAINERS_AND_OBJECTS_INHERIT,
                Trustee              = trustee
            };

            SetEntriesInAcl(1, ref access, IntPtr.Zero, out var newAcl);
            SetNamedSecurityInfo(filePath, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION, IntPtr.Zero, IntPtr.Zero, newAcl, IntPtr.Zero);
            if (newAcl != IntPtr.Zero) LocalFree(newAcl);
        }
        catch { /* non-fatal — injection may still succeed */ }
    }
}
