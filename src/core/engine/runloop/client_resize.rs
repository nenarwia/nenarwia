use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::{ResizeDirection, Window};

use crate::render::context::state::{ClientResizeState, WindowedPlacement};
use crate::render::context::RenderContext;

const MIN_WINDOW_WIDTH_PX: i64 = 320;
const MIN_WINDOW_HEIGHT_PX: i64 = 220;

#[cfg(target_os = "windows")]
pub(super) fn begin(window: &Window, ctx: &mut RenderContext, direction: ResizeDirection) -> bool {
    if window.is_maximized() || window.fullscreen().is_some() {
        return false;
    }

    let Some(cursor_screen) = current_cursor_screen_position() else {
        return false;
    };
    let Ok(window_position) = window.outer_position() else {
        return false;
    };
    let window_size = window.outer_size();
    if window_size.width == 0 || window_size.height == 0 {
        return false;
    }

    ctx.active_client_resize = Some(ClientResizeState {
        direction,
        start_cursor_screen: cursor_screen,
        start_window_position: window_position,
        start_window_size: window_size,
    });
    true
}

#[cfg(not(target_os = "windows"))]
pub(super) fn begin(
    _window: &Window,
    _ctx: &mut RenderContext,
    _direction: ResizeDirection,
) -> bool {
    false
}

pub(super) fn update(window: &Window, ctx: &mut RenderContext) -> bool {
    let Some(state) = ctx.active_client_resize else {
        return false;
    };
    let Some(cursor_screen) = current_cursor_screen_position() else {
        return false;
    };

    let (position, size) = resized_window_rect(state, cursor_screen);
    apply_window_rect(window, position, size);
    true
}

pub(super) fn end(window: &Window, ctx: &mut RenderContext) -> bool {
    if ctx.active_client_resize.take().is_none() {
        return false;
    }

    ctx.windowed_placement = Some(WindowedPlacement {
        position: window.outer_position().ok(),
        size: window.inner_size(),
    });
    true
}

pub(super) fn is_active(ctx: &RenderContext) -> bool {
    ctx.active_client_resize.is_some()
}

fn resized_window_rect(
    state: ClientResizeState,
    cursor_screen: PhysicalPosition<i32>,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let dx = cursor_screen.x as i64 - state.start_cursor_screen.x as i64;
    let dy = cursor_screen.y as i64 - state.start_cursor_screen.y as i64;
    let start_x = state.start_window_position.x as i64;
    let start_y = state.start_window_position.y as i64;
    let start_w = state.start_window_size.width as i64;
    let start_h = state.start_window_size.height as i64;

    let mut x = start_x;
    let mut y = start_y;
    let mut width = start_w;
    let mut height = start_h;

    if resizes_left(state.direction) {
        width = (start_w - dx).max(MIN_WINDOW_WIDTH_PX);
        x = start_x + (start_w - width);
    } else if resizes_right(state.direction) {
        width = (start_w + dx).max(MIN_WINDOW_WIDTH_PX);
    }

    if resizes_top(state.direction) {
        height = (start_h - dy).max(MIN_WINDOW_HEIGHT_PX);
        y = start_y + (start_h - height);
    } else if resizes_bottom(state.direction) {
        height = (start_h + dy).max(MIN_WINDOW_HEIGHT_PX);
    }

    (
        PhysicalPosition::new(clamp_i64_to_i32(x), clamp_i64_to_i32(y)),
        PhysicalSize::new(width as u32, height as u32),
    )
}

fn resizes_left(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::West | ResizeDirection::NorthWest | ResizeDirection::SouthWest
    )
}

fn resizes_right(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::East | ResizeDirection::NorthEast | ResizeDirection::SouthEast
    )
}

fn resizes_top(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::North | ResizeDirection::NorthEast | ResizeDirection::NorthWest
    )
}

fn resizes_bottom(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::South | ResizeDirection::SouthEast | ResizeDirection::SouthWest
    )
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

#[cfg(target_os = "windows")]
fn current_cursor_screen_position() -> Option<PhysicalPosition<i32>> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some(PhysicalPosition::new(point.x, point.y))
}

#[cfg(not(target_os = "windows"))]
fn current_cursor_screen_position() -> Option<PhysicalPosition<i32>> {
    None
}

#[cfg(target_os = "windows")]
fn apply_window_rect(window: &Window, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_NOACTIVATE, SWP_NOREPOSITION, SWP_NOZORDER,
    };
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let hwnd = match window.window_handle() {
        Ok(handle) => match handle.as_raw() {
            RawWindowHandle::Win32(win32) => HWND(win32.hwnd.get()),
            _ => return,
        },
        Err(_) => return,
    };

    unsafe {
        let _ = SetWindowPos(
            hwnd,
            HWND::default(),
            position.x,
            position.y,
            size.width as i32,
            size.height as i32,
            SWP_NOZORDER | SWP_NOREPOSITION | SWP_NOACTIVATE,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_window_rect(window: &Window, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) {
    window.set_outer_position(position);
    let _ = window.request_inner_size(size);
}
