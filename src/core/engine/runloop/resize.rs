use winit::dpi::PhysicalPosition;
use winit::window::{ResizeDirection, Window};

const BORDER_RESIZE_THICKNESS_PX: f64 = 4.0;
pub(super) const BORDER_RESIZE_RELEASE_THICKNESS_PX: f64 = 8.0;

pub(super) fn resize_direction_for_cursor(
    window: &Window,
    cursor_pos: PhysicalPosition<f64>,
) -> Option<ResizeDirection> {
    resize_direction_for_cursor_with_thickness(window, cursor_pos, BORDER_RESIZE_THICKNESS_PX)
}

pub(super) fn resize_direction_for_cursor_with_thickness(
    window: &Window,
    cursor_pos: PhysicalPosition<f64>,
    thickness_px: f64,
) -> Option<ResizeDirection> {
    if window.is_maximized() || window.fullscreen().is_some() {
        return None;
    }

    let size = window.inner_size();
    if size.width == 0 || size.height == 0 {
        return None;
    }
    let w = size.width as f64;
    let h = size.height as f64;
    let x = cursor_pos.x;
    let y = cursor_pos.y;
    if !(0.0..=w).contains(&x) || !(0.0..=h).contains(&y) {
        return None;
    }

    let thickness_px = thickness_px.max(0.0);
    let left = x <= thickness_px;
    let right = x >= (w - thickness_px);
    let top = y <= thickness_px;
    let bottom = y >= (h - thickness_px);

    match (left, right, top, bottom) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (true, _, _, true) => Some(ResizeDirection::SouthWest),
        (_, true, true, _) => Some(ResizeDirection::NorthEast),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        (true, _, _, _) => Some(ResizeDirection::West),
        (_, true, _, _) => Some(ResizeDirection::East),
        (_, _, true, _) => Some(ResizeDirection::North),
        (_, _, _, true) => Some(ResizeDirection::South),
        _ => None,
    }
}
