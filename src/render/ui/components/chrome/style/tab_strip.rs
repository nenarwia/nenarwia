use ab_glyph::{FontArc, PxScale};

use super::super::super::font::load_font;
use super::super::super::text::measure_text_width;
use super::super::state::ChromeTabView;
use super::constants::{
    ADD_TAB_SIZE_PX, MIN_DRAG_WIDTH_PX, TAB_GAP_PX, TAB_HEIGHT_PX, TAB_MAX_WIDTH_PX,
    TAB_MIN_VISIBLE_WIDTH_PX, TAB_MIN_WIDTH_PX, TAB_TEXT_SIDE_PADDING_PX, TAB_TEXT_SIZE,
    TAB_TOP_PX,
};
use super::geometry::compute_tab_close_rect;
use super::tab_paint::{draw_add_tab_button, draw_tab};

pub(super) struct TabStripBuildResult {
    pub(super) tab_indices: Vec<usize>,
    pub(super) tab_rects: Vec<[u32; 4]>,
    pub(super) tab_close_rects: Vec<Option<[u32; 4]>>,
    pub(super) add_tab_rect: Option<[u32; 4]>,
    pub(super) drag_rect: [u32; 4],
}

impl TabStripBuildResult {
    fn empty(drag_rect: [u32; 4]) -> Self {
        Self {
            tab_indices: Vec::new(),
            tab_rects: Vec::new(),
            tab_close_rects: Vec::new(),
            add_tab_rect: None,
            drag_rect,
        }
    }
}

pub(super) fn build_tab_strip(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tabs: &[ChromeTabView],
    active_tab: usize,
    hovered_tab: Option<usize>,
    hovered_close_tab: Option<usize>,
    hovered_add_tab: bool,
    controls_cluster_end: u32,
    drag_end: u32,
) -> TabStripBuildResult {
    if width == 0 || height == 0 {
        return TabStripBuildResult::empty([0, 0, 0, 0]);
    }

    let burger_right = super::super::super::BURGER_BTN_LEFT_PX
        .saturating_add(super::super::super::BURGER_BTN_SIZE_PX)
        .saturating_add(12)
        .min(width);
    let tab_start_x = if super::super::super::CHROME_CONTROLS_LEFT {
        burger_right.max(controls_cluster_end.saturating_add(10))
    } else {
        burger_right
    };
    let strip_limit_x = if super::super::super::CHROME_CONTROLS_LEFT {
        width.saturating_sub(MIN_DRAG_WIDTH_PX)
    } else {
        drag_end.saturating_sub(MIN_DRAG_WIDTH_PX)
    };
    if strip_limit_x <= tab_start_x.saturating_add(48) {
        let drag_start = if super::super::super::CHROME_CONTROLS_LEFT {
            controls_cluster_end
        } else {
            0
        };
        return TabStripBuildResult::empty(build_drag_rect(width, height, drag_start, drag_end));
    }
    if tabs.is_empty() {
        let drag_start = if super::super::super::CHROME_CONTROLS_LEFT {
            controls_cluster_end.max(tab_start_x)
        } else {
            tab_start_x.min(drag_end)
        };
        return TabStripBuildResult::empty(build_drag_rect(width, height, drag_start, drag_end));
    }

    let font = load_font();
    let scale = PxScale::from(TAB_TEXT_SIZE);
    let available_strip = strip_limit_x.saturating_sub(tab_start_x);
    let add_slot_width = ADD_TAB_SIZE_PX.saturating_add(TAB_GAP_PX);
    let show_add_button =
        available_strip >= TAB_MIN_VISIBLE_WIDTH_PX.saturating_add(add_slot_width + 12);
    let tab_space = if show_add_button {
        available_strip.saturating_sub(add_slot_width)
    } else {
        available_strip
    };
    if tab_space == 0 {
        let drag_start = if super::super::super::CHROME_CONTROLS_LEFT {
            controls_cluster_end.max(tab_start_x)
        } else {
            tab_start_x.min(drag_end)
        };
        return TabStripBuildResult::empty(build_drag_rect(width, height, drag_start, drag_end));
    }

    let widths: Vec<u32> = tabs
        .iter()
        .map(|tab| measure_tab_width(tab, font.as_ref(), scale, tab_space))
        .collect();
    let active_index = active_tab.min(tabs.len().saturating_sub(1));
    let (visible_start, visible_end) = select_visible_tab_range(&widths, active_index, tab_space);

    let visible_count = visible_end.saturating_sub(visible_start);
    let mut tab_indices = Vec::with_capacity(visible_count);
    let mut tab_rects = Vec::with_capacity(visible_count);
    let mut tab_close_rects = Vec::with_capacity(visible_count);
    let mut cursor_x = tab_start_x;
    for idx in visible_start..visible_end {
        let rect = [cursor_x, TAB_TOP_PX, widths[idx], TAB_HEIGHT_PX.min(height)];
        let show_close = tabs.len() > 1
            && (idx == active_index || hovered_tab == Some(idx) || hovered_close_tab == Some(idx));
        let close_rect = show_close.then(|| compute_tab_close_rect(rect));
        draw_tab(
            pixels,
            width,
            height,
            rect,
            &tabs[idx].title,
            font.as_ref(),
            scale,
            idx == active_index,
            hovered_tab == Some(idx),
            close_rect,
            hovered_close_tab == Some(idx),
            idx > visible_start,
        );
        tab_indices.push(idx);
        tab_rects.push(rect);
        tab_close_rects.push(close_rect);
        cursor_x = cursor_x.saturating_add(widths[idx]);
        if idx + 1 != visible_end {
            cursor_x = cursor_x.saturating_add(TAB_GAP_PX);
        }
    }

    let content_end = tab_rects
        .last()
        .map(|rect| rect[0].saturating_add(rect[2]))
        .unwrap_or(tab_start_x);
    let mut drag_start = content_end.saturating_add(TAB_GAP_PX);
    let add_tab_rect = if show_add_button {
        let add_x = content_end.saturating_add(TAB_GAP_PX);
        if add_x.saturating_add(ADD_TAB_SIZE_PX) <= strip_limit_x {
            let add_y = ((height.saturating_sub(ADD_TAB_SIZE_PX)) / 2).min(height);
            let rect = [add_x, add_y, ADD_TAB_SIZE_PX, ADD_TAB_SIZE_PX.min(height)];
            draw_add_tab_button(pixels, width, height, rect, font.as_ref(), hovered_add_tab);
            drag_start = rect[0].saturating_add(rect[2]).saturating_add(TAB_GAP_PX);
            Some(rect)
        } else {
            None
        }
    } else {
        None
    };

    if super::super::super::CHROME_CONTROLS_LEFT {
        drag_start = drag_start.max(controls_cluster_end);
    }

    TabStripBuildResult {
        tab_indices,
        tab_rects,
        tab_close_rects,
        add_tab_rect,
        drag_rect: build_drag_rect(width, height, drag_start, drag_end),
    }
}

