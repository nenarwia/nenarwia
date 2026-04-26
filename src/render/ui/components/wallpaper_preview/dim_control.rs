use ab_glyph::{Font, PxScale, ScaleFont};

use super::super::font::load_font;
use super::super::raster::draw_text_line;
use super::pixel_ops::{draw_circle_aa, fill_rect_region};
use super::style::{DIM_RECT_LOCAL, DIM_TRACK_LEFT_PAD_PX, DIM_TRACK_RIGHT_PAD_PX};

pub(super) fn compose_dim_control_pixels(dim_amount: f32) -> Vec<u8> {
    let width = DIM_RECT_LOCAL[2];
    let height = DIM_RECT_LOCAL[3];
    let mut pixels = vec![0u8; width as usize * height as usize * 4];

    fill_rect_region(
        &mut pixels,
        width,
        height,
        [0, 0, width, height],
        [66, 66, 66, 210],
    );

    let track_x0 = DIM_TRACK_LEFT_PAD_PX.min(width);
    let track_x1 = width.saturating_sub(DIM_TRACK_RIGHT_PAD_PX);
    if track_x1 > track_x0 {
        let track_w = track_x1 - track_x0;
        let track_h = 4u32;
        let track_y = (height.saturating_sub(track_h)) / 2;
        fill_rect_region(
            &mut pixels,
            width,
            height,
            [track_x0, track_y, track_w, track_h],
            [255, 255, 255, 26],
        );

        let t = dim_amount.clamp(0.0, 1.0);
        let knob_x = track_x0 as f32 + (track_w as f32 * t);
        let knob_y = track_y as f32 + (track_h as f32 * 0.5);
        let fill_w = ((knob_x - track_x0 as f32).round() as u32).min(track_w);
        if fill_w > 0 {
            fill_rect_region(
                &mut pixels,
                width,
                height,
                [track_x0, track_y, fill_w, track_h],
                [255, 255, 255, 70],
            );
        }
        draw_circle_aa(
            &mut pixels,
            width,
            height,
            knob_x,
            knob_y,
            7.0,
            [255, 255, 255, 230],
        );
    }

    if let Some(font) = load_font() {
        let item_scale = PxScale::from(14.0);
        let item_scaled = font.as_scaled(item_scale);
        let item_ascent = item_scaled.ascent();
        let dim_pct = (dim_amount.clamp(0.0, 1.0) * 100.0).round() as u32;
        let dim_label = format!("Dimming: {}%", dim_pct);
        draw_text_line(super::super::raster::TextLineParams {
            pixels: &mut pixels,
            width,
            height,
            font: &font,
            scale: item_scale,
            x: 10.0,
            y: ((height as f32 - 14.0) * 0.5) + item_ascent,
            text: &dim_label,
            color: [255, 255, 255, 235],
        });
    }

    pixels
}
