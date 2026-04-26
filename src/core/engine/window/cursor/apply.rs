use winit::window::{ResizeDirection, Window};

use ::windows::Win32::UI::WindowsAndMessaging::HCURSOR;

use super::custom::custom_cursor_handle;
use super::mode::{
    self, CURSOR_MODE_CUSTOM, CURSOR_MODE_RESIZE_EW, CURSOR_MODE_RESIZE_NESW,
    CURSOR_MODE_RESIZE_NS, CURSOR_MODE_RESIZE_NWSE,
};
use super::system::system_cursor_handles;

pub fn set_native_cursor_custom(window: &Window) {
    mode::set_custom_mode();
    apply_native_cursor_mode(window);
}

pub fn set_native_cursor_resize(window: &Window, direction: ResizeDirection) {
    mode::set_resize_mode(direction);
    apply_native_cursor_mode(window);
}

pub fn native_cursor_handle_for_current_mode() -> Option<HCURSOR> {
    resolve_cursor_handle_for_mode(mode::current_mode())
}

pub(super) fn apply_native_cursor_mode(window: &Window) {
    if let Some(cursor) = native_cursor_handle_for_current_mode() {
        apply_native_cursor_to_window(window, cursor);
    }
}

pub(super) fn apply_native_cursor_to_window(_window: &Window, cursor: HCURSOR) {
    use ::windows::Win32::UI::WindowsAndMessaging::SetCursor;

    unsafe {
        let _ = SetCursor(cursor);
    }
}

fn resolve_cursor_handle_for_mode(mode: u8) -> Option<HCURSOR> {
    if mode == CURSOR_MODE_CUSTOM {
        if let Some(cursor) = custom_cursor_handle() {
            return Some(cursor);
        }
        return system_cursor_handles().map(|handles| handles.arrow);
    }

    let handles = system_cursor_handles()?;
    match mode {
        CURSOR_MODE_RESIZE_EW => Some(handles.ew),
        CURSOR_MODE_RESIZE_NS => Some(handles.ns),
        CURSOR_MODE_RESIZE_NESW => Some(handles.nesw),
        CURSOR_MODE_RESIZE_NWSE => Some(handles.nwse),
        _ => Some(handles.arrow),
    }
}
