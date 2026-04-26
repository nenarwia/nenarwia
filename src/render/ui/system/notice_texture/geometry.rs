use winit::dpi::PhysicalSize;

use super::super::{UI_MARGIN_PX, UI_MAX_WIDTH_PX};

pub(super) fn point_in_rect(x: f32, y: f32, rect: [f32; 4]) -> bool {
    x >= rect[0] && y >= rect[1] && x <= rect[0] + rect[2] && y <= rect[1] + rect[3]
}

pub(super) fn layout_max_width(surface_size: PhysicalSize<u32>) -> u32 {
    let available = surface_size.width.saturating_sub(UI_MARGIN_PX * 2).max(120);
    UI_MAX_WIDTH_PX.min(available)
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalSize;

    use super::super::super::{UI_MARGIN_PX, UI_MAX_WIDTH_PX};
    use super::{layout_max_width, point_in_rect};

    #[test]
    fn point_in_rect_includes_edges() {
        let rect = [10.0, 20.0, 30.0, 40.0];

        assert!(point_in_rect(10.0, 20.0, rect));
        assert!(point_in_rect(40.0, 60.0, rect));
        assert!(!point_in_rect(9.9, 20.0, rect));
        assert!(!point_in_rect(40.1, 60.0, rect));
    }

    #[test]
    fn layout_max_width_honors_margin_floor_and_global_cap() {
        assert_eq!(layout_max_width(PhysicalSize::new(50, 100)), 120);
        assert_eq!(
            layout_max_width(PhysicalSize::new(
                UI_MAX_WIDTH_PX + UI_MARGIN_PX * 2 + 100,
                100
            )),
            UI_MAX_WIDTH_PX
        );
    }
}
