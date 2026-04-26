use winit::dpi::{PhysicalPosition, PhysicalSize};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ContentBounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl ContentBounds {
    pub(super) fn new(bounds: (f32, f32, f32, f32)) -> Self {
        let (x0, y0, x1, y1) = bounds;
        Self {
            min_x: (x0 as f64).min(x1 as f64),
            min_y: (y0 as f64).min(y1 as f64),
            max_x: (x0 as f64).max(x1 as f64),
            max_y: (y0 as f64).max(y1 as f64),
        }
    }

    fn width(&self) -> f64 {
        (self.max_x - self.min_x).max(1.0e-3)
    }

    fn height(&self) -> f64 {
        (self.max_y - self.min_y).max(1.0e-3)
    }
}

pub(super) fn minimum_zoom(bounds: Option<ContentBounds>, surface_size: PhysicalSize<u32>) -> f64 {
    let Some(bounds) = bounds else {
        return super::super::ZOOM_MIN;
    };

    let fit_zoom = ((2.0 * surface_aspect(surface_size)) / bounds.width())
        .min(2.0 / bounds.height())
        .max(super::super::ZOOM_MIN);
    (fit_zoom * super::super::MIN_ZOOM_IMAGE_RATIO)
        .clamp(super::super::ZOOM_MIN, super::super::ZOOM_MAX)
}

pub(super) fn constrain_center(
    bounds: Option<ContentBounds>,
    surface_size: PhysicalSize<u32>,
    center_x: f64,
    center_y: f64,
    zoom: f64,
) -> (f64, f64) {
    let Some(bounds) = bounds else {
        return (center_x, center_y);
    };

    let safe_zoom = zoom.max(super::super::ZOOM_MIN);
    let view_width = 2.0 * surface_aspect(surface_size) / safe_zoom;
    let view_height = 2.0 / safe_zoom;

    (
        constrain_axis(center_x, view_width, bounds.min_x, bounds.max_x),
        constrain_axis(center_y, view_height, bounds.min_y, bounds.max_y),
    )
}

pub(super) fn anchored_center_for_zoom(
    world: [f64; 2],
    screen: PhysicalPosition<f64>,
    surface_size: PhysicalSize<u32>,
    zoom: f64,
) -> (f64, f64) {
    let width = surface_size.width.max(1) as f64;
    let height = surface_size.height.max(1) as f64;
    let x_ndc = (screen.x / width) * 2.0 - 1.0;
    let y_ndc = 1.0 - (screen.y / height * 2.0);
    let safe_zoom = zoom.max(super::super::ZOOM_MIN);

    (
        world[0] - x_ndc * (surface_aspect(surface_size) / safe_zoom),
        world[1] - y_ndc * (1.0 / safe_zoom),
    )
}

pub(super) fn constrain_anchored_center(
    bounds: Option<ContentBounds>,
    surface_size: PhysicalSize<u32>,
    world: [f64; 2],
    screen: PhysicalPosition<f64>,
    zoom: f64,
) -> ((f64, f64), (f64, f64)) {
    let anchored = anchored_center_for_zoom(world, screen, surface_size, zoom);
    let constrained = constrain_center(bounds, surface_size, anchored.0, anchored.1, zoom);
    (anchored, constrained)
}

fn surface_aspect(surface_size: PhysicalSize<u32>) -> f64 {
    if surface_size.width > 0 && surface_size.height > 0 {
        surface_size.width as f64 / surface_size.height as f64
    } else {
        1.0
    }
}

fn constrain_axis(center: f64, view_size: f64, content_min: f64, content_max: f64) -> f64 {
    let content_size = (content_max - content_min).max(0.0);
    let half = view_size * 0.5;
    let left = center - half;
    let right = center + half;
    let threshold = if view_size > content_size {
        super::super::VISIBILITY_RATIO * content_size
    } else {
        super::super::VISIBILITY_RATIO * view_size
    };

    let left_dx = content_min - right + threshold;
    let right_dx = content_max - left - threshold;

    if threshold > content_size {
        center + (left_dx + right_dx) * 0.5
    } else if right_dx < 0.0 {
        center + right_dx
    } else if left_dx > 0.0 {
        center + left_dx
    } else {
        center
    }
}

#[cfg(test)]
mod tests {
    use super::{
        anchored_center_for_zoom, constrain_anchored_center, constrain_center, minimum_zoom,
        ContentBounds,
    };
    use crate::spatial::navigation::ZOOM_MIN;
    use winit::dpi::{PhysicalPosition, PhysicalSize};

    #[test]
    fn minimum_zoom_scales_to_fit_small_content() {
        let bounds = Some(ContentBounds::new((-0.25, -0.25, 0.25, 0.25)));
        let min_zoom = minimum_zoom(bounds, PhysicalSize::new(1000, 1000));

        assert!((min_zoom - 3.6).abs() < 1.0e-12);
    }

    #[test]
    fn minimum_zoom_falls_back_to_global_min_without_bounds() {
        let min_zoom = minimum_zoom(None, PhysicalSize::new(1000, 1000));
        assert_eq!(min_zoom, ZOOM_MIN);
    }

    #[test]
    fn constrain_center_preserves_offset_until_visibility_threshold_is_crossed() {
        let bounds = Some(ContentBounds::new((-0.25, -0.1, 0.25, 0.1)));

        let free = constrain_center(bounds, PhysicalSize::new(1000, 1000), 0.5, -0.4, 1.0);
        let clamped = constrain_center(bounds, PhysicalSize::new(1000, 1000), 2.0, -2.0, 1.0);

        assert!((free.0 - 0.5).abs() < 1.0e-12);
        assert!((free.1 + 0.4).abs() < 1.0e-12);
        assert!((clamped.0 - 1.0).abs() < 1.0e-12);
        assert!((clamped.1 + 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn anchored_center_round_trips_cursor_position_for_target_zoom() {
        let world = [2.0, -1.0];
        let screen = PhysicalPosition::new(750.0, 125.0);
        let anchored = anchored_center_for_zoom(world, screen, PhysicalSize::new(1000, 1000), 2.0);

        assert!((anchored.0 - 1.75).abs() < 1.0e-12);
        assert!((anchored.1 + 1.375).abs() < 1.0e-12);
    }

    #[test]
    fn constrain_anchored_center_applies_bounds_after_anchor_projection() {
        let bounds = Some(ContentBounds::new((-0.25, -0.25, 0.25, 0.25)));
        let world = [0.2, 0.2];
        let screen = PhysicalPosition::new(900.0, 100.0);

        let (anchored, constrained) =
            constrain_anchored_center(bounds, PhysicalSize::new(1000, 1000), world, screen, 2.0);

        assert!(constrained.0 <= anchored.0);
        assert!(constrained.1 >= anchored.1);
    }
}
