use crate::core::color::MissingCodecKind;

use super::super::text::build_notice_text;
use super::super::{UI_CLOSE_GAP_PX, UI_CLOSE_SIZE_PX, UI_PADDING_PX};
use super::paint::{fill_rounded_rect_region_aa, NOTICE_BG_COLOR, NOTICE_RADIUS_PX};
use super::text_layout::{draw_close_glyph, draw_notice_lines, prepare_notice_text};
use super::NoticeTexture;

pub(super) fn build_notice_texture(
    kinds: &[MissingCodecKind],
    max_width: u32,
) -> Option<NoticeTexture> {
    let text = build_notice_text(kinds);
    build_text_notice_texture(&text, max_width)
}

fn build_text_notice_texture(text: &str, max_width: u32) -> Option<NoticeTexture> {
    let prepared = prepare_notice_text(text, max_width, 120)?;
    let box_width = (prepared.text_width.ceil() as u32)
        .saturating_add(UI_PADDING_PX * 2 + UI_CLOSE_SIZE_PX + UI_CLOSE_GAP_PX)
        .clamp(prepared.min_box_width, prepared.max_width);
    let box_height = UI_PADDING_PX * 2 + prepared.text_height();

    let mut pixels = vec![0u8; (box_width as usize) * (box_height as usize) * 4];
    fill_rounded_rect_region_aa(
        &mut pixels,
        box_width,
        box_height,
        [0, 0, box_width, box_height],
        NOTICE_RADIUS_PX,
        NOTICE_BG_COLOR,
    );
    draw_notice_lines(&mut pixels, box_width, box_height, &prepared);
    let close_rect = draw_close_glyph(&mut pixels, box_width, box_height, &prepared, box_width);

    Some(NoticeTexture {
        pixels,
        width: box_width,
        height: box_height,
        close_rect,
    })
}

#[cfg(test)]
mod tests {
    use crate::core::color::MissingCodecKind;

    use super::build_notice_texture;

    #[test]
    fn build_notice_texture_returns_notice_with_close_button() {
        let notice = build_notice_texture(&[MissingCodecKind::Generic], 260)
            .expect("embedded UI font should build notice texture");

        assert!(rect_inside_texture(
            notice.close_rect,
            notice.width,
            notice.height
        ));
    }

    fn rect_inside_texture(rect: [u32; 4], width: u32, height: u32) -> bool {
        rect[0].saturating_add(rect[2]) <= width && rect[1].saturating_add(rect[3]) <= height
    }
}