fn build_drag_rect(width: u32, height: u32, drag_start: u32, drag_end: u32) -> [u32; 4] {
    let drag_origin = if super::super::super::CHROME_CONTROLS_LEFT {
        drag_start
    } else {
        drag_start.min(drag_end)
    };
    let drag_width = if super::super::super::CHROME_CONTROLS_LEFT {
        width.saturating_sub(drag_origin)
    } else {
        drag_end.saturating_sub(drag_origin)
    };
    [drag_origin, 0, drag_width, height]
}

fn measure_tab_width(
    tab: &ChromeTabView,
    font: Option<&FontArc>,
    scale: PxScale,
    max_width: u32,
) -> u32 {
    let title = if tab.title.trim().is_empty() {
        "Untitled"
    } else {
        tab.title.trim()
    };
    let desired = font
        .map(|font| {
            (measure_text_width(title, font, scale).ceil() as u32)
                .saturating_add(TAB_TEXT_SIDE_PADDING_PX * 2)
                .clamp(TAB_MIN_WIDTH_PX, TAB_MAX_WIDTH_PX)
        })
        .unwrap_or(112);
    let cap = max_width.max(1);
    desired.min(cap).max(TAB_MIN_VISIBLE_WIDTH_PX.min(cap))
}

fn select_visible_tab_range(widths: &[u32], active_index: usize, max_space: u32) -> (usize, usize) {
    if widths.is_empty() || max_space == 0 {
        return (0, 0);
    }

    let active = active_index.min(widths.len().saturating_sub(1));
    let mut start = active;
    let mut end = active.saturating_add(1);
    let mut used = widths[active].min(max_space);
    let mut prefer_left = true;

    loop {
        let expanded = if prefer_left {
            try_expand_left(widths, &mut start, &mut used, max_space)
                || try_expand_right(widths, &mut end, &mut used, max_space)
        } else {
            try_expand_right(widths, &mut end, &mut used, max_space)
                || try_expand_left(widths, &mut start, &mut used, max_space)
        };
        if !expanded {
            break;
        }
        prefer_left = !prefer_left;
    }

    (start, end)
}

fn try_expand_left(widths: &[u32], start: &mut usize, used: &mut u32, max_space: u32) -> bool {
    if *start == 0 {
        return false;
    }
    let candidate = used
        .saturating_add(TAB_GAP_PX)
        .saturating_add(widths[*start - 1]);
    if candidate > max_space {
        return false;
    }
    *start = start.saturating_sub(1);
    *used = candidate;
    true
}

fn try_expand_right(widths: &[u32], end: &mut usize, used: &mut u32, max_space: u32) -> bool {
    if *end >= widths.len() {
        return false;
    }
    let candidate = used.saturating_add(TAB_GAP_PX).saturating_add(widths[*end]);
    if candidate > max_space {
        return false;
    }
    *end = end.saturating_add(1);
    *used = candidate;
    true
}
