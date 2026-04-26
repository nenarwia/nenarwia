use ab_glyph::{FontArc, PxScale};

use super::constants::{
    TAB_ACTIVE_BORDER, TAB_ACTIVE_FILL, TAB_ACTIVE_GLOSS, TAB_ADD_BORDER, TAB_ADD_FILL,
    TAB_ADD_HOVER_BORDER, TAB_ADD_HOVER_FILL, TAB_ADD_TEXT, TAB_CLOSE_FILL, TAB_CLOSE_HOVER_FILL,
    TAB_CLOSE_HOVER_TEXT, TAB_CLOSE_TEXT, TAB_CLOSE_TEXT_SIZE, TAB_DIVIDER_COLOR, TAB_HOVER_FILL,
    TAB_INACTIVE_FILL, TAB_INACTIVE_TEXT, TAB_RADIUS_PX, TAB_TEXT_COLOR, TAB_TEXT_SIDE_PADDING_PX,
};
use super::geometry::{inset_rect, tab_title_rect};
use super::primitives::fill_rounded_rect_region_aa;
use super::text_utils::{draw_label_centered, fit_to_width};

pub(super) fn draw_tab(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [u32; 4],
    title: &str,
    font: Option<&FontArc>,
    scale: PxScale,
    is_active: bool,
    is_hovered: bool,
    close_rect: Option<[u32; 4]>,
    is_close_hovered: bool,
    show_left_divider: bool,
) {
    if is_active {
        draw_active_tab(
            pixels,
            width,
            height,
            rect,
            title,
            font,
            scale,
            close_rect,
            is_close_hovered,
        );
        return;
    }
    if rect[2] == 0 || rect[3] == 0 {
        return;
    }

    let fill = if is_hovered {
        TAB_HOVER_FILL
    } else {
        TAB_INACTIVE_FILL
    };
    fill_rounded_rect_region_aa(pixels, width, height, rect, TAB_RADIUS_PX, fill);
    if show_left_divider {
        draw_vertical_divider(
            pixels,
            width,
            height,
            rect[0],
            rect[1].saturating_add(5),
            rect[3].saturating_sub(10),
            TAB_DIVIDER_COLOR,
        );
    }

    if let Some(font) = font {
        let title_rect = tab_title_rect(rect, close_rect);
        let max_text_width = title_rect[2].saturating_sub(TAB_TEXT_SIDE_PADDING_PX * 2) as f32;
        let text = fit_to_width(title, font, scale, max_text_width);
        draw_label_centered(
            pixels,
            width,
            height,
            title_rect,
            &text,
            font,
            scale,
            if is_hovered {
                TAB_TEXT_COLOR
            } else {
                TAB_INACTIVE_TEXT
            },
        );
    }

    if let Some(close_rect) = close_rect {
        draw_tab_close_button(pixels, width, height, close_rect, font, is_close_hovered);
    }
}

pub(super) fn draw_add_tab_button(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [u32; 4],
    font: Option<&FontArc>,
    hovered: bool,
) {
    fill_rounded_rect_region_aa(
        pixels,
        width,
        height,
        rect,
        5.0,
        if hovered {
            TAB_ADD_HOVER_BORDER
        } else {
            TAB_ADD_BORDER
        },
    );
    let inner_rect = inset_rect(rect, 1, 1);
    fill_rounded_rect_region_aa(
        pixels,
        width,
        height,
        inner_rect,
        4.0,
        if hovered {
            TAB_ADD_HOVER_FILL
        } else {
            TAB_ADD_FILL
        },
    );

    if let Some(font) = font {
        draw_label_centered(
            pixels,
            width,
            height,
            rect,
            "+",
            font,
            PxScale::from(12.0),
            TAB_ADD_TEXT,
        );
    }
}

fn draw_active_tab(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [u32; 4],
    title: &str,
    font: Option<&FontArc>,
    scale: PxScale,
    close_rect: Option<[u32; 4]>,
    is_close_hovered: bool,
) {
    if rect[2] == 0 || rect[3] == 0 {
        return;
    }

    let fill = TAB_ACTIVE_FILL;
    let gloss = TAB_ACTIVE_GLOSS;
    let border = TAB_ACTIVE_BORDER;

    fill_rounded_rect_region_aa(pixels, width, height, rect, TAB_RADIUS_PX, fill);

    let gloss_rect = [rect[0], rect[1], rect[2], (rect[3] / 2).max(1)];
    fill_rounded_rect_region_aa(pixels, width, height, gloss_rect, TAB_RADIUS_PX, gloss);

    let left_border = [rect[0], rect[1], 1, rect[3]];
    let right_border_x = rect[0].saturating_add(rect[2]).saturating_sub(1);
    let right_border = [right_border_x, rect[1], 1, rect[3]];
    let bottom_border_y = rect[1].saturating_add(rect[3]).saturating_sub(1);
    let bottom_border = [rect[0], bottom_border_y, rect[2], 1];
    fill_rounded_rect_region_aa(pixels, width, height, left_border, 0.0, border);
    fill_rounded_rect_region_aa(pixels, width, height, right_border, 0.0, border);
    fill_rounded_rect_region_aa(pixels, width, height, bottom_border, 0.0, border);

    if let Some(font) = font {
        let title_rect = tab_title_rect(rect, close_rect);
        let max_text_width = title_rect[2].saturating_sub(TAB_TEXT_SIDE_PADDING_PX * 2) as f32;
        let text = fit_to_width(title, font, scale, max_text_width);
        draw_label_centered(
            pixels,
            width,
            height,
            title_rect,
            &text,
            font,
            scale,
            TAB_TEXT_COLOR,
        );
    }

    if let Some(close_rect) = close_rect {
        draw_tab_close_button(pixels, width, height, close_rect, font, is_close_hovered);
    }
}

fn draw_tab_close_button(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [u32; 4],
    font: Option<&FontArc>,
    is_hovered: bool,
) {
    fill_rounded_rect_region_aa(
        pixels,
        width,
        height,
        rect,
        4.0,
        if is_hovered {
            TAB_CLOSE_HOVER_FILL
        } else {
            TAB_CLOSE_FILL
        },
    );
    if let Some(font) = font {
        draw_label_centered(
            pixels,
            width,
            height,
            rect,
            "x",
            font,
            PxScale::from(TAB_CLOSE_TEXT_SIZE),
            if is_hovered {
                TAB_CLOSE_HOVER_TEXT
            } else {
                TAB_CLOSE_TEXT
            },
        );
    }
}

fn draw_vertical_divider(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    divider_height: u32,
    color: [u8; 4],
) {
    if divider_height == 0 {
        return;
    }
    fill_rounded_rect_region_aa(pixels, width, height, [x, y, 1, divider_height], 0.0, color);
}
