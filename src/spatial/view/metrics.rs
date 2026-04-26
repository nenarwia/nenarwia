use crate::spatial::view::ViewState;
use winit::dpi::{PhysicalPosition, PhysicalSize};

#[derive(Clone, Copy, Debug)]
pub struct ViewMetrics {
    view: ViewState,
    surface_size: PhysicalSize<u32>,
}

impl ViewMetrics {
    pub fn new(view: ViewState, surface_size: PhysicalSize<u32>) -> Self {
        Self { view, surface_size }
    }

    pub fn surface_size(&self) -> PhysicalSize<u32> {
        self.surface_size
    }

    pub fn pixels_per_world(&self) -> Option<(f64, f64)> {
        let width = self.surface_size.width.max(1);
        let height = self.surface_size.height.max(1);
        let zoom = self.view.zoom.max(f64::MIN_POSITIVE);
        let viewport_world_w = 2.0 * self.view.aspect / zoom;
        let viewport_world_h = 2.0 / zoom;
        if viewport_world_w <= f64::EPSILON || viewport_world_h <= f64::EPSILON {
            return None;
        }
        Some((
            width as f64 / viewport_world_w,
            height as f64 / viewport_world_h,
        ))
    }

    pub fn pixel_scale(&self) -> Option<f64> {
        let (_, px_per_world_y) = self.pixels_per_world()?;
        Some(px_per_world_y)
    }

    pub fn screen_to_world(&self, pos: PhysicalPosition<f64>) -> [f64; 2] {
        let width = self.surface_size.width.max(1);
        let height = self.surface_size.height.max(1);
        let x_ndc = (pos.x / width as f64) * 2.0 - 1.0;
        let y_ndc = 1.0 - (pos.y / height as f64 * 2.0);
        [
            x_ndc * (self.view.aspect / self.view.zoom) + self.view.center.x,
            y_ndc * (1.0 / self.view.zoom) + self.view.center.y,
        ]
    }

    pub fn world_to_screen(&self, world: [f64; 2]) -> PhysicalPosition<f64> {
        let width = self.surface_size.width.max(1) as f64;
        let height = self.surface_size.height.max(1) as f64;
        let x_ndc = (world[0] - self.view.center.x) / (self.view.aspect / self.view.zoom);
        let y_ndc = (world[1] - self.view.center.y) / (1.0 / self.view.zoom);
        let x = ((x_ndc + 1.0) * 0.5) * width;
        let y = ((1.0 - y_ndc) * 0.5) * height;
        PhysicalPosition::new(x, y)
    }

    pub fn world_size_to_pixels(&self, world_w: f32, world_h: f32) -> (f32, f32) {
        let Some((px_per_world_x, px_per_world_y)) = self.pixels_per_world() else {
            return (0.0, 0.0);
        };
        (
            (world_w.abs() as f64 * px_per_world_x) as f32,
            (world_h.abs() as f64 * px_per_world_y) as f32,
        )
    }

    pub fn pixel_delta_to_world_delta(&self, dx: f64, dy: f64) -> Option<(f64, f64)> {
        let scale = self.pixel_scale()?;
        Some((-dx / scale, dy / scale))
    }

    pub fn world_delta_to_pixel_distance(&self, dx: f64, dy: f64) -> f32 {
        let (dx_px, dy_px) = self.world_size_to_pixels(dx.abs() as f32, dy.abs() as f32);
        dx_px.hypot(dy_px)
    }

    pub fn point_distance_to_center_l1_px(&self, world_x: f64, world_y: f64) -> f32 {
        let Some((px_per_world_x, px_per_world_y)) = self.pixels_per_world() else {
            return 0.0;
        };
        let dx = (world_x - self.view.center.x).abs() as f32;
        let dy = (world_y - self.view.center.y).abs() as f32;
        dx * px_per_world_x as f32 + dy * px_per_world_y as f32
    }
}

#[cfg(test)]
mod tests {
    use super::ViewMetrics;
    use crate::spatial::view::ViewState;
    use winit::dpi::{PhysicalPosition, PhysicalSize};

    #[test]
    fn screen_world_round_trip_is_stable() {
        let mut view = ViewState::new(1600, 900);
        view.center.x = 2.25;
        view.center.y = -0.75;
        view.zoom = 2.4;
        let metrics = ViewMetrics::new(view, PhysicalSize::new(1600, 900));
        let screen = PhysicalPosition::new(1234.5, 432.25);
        let world = metrics.screen_to_world(screen);
        let round_trip = metrics.world_to_screen(world);
        assert!((round_trip.x - screen.x).abs() < 1.0e-9);
        assert!((round_trip.y - screen.y).abs() < 1.0e-9);
    }

    #[test]
    fn pixels_per_world_matches_expected_scale() {
        let mut view = ViewState::new(1000, 1000);
        view.zoom = 2.0;
        let metrics = ViewMetrics::new(view, PhysicalSize::new(1000, 1000));
        let (px_x, px_y) = metrics.pixels_per_world().unwrap();
        assert!((px_x - 1000.0).abs() < 1.0e-12);
        assert!((px_y - 1000.0).abs() < 1.0e-12);
    }
}
