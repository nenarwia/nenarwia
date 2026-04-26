use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

use super::super::font::load_font;
use super::super::raster::{draw_text_line, TextLineParams};
use super::super::text::{measure_text_width, wrap_text};
use super::super::{
    UI_CLOSE_GAP_PX, UI_CLOSE_SIZE_PX, UI_FONT_SIZE, UI_MIN_WIDTH_PX, UI_PADDING_PX,
};

pub(super) struct PreparedNoticeText {
    pub font: FontArc,
    pub scale: PxScale,
    pub ascent: f32,
    pub line_height: f32,
    pub lines: Vec<String>,
    pub text_width: f32,
    pub max_width: u32,
    pub min_box_width: u32,
}

impl PreparedNoticeText {
    pub fn text_height(&self) -> u32 {
        (self.line_height.ceil() as u32) * self.lines.len().max(1) as u32
    }
}

pub(super) fn prepare_notice_text(
    text: &str,
    max_width: u32,
    min_wrap_width: u32,
) -> Option<PreparedNoticeText> {
    let font = load_font()?;
    let scale = PxScale::from(UI_FONT_SIZE);
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();
    let line_height = (scaled.ascent() - scaled.descent() + scaled.line_gap()).max(1.0);

    let max_width = max_width.max(min_wrap_width);
    let wrap_width = max_width
        .saturating_sub(UI_PADDING_PX * 2 + UI_CLOSE_SIZE_PX + UI_CLOSE_GAP_PX)
        .max(min_wrap_width) as f32;
    let lines = wrap_text(text, &font, scale, wrap_width);
    let text_width = lines
        .iter()
        .map(|line| measure_text_width(line, &font, scale))
        .fold(0.0f32, f32::max);

    Some(PreparedNoticeText {
        font,
        scale,
        ascent,
        line_height,
        lines,
        text_width,
        max_width,
        min_box_width: UI_MIN_WIDTH_PX.min(max_width),
    })
}

pub(super) fn draw_notice_lines(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    prepared: &PreparedNoticeText,
) {
    let start_x = UI_PADDING_PX as f32;
    let start_y = UI_PADDING_PX as f32 + prepared.ascent;
    let text_color = [230, 230, 230, 255];

    for (idx, line) in prepared.lines.iter().enumerate() {
        let y = start_y + prepared.line_height * idx as f32;
        draw_text_line(TextLineParams {
            pixels,
            width,
            height,
            font: &prepared.font,
            scale: prepared.scale,
            x: start_x,
            y,
            text: line,
            color: text_color,
        });
    }
}

pub(super) fn draw_close_glyph(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    prepared: &PreparedNoticeText,
    box_width: u32,
) -> [u32; 4] {
    let close_x = box_width.saturating_sub(UI_PADDING_PX + UI_CLOSE_SIZE_PX) as f32;
    let close_y = UI_PADDING_PX as f32;
    let close_rect = [
        close_x as u32,
        close_y as u32,
        UI_CLOSE_SIZE_PX,
        UI_CLOSE_SIZE_PX,
    ];
    let close_center_x = close_x + (UI_CLOSE_SIZE_PX as f32 * 0.2);
    let close_center_y = close_y + prepared.ascent.min(UI_CLOSE_SIZE_PX as f32);
    draw_text_line(TextLineParams {
        pixels,
        width,
        height,
        font: &prepared.font,
        scale: prepared.scale,
        x: close_center_x,
        y: close_center_y,
        text: "X",
        color: [255, 255, 255, 255],
    });

    close_rect
}
