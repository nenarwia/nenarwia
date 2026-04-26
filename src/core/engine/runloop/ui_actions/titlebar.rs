use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::{Window, WindowId};

use crate::core::engine::window::refresh_platform_window_chrome;
use crate::render::context::RenderContext;

const TITLEBAR_DOUBLE_CLICK_MS: u64 = 500;
const TITLEBAR_DOUBLE_CLICK_RADIUS_PX: f64 = 8.0;
const TITLEBAR_DRAG_THRESHOLD_PX: f64 = 3.0;

#[derive(Clone, Copy, Debug)]
struct TitlebarClickStamp {
    window_id: WindowId,
    at: Instant,
    pos: PhysicalPosition<f64>,
}

pub(super) fn begin_titlebar_window_drag(window: &Window, ctx: &mut RenderContext) {
    if let Some(cursor_pos) = ctx.cursor_pos {
        if is_titlebar_double_click(window.id(), cursor_pos) {
            super::toggle_window_maximize(window, ctx);
            ctx.pending_titlebar_drag_origin = None;
            return;
        }
        ctx.pending_titlebar_drag_origin = Some(cursor_pos);
    } else {
        ctx.pending_titlebar_drag_origin = None;
    }
}

pub(super) fn maybe_start_pending_titlebar_drag(window: &Window, ctx: &mut RenderContext) {
    if !ctx.mouse_left_down {
        return;
    }
    let Some(start) = ctx.pending_titlebar_drag_origin else {
        return;
    };
    let Some(pos) = ctx.cursor_pos else {
        return;
    };

    if !titlebar_drag_threshold_reached(start, pos, TITLEBAR_DRAG_THRESHOLD_PX) {
        return;
    }

    ctx.pending_titlebar_drag_origin = None;
    restore_fake_maximized_for_drag(window, ctx, pos);
    if let Err(err) = window.drag_window() {
        log::warn!("Window drag failed: {err:?}");
    }
    ctx.mouse_left_down = false;
}

fn restore_fake_maximized_for_drag(
    window: &Window,
    ctx: &mut RenderContext,
    cursor_pos: PhysicalPosition<f64>,
) {
    if !ctx.window_fake_maximized {
        return;
    }

    let Some(placement) = ctx.windowed_placement else {
        return;
    };
    if placement.size.width == 0 || placement.size.height == 0 {
        return;
    }

    let current_outer = window
        .outer_position()
        .ok()
        .or(placement.position)
        .unwrap_or_else(|| PhysicalPosition::new(0, 0));
    let next_outer = restored_drag_outer_position(
        current_outer,
        window.inner_size(),
        placement.size,
        cursor_pos,
    );

    ctx.window_fake_maximized = false;
    window.set_maximized(false);
    let _ = window.request_inner_size(placement.size);
    window.set_outer_position(next_outer);
    refresh_platform_window_chrome(window, ctx.window_fake_maximized);
}

fn restored_drag_outer_position(
    current_outer: PhysicalPosition<i32>,
    current_inner: PhysicalSize<u32>,
    restored_inner: PhysicalSize<u32>,
    cursor_pos: PhysicalPosition<f64>,
) -> PhysicalPosition<i32> {
    let cursor_screen_x = current_outer.x as f64 + cursor_pos.x;
    let cursor_screen_y = current_outer.y as f64 + cursor_pos.y;
    let x_ratio = if current_inner.width == 0 {
        0.5
    } else {
        (cursor_pos.x / current_inner.width as f64).clamp(0.0, 1.0)
    };
    let restored_cursor_x = restored_inner.width as f64 * x_ratio;

    PhysicalPosition::new(
        round_to_i32(cursor_screen_x - restored_cursor_x),
        round_to_i32(cursor_screen_y - cursor_pos.y),
    )
}

fn round_to_i32(value: f64) -> i32 {
    value.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

fn titlebar_drag_threshold_reached(
    start: PhysicalPosition<f64>,
    pos: PhysicalPosition<f64>,
    threshold_px: f64,
) -> bool {
    (pos.x - start.x).abs() >= threshold_px || (pos.y - start.y).abs() >= threshold_px
}

fn titlebar_click_matches(
    previous: Option<TitlebarClickStamp>,
    window_id: WindowId,
    pos: PhysicalPosition<f64>,
    now: Instant,
) -> bool {
    previous
        .map(|prev| {
            prev.window_id == window_id
                && now.duration_since(prev.at) <= Duration::from_millis(TITLEBAR_DOUBLE_CLICK_MS)
                && (prev.pos.x - pos.x).abs() <= TITLEBAR_DOUBLE_CLICK_RADIUS_PX
                && (prev.pos.y - pos.y).abs() <= TITLEBAR_DOUBLE_CLICK_RADIUS_PX
        })
        .unwrap_or(false)
}

fn is_titlebar_double_click(window_id: WindowId, pos: PhysicalPosition<f64>) -> bool {
    static LAST: OnceLock<Mutex<Option<TitlebarClickStamp>>> = OnceLock::new();
    let now = Instant::now();
    let lock = LAST.get_or_init(|| Mutex::new(None));
    let mut guard = match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    let is_double = titlebar_click_matches(*guard, window_id, pos, now);

    if is_double {
        *guard = None;
        true
    } else {
        *guard = Some(TitlebarClickStamp {
            window_id,
            at: now,
            pos,
        });
        false
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use winit::dpi::{PhysicalPosition, PhysicalSize};
    use winit::window::WindowId;

    use super::{
        restored_drag_outer_position, titlebar_click_matches, titlebar_drag_threshold_reached,
        TitlebarClickStamp,
    };

    #[test]
    fn drag_threshold_requires_real_movement() {
        let origin = PhysicalPosition::new(10.0, 10.0);

        assert!(!titlebar_drag_threshold_reached(
            origin,
            PhysicalPosition::new(12.9, 10.0),
            3.0,
        ));
        assert!(titlebar_drag_threshold_reached(
            origin,
            PhysicalPosition::new(13.0, 10.0),
            3.0,
        ));
        assert!(titlebar_drag_threshold_reached(
            origin,
            PhysicalPosition::new(10.0, 13.0),
            3.0,
        ));
    }

    #[test]
    fn double_click_requires_same_window_time_and_radius() {
        let now = Instant::now();
        let window_id = WindowId::from(1_u64);
        let other_window_id = WindowId::from(2_u64);
        let previous = Some(TitlebarClickStamp {
            window_id,
            at: now,
            pos: PhysicalPosition::new(100.0, 200.0),
        });

        assert!(titlebar_click_matches(
            previous,
            window_id,
            PhysicalPosition::new(108.0, 208.0),
            now + Duration::from_millis(500),
        ));
        assert!(!titlebar_click_matches(
            previous,
            other_window_id,
            PhysicalPosition::new(108.0, 208.0),
            now + Duration::from_millis(500),
        ));
        assert!(!titlebar_click_matches(
            previous,
            window_id,
            PhysicalPosition::new(108.1, 208.0),
            now + Duration::from_millis(500),
        ));
        assert!(!titlebar_click_matches(
            previous,
            window_id,
            PhysicalPosition::new(108.0, 208.0),
            now + Duration::from_millis(501),
        ));
    }

    #[test]
    fn restore_drag_position_keeps_cursor_proportion_on_titlebar() {
        let next = restored_drag_outer_position(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(1920, 1040),
            PhysicalSize::new(1280, 800),
            PhysicalPosition::new(960.0, 14.0),
        );

        assert_eq!(next, PhysicalPosition::new(320, 0));
    }
}
