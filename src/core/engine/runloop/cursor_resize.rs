use crate::core::engine::window::{
    native_cursor_resize_direction, set_native_cursor_custom, set_native_cursor_resize,
};
use crate::render::context::RenderContext;
#[cfg(not(target_os = "windows"))]
use winit::window::CursorIcon;
use winit::window::{ResizeDirection, Window};

use super::resize;

pub(super) fn apply_cursor_for_current_position(window: &Window, ctx: &RenderContext) {
    #[cfg(target_os = "windows")]
    {
        let direction = if native_cursor_resize_direction().is_some() {
            active_resize_direction_with_thickness(
                window,
                ctx,
                resize::BORDER_RESIZE_RELEASE_THICKNESS_PX,
            )
        } else {
            active_resize_direction(window, ctx)
        };
        if let Some(direction) = direction {
            set_native_cursor_resize(window, direction);
            return;
        }
        apply_default_or_native_custom_cursor(window);
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(direction) = active_resize_direction(window, ctx) {
            window.set_cursor_icon(resize_cursor_icon(direction));
            return;
        }
        apply_default_or_native_custom_cursor(window);
    }
}

pub(super) fn active_resize_direction_with_thickness(
    window: &Window,
    ctx: &RenderContext,
    thickness_px: f64,
) -> Option<ResizeDirection> {
    ctx.cursor_pos.and_then(|pos| {
        resize::resize_direction_for_cursor_with_thickness(window, pos, thickness_px)
    })
}

pub(super) fn apply_default_or_native_custom_cursor(window: &Window) {
    #[cfg(target_os = "windows")]
    {
        set_native_cursor_custom(window);
    }
    #[cfg(not(target_os = "windows"))]
    {
        window.set_cursor_icon(CursorIcon::Default);
    }
}

fn active_resize_direction(window: &Window, ctx: &RenderContext) -> Option<ResizeDirection> {
    ctx.cursor_pos
        .and_then(|pos| resize::resize_direction_for_cursor(window, pos))
}

#[cfg(not(target_os = "windows"))]
fn resize_cursor_icon(direction: ResizeDirection) -> CursorIcon {
    match direction {
        ResizeDirection::East | ResizeDirection::West => CursorIcon::EwResize,
        ResizeDirection::North | ResizeDirection::South => CursorIcon::NsResize,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::NeswResize,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::NwseResize,
    }
}
