use winit::window::Window;

#[cfg(target_os = "windows")]
pub fn refresh_platform_window_chrome(window: &Window, fake_maximized: bool) {
    use std::sync::OnceLock;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND,
        DWM_WINDOW_CORNER_PREFERENCE,
    };
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn, HRGN};

    const LEGACY_CORNER_DIAMETER_PX: i32 = 16;
    static DWM_WARNED: OnceLock<()> = OnceLock::new();
    static REGION_WARNED: OnceLock<()> = OnceLock::new();
    static REGION_INFO: OnceLock<()> = OnceLock::new();

    let hwnd = match window.window_handle() {
        Ok(handle) => match handle.as_raw() {
            RawWindowHandle::Win32(win32) => HWND(win32.hwnd.get()),
            _ => return,
        },
        Err(_) => return,
    };

    enforce_borderless_window_style(hwnd);

    let is_full_bleed = window.is_maximized() || window.fullscreen().is_some();

    // Fake maximize deliberately stays in the windowed OS state to avoid the
    // Windows minimize/restore freeze. Keep the native frame styles stripped so
    // Win10 cannot repaint a legacy caption during manual resize.
    let corner_pref: DWM_WINDOW_CORNER_PREFERENCE = if is_full_bleed || fake_maximized {
        DWMWCP_DONOTROUND
    } else {
        DWMWCP_ROUND
    };
    let native_result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corner_pref as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        )
    };
    if native_result.is_err() && DWM_WARNED.set(()).is_ok() {
        log::info!("Native DWM rounded corners are unavailable on this window.");
    }

    if is_full_bleed {
        // Drop any region clipping in maximized/fullscreen to prevent right/bottom gaps.
        let cleared = unsafe { SetWindowRgn(hwnd, HRGN::default(), BOOL(1)) };
        if cleared == 0 && REGION_WARNED.set(()).is_ok() {
            log::warn!("Failed to clear rounded region for maximized/fullscreen window.");
        }
        return;
    }

    let size = window.inner_size();
    if size.width == 0 || size.height == 0 {
        return;
    }
    let corner_diameter = if window.is_maximized() {
        0
    } else {
        LEGACY_CORNER_DIAMETER_PX
    };
    let region = unsafe {
        CreateRoundRectRgn(
            0,
            0,
            size.width.saturating_add(1) as i32,
            size.height.saturating_add(1) as i32,
            corner_diameter,
            corner_diameter,
        )
    };
    if region.0 == 0 {
        if REGION_WARNED.set(()).is_ok() {
            log::warn!("Failed to create rounded region.");
        }
        return;
    }

    let applied = unsafe { SetWindowRgn(hwnd, region, BOOL(1)) };
    if applied == 0 {
        // Ownership is transferred only on success, so we free on failure.
        unsafe {
            let _ = DeleteObject(region);
        }
        if REGION_WARNED.set(()).is_ok() {
            log::warn!("Failed to apply rounded region.");
        }
    } else if REGION_INFO.set(()).is_ok() {
        log::info!("Rounded region clipping enabled for stable window corners.");
    }
}

#[cfg(target_os = "windows")]
fn enforce_borderless_window_style(hwnd: windows::Win32::Foundation::HWND) {
    use std::sync::OnceLock;

    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetWindowLongW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_BORDER, WS_CAPTION, WS_CLIPCHILDREN,
        WS_CLIPSIBLINGS, WS_DLGFRAME, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
    };

    static STYLE_INFO: OnceLock<()> = OnceLock::new();

    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    let frame_bits = WS_CAPTION.0 | WS_THICKFRAME.0 | WS_BORDER.0 | WS_DLGFRAME.0;
    let required_bits =
        WS_POPUP.0 | WS_CLIPCHILDREN.0 | WS_CLIPSIBLINGS.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0;
    let next_style = (style & !frame_bits) | required_bits;

    if next_style == style {
        return;
    }

    unsafe {
        let _ = SetWindowLongW(hwnd, GWL_STYLE, next_style as i32);
        let _ = SetWindowPos(
            hwnd,
            windows::Win32::Foundation::HWND::default(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }

    if STYLE_INFO.set(()).is_ok() {
        log::info!("Native Win32 frame styles stripped for custom chrome.");
    }
}

#[cfg(not(target_os = "windows"))]
pub fn refresh_platform_window_chrome(_window: &Window, _fake_maximized: bool) {}
