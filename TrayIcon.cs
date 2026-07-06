using System;
using System.Runtime.InteropServices;

namespace GlacierLauncher;

/// <summary>
/// Minimal system-tray icon via Shell_NotifyIcon, deliberately avoiding
/// System.Windows.Forms.NotifyIcon — pulling in WinForms would bloat the
/// self-contained publish. A hidden message-only window receives the tray
/// callbacks; left double-click restores, right-click shows a small menu.
/// </summary>
public sealed class TrayIcon : IDisposable
{
    private const int WM_APP_TRAY = 0x8000 + 1;   // WM_APP + 1
    private const int WM_LBUTTONUP = 0x0202;
    private const int WM_LBUTTONDBLCLK = 0x0203;
    private const int WM_RBUTTONUP = 0x0205;
    private const int NIM_ADD = 0x0;
    private const int NIM_DELETE = 0x2;
    private const int NIF_MESSAGE = 0x1;
    private const int NIF_ICON = 0x2;
    private const int NIF_TIP = 0x4;
    private const uint TPM_RETURNCMD = 0x0100;
    private const uint MF_STRING = 0x0;

    private const int CMD_OPEN = 1;
    private const int CMD_EXIT = 2;

    private readonly IntPtr _hwnd;
    private readonly WndProcDelegate _wndProc;   // kept alive to avoid GC of the callback
    private readonly IntPtr _hicon;
    private bool _added;

    public event Action? OnOpen;
    public event Action? OnExit;

    private delegate IntPtr WndProcDelegate(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);

    public TrayIcon(string tooltip, string iconPath)
    {
        _wndProc = WndProc;

        var hInstance = GetModuleHandle(null);
        var wc = new WNDCLASS
        {
            lpfnWndProc = Marshal.GetFunctionPointerForDelegate(_wndProc),
            hInstance = hInstance,
            lpszClassName = "GlacierTrayWnd_" + Guid.NewGuid().ToString("N"),
        };
        // RegisterClass MUST bind RegisterClassW (CharSet.Unicode): lpszClassName is
        // marshalled as a wide string and the window is created with CreateWindowExW.
        // Binding the ANSI RegisterClassA against a wide name registers a garbled
        // class, so CreateWindowExW then fails with ERROR_CANNOT_FIND_WND_CLASS (1407)
        // and the whole tray silently breaks.
        if (RegisterClass(ref wc) == 0)
            throw new InvalidOperationException($"Tray class registration failed (Win32 {Marshal.GetLastWin32Error()}).");

        // HWND_MESSAGE (-3) → message-only window: no taskbar/screen presence.
        _hwnd = CreateWindowEx(0, wc.lpszClassName, "", 0, 0, 0, 0, 0, new IntPtr(-3), IntPtr.Zero, hInstance, IntPtr.Zero);
        if (_hwnd == IntPtr.Zero)
            throw new InvalidOperationException($"Tray window creation failed (Win32 {Marshal.GetLastWin32Error()}).");

        // Prefer the branded icon at true tray size; fall back to default-size, then
        // to the generic application icon so the tray entry is ALWAYS visible — a
        // hidden window with no icon would be unrecoverable.
        _hicon = LoadImage(IntPtr.Zero, iconPath, 1 /*IMAGE_ICON*/, 16, 16, 0x00000010 /*LR_LOADFROMFILE*/);
        if (_hicon == IntPtr.Zero)
            _hicon = LoadImage(IntPtr.Zero, iconPath, 1, 0, 0, 0x00000010 | 0x00000040 /*LR_DEFAULTSIZE*/);
        if (_hicon == IntPtr.Zero)
            _hicon = LoadIcon(IntPtr.Zero, (IntPtr)32512 /*IDI_APPLICATION*/);

        var data = new NOTIFYICONDATA
        {
            cbSize = Marshal.SizeOf<NOTIFYICONDATA>(),
            hWnd = _hwnd,
            uID = 1,
            uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage = WM_APP_TRAY,
            hIcon = _hicon,
            szTip = tooltip,
        };
        _added = Shell_NotifyIcon(NIM_ADD, ref data);
        if (!_added)
        {
            DestroyWindow(_hwnd);
            throw new InvalidOperationException($"Shell_NotifyIcon add failed (Win32 {Marshal.GetLastWin32Error()}).");
        }
    }

