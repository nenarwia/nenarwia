use std::time::Instant;

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

use super::ViewportState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewportIntent {
    ResizeSurface(PhysicalSize<u32>),
    CursorMoved(PhysicalPosition<f64>),
    DragStart,
    DragEnd,
    ZoomAroundCursor {
        scroll: f64,
        cursor: PhysicalPosition<f64>,
    },
}

pub fn normalize_pixel_delta(delta_y: f64) -> f64 {
    delta_y / super::PIXELS_PER_WHEEL_LINE
}

pub fn apply_window_event_with_cursor(
    viewport: &mut ViewportState,
    event: &WindowEvent,
    cursor_override: Option<PhysicalPosition<f64>>,
    now: Instant,
) -> bool {
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            viewport.apply_intent(ViewportIntent::CursorMoved(*position), now)
        }
        WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } => viewport.apply_intent(ViewportIntent::DragStart, now),
        WindowEvent::MouseInput {
            state: ElementState::Released,
            button: MouseButton::Left,
            ..
        } => viewport.apply_intent(ViewportIntent::DragEnd, now),
        WindowEvent::MouseWheel { delta, .. } => {
            let scroll = match delta {
                MouseScrollDelta::LineDelta(_, y) => *y as f64,
                MouseScrollDelta::PixelDelta(pos) => normalize_pixel_delta(pos.y),
            };
            if scroll.abs() < f64::EPSILON {
                return false;
            }
            let cursor = cursor_override
                .or_else(|| viewport.last_cursor_position())
                .unwrap_or_else(|| {
                    let size = viewport.surface_size();
                    PhysicalPosition::new(size.width as f64 * 0.5, size.height as f64 * 0.5)
                });
            viewport.apply_intent(ViewportIntent::ZoomAroundCursor { scroll, cursor }, now)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::{apply_window_event_with_cursor, normalize_pixel_delta};
    use crate::spatial::navigation::{ViewRuntimeConfig, ViewportState};
    use crate::spatial::view::ViewState;
    use winit::dpi::{PhysicalPosition, PhysicalSize};
    use winit::event::{
        DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent,
    };

    #[test]
    fn normalize_pixel_delta_uses_line_constant() {
        assert!((normalize_pixel_delta(80.0) - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn wheel_uses_last_cursor_position() {
        let now = Instant::now();
        let mut viewport = ViewportState::new(
            PhysicalSize::new(1000, 1000),
            ViewState::new(1000, 1000),
            ViewRuntimeConfig::new(8, 12, 160.0, 8),
            now,
        );
        apply_window_event_with_cursor(
            &mut viewport,
            &WindowEvent::CursorMoved {
                device_id: unsafe { DeviceId::dummy() },
                position: PhysicalPosition::new(700.0, 200.0),
            },
            None,
            now,
        );

        let changed = apply_window_event_with_cursor(
            &mut viewport,
            &WindowEvent::MouseWheel {
                device_id: unsafe { DeviceId::dummy() },
                delta: MouseScrollDelta::LineDelta(0.0, 1.0),
                phase: TouchPhase::Moved,
            },
            None,
            now,
        );

        assert!(changed);
        assert!(viewport.target().zoom > viewport.current().zoom);
    }

    #[test]
    fn wheel_prefers_cursor_override_over_stale_viewport_cursor() {
        let now = Instant::now();
        let mut viewport = ViewportState::new(
            PhysicalSize::new(1000, 1000),
            ViewState::new(1000, 1000),
            ViewRuntimeConfig::new(8, 12, 160.0, 8),
            now,
        );
        apply_window_event_with_cursor(
            &mut viewport,
            &WindowEvent::CursorMoved {
                device_id: unsafe { DeviceId::dummy() },
                position: PhysicalPosition::new(100.0, 100.0),
            },
            None,
            now,
        );

        let wheel_cursor = PhysicalPosition::new(700.0, 200.0);
        let anchored_world_before = viewport.metrics().screen_to_world(wheel_cursor);
        let changed = apply_window_event_with_cursor(
            &mut viewport,
            &WindowEvent::MouseWheel {
                device_id: unsafe { DeviceId::dummy() },
                delta: MouseScrollDelta::LineDelta(0.0, 1.0),
                phase: TouchPhase::Moved,
            },
            Some(wheel_cursor),
            now,
        );

        assert!(changed);
        assert!(viewport.tick(now + std::time::Duration::from_millis(100)));
        let anchored_world_after = viewport.metrics().screen_to_world(wheel_cursor);
        assert!((anchored_world_after[0] - anchored_world_before[0]).abs() < 1.0e-9);
        assert!((anchored_world_after[1] - anchored_world_before[1]).abs() < 1.0e-9);
    }

    #[test]
    fn drag_start_and_end_flow_through_adapter() {
        let now = Instant::now();
        let mut viewport = ViewportState::new(
            PhysicalSize::new(1000, 1000),
            ViewState::new(1000, 1000),
            ViewRuntimeConfig::new(8, 12, 160.0, 8),
            now,
        );

        assert!(apply_window_event_with_cursor(
            &mut viewport,
            &WindowEvent::MouseInput {
                device_id: unsafe { DeviceId::dummy() },
                state: ElementState::Pressed,
                button: MouseButton::Left,
            },
            None,
            now,
        ));
        assert!(apply_window_event_with_cursor(
            &mut viewport,
            &WindowEvent::MouseInput {
                device_id: unsafe { DeviceId::dummy() },
                state: ElementState::Released,
                button: MouseButton::Left,
            },
            None,
            now,
        ));
    }
}
