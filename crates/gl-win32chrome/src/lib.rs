//! Custom window chrome, resize hit-testing, and DWM calls not covered by
//! Tauri's window-decoration APIs, ported from `MainWindow.xaml.cs`'s
//! borderless-window handling.
//!
//! Tauri v2 exposes the raw platform `HWND` via `WebviewWindow::hwnd()`, so
//! this crate takes an `HWND` directly rather than owning window creation —
//! it's meant to be called from a Tauri `setup` hook (for [`apply_chrome`]
//! and [`install_resize_hittest`]) and, once wired to a `WM_NCHITTEST`
//! subclass callback, from that callback (for [`hit_test`]).
//!
//! [`hit_test`] is the one piece of this crate that's pure logic (screen
//! coordinates + window size in, a `HitTestResult` out) and is unit-tested
//! directly; the DWM/subclass calls themselves can only be verified against
//! a real window.

#![cfg(windows)]

use windows::core::Result as WinResult;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{WM_NCHITTEST, WM_NCLBUTTONDBLCLK};

/// Width, in logical pixels, of the invisible resize border around a
/// borderless window — matches `MainWindow.xaml.cs`'s `ResizeBorder`.
pub const RESIZE_BORDER_PX: i32 = 6;

/// A `WM_NCHITTEST` result, matching the Win32 `HT*` constants used by
/// `MainWindow.xaml.cs`'s `WndProc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestResult {
    Left,
    Right,
    Top,
    TopLeft,
    TopRight,
    Bottom,
    BottomLeft,
    BottomRight,
}

impl HitTestResult {
    /// The raw Win32 `HT*` value to return from a `WM_NCHITTEST` handler.
    pub const fn win32_value(self) -> isize {
        match self {
            HitTestResult::Left => 10,        // HTLEFT
            HitTestResult::Right => 11,       // HTRIGHT
            HitTestResult::Top => 12,         // HTTOP
            HitTestResult::TopLeft => 13,     // HTTOPLEFT
            HitTestResult::TopRight => 14,    // HTTOPRIGHT
            HitTestResult::Bottom => 15,      // HTBOTTOM
            HitTestResult::BottomLeft => 16,  // HTBOTTOMLEFT
            HitTestResult::BottomRight => 17, // HTBOTTOMRIGHT
        }
    }
}

/// Pure resize-border hit test: given a point in *client* coordinates
/// (already converted from screen coordinates by the caller, as
/// `MainWindow.xaml.cs`'s `WndProc` does via `PointFromScreen`) and the
/// window's client size, decides which resize handle (if any) the point
/// falls in. Returns `None` for points in the interior (not a resize edge).
///
/// Mirrors `MainWindow.xaml.cs::WndProc`'s hit-test branch exactly,
/// including checking corners before edges.
pub fn hit_test(
    client_x: f64,
    client_y: f64,
    client_width: f64,
    client_height: f64,
    border: i32,
) -> Option<HitTestResult> {
    let b = border as f64;
    let left = client_x <= b;
    let right = client_x >= client_width - b;
    let top = client_y <= b;
    let bottom = client_y >= client_height - b;

    if top && left {
        Some(HitTestResult::TopLeft)
    } else if top && right {
        Some(HitTestResult::TopRight)
    } else if bottom && left {
        Some(HitTestResult::BottomLeft)
    } else if bottom && right {
        Some(HitTestResult::BottomRight)
    } else if left {
        Some(HitTestResult::Left)
    } else if right {
        Some(HitTestResult::Right)
    } else if top {
        Some(HitTestResult::Top)
    } else if bottom {
        Some(HitTestResult::Bottom)
    } else {
        None
    }
}

/// Applies rounded corners + the dark immersive titlebar to `hwnd`, matching
/// the `DwmSetWindowAttribute` calls in `MainWindow.xaml.cs`'s
/// `SourceInitialized` handler. Call once, after the window's native handle
/// exists (e.g. in a Tauri `setup` hook via `window.hwnd()`).
pub fn apply_chrome(hwnd: HWND) -> WinResult<()> {
    unsafe {
        let round = DWMWCP_ROUND;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &round as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&round) as u32,
        )?;

        let dark: i32 = 1;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&dark) as u32,
        )?;
    }
    Ok(())
}

/// Subclass ID used for [`install_resize_hittest`]; arbitrary but constant so
/// a second call is idempotent (`SetWindowSubclass` replaces an existing
/// subclass with the same id/data rather than stacking a duplicate).
const HITTEST_SUBCLASS_ID: usize = 0xC1AC1E5; // "GLACIER", roughly.

