use cgmath::{Point2, Point3, Vector3};

use super::ViewMetrics;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewState {
    pub center: Point2<f64>,
    pub aspect: f64,
    pub zoom: f64,
}

impl ViewState {
    pub fn new(width: u32, height: u32) -> Self {
        let aspect = if width > 0 && height > 0 {
            width as f64 / height as f64
        } else {
            1.0
        };
        Self {
            center: Point2::new(0.0, 0.0),
            aspect,
            zoom: 1.0,
        }
    }

    pub fn resized(mut self, width: u32, height: u32) -> Self {
        self.resize(width, height);
        self
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.aspect = width as f64 / height as f64;
        }
    }

    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let eye = Point3::new(self.center.x, self.center.y, 1.0);
        let target = Point3::new(self.center.x, self.center.y, 0.0);
        let view = cgmath::Matrix4::look_at_rh(eye, target, Vector3::unit_y());
        let half_w = self.aspect / self.zoom;
        let half_h = 1.0 / self.zoom;
        let proj = cgmath::ortho(-half_w, half_w, -half_h, half_h, -100.0, 100.0);

        #[rustfmt::skip]
        let opengl_to_wgpu = cgmath::Matrix4::new(
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
        );

        let m: [[f64; 4]; 4] = (opengl_to_wgpu * proj * view).into();
        let m32 = [
            [
                m[0][0] as f32,
                m[0][1] as f32,
                m[0][2] as f32,
                m[0][3] as f32,
            ],
            [
                m[1][0] as f32,
                m[1][1] as f32,
                m[1][2] as f32,
                m[1][3] as f32,
            ],
            [
                m[2][0] as f32,
                m[2][1] as f32,
                m[2][2] as f32,
                m[2][3] as f32,
            ],
            [
                m[3][0] as f32,
                m[3][1] as f32,
                m[3][2] as f32,
                m[3][3] as f32,
            ],
        ];
        cgmath::Matrix4::from(m32)
    }

    pub fn viewport_rect(&self) -> (f64, f64, f64, f64) {
        let half_h = 1.0 / self.zoom;
        let half_w = half_h * self.aspect;
        (
            self.center.x - half_w,
            self.center.x + half_w,
            self.center.y - half_h,
            self.center.y + half_h,
        )
    }

    pub fn metrics(&self, surface_size: winit::dpi::PhysicalSize<u32>) -> ViewMetrics {
        ViewMetrics::new(*self, surface_size)
    }

    pub fn fit_bounds(bounds: (f32, f32, f32, f32), aspect: f64, padding_factor: f64) -> Self {
        let (min_x, min_y, max_x, max_y) = bounds;
        let width = ((max_x - min_x) as f64).max(1.0e-3) * padding_factor.max(1.0);
        let height = ((max_y - min_y) as f64).max(1.0e-3) * padding_factor.max(1.0);
        let center_x = (min_x as f64 + max_x as f64) * 0.5;
        let center_y = (min_y as f64 + max_y as f64) * 0.5;
        let safe_aspect = aspect.max(1.0e-6);
        let zoom_x = (2.0 * safe_aspect) / width;
        let zoom_y = 2.0 / height;
        let mut zoom = zoom_x.min(zoom_y);
        if !zoom.is_finite() || zoom <= 0.0 {
            zoom = 1.0;
        }

        Self {
            center: Point2::new(center_x, center_y),
            aspect: safe_aspect,
            zoom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ViewState;

    #[test]
    fn viewport_rect_matches_zoom_and_aspect() {
        let mut view = ViewState::new(1600, 900);
        view.center.x = 2.0;
        view.center.y = -3.0;
        view.zoom = 4.0;

        let rect = view.viewport_rect();
        assert!((rect.0 - 1.555_555_555_555_555_6).abs() < 1.0e-12);
        assert!((rect.1 - 2.444_444_444_444_444_6).abs() < 1.0e-12);
        assert!((rect.2 + 3.25).abs() < 1.0e-12);
        assert!((rect.3 + 2.75).abs() < 1.0e-12);
    }

    #[test]
    fn fit_bounds_respects_wide_scene() {
        let view = ViewState::fit_bounds((0.0, 0.0, 10.0, 2.0), 16.0 / 9.0, 1.1);
        assert!((view.center.x - 5.0).abs() < 1.0e-12);
        assert!((view.center.y - 1.0).abs() < 1.0e-12);
        assert!((view.zoom - 0.323_232_323_232_323_26).abs() < 1.0e-12);
    }

    #[test]
    fn fit_bounds_respects_tall_scene() {
        let view = ViewState::fit_bounds((0.0, 0.0, 2.0, 12.0), 16.0 / 9.0, 1.1);
        assert!((view.center.x - 1.0).abs() < 1.0e-12);
        assert!((view.center.y - 6.0).abs() < 1.0e-12);
        assert!((view.zoom - 0.151_515_151_515_151_52).abs() < 1.0e-12);
    }
}
