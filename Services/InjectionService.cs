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

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr GetProcAddress(IntPtr hModule, string lpProcName);

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr GetModuleHandle(string lpModuleName);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr CreateRemoteThread(IntPtr hProcess, IntPtr lpThreadAttributes, nuint dwStackSize, IntPtr lpStartAddress, IntPtr lpParameter, uint dwCreationFlags, out uint lpThreadId);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint WaitForSingleObject(IntPtr hHandle, uint dwMilliseconds);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool VirtualFreeEx(IntPtr hProcess, IntPtr lpAddress, nuint dwSize, uint dwFreeType);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr hObject);

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

    private const uint PROCESS_ALL_ACCESS       = 0x1F0FFF;
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

    public static void InjectDll(uint processId, string dllPath)
    {
        // 1. Grant "ALL APPLICATION PACKAGES" full access to the DLL and its parent
        //    directory — some DLLs load dependencies from the same folder, and the
        //    UWP AppContainer blocks access without explicit permissions.
        GrantUwpAccess(dllPath);
        var parentDir = System.IO.Path.GetDirectoryName(dllPath);
        if (!string.IsNullOrEmpty(parentDir))
            GrantUwpAccess(parentDir);

        // 2. Open the target process
        var hProcess = OpenProcess(PROCESS_ALL_ACCESS, false, processId);
        if (hProcess == IntPtr.Zero)
            throw new InvalidOperationException($"OpenProcess failed (err {Marshal.GetLastWin32Error()}).");

        var hAlloc   = IntPtr.Zero;
        var hThread  = IntPtr.Zero;
        var newAcl   = IntPtr.Zero;

        try
        {
            // 3. Allocate memory for the DLL path inside the target process
            var pathBytes = System.Text.Encoding.Unicode.GetBytes(dllPath + "\0");
            hAlloc = VirtualAllocEx(hProcess, IntPtr.Zero, (nuint)pathBytes.Length, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
            if (hAlloc == IntPtr.Zero)
                throw new InvalidOperationException($"VirtualAllocEx failed (err {Marshal.GetLastWin32Error()}).");

            WriteProcessMemory(hProcess, hAlloc, pathBytes, (nuint)pathBytes.Length, out _);

            // 4. Spawn a remote thread calling LoadLibraryW
            var loadLibrary = GetProcAddress(GetModuleHandle("kernel32.dll"), "LoadLibraryW");
            hThread = CreateRemoteThread(hProcess, IntPtr.Zero, 0, loadLibrary, hAlloc, 0, out _);
            if (hThread == IntPtr.Zero)
                throw new InvalidOperationException($"CreateRemoteThread failed (err {Marshal.GetLastWin32Error()}).");

            WaitForSingleObject(hThread, INFINITE);
        }
        finally
        {
            if (hAlloc  != IntPtr.Zero) VirtualFreeEx(hProcess, hAlloc, 0, MEM_RELEASE);
            if (hThread != IntPtr.Zero) CloseHandle(hThread);
            if (newAcl  != IntPtr.Zero) LocalFree(newAcl);
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
