use std::time::{Duration, Instant};

use super::{spring::spring_transform, NavigationController};
use crate::spatial::navigation::ViewportIntent;
use crate::spatial::view::ViewState;
use winit::dpi::{PhysicalPosition, PhysicalSize};

mod zoom_anchoring {
    use super::*;

    #[test]
    fn zoom_anchor_keeps_world_point_under_cursor_during_animation() {
        let mut view = ViewState::new(1000, 1000);
        view.center.x = 0.35;
        view.center.y = -0.18;
        view.zoom = 1.25;

        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);

        let cursor = PhysicalPosition::new(640.0, 410.0);
        let anchor_world = view
            .metrics(PhysicalSize::new(1000, 1000))
            .screen_to_world(cursor);

        controller.interaction.capture_zoom_point(anchor_world);
        assert!(controller.view.spring_zoom_to(2.0, start));
        controller.advance_to(start + Duration::from_millis(100));

        let anchored_world = controller
            .current_view()
            .metrics(PhysicalSize::new(1000, 1000))
            .screen_to_world(cursor);
        assert!((anchored_world[0] - anchor_world[0]).abs() < 1.0e-9);
        assert!((anchored_world[1] - anchor_world[1]).abs() < 1.0e-9);
    }

    #[test]
    fn pan_during_zoom_keeps_zoom_point_at_panned_screen_position() {
        let mut view = ViewState::new(1000, 1000);
        view.center.x = 0.35;
        view.center.y = -0.18;
        view.zoom = 1.25;

        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);

        let cursor = PhysicalPosition::new(640.0, 410.0);
        let zoom_point_world = view
            .metrics(PhysicalSize::new(1000, 1000))
            .screen_to_world(cursor);

        controller.interaction.capture_zoom_point(zoom_point_world);
        assert!(controller.view.spring_zoom_to(2.0, start));

        let pan_time = start + Duration::from_millis(100);
        controller.advance_to(pan_time);
        let snap_time = pan_time + Duration::from_millis(1);
        assert!(controller.pan_by_pixels(120.0, -45.0, snap_time));
        assert!(controller.interaction.zoom_point().is_some());

        let target_after_pan = controller.target_view();
        let mut snapped_view = controller.current_view();
        snapped_view.center = target_after_pan.center;
        controller.view.reset_to(snapped_view, snap_time);
        assert!(controller
            .view
            .spring_zoom_to(target_after_pan.zoom, snap_time));

        let panned_screen = controller
            .current_view()
            .metrics(PhysicalSize::new(1000, 1000))
            .world_to_screen(zoom_point_world);
        assert!((panned_screen.x - cursor.x).abs() > 1.0);
        assert!((panned_screen.y - cursor.y).abs() > 1.0);

        controller.advance_to(snap_time + Duration::from_millis(150));

        let later_screen = controller
            .current_view()
            .metrics(PhysicalSize::new(1000, 1000))
            .world_to_screen(zoom_point_world);
        assert!((later_screen.x - panned_screen.x).abs() < 1.0e-6);
        assert!((later_screen.y - panned_screen.y).abs() < 1.0e-6);
    }

    #[test]
    fn repeated_wheel_uses_target_zoom_not_current_zoom() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);

        assert!(controller.zoom_by_scroll(1.0, PhysicalPosition::new(500.0, 500.0), start));
        let first_target = controller.target_view().zoom;

        assert!(controller.zoom_by_scroll(
            1.0,
            PhysicalPosition::new(500.0, 500.0),
            start + Duration::from_millis(10),
        ));

        assert!(
            (controller.target_view().zoom - first_target * super::super::super::ZOOM_PER_SCROLL)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn zoom_constraints_keep_target_inside_content_bounds() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        controller.set_content_bounds(Some((-0.25, -0.25, 0.25, 0.25)));

        assert!(controller.zoom_by_scroll(1.0, PhysicalPosition::new(900.0, 100.0), start));
        super::simulate_for(&mut controller, start, 1.0 / 60.0, 90);

        let current = controller.current_view();
        let target = controller.target_view();
        let constrained_current = super::super::constraints::constrain_center(
            controller.content_bounds,
            controller.surface_size,
            current.center.x,
            current.center.y,
            current.zoom,
        );
        let constrained_target = super::super::constraints::constrain_center(
            controller.content_bounds,
            controller.surface_size,
            target.center.x,
            target.center.y,
            target.zoom,
        );
        assert!((constrained_current.0 - current.center.x).abs() < 1.0e-9);
        assert!((constrained_current.1 - current.center.y).abs() < 1.0e-9);
        assert!((constrained_target.0 - target.center.x).abs() < 1.0e-9);
        assert!((constrained_target.1 - target.center.y).abs() < 1.0e-9);
        assert!(target.zoom > view.zoom);
    }

    #[test]
    fn zoom_out_clamps_to_dynamic_minimum_zoom() {
        let mut view = ViewState::new(1000, 1000);
        view.zoom = 4.0;

        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        controller.set_content_bounds(Some((-0.25, -0.25, 0.25, 0.25)));

        assert!(controller.zoom_by_scroll(-10.0, PhysicalPosition::new(900.0, 100.0), start));

        let expected_min_zoom = 3.6;
        assert!((controller.target_view().zoom - expected_min_zoom).abs() < 1.0e-12);
    }
}

