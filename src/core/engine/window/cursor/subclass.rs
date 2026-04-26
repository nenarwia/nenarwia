use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use winit::window::Window;

static CURSOR_WNDPROC_PREV: AtomicIsize = AtomicIsize::new(0);
static CURSOR_WNDPROC_INSTALLED: AtomicBool = AtomicBool::new(false);

pub(in crate::core::engine::window) fn install_native_cursor_wndproc_subclass(window: &Window) {
    use ::windows::Win32::Foundation::HWND;
    use ::windows::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWLP_WNDPROC};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    if CURSOR_WNDPROC_INSTALLED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let hwnd = match window.window_handle() {
        Ok(handle) => match handle.as_raw() {
            RawWindowHandle::Win32(win32) => HWND(win32.hwnd.get()),
            _ => {
                CURSOR_WNDPROC_INSTALLED.store(false, Ordering::Release);
                return;
            }
        },
        Err(_) => {
            CURSOR_WNDPROC_INSTALLED.store(false, Ordering::Release);
            return;
        }
    };

    let prev = unsafe {
        SetWindowLongPtrW(
            hwnd,
            GWLP_WNDPROC,
            native_cursor_subclass_wndproc as *const () as isize,
        )
    };
    if prev == 0 {
        CURSOR_WNDPROC_INSTALLED.store(false, Ordering::Release);
        log::warn!("Failed to install native cursor wndproc subclass.");
        return;
    }

    CURSOR_WNDPROC_PREV.store(prev, Ordering::Release);
}

unsafe extern "system" fn native_cursor_subclass_wndproc(
    hwnd: ::windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: ::windows::Win32::Foundation::WPARAM,
    lparam: ::windows::Win32::Foundation::LPARAM,
) -> ::windows::Win32::Foundation::LRESULT {
    use ::windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, SetCursor, SetWindowLongPtrW, GWLP_WNDPROC, HTCLIENT,
        WM_NCDESTROY, WM_SETCURSOR, WNDPROC,
    };

    if msg == WM_SETCURSOR {
        let hit_test = (lparam.0 as u32 & 0xFFFF) as i32;
        if hit_test == HTCLIENT as i32 {
            if let Some(cursor) = super::apply::native_cursor_handle_for_current_mode() {
                let _ = SetCursor(cursor);
                return ::windows::Win32::Foundation::LRESULT(1);
            }
        }
    }

    let prev_ptr = CURSOR_WNDPROC_PREV.load(Ordering::Acquire);
    if prev_ptr == 0 {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let prev_proc: WNDPROC = Some(std::mem::transmute(prev_ptr));
    if msg == WM_NCDESTROY {
        let _ = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, prev_ptr);
        CURSOR_WNDPROC_PREV.store(0, Ordering::Release);
        CURSOR_WNDPROC_INSTALLED.store(false, Ordering::Release);
    }
    CallWindowProcW(prev_proc, hwnd, msg, wparam, lparam)
}
