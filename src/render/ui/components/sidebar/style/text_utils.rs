use ab_glyph::{FontArc, PxScale};

use super::super::super::text::measure_text_width;

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
