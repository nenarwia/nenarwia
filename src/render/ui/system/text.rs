use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

use crate::core::color::MissingCodecKind;
use crate::render::ui::font::font_for_char;

pub(super) fn build_notice_text(kinds: &[MissingCodecKind]) -> String {
    let mut out = String::new();
    out.push_str("Missing Windows image codecs were detected.\n");
    out.push_str("Some files cannot be decoded.\n\n");
    out.push_str("Install:\n");
    for kind in kinds {
        let (label, hint) = kind_line(*kind);
        out.push_str("- ");
        out.push_str(label);
        out.push_str(": ");
        out.push_str(hint);
        out.push('\n');
    }
    out.push_str("Restart the application after codec installation.\n");
    out.push_str("If codecs are already installed, the file may be damaged.");
    out
}

pub(super) fn wrap_text(text: &str, font: &FontArc, scale: PxScale, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            append_wrapped_word(&mut lines, &mut current, word, font, scale, max_width);
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}

fn append_wrapped_word(
    lines: &mut Vec<String>,
    current: &mut String,
    word: &str,
    font: &FontArc,
    scale: PxScale,
    max_width: f32,
) {
    let candidate = if current.is_empty() {
        word.to_string()
    } else {
        format!("{} {}", current, word)
    };
    if measure_text_width(&candidate, font, scale) <= max_width {
        *current = candidate;
        return;
    }

    if !current.is_empty() {
        lines.push(std::mem::take(current));
    }
    if measure_text_width(word, font, scale) <= max_width {
        current.push_str(word);
        return;
    }

    let mut segment = String::new();
    for ch in word.chars() {
        let candidate = format!("{segment}{ch}");
        if !segment.is_empty() && measure_text_width(&candidate, font, scale) > max_width {
            lines.push(std::mem::take(&mut segment));
        }
        segment.push(ch);
    }
    *current = segment;
}

pub(super) fn measure_text_width(text: &str, font: &FontArc, scale: PxScale) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for ch in text.chars() {
        let (glyph_font, id, font_index) = font_for_char(font, ch);
        let scaled = glyph_font.as_scaled(scale);
        if let Some((prev_font_index, prev_id)) = previous {
            if prev_font_index == font_index {
                width += scaled.kern(prev_id, id);
            }
        }
        width += scaled.h_advance(id);
        previous = Some((font_index, id));
    }
    width
}

fn kind_line(kind: MissingCodecKind) -> (&'static str, &'static str) {
    match kind {
        MissingCodecKind::Heif => (
            "HEIF/HEIC",
            "HEIF Image Extensions (and HEVC Video Extensions if required)",
        ),
        MissingCodecKind::Avif => ("AVIF", "AV1 Video Extension"),
        MissingCodecKind::Raw => ("RAW", "Raw Image Extension"),
        MissingCodecKind::Jpeg2000 => ("JPEG 2000", "JPEG 2000 codec"),
        MissingCodecKind::JpegXr => ("JPEG XR", "JPEG XR codec"),
        MissingCodecKind::Generic => ("Other formats", "matching WIC codec"),
    }
}

#[cfg(test)]
mod tests {
    use super::wrap_text;
    use crate::render::ui::font::load_font;
    use ab_glyph::PxScale;

    #[test]
    fn wrap_text_splits_overlong_words() {
        let font = load_font().expect("embedded UI font should load");

        let lines = wrap_text(&"a".repeat(64), &font, PxScale::from(13.0), 48.0);

        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| !line.is_empty()));
    }
}
