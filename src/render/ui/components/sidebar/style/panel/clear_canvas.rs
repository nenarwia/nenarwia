use ab_glyph::{Font, PxScale, ScaleFont};

use crate::core::process_memory::ProcessRamUsage;
use crate::render::ui::raster::{draw_text_line, TextLineParams};
use crate::render::ui::sidebar::style::constants::{
    SIDEBAR_TEXT_ALPHA_ACTIVE, SIDEBAR_TEXT_ALPHA_IDLE, SIDEBAR_TEXT_RGB,
};
use crate::render::ui::text::measure_text_width;

use super::super::text_utils::fit_to_width;

pub(in crate::render::ui::sidebar::style::panel) fn draw_clear_canvas_item(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [u32; 4],
    font: &ab_glyph::FontArc,
    highlighted: bool,
    clear_canvas_ram_usage: Option<ProcessRamUsage>,
) {
    let label_scale = PxScale::from(15.0);
    let label_scaled = font.as_scaled(label_scale);
    let label_ascent = label_scaled.ascent();
    let label_line_height =
        (label_scaled.ascent() - label_scaled.descent() + label_scaled.line_gap()).max(1.0);
    let alpha = if highlighted {
        SIDEBAR_TEXT_ALPHA_ACTIVE
    } else {
        SIDEBAR_TEXT_ALPHA_IDLE
    };

    let left_x = rect[0] as f32 + 10.0;
    let right_x = rect[0] as f32 + rect[2] as f32 - 10.0;

    if let Some(ram_usage) = clear_canvas_ram_usage {
        let detail_scale = PxScale::from(11.0);
        let detail_scaled = font.as_scaled(detail_scale);
        let detail_ascent = detail_scaled.ascent();
        let detail_line_height =
            (detail_scaled.ascent() - detail_scaled.descent() + detail_scaled.line_gap()).max(1.0);
        let gap = 2.0;
        let block_height = label_line_height + gap + detail_line_height;
        let block_top = rect[1] as f32 + ((rect[3] as f32 - block_height) * 0.5).max(0.0);
        let label_baseline = block_top + label_ascent;
        let detail_baseline = block_top + label_line_height + gap + detail_ascent;

        let percent = format!("{}%", ram_usage.usage_percent());
        let percent_width = measure_text_width(&percent, font, label_scale);
        let percent_x = (right_x - percent_width).max(left_x);
        let label_max_width = (percent_x - left_x - 8.0).max(0.0);
        let label = fit_to_width("Clear Canvas", font, label_scale, label_max_width);
        let detail = format!(
            "{} / {}",
            format_ram_gib(ram_usage.working_set_bytes),
            format_ram_gib(ram_usage.total_physical_bytes),
        );
        let detail = fit_to_width(&detail, font, detail_scale, rect[2] as f32 - 20.0);

        draw_text_line(TextLineParams {
            pixels,
            width,
            height,
            font,
            scale: label_scale,
            x: left_x,
            y: label_baseline,
            text: &label,
            color: [
                SIDEBAR_TEXT_RGB[0],
                SIDEBAR_TEXT_RGB[1],
                SIDEBAR_TEXT_RGB[2],
                alpha,
            ],
        });
        draw_text_line(TextLineParams {
            pixels,
            width,
            height,
            font,
            scale: label_scale,
            x: percent_x,
            y: label_baseline,
            text: &percent,
            color: [
                SIDEBAR_TEXT_RGB[0],
                SIDEBAR_TEXT_RGB[1],
                SIDEBAR_TEXT_RGB[2],
                SIDEBAR_TEXT_ALPHA_ACTIVE,
            ],
        });
        draw_text_line(TextLineParams {
            pixels,
            width,
            height,
            font,
            scale: detail_scale,
            x: left_x,
            y: detail_baseline,
            text: &detail,
            color: [
                SIDEBAR_TEXT_RGB[0],
                SIDEBAR_TEXT_RGB[1],
                SIDEBAR_TEXT_RGB[2],
                108,
            ],
        });
    } else {
        let baseline =
            rect[1] as f32 + ((rect[3] as f32 - label_line_height) * 0.5).max(0.0) + label_ascent;
        let text = fit_to_width("Clear Canvas", font, label_scale, rect[2] as f32 - 20.0);
        draw_text_line(TextLineParams {
            pixels,
            width,
            height,
            font,
            scale: label_scale,
            x: left_x,
            y: baseline,
            text: &text,
            color: [
                SIDEBAR_TEXT_RGB[0],
                SIDEBAR_TEXT_RGB[1],
                SIDEBAR_TEXT_RGB[2],
                alpha,
            ],
        });
    }
}

pub(in crate::render::ui::sidebar::style::panel) fn format_ram_gib(bytes: u64) -> String {
    let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gib >= 10.0 {
        format!("{gib:.0} GB")
    } else {
        format!("{gib:.1} GB")
    }
}

#[cfg(test)]
mod tests {
    use super::format_ram_gib;
    use crate::core::process_memory::ProcessRamUsage;

    #[test]
    fn format_ram_gib_uses_compact_decimal_for_small_values() {
        let bytes = (1.5f64 * 1024.0 * 1024.0 * 1024.0) as u64;

        assert_eq!(format_ram_gib(bytes), "1.5 GB");
    }

    #[test]
    fn process_ram_percent_is_available_for_clear_canvas_badge() {
        let usage = ProcessRamUsage {
            working_set_bytes: 2 * 1024 * 1024 * 1024,
            total_physical_bytes: 8 * 1024 * 1024 * 1024,
        };

        assert_eq!(usage.usage_percent(), 25);
    }
}