mod spring_behavior {
    use super::*;

    #[test]
    fn spring_progress_is_consistent_across_refresh_rates() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller_60 =
            NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        let mut controller_144 =
            NavigationController::new(view, PhysicalSize::new(1000, 1000), start);

        assert!(controller_60.view.spring_zoom_to(3.0, start));
        assert!(controller_144.view.spring_zoom_to(3.0, start));

        super::simulate_for(&mut controller_60, start, 1.0 / 60.0, 15);
        super::simulate_for(&mut controller_144, start, 1.0 / 144.0, 36);

        assert!(
            (controller_60.current_view().zoom - controller_144.current_view().zoom).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn exponential_spring_matches_osd_curve() {
        let start = Instant::now();
        let mut controller = NavigationController::new(
            ViewState::new(1000, 1000),
            PhysicalSize::new(1000, 1000),
            start,
        );
        assert!(controller.view.spring_zoom_to(4.0, start));

        let sample_time =
            start + Duration::from_secs_f64(super::super::super::SPRING_ANIMATION_TIME_SECS) / 2;
        controller.view.update_zoom(sample_time);

        let expected = (1.0f64.ln()
            + (4.0f64.ln() - 1.0f64.ln())
                * spring_transform(super::super::super::SPRING_STIFFNESS, 0.5))
        .exp();
        assert!((controller.view.current_zoom() - expected).abs() < 1.0e-12);
    }
}

mod content_constraints {
    use super::*;

    #[test]
    fn constraints_allow_offset_when_content_is_smaller_than_viewport() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        controller.set_content_bounds(Some((-0.25, -0.1, 0.25, 0.1)));

