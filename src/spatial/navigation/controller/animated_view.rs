use std::time::Instant;

use crate::spatial::view::ViewState;
use winit::dpi::PhysicalSize;

use super::spring::Spring;

pub(super) struct AnimatedView {
    center_x: Spring,
    center_y: Spring,
    zoom: Spring,
}

impl AnimatedView {
    pub(super) fn new(view: ViewState, now: Instant) -> Self {
        Self {
            center_x: Spring::new(
                view.center.x,
                false,
                now,
                super::super::SPRING_STIFFNESS,
                super::super::SPRING_ANIMATION_TIME_SECS,
            ),
            center_y: Spring::new(
                view.center.y,
                false,
                now,
                super::super::SPRING_STIFFNESS,
                super::super::SPRING_ANIMATION_TIME_SECS,
            ),
            zoom: Spring::new(
                view.zoom,
                true,
                now,
                super::super::SPRING_STIFFNESS,
                super::super::SPRING_ANIMATION_TIME_SECS,
            ),
        }
    }

    pub(super) fn current_view(&self, surface_size: PhysicalSize<u32>) -> ViewState {
        self.build_view(
            surface_size,
            self.center_x.current_value(),
            self.center_y.current_value(),
            self.zoom.current_value(),
        )
    }

    pub(super) fn target_view(&self, surface_size: PhysicalSize<u32>) -> ViewState {
        self.build_view(
            surface_size,
            self.center_x.target_value(),
            self.center_y.target_value(),
            self.zoom.target_value(),
        )
    }

    pub(super) fn target_center(&self) -> (f64, f64) {
        (self.center_x.target_value(), self.center_y.target_value())
    }

    pub(super) fn current_zoom(&self) -> f64 {
        self.zoom.current_value()
    }

    pub(super) fn target_zoom(&self) -> f64 {
        self.zoom.target_value()
    }

    pub(super) fn zoom_is_at_target(&self) -> bool {
        self.zoom.is_at_target()
    }

    pub(super) fn is_animating(&self) -> bool {
        !self.center_x.is_at_target() || !self.center_y.is_at_target() || !self.zoom.is_at_target()
    }

    pub(super) fn reset_to(&mut self, view: ViewState, now: Instant) {
        self.center_x.reset_to(view.center.x, now);
        self.center_y.reset_to(view.center.y, now);
        self.zoom.reset_to(view.zoom, now);
    }

    pub(super) fn spring_center_to(&mut self, center_x: f64, center_y: f64, now: Instant) -> bool {
        let mut changed = false;
        if (center_x - self.center_x.target_value()).abs() > f64::EPSILON {
            self.center_x.spring_to(center_x, now);
            changed = true;
        }
        if (center_y - self.center_y.target_value()).abs() > f64::EPSILON {
            self.center_y.spring_to(center_y, now);
            changed = true;
        }
        changed
    }

    pub(super) fn spring_zoom_to(&mut self, zoom: f64, now: Instant) -> bool {
        if (zoom - self.zoom.target_value()).abs() < f64::EPSILON {
            return false;
        }

        self.zoom.spring_to(zoom, now);
        true
    }

    pub(super) fn shift_center_path(&mut self, dx: f64, dy: f64) {
        if dx.abs() > f64::EPSILON {
            self.center_x.shift_by(dx);
        }
        if dy.abs() > f64::EPSILON {
            self.center_y.shift_by(dy);
        }
    }

    pub(super) fn update_zoom(&mut self, now: Instant) -> bool {
        self.zoom.update(now)
    }

    pub(super) fn update_center(&mut self, now: Instant) -> bool {
        let x_animating = self.center_x.update(now);
        let y_animating = self.center_y.update(now);
        x_animating || y_animating
    }

    fn build_view(
        &self,
        surface_size: PhysicalSize<u32>,
        center_x: f64,
        center_y: f64,
        zoom: f64,
    ) -> ViewState {
        let mut view = ViewState::new(surface_size.width.max(1), surface_size.height.max(1));
        view.center.x = center_x;
        view.center.y = center_y;
        view.zoom = zoom;
        view
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::AnimatedView;
    use crate::spatial::view::ViewState;
    use winit::dpi::PhysicalSize;

    #[test]
    fn current_and_target_view_use_surface_aspect_and_spring_values() {
        let mut view = ViewState::new(1000, 1000);
        view.center.x = 1.0;
        view.center.y = -2.0;
        view.zoom = 1.5;

        let start = Instant::now();
        let mut animated = AnimatedView::new(view, start);
        assert!(animated.spring_center_to(3.0, 4.0, start));
        assert!(animated.spring_zoom_to(2.5, start));

        let current = animated.current_view(PhysicalSize::new(1600, 900));
        let target = animated.target_view(PhysicalSize::new(1600, 900));

        assert!((current.aspect - 1600.0 / 900.0).abs() < 1.0e-12);
        assert_eq!(current.center.x, 1.0);
        assert_eq!(current.center.y, -2.0);
        assert_eq!(current.zoom, 1.5);
        assert_eq!(target.center.x, 3.0);
        assert_eq!(target.center.y, 4.0);
        assert_eq!(target.zoom, 2.5);
    }

    #[test]
    fn animated_view_uses_navigation_spring_profile() {
        let view = ViewState::new(1000, 1000);
        let animated = AnimatedView::new(view, Instant::now());

        assert!(
            (animated.center_x.stiffness() - super::super::super::SPRING_STIFFNESS).abs() < 1.0e-12
        );
        assert!(
            (animated.center_y.stiffness() - super::super::super::SPRING_STIFFNESS).abs() < 1.0e-12
        );
        assert!(
            (animated.zoom.stiffness() - super::super::super::SPRING_STIFFNESS).abs() < 1.0e-12
        );
        assert_eq!(
            animated.center_x.animation_duration(),
            Duration::from_secs_f64(super::super::super::SPRING_ANIMATION_TIME_SECS)
        );
        assert_eq!(
            animated.center_y.animation_duration(),
            Duration::from_secs_f64(super::super::super::SPRING_ANIMATION_TIME_SECS)
        );
        assert_eq!(
            animated.zoom.animation_duration(),
            Duration::from_secs_f64(super::super::super::SPRING_ANIMATION_TIME_SECS)
        );
    }
}
