#[cfg(not(target_os = "windows"))]
use winit::window::Window;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::{
    install_native_custom_cursor, native_cursor_handle_for_current_mode,
    native_cursor_resize_direction, set_native_cursor_custom, set_native_cursor_resize,
};

#[cfg(target_os = "windows")]
pub(super) use windows::install_native_cursor_wndproc_subclass;

#[cfg(not(target_os = "windows"))]
pub fn install_native_custom_cursor(_window: &Window) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn set_native_cursor_custom(_window: &Window) {}

#[cfg(not(target_os = "windows"))]
pub fn set_native_cursor_resize(_window: &Window, _direction: winit::window::ResizeDirection) {}

#[cfg(not(target_os = "windows"))]
pub fn native_cursor_resize_direction() -> Option<winit::window::ResizeDirection> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn native_cursor_handle_for_current_mode() -> Option<()> {
    None
}
