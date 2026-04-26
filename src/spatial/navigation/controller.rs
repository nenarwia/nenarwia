use std::time::Instant;

use crate::spatial::view::{ViewMetrics, ViewState};
use winit::dpi::{PhysicalPosition, PhysicalSize};

use super::ViewportIntent;

mod animated_view;
mod constraints;
mod interaction;
mod spring;

#[cfg(test)]
mod tests;

use animated_view::AnimatedView;
use constraints::{constrain_anchored_center, constrain_center, minimum_zoom, ContentBounds};
use interaction::{InteractionState, ZoomPoint};

pub struct NavigationController {
    surface_size: PhysicalSize<u32>,
    content_bounds: Option<ContentBounds>,
    view: AnimatedView,
    interaction: InteractionState,
}

impl NavigationController {
    pub fn new(view: ViewState, surface_size: PhysicalSize<u32>, now: Instant) -> Self {
        Self {
            surface_size,
            content_bounds: None,
            view: AnimatedView::new(view, now),
            interaction: InteractionState::default(),
        }
    }

    pub fn surface_size(&self) -> PhysicalSize<u32> {
        self.surface_size
    }

    pub fn current_view(&self) -> ViewState {
        self.view.current_view(self.surface_size)
    }

    pub fn target_view(&self) -> ViewState {
        self.view.target_view(self.surface_size)
    }

    pub fn last_cursor_position(&self) -> Option<PhysicalPosition<f64>> {
        self.interaction.last_cursor_position()
    }

    pub fn set_content_bounds(&mut self, bounds: Option<(f32, f32, f32, f32)>) {
        self.content_bounds = bounds.map(ContentBounds::new);
    }

    pub fn resize_surface(&mut self, surface_size: PhysicalSize<u32>) {
        if surface_size.width > 0 && surface_size.height > 0 {
            self.surface_size = surface_size;
        }
    }

    pub fn load_view(&mut self, view: ViewState, now: Instant) {
        self.view.reset_to(self.resized_view(view), now);
        self.interaction.reset();
    }

    pub fn jump_to(&mut self, view: ViewState, now: Instant) -> bool {
        let next = self.resized_view(view);
        if self.current_view() == next && self.target_view() == next {
            return false;
        }

        self.load_view(next, now);
        true
    }

    pub fn apply_intent(&mut self, intent: ViewportIntent, now: Instant) -> bool {
        match intent {
            ViewportIntent::ResizeSurface(surface_size) => self.handle_resize(surface_size),
            ViewportIntent::CursorMoved(position) => self.handle_cursor_moved(position, now),
            ViewportIntent::DragStart => self.handle_drag_start(),
            ViewportIntent::DragEnd => self.handle_drag_end(now),
            ViewportIntent::ZoomAroundCursor { scroll, cursor } => {
                self.handle_zoom_scroll(scroll, cursor, now)
            }
        }
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        if self.surface_size.width == 0 || self.surface_size.height == 0 {
            return false;
        }

        self.advance_to(now);
        if !self.interaction.is_drag_active() && self.interaction.zoom_point().is_none() {
            self.spring_center_to_constrained_target(now);
        }

        let animating = self.view.is_animating();
        if !animating {
            self.interaction.clear_zoom_point();
        }
        animating
    }

    pub fn is_animating(&self) -> bool {
        self.view.is_animating()
    }

