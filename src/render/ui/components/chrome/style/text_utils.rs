use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

use super::super::super::raster::{draw_text_line, TextLineParams};
use super::super::super::text::measure_text_width;

pub(super) fn draw_label_centered(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [u32; 4],
    text: &str,
    font: &FontArc,
    scale: PxScale,
    color: [u8; 4],
) {
    if rect[2] == 0 || rect[3] == 0 || text.is_empty() {
        return;
    }

    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();
    let line_height = (scaled.ascent() - scaled.descent() + scaled.line_gap()).max(1.0);
    let text_width = measure_text_width(text, font, scale);
    let x = rect[0] as f32 + ((rect[2] as f32 - text_width) * 0.5).max(0.0);
    let baseline = rect[1] as f32 + ((rect[3] as f32 - line_height) * 0.5).max(0.0) + ascent;
    draw_text_line(TextLineParams {
        pixels,
        width,
        height,
        font,
        scale,
        x,
        y: baseline,
        text,
        color,
    });
}

pub(super) fn fit_to_width(text: &str, font: &FontArc, scale: PxScale, max_width: f32) -> String {
    if max_width <= 8.0 {
        return String::new();
    }
    if measure_text_width(text, font, scale) <= max_width {
        return text.to_string();
    }

    let mut chars: Vec<char> = text.chars().collect();
    let ellipsis = "...";
    while !chars.is_empty() {
        chars.pop();
        let mut candidate = chars.iter().collect::<String>();
        candidate.push_str(ellipsis);
        if measure_text_width(&candidate, font, scale) <= max_width {
            return candidate;
        }
    }
    ellipsis.to_string()
}