        let (free_x, free_y) = super::super::constraints::constrain_center(
            controller.content_bounds,
            controller.surface_size,
            0.5,
            -0.4,
            1.0,
        );
        let (clamped_x, clamped_y) = super::super::constraints::constrain_center(
            controller.content_bounds,
            controller.surface_size,
            2.0,
            -2.0,
            1.0,
        );
        assert!((free_x - 0.5).abs() < 1.0e-12);
        assert!((free_y + 0.4).abs() < 1.0e-12);
        assert!((clamped_x - 1.0).abs() < 1.0e-12);
        assert!((clamped_y + 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn constraints_respect_visibility_ratio_on_large_content() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        controller.set_content_bounds(Some((-4.0, -2.0, 4.0, 2.0)));

        let (center_x, center_y) = super::super::constraints::constrain_center(
            controller.content_bounds,
            controller.surface_size,
            -4.75,
            2.25,
            1.0,
        );
        assert!((center_x + 4.0).abs() < 1.0e-12);
        assert!((center_y - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn pan_by_pixels_animates_toward_world_delta() {
        let mut view = ViewState::new(1000, 1000);
        view.zoom = 2.0;

        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        assert!(controller.pan_by_pixels(50.0, 20.0, start));
        assert!(controller.is_animating());

        let scale = 1000.0 * view.zoom / 2.0;
        let target_world_x = -50.0 / scale;
        let target_world_y = 20.0 / scale;

        controller.advance_to(
            start + Duration::from_secs_f64(super::super::super::SPRING_ANIMATION_TIME_SECS) / 2,
        );
        let current = controller.current_view();
        assert!(current.center.x < 0.0);
        assert!(current.center.y > 0.0);
        assert!(current.center.x > target_world_x);
        assert!(current.center.y < target_world_y);
    }
}

mod drag_lifecycle {
    use super::*;

    #[test]
    fn drag_release_allows_overscroll_then_springs_back_into_bounds() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        controller.set_content_bounds(Some((-1.0, -1.0, 1.0, 1.0)));

        assert!(controller.apply_intent(ViewportIntent::DragStart, start));
        assert!(!controller.apply_intent(
            ViewportIntent::CursorMoved(PhysicalPosition::new(500.0, 500.0)),
            start,
        ));
        assert!(controller.apply_intent(
            ViewportIntent::CursorMoved(PhysicalPosition::new(1000.0, 500.0)),
            start + Duration::from_millis(16),
        ));
        assert!(controller.target_view().center.x < 0.0);

        assert!(
            controller.apply_intent(ViewportIntent::DragEnd, start + Duration::from_millis(17),)
        );
        let target = controller.target_view();
        let constrained = super::super::constraints::constrain_center(
            controller.content_bounds,
            controller.surface_size,
            target.center.x,
            target.center.y,
            target.zoom,
        );
        assert!((target.center.x + 1.0).abs() < 1.0e-12);
        assert!((target.center.y - 0.0).abs() < 1.0e-12);
        assert!((constrained.0 - target.center.x).abs() < 1.0e-12);
        assert!((constrained.1 - target.center.y).abs() < 1.0e-12);
    }

    #[test]
    fn drag_during_zoom_allows_overscroll_then_reconstrains_on_release() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        controller.set_content_bounds(Some((-1.0, -1.0, 1.0, 1.0)));

        assert!(!controller.apply_intent(
            ViewportIntent::CursorMoved(PhysicalPosition::new(500.0, 500.0)),
            start,
        ));
        assert!(controller.zoom_by_scroll(1.0, PhysicalPosition::new(900.0, 100.0), start));
        assert!(controller.apply_intent(ViewportIntent::DragStart, start));
        assert!(controller.apply_intent(
            ViewportIntent::CursorMoved(PhysicalPosition::new(1400.0, 500.0)),
            start + Duration::from_millis(16),
        ));

        let target_during_drag = controller.target_view();
        let constrained_during_drag = super::super::constraints::constrain_center(
            controller.content_bounds,
            controller.surface_size,
            target_during_drag.center.x,
            target_during_drag.center.y,
            target_during_drag.zoom,
        );
        assert!(
            (constrained_during_drag.0 - target_during_drag.center.x).abs() > 1.0e-9
                || (constrained_during_drag.1 - target_during_drag.center.y).abs() > 1.0e-9
        );
        assert!(controller.interaction.zoom_point().is_some());

        assert!(
            controller.apply_intent(ViewportIntent::DragEnd, start + Duration::from_millis(17),)
        );
        let target_after_release = controller.target_view();
        let constrained_after_release = super::super::constraints::constrain_center(
            controller.content_bounds,
            controller.surface_size,
            target_after_release.center.x,
            target_after_release.center.y,
            target_after_release.zoom,
        );
        assert!((constrained_after_release.0 - target_after_release.center.x).abs() < 1.0e-12);
        assert!((constrained_after_release.1 - target_after_release.center.y).abs() < 1.0e-12);
    }
}

mod public_lifecycle {
    use super::*;

    #[test]
    fn load_view_resets_transient_state() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);
        controller.apply_intent(ViewportIntent::DragStart, start);
        controller.apply_intent(
            ViewportIntent::CursorMoved(PhysicalPosition::new(10.0, 20.0)),
            start,
        );
        assert!(controller.zoom_by_scroll(1.0, PhysicalPosition::new(120.0, 80.0), start));

        let mut next = view;
        next.center.x = 1.2;
        next.center.y = -0.4;
        next.zoom = 2.5;
        controller.load_view(next, start + Duration::from_millis(16));

        assert_eq!(controller.current_view(), next);
        assert_eq!(controller.target_view(), next);
        assert!(controller.last_cursor_position().is_none());
        assert!(!controller.interaction.is_drag_active());
        assert!(controller.interaction.zoom_point().is_none());
    }

    #[test]
    fn jump_to_returns_false_when_view_is_already_current_and_target() {
        let view = ViewState::new(1000, 1000);
        let start = Instant::now();
        let mut controller = NavigationController::new(view, PhysicalSize::new(1000, 1000), start);

        assert!(!controller.jump_to(view, start + Duration::from_millis(16)));
    }
}

fn simulate_for(controller: &mut NavigationController, start: Instant, dt_secs: f64, steps: usize) {
    for step in 1..=steps {
        let now = start + Duration::from_secs_f64(dt_secs * step as f64);
        controller.advance_to(now);
    }
}