    fn handle_resize(&mut self, surface_size: PhysicalSize<u32>) -> bool {
        let changed = self.surface_size != surface_size;
        self.resize_surface(surface_size);
        changed
    }

    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>, now: Instant) -> bool {
        let mut changed = false;
        if self.interaction.is_drag_active() {
            if let Some(last) = self.interaction.last_cursor_position() {
                changed = self.pan_by_pixels(position.x - last.x, position.y - last.y, now);
            }
        }

        self.interaction.record_cursor(position);
        changed
    }

    fn handle_drag_start(&mut self) -> bool {
        self.interaction.begin_drag()
    }

    fn handle_drag_end(&mut self, now: Instant) -> bool {
        let mut changed = self.interaction.end_drag();
        changed |= self.spring_center_to_constrained_target(now);
        changed
    }

    fn handle_zoom_scroll(
        &mut self,
        scroll: f64,
        cursor: PhysicalPosition<f64>,
        now: Instant,
    ) -> bool {
        self.zoom_by_scroll(scroll, cursor, now)
    }

    fn resized_view(&self, view: ViewState) -> ViewState {
        view.resized(
            self.surface_size.width.max(1),
            self.surface_size.height.max(1),
        )
    }

    fn current_metrics(&self) -> ViewMetrics {
        self.current_view().metrics(self.surface_size)
    }

    fn minimum_zoom(&self) -> f64 {
        minimum_zoom(self.content_bounds, self.surface_size)
    }

    fn advance_to(&mut self, now: Instant) {
        let zoom_point = self.interaction.zoom_point();
        let prior_zoom_point_screen =
            zoom_point.map(|point| self.current_metrics().world_to_screen(point.world));

        self.view.update_zoom(now);

        if let (Some(point), Some(screen)) = (zoom_point, prior_zoom_point_screen) {
            // Match OSD's behavior by compensating only for the screen-space drift
            // introduced by the zoom step. Pan can then move the point freely.
            self.realign_center_to_zoom_point(point, screen);
            if !self.interaction.is_drag_active() {
                self.apply_zoom_point_bounds(point, screen);
            }
        }

        self.view.update_center(now);

        if self.view.zoom_is_at_target() {
            self.interaction.clear_zoom_point();
        }
    }

    fn realign_center_to_zoom_point(
        &mut self,
        point: ZoomPoint,
        prior_screen: PhysicalPosition<f64>,
    ) {
        let metrics = self.current_metrics();
        let new_screen = metrics.world_to_screen(point.world);

        if let Some((world_dx, world_dy)) = metrics.pixel_delta_to_world_delta(
            prior_screen.x - new_screen.x,
            prior_screen.y - new_screen.y,
        ) {
            self.view.shift_center_path(world_dx, world_dy);
        }
    }

    fn spring_center_to_constrained_target(&mut self, now: Instant) -> bool {
        let (target_x, target_y) = self.view.target_center();
        let (constrained_x, constrained_y) = constrain_center(
            self.content_bounds,
            self.surface_size,
            target_x,
            target_y,
            self.view.target_zoom(),
        );

        self.view
            .spring_center_to(constrained_x, constrained_y, now)
    }

    fn apply_zoom_point_bounds(&mut self, point: ZoomPoint, screen: PhysicalPosition<f64>) -> bool {
        let ((anchored_x, anchored_y), (constrained_x, constrained_y)) = constrain_anchored_center(
            self.content_bounds,
            self.surface_size,
            point.world,
            screen,
            self.view.current_zoom(),
        );

        let dx = constrained_x - anchored_x;
        let dy = constrained_y - anchored_y;
        if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
            return false;
        }

        self.view.shift_center_path(dx, dy);
        true
    }

    fn pan_by_pixels(&mut self, dx: f64, dy: f64, now: Instant) -> bool {
        if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
            return false;
        }

        self.advance_to(now);
        let Some((world_dx, world_dy)) = self.current_metrics().pixel_delta_to_world_delta(dx, dy)
        else {
            return false;
        };

        let (target_x, target_y) = self.view.target_center();
        self.view
            .spring_center_to(target_x + world_dx, target_y + world_dy, now);
        true
    }

    fn zoom_by_scroll(&mut self, scroll: f64, cursor: PhysicalPosition<f64>, now: Instant) -> bool {
        if scroll.abs() < f64::EPSILON {
            return false;
        }

        self.advance_to(now);
        let world = self.current_metrics().screen_to_world(cursor);
        self.interaction.capture_zoom_point(world);

        let next_zoom = (self.view.target_zoom() * super::ZOOM_PER_SCROLL.powf(scroll))
            .clamp(self.minimum_zoom(), super::ZOOM_MAX);
        self.view.spring_zoom_to(next_zoom, now)
    }
}