/// Installs a `WM_NCHITTEST` subclass on `hwnd` that:
/// - swallows `WM_NCLBUTTONDBLCLK` (matches the C# code: a borderless window
///   has no OS titlebar to double-click-maximize, so this is intentionally a
///   no-op rather than falling through to default double-click handling)
/// - answers `WM_NCHITTEST` using [`hit_test`] against [`RESIZE_BORDER_PX`]
/// - defers every other message to `DefSubclassProc`
///
/// This is the Rust equivalent of `HwndSource.AddHook(WndProc)` in the
/// original WPF window — WPF's own window-message hook mechanism has no
/// Tauri/`wry` equivalent, so this uses the standard Win32 `comctl32`
/// subclassing API directly.
pub fn install_resize_hittest(hwnd: HWND) -> WinResult<()> {
    let ok =
        unsafe { SetWindowSubclass(hwnd, Some(hittest_subclass_proc), HITTEST_SUBCLASS_ID, 0) };
    if ok.as_bool() {
        Ok(())
    } else {
        Err(windows::core::Error::from_win32())
    }
}

unsafe extern "system" fn hittest_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    _ref_data: usize,
) -> LRESULT {
    if msg == WM_NCLBUTTONDBLCLK {
        return LRESULT(0);
    }

    if msg == WM_NCHITTEST {
        if let Some(result) = hittest_from_lparam(hwnd, lparam) {
            return LRESULT(result.win32_value());
        }
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Extracts the screen point from a `WM_NCHITTEST` `lParam`, converts it to
/// client coordinates, and runs [`hit_test`] against the window's current
/// client rect.
unsafe fn hittest_from_lparam(hwnd: HWND, lparam: LPARAM) -> Option<HitTestResult> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let raw = lparam.0 as u32;
    let screen_x = (raw & 0xFFFF) as i16 as i32;
    let screen_y = ((raw >> 16) & 0xFFFF) as i16 as i32;

    let mut pt = POINT {
        x: screen_x,
        y: screen_y,
    };
    if ScreenToClient(hwnd, &mut pt).as_bool() {
        let mut rect = Default::default();
        if GetClientRect(hwnd, &mut rect).is_ok() {
            let width = (rect.right - rect.left) as f64;
            let height = (rect.bottom - rect.top) as f64;
            return hit_test(pt.x as f64, pt.y as f64, width, height, RESIZE_BORDER_PX);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interior_point_is_not_a_resize_edge() {
        assert_eq!(hit_test(400.0, 300.0, 800.0, 600.0, 6), None);
    }

    #[test]
    fn corners_take_priority_over_edges() {
        assert_eq!(
            hit_test(0.0, 0.0, 800.0, 600.0, 6),
            Some(HitTestResult::TopLeft)
        );
        assert_eq!(
            hit_test(799.0, 0.0, 800.0, 600.0, 6),
            Some(HitTestResult::TopRight)
        );
        assert_eq!(
            hit_test(0.0, 599.0, 800.0, 600.0, 6),
            Some(HitTestResult::BottomLeft)
        );
        assert_eq!(
            hit_test(799.0, 599.0, 800.0, 600.0, 6),
            Some(HitTestResult::BottomRight)
        );
    }

    #[test]
    fn plain_edges_outside_corner_zone() {
        assert_eq!(
            hit_test(2.0, 300.0, 800.0, 600.0, 6),
            Some(HitTestResult::Left)
        );
        assert_eq!(
            hit_test(798.0, 300.0, 800.0, 600.0, 6),
            Some(HitTestResult::Right)
        );
        assert_eq!(
            hit_test(400.0, 2.0, 800.0, 600.0, 6),
            Some(HitTestResult::Top)
        );
        assert_eq!(
            hit_test(400.0, 598.0, 800.0, 600.0, 6),
            Some(HitTestResult::Bottom)
        );
    }

    #[test]
    fn boundary_is_inclusive_matching_csharp_le_ge() {
        // C#: `left = pt.X <= b`, `right = pt.X >= w - b` — boundary pixels
        // count as inside the border, not the interior.
        assert_eq!(
            hit_test(6.0, 300.0, 800.0, 600.0, 6),
            Some(HitTestResult::Left)
        );
        assert_eq!(hit_test(7.0, 300.0, 800.0, 600.0, 6), None);
    }

    #[test]
    fn hit_test_result_win32_values_match_win32_ht_constants() {
        assert_eq!(HitTestResult::Left.win32_value(), 10);
        assert_eq!(HitTestResult::Right.win32_value(), 11);
        assert_eq!(HitTestResult::Top.win32_value(), 12);
        assert_eq!(HitTestResult::TopLeft.win32_value(), 13);
        assert_eq!(HitTestResult::TopRight.win32_value(), 14);
        assert_eq!(HitTestResult::Bottom.win32_value(), 15);
        assert_eq!(HitTestResult::BottomLeft.win32_value(), 16);
        assert_eq!(HitTestResult::BottomRight.win32_value(), 17);
    }
}
