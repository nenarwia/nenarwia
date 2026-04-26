use std::time::Instant;

use crate::spatial::view::{ViewMetrics, ViewState};
use winit::dpi::PhysicalSize;

use super::{NavigationController, ViewRuntime, ViewRuntimeConfig, ViewportIntent};

pub struct ViewportState {
    navigation: NavigationController,
    runtime: ViewRuntime,
}

impl ViewportState {
    pub fn new(
        surface_size: PhysicalSize<u32>,
        view: ViewState,
        runtime_config: ViewRuntimeConfig,
        now: Instant,
    ) -> Self {
        let view = view.resized(surface_size.width.max(1), surface_size.height.max(1));
        Self {
            navigation: NavigationController::new(view, surface_size, now),
            runtime: ViewRuntime::new(view, runtime_config),
        }
    }

    pub fn current(&self) -> ViewState {
        self.navigation.current_view()
    }

    #[cfg(test)]
    pub fn target(&self) -> ViewState {
        self.navigation.target_view()
    }

    pub fn metrics(&self) -> ViewMetrics {
        self.current().metrics(self.surface_size())
    }

    pub fn surface_size(&self) -> PhysicalSize<u32> {
        self.navigation.surface_size()
    }

    pub fn runtime(&self) -> &ViewRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut ViewRuntime {
        &mut self.runtime
    }

    pub fn last_cursor_position(&self) -> Option<winit::dpi::PhysicalPosition<f64>> {
        self.navigation.last_cursor_position()
    }

    pub fn set_content_bounds(&mut self, bounds: Option<(f32, f32, f32, f32)>) {
        self.navigation.set_content_bounds(bounds);
    }

    pub fn apply_intent(&mut self, intent: ViewportIntent, now: Instant) -> bool {
        self.navigation.apply_intent(intent, now)
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        self.navigation.tick(now)
    }

    pub fn is_animating(&self) -> bool {
        self.navigation.is_animating()
    }

    pub fn load_view(&mut self, view: ViewState, now: Instant) {
        self.navigation.load_view(view, now);
    }

    pub fn jump_to(&mut self, view: ViewState, now: Instant) -> bool {
        self.navigation.jump_to(view, now)
    }

    pub fn fit_bounds(
        &mut self,
        bounds: (f32, f32, f32, f32),
        padding_factor: f64,
        now: Instant,
    ) -> bool {
        let view = ViewState::fit_bounds(bounds, self.current().aspect, padding_factor);
        self.jump_to(view, now)
    }
}