    private IntPtr WndProc(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam)
    {
        if (msg == WM_APP_TRAY)
        {
            var evt = lParam.ToInt32();
            // Single OR double left-click restores — a single click is what most
            // users try first, and doing nothing there reads as "tray is broken".
            if (evt == WM_LBUTTONUP || evt == WM_LBUTTONDBLCLK) OnOpen?.Invoke();
            else if (evt == WM_RBUTTONUP) ShowMenu();
            return IntPtr.Zero;
        }
        return DefWindowProc(hWnd, msg, wParam, lParam);
    }

    private void ShowMenu()
    {
        var menu = CreatePopupMenu();
        AppendMenu(menu, MF_STRING, CMD_OPEN, "Open Glacier Launcher");
        AppendMenu(menu, MF_STRING, CMD_EXIT, "Exit");
        GetCursorPos(out var pt);
        // Required so the menu dismisses correctly when clicking away.
        SetForegroundWindow(_hwnd);
        var cmd = TrackPopupMenuEx(menu, TPM_RETURNCMD, pt.X, pt.Y, _hwnd, IntPtr.Zero);
        DestroyMenu(menu);
        if (cmd == CMD_OPEN) OnOpen?.Invoke();
        else if (cmd == CMD_EXIT) OnExit?.Invoke();
    }

    public void Dispose()
    {
        if (_added)
        {
            var data = new NOTIFYICONDATA { cbSize = Marshal.SizeOf<NOTIFYICONDATA>(), hWnd = _hwnd, uID = 1 };
            Shell_NotifyIcon(NIM_DELETE, ref data);
            _added = false;
        }
        if (_hwnd != IntPtr.Zero) DestroyWindow(_hwnd);
    }

    // ── P/Invoke ───────────────────────────────────────────────
    [StructLayout(LayoutKind.Sequential)]
    private struct WNDCLASS
    {
        public uint style;
        public IntPtr lpfnWndProc;
        public int cbClsExtra;
        public int cbWndExtra;
        public IntPtr hInstance;
        public IntPtr hIcon;
        public IntPtr hCursor;
        public IntPtr hbrBackground;
        [MarshalAs(UnmanagedType.LPWStr)] public string? lpszMenuName;
        [MarshalAs(UnmanagedType.LPWStr)] public string lpszClassName;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct NOTIFYICONDATA
    {
        public int cbSize;
        public IntPtr hWnd;
        public int uID;
        public int uFlags;
        public int uCallbackMessage;
        public IntPtr hIcon;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string szTip;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct POINT { public int X; public int Y; }

    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern ushort RegisterClass(ref WNDCLASS lpWndClass);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr GetModuleHandle(string? lpModuleName);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr LoadIcon(IntPtr hInstance, IntPtr lpIconName);

    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr CreateWindowEx(int dwExStyle, string lpClassName, string lpWindowName, int dwStyle,
        int x, int y, int nWidth, int nHeight, IntPtr hWndParent, IntPtr hMenu, IntPtr hInstance, IntPtr lpParam);

    [DllImport("user32.dll")] private static extern IntPtr DefWindowProc(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] private static extern bool DestroyWindow(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr LoadImage(IntPtr hinst, string lpszName, uint uType, int cx, int cy, uint fuLoad);
    [DllImport("user32.dll")] private static extern IntPtr CreatePopupMenu();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern bool AppendMenu(IntPtr hMenu, uint uFlags, int uIDNewItem, string lpNewItem);
    [DllImport("user32.dll")] private static extern bool DestroyMenu(IntPtr hMenu);
    [DllImport("user32.dll")] private static extern bool GetCursorPos(out POINT lpPoint);
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    private static extern int TrackPopupMenuEx(IntPtr hMenu, uint uFlags, int x, int y, IntPtr hWnd, IntPtr lptpm);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern bool Shell_NotifyIcon(int dwMessage, ref NOTIFYICONDATA lpData);
}
