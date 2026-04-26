use super::constants::{
    CHROME_SCRIM_COLOR, WINDOW_CLOSE_COLOR, WINDOW_MAXIMIZE_COLOR, WINDOW_MINIMIZE_COLOR,
};
use super::geometry::circle_rect;
use super::primitives::{draw_filled_circle_aa, paint_vertical_gradient};

pub(super) struct WindowControlsLayout {
    pub(super) close_rect: [u32; 4],
    pub(super) minimize_rect: [u32; 4],
    pub(super) maximize_rect: [u32; 4],
    pub(super) controls_cluster_end: u32,
    pub(super) drag_end: u32,
    close_center_x: i32,
    minimize_center_x: i32,
    maximize_center_x: i32,
    center_y: i32,
    radius: i32,
}

pub(super) fn paint_chrome_base(
    pixels: &mut [u8],
    width: u32,
    height: u32,
) -> WindowControlsLayout {
    paint_vertical_gradient(
        pixels,
        width,
        height,
        CHROME_SCRIM_COLOR,
        CHROME_SCRIM_COLOR,
    );

    let layout = WindowControlsLayout::new(width, height);
    layout.paint_buttons(pixels, width, height);
    layout
}

impl WindowControlsLayout {
    fn new(width: u32, height: u32) -> Self {
        let radius = super::super::super::CHROME_BTN_RADIUS_PX as i32;
        let center_y = (height / 2) as i32;
        let step = (super::super::super::CHROME_BTN_RADIUS_PX * 2
            + super::super::super::CHROME_BTN_GAP_PX) as i32;
        let side = super::super::super::CHROME_SIDE_PADDING_PX as i32;

        let (close_center_x, minimize_center_x, maximize_center_x, controls_cluster_end, drag_end) =
            if super::super::super::CHROME_CONTROLS_LEFT {
                let close = side + radius;
                let minimize = close + step;
                let maximize = minimize + step;
                (
                    close,
                    minimize,
                    maximize,
                    ((maximize + radius + side).max(0) as u32).min(width),
                    width,
                )
            } else {
                let close = (width as i32 - side - radius).max(radius);
                let maximize = (close - step).max(radius);
                let minimize = (maximize - step).max(radius);
                (
                    close,
                    minimize,
                    maximize,
                    0,
                    (maximize - radius - side).max(0) as u32,
                )
            };

        Self {
            close_rect: circle_rect(close_center_x, center_y, radius, width, height),
            minimize_rect: circle_rect(minimize_center_x, center_y, radius, width, height),
            maximize_rect: circle_rect(maximize_center_x, center_y, radius, width, height),
            controls_cluster_end,
            drag_end,
            close_center_x,
            minimize_center_x,
            maximize_center_x,
            center_y,
            radius,
        }
    }

    fn paint_buttons(&self, pixels: &mut [u8], width: u32, height: u32) {
        draw_filled_circle_aa(
            pixels,
            width,
            height,
            self.close_center_x as f32 + 0.5,
            self.center_y as f32 + 0.5,
            self.radius as f32,
            WINDOW_CLOSE_COLOR,
        );
        draw_filled_circle_aa(
            pixels,
            width,
            height,
            self.minimize_center_x as f32 + 0.5,
            self.center_y as f32 + 0.5,
            self.radius as f32,
            WINDOW_MINIMIZE_COLOR,
        );
        draw_filled_circle_aa(
            pixels,
            width,
            height,
            self.maximize_center_x as f32 + 0.5,
            self.center_y as f32 + 0.5,
            self.radius as f32,
            WINDOW_MAXIMIZE_COLOR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::WindowControlsLayout;
    use crate::render::ui::CHROME_CONTROLS_LEFT;

    #[test]
    fn window_controls_follow_platform_order() {
        let layout = WindowControlsLayout::new(200, 32);

        if CHROME_CONTROLS_LEFT {
            assert!(layout.close_center_x < layout.minimize_center_x);
            assert!(layout.minimize_center_x < layout.maximize_center_x);
        } else {
            assert!(layout.minimize_center_x < layout.maximize_center_x);
            assert!(layout.maximize_center_x < layout.close_center_x);
        }
    }
}
