use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};

use super::font::font_for_char;

pub(super) struct TextLineParams<'a> {
    pub pixels: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    pub font: &'a FontArc,
    pub scale: PxScale,
    pub x: f32,
    pub y: f32,
    pub text: &'a str,
    pub color: [u8; 4],
}

pub(super) fn draw_text_line(params: TextLineParams<'_>) {
    draw_text_line_inner(params, None);
}

fn draw_text_line_inner(params: TextLineParams<'_>, clip_rect: Option<[u32; 4]>) {
    let TextLineParams {
        pixels,
        width,
        height,
        font,
        scale,
        x,
        y,
        text,
        color,
    } = params;
    let (clip_x0, clip_y0, clip_x1, clip_y1) = if let Some(rect) = clip_rect {
        (
            rect[0].min(width),
            rect[1].min(height),
            rect[0].saturating_add(rect[2]).min(width),
            rect[1].saturating_add(rect[3]).min(height),
        )
    } else {
        (0, 0, width, height)
    };
    if clip_x1 <= clip_x0 || clip_y1 <= clip_y0 {
        return;
    }

    let mut x = x;
    let mut previous = None;
    for ch in text.chars() {
        let (glyph_font, id, font_index) = font_for_char(font, ch);
        let scaled = glyph_font.as_scaled(scale);
        if let Some((prev_font_index, prev_id)) = previous {
            if prev_font_index == font_index {
                x += scaled.kern(prev_id, id);
            }
        }
        let glyph = id.with_scale_and_position(scale, point(x, y));
        if let Some(outlined) = glyph_font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let px = bounds.min.x + gx as f32;
                let py = bounds.min.y + gy as f32;
                if px < 0.0 || py < 0.0 {
                    return;
                }
                let ix = px as u32;
                let iy = py as u32;
                if ix >= width || iy >= height {
                    return;
                }
                if ix < clip_x0 || ix >= clip_x1 || iy < clip_y0 || iy >= clip_y1 {
                    return;
                }
                let idx = ((iy * width + ix) * 4) as usize;
                blend_pixel(&mut pixels[idx..idx + 4], color, coverage);
            });
        }
        x += scaled.h_advance(id);
        previous = Some((font_index, id));
    }
}

fn blend_pixel(dst: &mut [u8], src: [u8; 4], coverage: f32) {
    let src_a = (src[3] as f32 / 255.0) * coverage.clamp(0.0, 1.0);
    if src_a <= 0.0 {
        return;
    }
    let dst_a = dst[3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0 {
        dst[0] = 0;
        dst[1] = 0;
        dst[2] = 0;
        dst[3] = 0;
        return;
    }
    for i in 0..3 {
        let src_c = src[i] as f32 / 255.0;
        let dst_c = dst[i] as f32 / 255.0;
        let out_c = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
        dst[i] = (out_c * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}
